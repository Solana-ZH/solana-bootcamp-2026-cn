**~8 分钟**

# 第 3 步: 防止双重花费

## 目标

添加 nullifier 追踪机制, 用于证明对特定 commitment 的所有权, 并防止同一笔存款被多次提取.

---
## 更新程序

**文件: ** `anchor/programs/private_transfers/src/lib.rs`

---

nullifier 是从存款密钥(secret)派生出的唯一值. 提款时, 你需要公开 nullifier(而非 secret). 程序会记录它, 如果你尝试再次使用相同的 nullifier 提款, 就会被拒绝.

每笔存款对应且仅对应一个 nullifier, 并且 nullifier 无法被反推回 commitment. 这样我们就能知道"某笔存款已被花费", 却不知道"哪笔存款被花费了".

### 1. 添加 NullifierSet 结构体

替换为:

```rust

#[account]  // Anchor 宏, 将此结构体标记为可存储于链上的 Solana 账户
#[derive(InitSpace)]  // Anchor 宏, 自动计算此结构体所需的字节数(用于租金计算)
pub struct NullifierSet {
    // 此 nullifier 集合所属的资金池
    pub pool: Pubkey,
    // 所有已使用的 nullifier 列表(本演示最多 256 个)
    #[max_len(256)]  // Anchor 宏, 告知 Anchor Vec 的最大容量, 用于空间计算
    pub nullifiers: Vec<[u8; 32]>,  // Vec = 可动态增长的数组, 存储 32 字节哈希的列表
}

// 在生产环境中通常会使用另一棵 Merkle 树来存储 nullifier

impl NullifierSet {
    // 检查此 nullifier 是否已被使用(即存款是否已被提取)
    pub fn is_nullifier_used(&self, nullifier_hash: &[u8; 32]) -> bool {
        self.nullifiers.contains(nullifier_hash)
    }

    // 将 nullifier 标记为已使用, 在提款成功后调用
    pub fn mark_nullifier_used(&mut self, nullifier_hash: [u8; 32]) -> Result<()> {
        require!( // AFTER
            self.nullifiers.len() < 256,
            PrivateTransfersError::NullifierSetFull
        );
        self.nullifiers.push(nullifier_hash);
        Ok(())
    }
}
```
## Solana 深入: 为什么要单独创建账户?

你可能会问, 为什么不直接在 Pool 结构体中添加 `nullifiers` 字段? 原因如下:

**账户大小限制: ** Solana 账户最大可达 10MB, 但你需要按大小支付租金(约 6.9 SOL/MB/年). 我们的 NullifierSet 包含 256 个 nullifier, 约 8KB. 如果需要存储数千个 nullifier, 单独创建账户可以独立分配空间.

**关注点分离: ** Pool 追踪树状态(根节点, 叶节点索引), NullifierSet 追踪已花费的存款. 数据类型不同, 访问模式也不同. 在生产环境中, 你甚至可以用 Merkle 树存储 nullifier(如 Light Protocol 所做的那样), 以固定的链上存储空间支持无限数量的 nullifier.

**升级灵活性: ** 如果你想改变 nullifier 的存储方式(比如切换为 Merkle 树), 可以部署新的 NullifierSet 实现而无需改动 Pool.

---
---

## 第二部分: 初始化时设置 NullifierSet

我们需要在初始化资金池时创建 NullifierSet 账户, 并将其关联到资金池.

### 2. 将 NullifierSet 添加到 Initialize 账户


替换为:

```rust
    pub pool: Account<'info, Pool>,

    // 存储该资金池所有已使用 nullifier 的 PDA
    #[account( // 新增
        init,
        payer = authority,
        space = 8 + NullifierSet::INIT_SPACE,
        seeds = [b"nullifiers", pool.key().as_ref()],
        bump
    )]
    pub nullifier_set: Account<'info, NullifierSet>, // 新增

    #[account(seeds = [b"vault", pool.key().as_ref()], bump)]
    pub pool_vault: SystemAccount<'info>,
```

### 3. 在初始化函数中初始化 nullifier_set

找到:


替换为:

```rust
        pool.roots[0] = EMPTY_ROOT;

        // 将 nullifier 集合关联到此资金池
        let nullifier_set = &mut ctx.accounts.nullifier_set;
        nullifier_set.pool = pool.key();

        msg!("Pool initialized");
```

