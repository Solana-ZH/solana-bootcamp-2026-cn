**~12 分钟**

# 第 4.4 步: 前端与演示

## 目标

了解前端如何使用 Solana Kit 构建并发送提款交易.

---

## 你将做什么

* 使用 Codama 生成 TypeScript 客户端
* 逐行解读提款交易代码
* 运行并测试完整流程

---

## 使用 Codama 生成客户端

### 什么是 IDL?

IDL 是**接口定义语言(Interface Definition Language)**的缩写. 可以把它理解为一个 JSON 文件, 描述了你的 Solana 程序的 API, 它有哪些指令, 这些指令需要哪些账户, 以及使用了哪些数据类型.

运行 `anchor build` 时, Anchor 会通过读取你的 Rust 代码自动生成 IDL 文件(`target/idl/your_program.json`). 任何想调用该程序的前端或脚本都可以读取此 IDL, 了解如何正确格式化请求. 我们的 IDL 位于 `anchor/target/idl/private_transfers.json`.

例如, 我们的 IDL 描述了 `withdraw` 指令: 它需要一个证明(字节数组), 一个 nullifier hash(32 字节), 一个 root(32 字节)等. 没有 IDL, 你就必须手动弄清楚如何序列化数据, 使用什么字节序, 以及指令判别符(discriminator)是什么, 既容易出错, 又繁琐.

### Codama 如何使用 IDL

Codama 读取你的 Anchor IDL, 并生成自动处理所有序列化逻辑的 TypeScript 代码, 提供类型安全的编码器. 如果你遗漏了某个字段或使用了错误的类型, TypeScript 会在编译时就报错.

```bash
cd frontend
bun run scripts/generate-client.ts
```

这将在 `generated/` 文件夹中生成:
- 指令编码器(处理 8 字节判别符和字段序列化)
- 账户解码器
- 类型定义

---

## 更新常量

在 `frontend/src/constants.ts` 中, 设置你的验证程序 ID:

```typescript
export const SUNSPOT_VERIFIER_ID = address("YOUR_VERIFIER_PROGRAM_ID_HERE")
```

在 `backend/src/server.ts` 中设置私密转账程序的 ID.

前端通过 Codama 会自动完成这一步.

---

## 提款交易

让我们逐行解读 `frontend/src/components/WithdrawSection.tsx`.

**文件: ** `frontend/src/components/WithdrawSection.tsx`

### 导入, 每个 Kit 函数的作用

```typescript
import { useWalletConnection, useSendTransaction } from '@solana/react-hooks'
// useWalletConnection - React hook, 提供已连接的钱包
// useSendTransaction - React hook, 处理签名和发送交易

import { address, getProgramDerivedAddress, getBytesEncoder, getAddressEncoder } from '@solana/kit'
// address() - 将 base58 字符串转换为 Solana Address 类型
// getProgramDerivedAddress() - 根据种子和程序 ID 推导 PDA
// getBytesEncoder() - 创建将字符串/Uint8Array 转换为字节的编码器
// getAddressEncoder() - 创建将 Address 转换为 32 字节的编码器

import { getWithdrawInstructionDataEncoder, PRIVATE_TRANSFERS_PROGRAM_ADDRESS } from '../generated'
// getWithdrawInstructionDataEncoder() - Codama 为我们的 withdraw 指令生成的编码器
// PRIVATE_TRANSFERS_PROGRAM_ADDRESS - 从 IDL 中获取的程序地址
```

### 准备证明数据

