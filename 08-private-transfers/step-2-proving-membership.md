**~8 分钟**

# 第 2 步: 证明提款资格

## 目标

将资金池改为存储 Merkle 树根节点. 我们不需要把整棵 Merkle 树存储在链上, 它可以在链下维护. 这也是我们在存款事件中附带 commitment 的原因, 方便索引器维护完整的 Merkle 树. 因此, 链上只需追踪当前在树中的位置和 Merkle 根节点即可.

---

## 更新程序

文件: `anchor/programs/private_transfers/src/lib.rs`

---

## 第一部分: 定义树结构

首先, 我们需要定义 Merkle 树属性的常量.

### 1. 在顶部添加常量

```rust
pub const TREE_DEPTH: usize = 10;
pub const MAX_LEAVES: u64 = 1 << TREE_DEPTH;  // 1024
pub const ROOT_HISTORY_SIZE: usize = 10;

// uncomment empty root and explain
pub const MIN_DEPOSIT_AMOUNT: u64 = 1_000_000;
```

**各常量的含义:**

- `TREE_DEPTH`: 我们的 Merkle 树有 10 层. 每层叶节点数翻倍, 因此深度为 10 时, 可容纳 2^10 = 1024 个 commitment 叶节点位置.
- `MAX_LEAVES`: 资金池可容纳的最大存款数. `1 << 10` 是位移运算, 计算结果为 2^10 = 1024. 树满后不再接受新的存款.
- `ROOT_HISTORY_SIZE`: 我们在环形缓冲区中存储最近 10 个 Merkle 根节点. 原因如下: 每次存款后根节点都会更新. 如果只存储当前根节点, 那么基于旧根节点生成证明的用户在提款时就会失败. 保留最近根节点的历史记录, 为用户在证明失效前提供一定的时间窗口.
- `EMPTY_ROOT`: 这是空树(所有位置填充零值, 逐层哈希到根节点)对应的 Merkle root. 我们预先计算这个值, 因为需要用一个有效的初始根节点来初始化资金池. 这个特定的 32 字节值是深度为 10 的空树的 Poseidon 哈希结果.

它们是对零值的哈希, 这就是 Poseidon 哈希的样子, 一个 32 字节的根节点.

---

## 第二部分: 在链上存储树状态

现在更新 Pool 结构体, 用于追踪树中的当前位置并存储近期根节点.

### 2. 更新 Pool 结构体

找到:

替换为:

```rust
#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub authority: Pubkey,
    pub total_deposits: u64,
    // 下一个 commitment 将被插入的叶节点位置(0, 1, 2, ...)
    // Merkle 树是只追加的, 新 commitment 始终插入下一个可用的叶节点位置, 从不覆盖已有数据.
    // 我们需要 `next_leaf_index` 来确定每笔存款的精确插入位置(叶 0, 叶 1, 叶 2……).
    pub next_leaf_index: u64,
    // 指向环形缓冲区中最新根节点所在的槽位
    pub current_root_index: u64,
    // 存储最近 10 个 Merkle 根节点的环形缓冲区
    pub roots: [[u8; 32]; ROOT_HISTORY_SIZE], // 10 个 Poseidon 哈希的数组
}


```

### 3. 初始化 Pool 字段

创建资金池时, 设置初始树状态.

替换为:

```rust
        pool.total_deposits = 0;
        pool.next_leaf_index = 0;
        pool.current_root_index = 0;
        pool.roots[0] = EMPTY_ROOT;
        // Step 3: Initialize nullifier_set.pool

        msg!("Pool initialized");
```

---

 我们已经在链上建立了 Merkle 树的存储结构, Pool 现在会追踪下一个叶节点位置并存储近期根节点. 但存款和提款函数还没有使用这些数据. 接下来我们来更新它们:


## 第三部分: 更新存款函数以追踪叶节点和根节点

每次存款需要:
1. 接收客户端插入 commitment 后在链下计算的新 Merkle 根节点
2. 将该根节点存入历史记录
3. 触发叶节点索引事件, 方便客户端构建 Merkle proof(严格来说, 如果每次存款事件都附带 commitment, 我们不一定需要这个, 但这是良好实践, 方便查找特定 commitment 或处理遗漏的存款事件)

### 4. 更新存款函数签名


替换为:

```rust
    pub fn deposit(
        ctx: Context<Deposit>,
        commitment: [u8; 32],
        // Merkle 树由客户端在链下维护.
        // 将 commitment 作为叶节点插入后, 客户端计算新的根节点并在此传入.
        // 程序只负责存储它.
        new_root: [u8; 32],
        amount: u64,
    ) -> Result<()> {
```

### 5. 转账后更新根节点历史

SOL 转账成功后, 更新树状态并触发事件.


替换为:

