**~14 分钟**

# 第 4.3 步: 验证 CPI

## 目标

更新程序, 通过 CPI 调用 Sunspot 验证程序来验证 ZK 证明.

---

## 更新程序

**文件:** `anchor/programs/private_transfers/src/lib.rs`

---

> `Instruction` 是一个包含 program_id, accounts 和 data 的结构体. `invoke` 将该指令发送给另一个程序.

### 2. 添加验证程序 ID 常量

这是你在第 5 步部署的 Sunspot 验证程序的 Program ID. 每次 CPI 调用都需要目标程序的地址.

替换为(使用你在第 5 步获得的验证程序 ID):

```rust
pub const SUNSPOT_VERIFIER_ID: Pubkey = pubkey!("YOUR_VERIFIER_PROGRAM_ID_HERE");

pub const MIN_DEPOSIT_AMOUNT: u64 = 1_000_000;
```

---

## 第二部分: 为验证程序编码公开输入

Gnark/Sunspot 验证程序对输入有特定的二进制格式要求. 我们需要将公开输入(root, nullifier, recipient, amount)精确按照验证程序预期的格式进行编码.

### 验证程序的预期格式

指令数据格式为: `proof_bytes || public_witness_bytes`


### 3. 添加 encode_public_inputs 函数

该函数以验证程序预期的确切格式构建公开输入.

找到:

```rust
// Step 5: Add encode_public_inputs function here

#[derive(Accounts)]
pub struct Initialize<'info> {
```

替换为:

```rust
/// 按 Gnark/Sunspot 验证程序预期的格式编码公开输入.
/// 验证程序期望特定的二进制格式: 12 字节头部,
/// 后跟每个公开输入, 以 32 字节大端序字段元素表示.
///
/// 大端序(Big-endian)指最高有效字节在前, 与 Solana
/// 和大多数 CPU 存储数字的方式(小端序)相反.
fn encode_public_inputs(
    root: &[u8; 32],
    nullifier_hash: &[u8; 32],
    recipient: &Pubkey,
    amount: u64,
) -> Vec<u8> {
    // NR = "number of"(数量), 密码学代码中的标准缩写
    const NR_PUBLIC_INPUTS: u32 = 4;

    // 预分配空间: 12 字节头部 + 4 个输入 × 32 字节 = 140 字节
    // 写成 12 + 128 而非 140, 是为了直观展示数字的来源
    // Vec::with_capacity 预分配内存, 避免 extend_from_slice 调用时触发内存重分配
    let mut inputs = Vec::with_capacity(12 + 128);

    // === Gnark 头部(12 字节)===
    // Gnark 验证程序期望:
    // - 字节 0-3: 公开输入数量(大端序 u32)
    // - 字节 4-7: commitment 数量, 我们这里始终为 0(大端序 u32)
    //              (Gnark 支持 "commitment" 方案, 但我们不使用, 与存款 commitment 是不同概念! )
    // - 字节 8-11: 公开输入数量再次出现(大端序 u32), 是的, 重复了, 这是 Gnark 的格式要求

    // extend_from_slice 将字节切片追加到 Vec 中, 构建字节数组的高效方式
    inputs.extend_from_slice(&NR_PUBLIC_INPUTS.to_be_bytes());
    inputs.extend_from_slice(&0u32.to_be_bytes());  // 0 个 commitments
    inputs.extend_from_slice(&NR_PUBLIC_INPUTS.to_be_bytes());

    // === 公开输入(每个 32 字节, 大端序)===
    // 重要: 顺序必须与电路中公开输入的声明顺序完全一致!
    // 我们的电路声明顺序为: root, nullifier_hash, recipient, amount

    // 1. Merkle root, 证明 commitment 存在于树中
    inputs.extend_from_slice(root);

    // 2. Nullifier hash, 防止双重花费
    inputs.extend_from_slice(nullifier_hash);

    // 3. 收款方公钥, 接收资金的地址(32 字节)
    // as_ref() 将 Pubkey 转换为 &[u8], 因为 Pubkey 不能直接作为字节数组使用
    // root 和 nullifier_hash 已经是 [u8; 32], 无需转换
    inputs.extend_from_slice(recipient.as_ref());

    // 4. 金额, 填充至 32 字节(u64 只有 8 字节)
    //    左侧补 24 个零字节, 然后是 8 字节的大端序值
    let mut amount_bytes = [0u8; 32];
    amount_bytes[24..32].copy_from_slice(&amount.to_be_bytes());
    inputs.extend_from_slice(&amount_bytes);

    inputs
}

#[derive(Accounts)]
pub struct Initialize<'info> {
```

---

## 第三部分: 将验证程序添加到 Withdraw 账户

提款指令需要访问验证程序, 以便通过 CPI 调用它. 我们验证传入的程序确实是我们的验证程序(而非恶意程序).

### 4. 将验证程序添加到 Withdraw 账户


在下方添加:

```rust
    // #[account(constraint = ...)] 是 Anchor 添加自定义验证的方式
    // constraint 在你的指令代码执行前运行, 如果失败, 交易回滚
    // @ PrivateTransfersError::InvalidVerifier 设置自定义报错信息
    #[account(
        constraint = verifier_program.key() == SUNSPOT_VERIFIER_ID @ PrivateTransfersError::InvalidVerifier
    )]
    /// CHECK: 这是一个没有 Anchor IDL 的外部程序.
    /// 我们使用 UncheckedAccount 是因为无法在不导入该程序类型的情况下
    /// 使用 Program<'info, SunspotVerifier>. 上面的 constraint 已完成验证.
    pub verifier_program: UncheckedAccount<'info>,
}
```

