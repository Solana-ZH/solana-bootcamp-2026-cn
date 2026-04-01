# 90分钟视频讲座：全栈Solana预测市场

## 概述

**形式:** 预录制视频讲座
**时长:** 90分钟
**风格:** 架构图 + 函数讲解

### 学习目标

课程结束后，观众将理解：
1. Solana程序如何存储和管理状态
2. Anchor框架在简化开发中的作用
3. 如何从链上代码生成类型安全的客户端
4. 真实dApp中的端到端数据流

---

## 第一部分：为什么 & 是什么 (15分钟)

### 1.1 — 问题描述 (5分钟)

- 传统预测市场：中心化、托管式、受限
- 区块链解决方案：无需信任的托管、全球访问、透明赔率

### 1.2 — 架构概述 (10分钟)

**图1：系统层级**

```
┌────────────────────────────────────────────────────────┐
│                   浏览器                               │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Next.js应用                                     │  │
│  │  • MarketsList, MarketCard, PositionsList        │  │
│  │  • 通过autoDiscover()连接钱包                    │  │
│  └──────────────────────────────────────────────────┘  │
│                         │                              │
│                         ▼                              │
│  ┌──────────────────────────────────────────────────┐  │
│  │  生成的客户端 (Codama)                           │  │
│  │  • 类型安全的指令构建器                          │  │
│  │  • 账户编码器/解码器                             │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
                          │ JSON-RPC
                          ▼
┌────────────────────────────────────────────────────────┐
│                  SOLANA开发网                          │
│  ┌─────────────────┐    ┌─────────────────────────┐   │
│  │ Anchor程序      │───▶│ 账户（状态）            │   │
│  │ （无状态）      │    │ • 市场PDAs              │   │
│  │                 │    │ • 用户持仓PDAs          │   │
│  └─────────────────┘    └─────────────────────────┘   │
└────────────────────────────────────────────────────────┘
```

**图2：数据所有权**

```
        ┌─────────────┐
        │   市场      │ ← 持有池化的SOL
        │    PDA      │   （无私钥！）
        └─────────────┘
              │
    ┌─────────┴─────────┐
    ▼                   ▼
┌─────────┐       ┌─────────┐
│持仓     │       │持仓     │  ← 追踪每个用户的投注
│ 用户A   │       │ 用户B   │
└─────────┘       └─────────┘
```

**关键洞察：** PDAs是没有私钥的地址——只有程序可以为它们签名。这使得它们非常适合无需信任地持有资金。

---

## 第二部分：链上程序 (30分钟)

### 2.1 — 账户结构 (10分钟)

**讲解：`state.rs`**

```rust
pub struct Market {
    pub creator: Pubkey,        // 32字节 - 谁可以结算
    pub market_id: u64,         // 8字节 - 唯一ID
    pub question: String,       // 可变长度 - "X会发生吗？"
    pub resolution_time: i64,   // 8字节 - 投注截止时间
    pub yes_pool: u64,          // 8字节 - 总YES lamports
    pub no_pool: u64,           // 8字节 - 总NO lamports
    pub resolved: bool,         // 1字节 - 结果是否已确定？
    pub outcome: Option<bool>,  // 2字节 - None, Some(true), Some(false)
    pub bump: u8,               // 1字节 - PDA bump种子
}
```

**讨论要点：**
- 每个字段存在的原因
- `Option<bool>`如何表示三种状态（未结算、是、否）
- 租金豁免的空间计算

---

### 2.2 — 核心函数 (20分钟)

#### 函数1：`create_market` (5分钟)

```rust
pub fn create_market(
    ctx: Context<CreateMarket>,
    market_id: u64,
    question: String,
    resolution_time: i64,
) -> Result<()> {
    // 验证
    require!(resolution_time > Clock::get()?.unix_timestamp,
             ResolutionTimeInPast);
    require!(question.len() <= 200, QuestionTooLong);

    // 初始化状态
    let market = &mut ctx.accounts.market;
    market.creator = ctx.accounts.creator.key();
    market.question = question;
    market.yes_pool = 0;
    market.no_pool = 0;
    // ...
}
```

**关键点：** 验证发生在状态更改之前

---

#### 函数2：`place_bet` (7分钟)

**图：资金流向**

```
   用户钱包                    市场PDA
       │                           │
       │    transfer(amount)       │
       ├──────────────────────────►│
       │                           │
       ▼                           ▼
  余额 -= 金额              yes_pool += 金额
                                (或no_pool)
```