---

我们已经创建了存储 nullifier 的 `NullifierSet` 结构体, 并在资金池初始化时对其进行了设置. nullifier 集合通过 PDA(程序派生地址)与资金池关联. 现在我们需要真正使用它.

---


## 第三部分: 更新提款函数以检查和记录 Nullifier

提款流程现在需要:
1. 接收用户提供的 nullifier_hash
2. 检查它是否已被使用
3. 在转账前将其标记为已使用(防止重入攻击)

### 4. 将 NullifierSet 添加到 Withdraw 账户

找到:

添加:
```rust
    // 必须设为可变, 以便在提款后添加 nullifier
    #[account(
        mut,  // 我们正在修改此账户(添加 nullifier), 因此必须设为可变
        seeds = [b"nullifiers", pool.key().as_ref()],  // PDA 种子, 从 "nullifiers" 推导地址
        bump  // 使用 PDA 推导时找到的 bump 种子(Anchor 会自动查找)
    )]
    pub nullifier_set: Account<'info, NullifierSet>, // 存储 NullifierSet 结构体数据的 Solana 账户, info = 交易的生命周期

```

### 5. 更新提款函数签名并添加 nullifier 检查

找到:

替换为:

```rust
    pub fn withdraw(
        ctx: Context<Withdraw>,

        nullifier_hash: [u8; 32], // 新增
        root: [u8; 32],
        recipient: Pubkey,
        amount: u64,
    ) -> Result<()> {
        let nullifier_set = &mut ctx.accounts.nullifier_set; // 新增

        // 检查此 nullifier 是否已被使用(双重花费尝试)
        require!( // 新增
            !nullifier_set.is_nullifier_used(&nullifier_hash),
            PrivateTransfersError::NullifierUsed
        );

        require!(
            ctx.accounts.pool.is_known_root(&root),
            PrivateTransfersError::InvalidRoot
        );
```


### 6. 在转账前标记 nullifier 为已使用

我们在转账**之前**标记 nullifier. 这对安全性至关重要, 如果在之后标记, 重入调用可能导致重复提款:
1. 攻击者使用有效的 nullifier 调用 `withdraw`
2. 检查通过(nullifier 尚未被使用)
3. 开始向攻击者账户转账
4. 在第 3 步执行前, 攻击者的接收程序可能再次使用相同的 nullifier 调用 `withdraw`
5. 第二次调用也通过了 nullifier 检查(它仍未被标记! )
6. 攻击者从一笔存款中获得了两次(或更多次)付款

通过在转账前标记 nullifier 为已使用, 任何重入调用都会在 nullifier 检查时失败.

替换为:

```rust
        require!(
            ctx.accounts.pool_vault.lamports() >= amount,
            PrivateTransfersError::InsufficientVaultBalance
        );

        // Step 5: Verify ZK proof via CPI

        // 在转账前标记 nullifier 为已使用, 防止重入攻击
        nullifier_set.mark_nullifier_used(nullifier_hash)?;

        let pool_key = ctx.accounts.pool.key();
```

---

## 第四部分: 更新事件和日志

提款事件应包含 nullifier, 方便客户端追踪哪些 nullifier 已被使用, 从而避免失败的交易, 也可用于追踪某个特定钱包的花费记录.

### 8. 更新提款中的 emit!

替换为:

```rust
        emit!(WithdrawEvent {
            nullifier_hash,
            recipient: ctx.accounts.recipient.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });
```

### 9. 更新日志消息

找到:

```rust
        msg!("Public withdrawal: {} lamports to {}", amount, recipient);
```

替换为:

```rust
        msg!("Withdrawal: {} lamports to {} with nullifier {:x?}", amount, recipient, nullifier_hash);
```
---

## 构建

```bash
cd anchor
anchor build
```

---

## 改动了什么

- 新 `NullifierSet` 账户存储所有已使用的 nullifier hash
- 提款时必须提供 nullifier_hash
- 同一 nullifier_hash 被提交两次时, 第二次提款将被拒绝
- nullifier_hash 无法被反推回原始 commitment

---

## 下一步

我们可以隐藏存款, 证明成员资格并防止双重花费了. 但目前还没有验证任何证明, 任何人都可以提交伪造数据!

继续 [第 4 步: ZK 电路](./step-4-zk-circuit.md).
