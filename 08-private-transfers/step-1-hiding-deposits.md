**~6 分钟**

# 第 1 步: 隐藏存款

## 目标

更新存款函数, 使其接受 commitment 而非存储存款人地址. 我们不再直接存储存款信息, 而是存储一个 commitment, 提款时由用户证明自己知晓该 commitment.

---

## 更新程序

## 核心改动

修改存款人结构体, 移除账户字段, 改为存储 commitment;
同时删除存款事件中对账户的引用.

**文件: ** `anchor/programs/private_transfers/src/lib.rs`

### 1. 更新存款函数签名

找到:

```rust
    pub fn deposit(
        ctx: Context<Deposit>,
        // Step 1: Add commitment: [u8; 32]
        // Step 2: Add new_root: [u8; 32]
        amount: u64,
    ) -> Result<()> {
```

替换为:

```rust
    pub fn deposit(
        ctx: Context<Deposit>,
        commitment: [u8; 32],
        amount: u64,
    ) -> Result<()> {
```

> **`[u8; 32]` 是什么? **
>
> 这是 Rust 中固定长度字节数组的语法, 表示 32 个字节. commitment 是一个哈希值, 输出 256 位(32 字节 × 8 位 = 256 位). 后端计算出这个哈希后将其转为字节数组发送给程序.

---

### 2. 更新 DepositEvent 结构体

替换为:

```rust
#[event]
pub struct DepositEvent {
    pub commitment: [u8; 32],
    pub amount: u64,
    pub timestamp: i64,
}
```

---

### 3. 更新 emit! 调用


替换为:

```rust
        emit!(DepositEvent {
            commitment,
            amount,
            timestamp: Clock::get()?.unix_timestamp,
        });
```

---

### 4. 更新日志消息

找到:

```rust
        msg!("Public deposit: {} lamports from {}", amount, ctx.accounts.depositor.key());
```

替换为:

```rust
        msg!("Deposit: {} lamports, commitment: {:?}", amount);
```

---


## Commitment 如何传递到 Solana

让我们看看这个 commitment 是如何计算的.

### 1. 后端计算 Poseidon 哈希

在 `backend/src/server.ts` 中, 我们使用 JavaScript 的 Poseidon2 库:

```typescript
import { poseidon2Hash } from "@zkpassport/poseidon2";

// 生成随机密钥
const nullifier = generateRandomField();  // 随机 256 位数
const secret = generateRandomField();     // 另一个随机 256 位数

// 计算 commitment = Poseidon2(nullifier, secret, amount)
const commitment = poseidon2Hash([nullifier, secret, amount]);
const commitmentHex = "0x" + commitment.toString(16).padStart(64, "0");
```

**为什么用 Poseidon?** SHA-256 等常规哈希函数在 ZK 电路中需要数百万个约束条件. Poseidon 专为 ZK 友好而设计, 在相同安全性下, 约束数量大幅减少, 使证明更快, 成本更低.

Poseidon 正在 Solana 生态中成为标准, 不仅限于隐私场景. Light Protocol 将 Poseidon 哈希用于压缩状态, 目的不是隐私而是扩展性: 他们将账户数据在链下哈希, 只在链上存储哈希值, 并用 ZK 证明验证状态转换. 相同的密码学原语, 不同的使用场景.

### 2. 转换为字节数组供 Solana 使用

查看 `api/deposit` 实际调用的位置. Poseidon 哈希是 BigInt 类型, 而 Solana 需要字节数组:

```typescript
// 去掉 "0x" 前缀, 将十六进制字符串转为字节
const commitmentBytes = Array.from(
  Buffer.from(commitmentHex.slice(2), "hex")
);
// 结果: [122, 59, ...] - 32 个数字, 每个值在 0-255 之间
```

### 3. 前端发送到 Solana

前端从后端接收这些字节, 然后使用 Solana Kit 构建并发送交易. 完整的前端代码我们会在演示步骤中介绍, 现在只需了解 Kit 的 `sendTransaction` 会处理钱包签名和交易提交.

---

## 改动了什么


存款人的钱包仍然是交易的签名者, 但链上事件不再将其身份与这笔存款关联起来.

---

## 下一步

我们已经隐藏了存款信息. 但如何在不暴露是哪笔存款的情况下证明存款存在? 我们需要 Merkle 树.

继续 [第 2 步: 证明成员资格](./step-2-proving-membership.md).