```rust
pub fn place_bet(
    ctx: Context<PlaceBet>,
    amount: u64,
    bet_yes: bool,
) -> Result<()> {
    let market = &mut ctx.accounts.market;

    // 时间检查 - 截止时间后不能投注
    require!(Clock::get()?.unix_timestamp < market.resolution_time,
             BettingClosed);

    // 转移SOL：用户 → 市场PDA
    let transfer_ix = system_instruction::transfer(
        &ctx.accounts.user.key(),
        &ctx.accounts.market.key(),
        amount,
    );
    invoke(/* ... */)?;

    // 更新资金池
    if bet_yes {
        market.yes_pool = market.yes_pool.checked_add(amount)
            .ok_or(PredictionMarketError::Overflow)?;
    } else {
        market.no_pool = market.no_pool.checked_add(amount)
            .ok_or(PredictionMarketError::Overflow)?;
    }

    // 更新用户持仓...
}
```

**关键洞察：** `checked_add`在溢出时返回`None`而不是回绕。这可以防止有人通过溢出资金池到零来攻击。

---

#### 函数3：`claim_winnings` (8分钟)

**图：赔付计算**

```
示例市场：
┌─────────────────────────────────────┐
│  YES池：100 SOL                     │
│  NO池：  50 SOL                     │
│  结果：YES获胜                      │
└─────────────────────────────────────┘

用户在YES上投注10 SOL：
┌─────────────────────────────────────┐
│  获胜者份额：10/100 = 10%           │
│  从失败者索赔：50 × 10% = 5         │
│  总赔付：10 + 5 = 15 SOL            │
└─────────────────────────────────────┘
```

```rust
pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
    let market = &ctx.accounts.market;
    let position = &mut ctx.accounts.position;

    // 守卫条件
    require!(market.resolved, NotResolved);
    require!(!position.claimed, AlreadyClaimed);

    // 确定获胜方
    let (user_bet, winning_pool, losing_pool) = match market.outcome {
        Some(true) => (position.yes_amount, market.yes_pool, market.no_pool),
        Some(false) => (position.no_amount, market.no_pool, market.yes_pool),
        None => return err!(NotResolved),
    };

    require!(user_bet > 0, NoWinnings);

    // 计算赔付
    let winnings = user_bet
        .checked_mul(losing_pool).ok_or(Overflow)?
        .checked_div(winning_pool).ok_or(Overflow)?;
    let total_payout = user_bet.checked_add(winnings).ok_or(Overflow)?;

    // 从PDA转移到用户...
    position.claimed = true;
}
```

---

## 第三部分：代码生成层 (15分钟)

### 3.1 — 生成管道 (5分钟)

**图：代码生成流程**

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   lib.rs    │      │  IDL.json   │      │ TypeScript  │
│   (Rust)    │─────▶│ (架构)      │─────▶│  客户端     │
└─────────────┘      └─────────────┘      └─────────────┘
     anchor build         codama:js

   真理之源             中间表示          前端导入的内容
```

### 3.2 — 生成代码讲解 (10分钟)

**从IDL到TypeScript：**

```json
// IDL片段
{
  "name": "placeBet",
  "args": [
    { "name": "amount", "type": "u64" },
    { "name": "betYes", "type": "bool" }
  ]
}
```

```typescript
// 生成：placeBet.ts
export async function getPlaceBetInstructionAsync(
  input: PlaceBetAsyncInput
): Promise<PlaceBetInstruction> {
  // 自动从种子推导PDAs
  const marketAddress = await findMarketPda(input.creator, input.marketId);
  const positionAddress = await findPositionPda(marketAddress, input.user);

  // 将参数编码为字节
  const data = getPlaceBetInstructionDataEncoder().encode({
    amount: input.amount,
    betYes: input.betYes,
  });

  return { keys: [...], data, programId };
}
```

**关键洞察：** 生成的客户端处理两个难题：**PDA推导**（计算确定性地址）和**序列化**（将参数编码为字节）。你永远不需要手动编写这些代码。

---

## 第四部分：前端架构 (20分钟)

### 4.1 — 组件层次结构 (5分钟)

**图：**

```
App (layout.tsx)
 └─ Providers (Solana客户端 + 钱包)
     ├─ HomePage (page.tsx)
     │   ├─ CreateMarketForm
     │   └─ MarketsList
     │       └─ MarketCard (×N)
     │
     └─ ActivityPage (activity/page.tsx)
         └─ PositionsList
             └─ PositionCard (×N)