> 关于 `/// CHECK:`, Anchor 要求在每个 UncheckedAccount 上添加此注释, 以证明你已认真考虑了安全问题. 缺少它, `anchor build` 会失败. 务必解释清楚为什么这样做是安全的.

> 关于 `#[account(mut)]` 对 recipient 的标注, 之所以标为可变, 是因为我们要向它转账 SOL. 任何接收 lamports 的账户都必须是可写的.


现在我们的程序已经能够访问验证程序了.

---

## 第四部分: 更新提款函数以验证证明

现在更新提款函数, 使其能够:
1. 接收证明作为输入
2. 编码公开输入
3. 通过 CPI 调用验证程序
4. 仅在验证成功后继续执行(CPI 失败 = 交易回滚)

### 5. 更新提款函数签名


```rust
    pub fn withdraw(

        // 客户端生成的 ZK 证明(324 字节)
        proof: Vec<u8>,

    ) -> Result<()> {
```

### 6. 添加验证 CPI 调用

这就是魔法发生的地方. 我们将证明和公开输入传入验证程序. 如果证明无效, CPI 调用失败, 整笔交易回滚, 资金不会发生转移.

找到:


添加:
```rust

        // === 通过 CPI 验证 ZK 证明 ===
        // 按验证程序预期的格式编码公开输入
        let public_inputs = encode_public_inputs(&root, &nullifier_hash, &recipient, amount);

        // .as_slice() 将 Vec<u8> 转换为 &[u8](切片引用)
        // .concat() 将多个切片合并为一个新的 Vec
        // 结果: 证明字节后跟公开输入字节
        let instruction_data = [proof.as_slice(), public_inputs.as_slice()].concat();

        // 为什么用 invoke() 而不是像转账时用 CpiContext?
        // - CpiContext 是 Anchor 调用其他 Anchor 程序的辅助工具
        // - invoke() 是底层 Solana 方式, 适用于任何程序
        // - Sunspot 验证程序不是 Anchor 程序, 所以我们使用 invoke()
        //
        // invoke() 接收:
        // 1. &Instruction - 调用内容(程序 + 账户 + 数据)
        // 2. &[AccountInfo] - 被调用程序需要访问的账户
        invoke(
            &Instruction {
                program_id: ctx.accounts.verifier_program.key(),  // 调用谁
                accounts: vec![],  // 验证程序不需要账户, 只需指令数据
                data: instruction_data,  // 发送什么(证明 + 公开输入)
            },
            &[ctx.accounts.verifier_program.to_account_info()],  // 运行时所需的账户信息
        )?;

        // 执行到这里说明证明有效! 标记 nullifier 并转移资金
        nullifier_set.mark_nullifier_used(nullifier_hash)?;
```

---

## Solana 深入: CPI 模式

跨程序调用(CPI)是 Solana 程序之间通信的方式. 让我们理解相关模式:

**invoke 与 invoke_signed 的区别:**
- `invoke` - 调用另一个程序, 调用方的权限会被传递.
- `invoke_signed` - 相同, 但额外使用 PDA 种子进行签名. 当你的 PDA 需要授权某些操作时使用(例如从 PDA 控制的金库转账).

我们这里使用 `invoke`, 因为验证程序不需要任何签名, 它只是验证数学运算.

**invoke 与 CpiContext 的区别:**
- `CpiContext` 是 Anchor 调用其他 Anchor 程序的辅助工具, 提供类型安全和自动账户验证.
- `invoke` 是原生 Solana, 适用于任何程序, 但需要手动构建指令.

Sunspot 验证程序不是 Anchor 程序(它是生成的 Rust 代码), 所以我们使用 `invoke`. 之前转账 SOL 时, 我们使用 `CpiContext`, 是因为系统程序有 Anchor 绑定.

**为什么验证程序不需要账户:**

大多数 CPI 调用都需要传递账户, 被调用的程序需要读写数据. 但我们的验证程序是纯计算的: 接收证明字节和公开输入, 进行椭圆曲线数学运算, 要么成功要么失败. 没有状态需要读取, 也没有数据需要写入. 这使它既廉价又简单.

**原子性失败:**

如果 `invoke` 返回错误, 整笔交易回滚. 这对安全性至关重要, 我们不能允许验证失败但资金仍然转移的情况发生. `invoke(...)` 后面的 `?` 会传播错误, 在验证失败时使交易失败.

---

## 构建并部署

在 Anchor.toml 中更新私密转账程序和 Sunspot 验证程序的正确 Program ID.

```bash
cd anchor
anchor build
anchor program deploy --provider.cluster devnet
```

---

## 改动了什么

- 提款现在需要 ZK 证明
- 程序在释放资金前通过 CPI 调用验证程序
- 如果验证失败, 整笔交易回滚
- 没有有效证明, 资金不会发生任何转移

---

## 下一步

程序已经完成! 让我们构建前端并进行完整测试.

继续 [第 7 步: 前端与演示](./step-7-frontend-demo.md).