```typescript
// 后端生成 ZK 证明并返回所需的所有数据
const { withdrawalProof }: WithdrawApiResponse = await response.json()

// 将证明从 number[] 转换为 Uint8Array(Solana 首选的字节格式)
const proof = new Uint8Array(withdrawalProof.proof)

// 使用工具函数将十六进制字符串转换为字节数组
// hexToBytes("0x1234...") -> Uint8Array([0x12, 0x34, ...])
const nullifierHash = hexToBytes(withdrawalProof.nullifierHash)
const root = hexToBytes(withdrawalProof.merkleRoot)

// address() 验证并将 base58 字符串转换为 Kit 的 Address 类型
// 确保在构建交易前收款方是有效的 Solana 地址
const recipientAddress = address(withdrawalProof.recipient)

// JavaScript 对超过 2^53 的数字会丢失精度, 因此对 u64 值使用 BigInt
// Solana 金额以 lamports 计(1 SOL = 1_000_000_000 lamports)
const amountBN = BigInt(withdrawalProof.amount)
```

### 推导 PDA

```typescript
// getProgramDerivedAddress 查找 PDA 的确定性地址
// 返回 [address, bump], 这里我们只需要地址
const [poolPda] = await getProgramDerivedAddress({
  programAddress,  // 拥有此 PDA 的程序
  seeds: [getBytesEncoder().encode(SEEDS.POOL)],  // "pool" 的字节表示
})

// NullifierSet PDA 使用两个种子: "nullifiers" + 资金池地址
// 这将 nullifier 集合与特定资金池相关联
const [nullifierSetPda] = await getProgramDerivedAddress({
  programAddress,
  seeds: [
    getBytesEncoder().encode(SEEDS.NULLIFIERS),  // "nullifiers" 的字节表示
    getAddressEncoder().encode(poolPda),          // 资金池地址, 以 32 字节表示
  ],
})

// 金库 PDA, 存储实际的 SOL
const [poolVaultPda] = await getProgramDerivedAddress({
  programAddress,
  seeds: [
    getBytesEncoder().encode(SEEDS.VAULT),
    getAddressEncoder().encode(poolPda),
  ],
})
```

### 编码指令数据

```typescript
// Codama 根据你的 IDL 生成此编码器
// 它知道: 8 字节判别符 + 字段的确切顺序 + 字节大小
const withdrawDataEncoder = getWithdrawInstructionDataEncoder()

// encode() 将所有字段序列化为一个 Uint8Array
// 编码器处理: 判别符, 证明字节, 哈希值, 地址, 金额
const instructionData = withdrawDataEncoder.encode({
  proof,              // Vec<u8>, 可变长度, 带长度前缀
  nullifierHash,      // [u8; 32], 精确 32 字节
  root,               // [u8; 32], 精确 32 字节
  recipient: recipientAddress,  // Pubkey, 32 字节
  amount: amountBN,   // u64, 8 字节, 小端序
})
```

### 构建指令

```typescript
// Instruction 告诉 Solana: 调用哪个程序, 哪些账户, 传什么数据
const withdrawInstruction = {
  programAddress,  // 要调用的程序

  // 账户角色:
  // 0 = 只读        - 程序可读取但不能修改
  // 1 = 可写        - 程序可修改该账户
  // 2 = 只读 + 签名者  - 必须签名, 但不被修改
  // 3 = 可写 + 签名者  - 必须签名且可被修改
  //
  // 顺序很重要! 必须与 Anchor 程序中 Withdraw 结构体的顺序一致
  accounts: [
    { address: poolPda, role: 1 },          // pool: 可写(更新根节点历史)
    { address: nullifierSetPda, role: 1 },  // nullifier_set: 可写(标记 nullifier 已使用)
    { address: poolVaultPda, role: 1 },     // pool_vault: 可写(发送 SOL)
    { address: recipientAddress, role: 1 }, // recipient: 可写(接收 SOL)
    { address: SUNSPOT_VERIFIER_ID, role: 0 }, // verifier_program: 只读(CPI 目标)
    { address: SYSTEM_PROGRAM_ID, role: 0 },   // system_program: 只读
  ],
  data: instructionData,
}
```

### 设置计算预算