```

### 4.2 — 数据获取模式 (8分钟)

**讲解：`markets-list.tsx`**

```typescript
// 从程序获取所有市场账户
const fetchMarkets = async () => {
  const response = await fetch(RPC_URL, {
    method: 'POST',
    body: JSON.stringify({
      method: 'getProgramAccounts',
      params: [
        PROGRAM_ID,
        {
          filters: [{
            memcmp: {
              offset: 0,
              bytes: "dkokXHR3DTw"  // 市场鉴别器
            }
          }]
        }
      ]
    })
  });

  // 解码每个账户
  const markets = accounts.map(acc => {
    const data = base64ToBytes(acc.data);
    return getMarketDecoder().decode(data);
  });
};

// 每3秒轮询一次
useEffect(() => {
  const interval = setInterval(fetchMarkets, 3000);
  return () => clearInterval(interval);
}, []);
```

**图：鉴别器过滤**

```
程序拥有许多账户：
┌──────────────────┐
│ [disc: Market]   │ ← 匹配过滤器 ✓
│ question: "..."  │
└──────────────────┘
┌──────────────────┐
│ [disc: Position] │ ← 不匹配 ✗
│ user: 0x...      │
└──────────────────┘
┌──────────────────┐
│ [disc: Market]   │ ← 匹配过滤器 ✓
│ question: "..."  │
└──────────────────┘
```

### 4.3 — 交易流程 (7分钟)

**讲解：`market-card.tsx`中的投注**

```typescript
const handleBet = async (betYes: boolean) => {
  // 1. 使用生成的客户端构建指令
  const instruction = await getPlaceBetInstructionAsync({
    market: marketAddress,
    user: wallet.address,
    amount: BigInt(solAmount * LAMPORTS_PER_SOL),
    betYes,
  });

  // 2. 发送交易
  await sendTransaction({
    instructions: [instruction],
  });

  // 3. 下一次轮询时UI更新（3秒后）
};
```

**图：完整往返流程**

```
用户点击         构建           签名 &          程序         账户
"投注YES"    →   指令     →     发送      →   执行     →   更新
                    │                               │
               生成的               验证 + 状态更改
               客户端               (checked_add, 时间检查)
```

---

## 第五部分：安全与权衡 (5分钟)

### 设计决策

| 选择 | 权衡 |
|--------|-----------|
| 创建者结算市场 | 简单但中心化信任 |
| 轮询 vs WebSockets | 代码更简单，更新略有延迟 |
| 全有或全无投注 | 没有部分持仓，数学更简单 |
| 无费用 | 无协议收入，纯粹市场 |

### 安全保护

| 攻击向量 | 保护措施 |
|---------------|------------|
| 溢出攻击 | `checked_add/mul/div` |
| 双重索赔 | `position.claimed`标志 |
| 延迟投注 | 时间窗口验证 |
| 未授权结算 | 仅创建者检查 |

---

## 第六部分：回顾 (5分钟)

### 完整图景

```
┌─────────────────────────────────────────────────────────────┐
│  1. 用户与React组件交互                                     │
│  2. 组件调用生成的指令构建器                               │
│  3. 钱包签名，交易发送到Solana                             │
│  4. Anchor程序验证并修改账户状态                          │
│  5. 前端轮询更新状态，重新渲染                             │
└─────────────────────────────────────────────────────────────┘
```

### 关键要点

1. **程序是无状态的** — 账户持有所有状态
2. **PDAs实现无需信任的托管** — 没有私钥持有资金
3. **代码生成消除序列化错误** — 构造时类型安全
4. **IDL是合约** — 连接链上和链下

---

## 时间总结

| 部分 | 时长 | 内容类型 |
|---------|----------|--------------|
| 第一部分：为什么 & 是什么 | 15分钟 | 图表 |
| 第二部分：链上程序 | 30分钟 | 代码讲解 |
| 第三部分：代码生成层 | 15分钟 | 管道 + 代码 |
| 第四部分：前端架构 | 20分钟 | 图表 + 代码 |
| 第五部分：安全与权衡 | 5分钟 | 讨论 |
| 第六部分：回顾 | 5分钟 | 总结 |
| **总计** | **90分钟** | |

---

## 文件参考

| 主题 | 文件路径 |
|-------|-----------|
| 账户结构 | `anchor/programs/prediction_market/src/state.rs` |
| 程序指令 | `anchor/programs/prediction_market/src/lib.rs` |
| 错误定义 | `anchor/programs/prediction_market/src/errors.rs` |
| 生成的客户端 | `app/generated/prediction_market/` |
| 市场列表组件 | `app/components/markets-list.tsx` |
| 市场卡片组件 | `app/components/market-card.tsx` |
| 钱包提供者 | `app/components/providers.tsx` |
| Codama配置 | `codama.json` |