```rust
        system_program::transfer(cpi_context, amount)?;

        // 记录此 commitment 被插入的叶节点位置
        let leaf_index = pool.next_leaf_index;

        // 使用取模运算计算环形缓冲区的下一个槽位.
        // 取模(%)使其循环:
        // % 10 表示"除以 10 取余数". 当计数器到达 10 时, 余数为 0, 回到起点.
        // 这样我们可以使用固定大小的存储空间, 而不是无限增长. 最旧的根节点会被覆盖.

        let new_root_index = ((pool.current_root_index + 1) % ROOT_HISTORY_SIZE as u64) as usize;

        // 将新的 Merkle 根节点存入此槽位(覆盖最旧的根节点)
        pool.roots[new_root_index] = new_root;

        // 更新指针, 追踪哪个槽位存储的是当前根节点
        pool.current_root_index = new_root_index as u64;

        emit!(DepositEvent {
            commitment,
            leaf_index,      // 客户端可用此值追踪 commitment 在树中的位置
            timestamp: Clock::get()?.unix_timestamp,
            new_root,        // 客户端可用此值更新本地树副本
        });

        // 移动到下一个叶节点位置, 供下次存款使用
        pool.next_leaf_index += 1;
        pool.total_deposits += 1;

        msg!("Deposit: {} lamports at leaf index {}", amount, leaf_index);
```

### 6. 添加树已满检查

1024 个叶节点位置全部占满后, 不再接受新的存款.

替换为:

```rust
        require!(
            amount >= MIN_DEPOSIT_AMOUNT,
            PrivateTransfersError::DepositTooSmall
        );

        require!(
            pool.next_leaf_index < MAX_LEAVES,
            PrivateTransfersError::TreeFull
        );

        let cpi_context = CpiContext::new(
```


**存款小结:** 我们已将 `deposit` 更新为:

- 接收客户端传来的 `new_root`
- 将该根节点存入环形缓冲区(满时覆盖最旧的)
- 触发 `leaf_index` 事件, 让客户端知道 commitment 在树中的位置
- 接受新存款前检查树是否已满

---

## 第四部分: 更新提款函数以验证根节点

提款必须证明 commitment 存在于树中. 这里我们先验证提供的根节点是否在历史记录中出现过(实际的 ZK 证明验证将在第 5 步实现).

### 8. 更新提款函数签名

替换为:

```rust
    pub fn withdraw(
        ctx: Context<Withdraw>,
        // Step 5: Add proof: Vec<u8>
        // Step 3: Add nullifier_hash: [u8; 32]
        root: [u8; 32],
        recipient: Pubkey,
        amount: u64,
    ) -> Result<()> {
```

### 客户端如何计算根节点

**客户端在本地维护自己的 Merkle 树副本:**

1. **索引存款事件** - 客户端监听 `DepositEvent` 事件. 每个事件包含 `commitment`, `leaf_index` 和 `new_root`. 通过重放所有事件, 客户端在本地重建完整的树.
2. **计算当前根节点** - 拥有完整的树后, 客户端从自己 commitment 所在的叶节点位置向上哈希, 得到当前根节点.
3. **生成 Merkle proof** - 证明由从叶节点到根节点路径上的兄弟哈希组成. 深度为 10 的树需要 10 个哈希值.

### 9. 在提款函数中添加根节点验证

如果根节点不在近期历史中, 拒绝提款请求.

替换为:

```rust
    ) -> Result<()> {
        // Step 3: Check nullifier not used

        require!(
            ctx.accounts.pool.is_known_root(&root),
            PrivateTransfersError::InvalidRoot
        );
```

在 `pub struct Pool { ... }` 之后添加:

```rust
impl Pool {
    // 检查根节点是否存在于历史记录中, 提款时用于验证
    // 用户的证明是否基于某个有效的近期根节点
    pub fn is_known_root(&self, root: &[u8; 32]) -> bool {
        self.roots.iter().any(|r| r == root)
    }
}
```

在 Rust 中, `struct` 定义类型持有的数据, `impl` 定义该类型拥有的方法(函数).

- `&self` - 获取 Pool 实例的引用(类似其他语言中的 `this`)
- `root: &[u8; 32]` - 参数 `root` 是对 32 字节数组的引用(`&`)
- `self.roots.iter()` - 遍历 roots 数组
- `.any(|r| r == root)` - 检查是否有任意元素匹配. 对每个元素 `|r|` 检查 `r` 是否等于 `root`

---

## 构建

```bash
cd anchor
anchor build
```

---

## 改动了什么

- Pool 现在存储近期 Merkle 根节点的环形缓冲区
- 每笔存款都会获得 `leaf_index` 并更新根节点
- 提款必须提供存在于历史记录中的根节点

---

## 下一步

我们可以证明 commitment 的存在了. 但你可能注意到, 如果我们将每个 commitment 都公开发布, 那么任何人都可以构造自己的 Merkle 根节点, 进而完成提款. 下一步我们将讨论 nullifier, 它同时解决两个问题, 证明我们是某个 commitment 的所有者, 以及证明它尚未被提款.

继续 [第 3 步: 防止双重花费](./step-3-preventing-double-spend.md).