```typescript
// ZK 验证约需要 140 万计算单元, 但默认预算只有 20 万
// 必须申请更多计算单元, 否则交易会失败
const computeBudgetData = new Uint8Array(5)
computeBudgetData[0] = 2  // SetComputeUnitLimit 的指令索引
new DataView(computeBudgetData.buffer).setUint32(1, ZK_VERIFY_COMPUTE_UNITS, true)
// true = 小端序(Solana 的字节序)

const computeBudgetInstruction = {
  programAddress: COMPUTE_BUDGET_PROGRAM_ID,  // Solana 原生程序
  accounts: [] as const,  // 不需要任何账户
  data: computeBudgetData,
}
```

### 发送交易

```typescript
// useSendTransaction hook 的 sendTransaction 处理:
// 1. 获取最新的区块哈希
// 2. 构建交易消息
// 3. 请求钱包签名
// 4. 提交到网络
// 5. 等待确认
const result = await sendTransaction({
  // 多条指令原子性执行, 任何一条失败, 全部回滚
  instructions: [computeBudgetInstruction, withdrawInstruction],
})
```

---

## 运行并测试演示

### 同步 anchor 密钥

```bash
cd anchor
anchor keys sync
```

### 1. 启动后端

后端负责生成证明(在服务端运行 Noir prover).

```bash
cd backend
bun install
bun run dev
```

运行在 `http://localhost:4001`.

### 2. 启动前端

```bash
cd frontend
bun install
bun run dev
```

打开 `http://localhost:3000`.

## 初始化资金池(仅首次需要! )

在任何存款或提款发生之前, 必须先初始化资金池. 这将在链上创建 Pool 和 NullifierSet 账户.

```bash
cd anchor
bun run init-pool
```

> 每次部署只需运行一次. 如果重新部署程序, 需要再次初始化. 初始化指令会为资金池, 金库和 nullifier 集合创建 PDA.

---

## 测试完整流程

### 存款

1. 连接钱包(devnet)
2. 输入金额(例如 0.1 SOL)
3. 点击**存款**
4. **保存你的存款凭据! **(包含提款所需的 secret + nullifier)

### 提款

1. 切换到另一个钱包(可选, 模拟 Alice → Bob 的场景)
2. 粘贴存款凭据
3. 输入收款地址
4. 点击**提款**
5. 等待约 30 秒以生成证明
6. 批准交易

---

## 在区块链浏览器上验证隐私性

打开 [Solana Explorer](https://explorer.solana.com/?cluster=devnet).

**存款交易显示: **
- `commitment: 0x7a3b...`(而非你的地址! )
- `leaf_index: 0`
- `new_root: 0xabc1...`

**提款交易显示: **
- `nullifier_hash: 0x9c2f...`(不同的哈希)
- `recipient: Bob 的地址`
- 与原始存款**没有任何关联**!

**为什么无法关联: **

```
存款:     commitment = Hash(nullifier, secret, amount)
提款:     nullifier_hash = Hash(nullifier)

不同的哈希函数, 相同的 nullifier 内部, 不知道 secret 就无法建立关联!
```

---

## 你构建了什么

| 组件 | 用途 |
|-----------|---------|
| Commitment | 隐藏存款人身份 |
| Merkle 树 | 证明存款存在 |
| Nullifier hash | 防止双重花费 |
| ZK 证明 | 私密证明所有内容 |
| 验证程序 CPI | 链上证明验证 |
| Codama | 类型安全的客户端生成 |

---

## 恭喜!

你已经在 Solana 上使用以下技术构建了一个保护隐私的转账系统:
- **Noir** - 用于 ZK 电路
- **Sunspot** - 用于 Groth16 证明
- **Anchor** - 用于 Solana 程序
- **Codama** - 用于客户端生成
- **Solana Kit** - 用于前端

这些核心概念, commitments, nullifiers, Merkle 树, ZK 证明, 是 Tornado Cash 和 Zcash 等隐私应用的基础, 现在它们在 Solana 上运行了.
