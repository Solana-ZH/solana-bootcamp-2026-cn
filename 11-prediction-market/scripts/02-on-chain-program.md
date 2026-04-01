# 第二部分：链上程序

**时长:** 30 分钟

---

## 2.1 — 账户结构 (10 分钟 | ~900 字)

<!-- 屏幕共享代码 - 讲解 state.rs -->

好的，在编写任何代码之前，我们需要决定链上存储什么。在 Solana 上，程序基本上是对账户的纯函数：每个指令接收账户，读取它们，然后将它们写回。所以账户布局是真正的 API。把这个弄对了，其他一切都变成简单的状态转换。

对于这个构建，我们保持最小化。两个账户：`Market` 用于全局状态，`UserPosition` 用于单个用户的持仓。没有订单簿，没有价格历史，没有链下引用。只是我们验证下注和支付所需的数据。这就是整个想法。

我们在 `state.rs` 中使用 `#[account]` 和 `#[derive(InitSpace)]` 定义这些。快速说明：`InitSpace` 很重要，因为 Solana 要求您预先分配空间。太小会导致交易失败，太大则会永远消耗 lamports。使用 `InitSpace` 和 `#[max_len]`，Anchor 会为我们计算大小，所以我们不需要手动计算。

**讲解: `anchor/programs/prediction_market/src/state.rs`**

```rust
pub struct Market {
    pub creator: Pubkey,        // 32 字节 - 谁可以结算
    pub market_id: u64,         // 8 字节 - 唯一 ID
    pub question: String,       // 可变 - "X 会发生吗？"
    pub resolution_time: i64,   // 8 字节 - 下注截止时间
    pub yes_pool: u64,          // 8 字节 - 总 YES lamports
    pub no_pool: u64,           // 8 字节 - 总 NO lamports
    pub resolved: bool,         // 1 字节 - 结果是否已设置？
    pub outcome: Option<bool>,  // 2 字节 - None, Some(true), Some(false)
    pub bump: u8,               // 1 字节 - PDA bump seed
}
```

让我们快速过一遍，我会保持简洁。`creator` 是可以结算市场的权限。可以把它看作是管理员。我们存储它以便后续指令可以检查签名者。它也进入 PDA seeds，所以一个创建者可以运行多个市场而不会冲突。

`market_id` 是我们传入的 u64。它只在每个创建者内唯一，就像每个创建者的序列号。PDA seed 使用 `b"market" + creator + market_id`，所以两个创建者都可以有 market_id 1 并获得不同的地址。很酷的部分是确定性发现：给定创建者和 id，客户端可以在没有任何注册表的情况下推导出市场地址。

`question` 是问题提示。我们使用 `MAX_QUESTION_LEN` 将其限制在 200 个字符，使账户保持小而可预测。Anchor 中的 `String` 包括 4 字节长度前缀加上字节，所以最大长度对租金很重要。它也保持 UI 整洁。

`resolution_time` 是 i64 Unix 时间戳。我们用它来拒绝过去创建的市场，并在截止时间后关闭下注。在 Solana 上，`Clock::get()?.unix_timestamp` 是时间来源。它不是完全精确的，但对于教程来说没问题。

`yes_pool` 和 `no_pool` 只是每边 lamports 的运行总计。当用户下注时，我们将 lamports 移动到市场 PDA 并增加其中一个池。这里没有定价曲线；隐含概率只是两个池的比率。简单的同注分彩数学，没什么花哨的。

`resolved` 和 `outcome` 一起工作。`resolved` 是一个快速防护，防止重复结算。`outcome` 是一个 `Option<bool>`，所以我们可以表示三种状态：`None`（未结算）、`Some(true)`（YES 获胜）和 `Some(false)`（NO 获胜）。这避免了"未结算"与"NO"的混淆。

`bump` 存储 PDA bump seed。我们在账户创建时计算它并存储在链上，以便后续指令可以在客户端不传递的情况下重新推导 PDA。您将在 `claim_winnings` 的账户约束中看到这一点，其中 PDA 通过 seeds 和 bump 进行验证。

接下来是 `UserPosition`。这个账户按市场按用户创建，并随时间聚合他们的下注。我们不是为每次下注创建新账户，而是保留一个账户并更新其总计。这使账户管理和 UI 逻辑保持简单。

```rust
pub struct UserPosition {
    pub market: Pubkey,      // 此持仓属于哪个市场
    pub user: Pubkey,        // 持仓的所有者
    pub yes_amount: u64,     // 在 YES 上下注的 lamports
    pub no_amount: u64,      // 在 NO 上下注的 lamports
    pub claimed: bool,       // 奖金是否已领取？
    pub bump: u8,            // PDA bump seed
}
```

所以 `market` 和 `user` 只是将此持仓绑定到特定市场和所有者。存储两者使账户自描述，并允许我们稍后添加约束，如 `user_position.user == user.key()`。

`yes_amount` 和 `no_amount` 是累积总计。我们有意允许两者都非零，这意味着用户可以通过在双方下注来对冲或改变主意。我们这里不进行任何净额结算；当我们支付时，只有获胜方计入。

`claimed` 是一个单向标志。一旦用户提取了他们的奖金，我们将其设置为 true 并拒绝任何进一步的领取。它防止了即使有人重新提交相同交易的双重支付。`bump` 扮演与 `Market` 账户中相同的角色：它让我们确定性地重新推导 PDA。

关于空间的快速说明：对于 `Market`，大小是固定字段加上 4 字节字符串长度前缀和 200 字节最大问题。对于 `UserPosition`，它主要是固定的：两个 pubkey，两个 u64 总计，一个 bool 和一个 bump。Anchor 的 `INIT_SPACE` 保持这个准确，所以我们可以分配 `8 + Market::INIT_SPACE` 和 `8 + UserPosition::INIT_SPACE` 而无需手动计算。

最终结果：一个小的、稳定的账户模型。它也很容易扩展：您可以添加 `fee_bps` 字段、`oracle` pubkey 或 `category` 枚举，而无需触及核心流程。对于教程来说，这两个账户就足够了。现在我们可以继续讨论修改它们的指令逻辑。

**讨论要点:**
- 为什么每个字段存在
- `Option<bool>` 如何表示三种状态（未结算、是、否）
- 为什么我们为每个用户的每个市场保留一个 `UserPosition`
- PDA seeds 如何提供确定性地址
- 租金豁免的空间计算

---

## 2.2a — 函数: create_market (5 分钟 | ~400 字)

<!-- 代码讲解 - 突出验证模式 -->

好的，`create_market` 是一切开始的地方。可以把它看作是设置步骤。它分配市场 PDA，存储元数据，并将资金池清零。在 Anchor 中，大部分设置位于账户结构中，而不是指令体中。

快速查看一下 `lib.rs` 中的 `CreateMarket` 账户：我们使用 `init`、`payer = creator`、`space = 8 + Market::INIT_SPACE`，以及 seeds `b"market"`、创建者公钥和 `market_id` 字节。这为我们提供了一个确定性地址。相同的创建者 + id 意味着每次都是相同的 PDA。尝试创建两次，第二次交易将失败，因为账户已存在。

输入很简单：`market_id`、问题字符串和结算时间。我们在接触状态之前验证两者。问题长度检查强制执行 `MAX_QUESTION_LEN`，使账户适合我们分配的空间。时间检查确保市场在未来；否则您将创建一个已经关闭的市场。

**讲解: `anchor/programs/prediction_market/src/lib.rs`**

```rust
pub fn create_market(
    ctx: Context<CreateMarket>,
    market_id: u64,
    question: String,
    resolution_time: i64,
) -> Result<()> {
    require!(question.len() <= MAX_QUESTION_LEN, MarketError::Overflow);

    let clock = Clock::get()?;
    require!(
        resolution_time > clock.unix_timestamp,
        MarketError::ResolutionTimeInPast
    );

    let market = &mut ctx.accounts.market;
    market.creator = ctx.accounts.creator.key();
    market.market_id = market_id;
    market.question = question;
    market.resolution_time = resolution_time;
    market.yes_pool = 0;
    market.no_pool = 0;
    market.resolved = false;
    market.outcome = None;
    market.bump = ctx.bumps.market;

    Ok(())
}
```

验证之后，我们明确填充每个字段。没有意外。这使账户可预测，并避免依赖默认值。我们还存储来自 `ctx.bumps.market` 的 bump，以便后续指令可以在客户端不每次提供 bump 的情况下验证 PDA。

顺便说一下，即使 `market_id` 已经是 PDA seeds 的一部分，我们仍然将其存储在账户上。这对 UI 很方便，并让我们稍后在 `claim_winnings` 中重新推导 PDA，而无需用户传递它。我们还将 `resolution_time` 存储为 i64，因为时钟系统变量使用 i64，所以我们避免转换。在生产环境中，您可能会在这里添加更多验证 — 最小持续时间、问题内容的健全性检查，或防止垃圾邮件的小额创建费。对于教程来说，两个防护就足够了。

一个快速的设计选择：我们不存储市场的全局列表。在 Solana 上，枚举所有账户是 RPC 或索引器的问题，而不是程序的责任。保持程序专注使其运行更便宜且更容易审计。客户端可以通过扫描 PDA 或如果需要搜索体验则通过链下索引来发现市场。

**关键点:** 验证发生在状态更改之前

---

## 2.2b — 函数: place_bet (7 分钟 | ~500 字)

<!-- 代码 + 资金流图 - 慢节奏 -->

好的，`place_bet` 是热路径。每个是/否点击都会命中它，所以我们保持它精简。可以将其视为三个步骤：验证、将 lamports 移动到市场 PDA，并更新市场和用户持仓的会计。

第一个防护：`amount > 0`。听起来很简单，但它防止了仍然可以创建持仓账户并浪费租金的空操作交易。下一个防护检查截止时间：如果 `Clock::get()?.unix_timestamp` 大于或等于 `resolution_time`，下注已关闭。我们在任何转移之前执行此操作，因此我们永远不会在截止时间后移动资金。

然后我们进行转移。没什么花哨的：我们使用系统程序将原生 lamports 从用户移动到市场 PDA。用户签署交易，市场账户只是接收资金。因为 PDA 是程序拥有的，它不需要签名来接收 lamports。如果您想用像 USDC 这样的 SPL 代币下注，这就是您将 CPI 到代币程序的地方。

**图表: 资金流**

```
   用户钱包                    市场 PDA
       │                              │
       │    转移(金额)                 │
       ├─────────────────────────────►│
       │                              │
       ▼                              ▼
  余额 -= 金额              yes_pool += 金额
                                  (或 no_pool)
```

**讲解: `anchor/programs/prediction_market/src/lib.rs`**

```rust
pub fn place_bet(
    ctx: Context<PlaceBet>,
    amount: u64,
    bet_yes: bool,
) -> Result<()> {
    require!(amount > 0, MarketError::InvalidBetAmount);

    let clock = Clock::get()?;
    let market = &ctx.accounts.market;
    require!(
        clock.unix_timestamp < market.resolution_time,
        MarketError::BettingClosed
    );

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user.to_account_info(),
                to: ctx.accounts.market.to_account_info(),
            },
        ),
        amount,
    )?;

    // 更新资金池
    let market = &mut ctx.accounts.market;
    if bet_yes {
        market.yes_pool = market.yes_pool.checked_add(amount)
            .ok_or(MarketError::Overflow)?;
    } else {
        market.no_pool = market.no_pool.checked_add(amount)
            .ok_or(MarketError::Overflow)?;
    }

    // 更新用户的持仓...
}
```

只有在转移成功后，我们才更新资金池总计。我们在 u64 上使用 `checked_add` 来防止溢出。溢出在正常使用中很少见，但它是一个常见的攻击面：如果有人可以将资金池包装为零，他们可以扭曲隐含价格或窃取资金。防御性数学保持会计合理。

接下来我们更新用户的持仓。`user_position` 账户使用 `init_if_needed` 创建，这意味着第一次下注支付租金，后续下注重用同一账户。在第一次下注时，我们设置 `market` 和 `user`，并存储 bump，以便稍后验证 PDA。在每次下注时，我们增加 `yes_amount` 或 `no_amount`。

是的，用户可以随时间在双方下注。这是有意的。一些交易者想要对冲，或者想在不关闭账户的情况下改变主意。我们不净额结算这些持仓。当市场结算时，只有获胜方计入。失败方保留在资金池中，并分配给获胜者。

这是一个简单的同注分彩模型，不是 AMM。没有曲线，没有滑点，没有价格保护。"价格"只是任何给定时刻的资金池比率。这使得逻辑易于推理，非常适合教程。一旦您理解了这个流程，如果您愿意，可以换入更高级的定价。

我们也不存储明确的价格；UI 从资金池比率动态推导它。这使链上状态保持最小，并避免额外的舍入逻辑。

**关键洞察:** `checked_add` 在溢出时返回 `None` 而不是包装。这防止了有人可能溢出资金池为零的攻击。

---

## 2.2c — 函数: claim_winnings (8 分钟 | ~550 字)

<!-- 奖金图 + 代码 - 让数学深入人心 -->

好的，`claim_winnings` 是结算步骤。它是唯一将 lamports 移出市场 PDA 的指令，所以我们放慢速度并在这里仔细检查一切。流程是：验证市场已结算，验证用户尚未领取，计算用户的份额，转移 lamports，并将领取标记为完成。

在此之前，必须有人调用 `resolve_market` 来设置结果。在本教程中，只有市场创建者可以这样做，并且他们只能在结算时间之后进行。这是我们为使程序简单而做出的信任假设。在生产环境中，您通常会将其替换为预言机或多签，以避免单点故障。

**图表: 奖金计算**

```
示例市场:
┌─────────────────────────────────────┐
│  YES 池: 100 SOL                    │
│  NO 池:   50 SOL                    │
│  结果: YES 获胜                     │
└─────────────────────────────────────┘

用户在 YES 上下注 10 SOL:
┌─────────────────────────────────────┐
│  获胜者份额: 10/100 = 10%           │
│  从失败者池领取: 50 × 10% = 5       │
│  总支付: 10 + 5 = 15 SOL            │
└─────────────────────────────────────┘
```

**讲解: `anchor/programs/prediction_market/src/lib.rs`**

```rust
pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
    let market = &ctx.accounts.market;
    let position = &ctx.accounts.user_position;

    // 防护
    require!(market.resolved, MarketError::NotResolved);
    require!(!position.claimed, MarketError::AlreadyClaimed);

    // 确定获胜方
    let outcome = market.outcome.unwrap();
    let (user_winning_bet, total_winning_pool, total_losing_pool) = if outcome {
        (position.yes_amount, market.yes_pool, market.no_pool)
    } else {
        (position.no_amount, market.no_pool, market.yes_pool)
    };

    require!(user_winning_bet > 0, MarketError::NoWinnings);

    // 计算奖金
    let winnings = (user_winning_bet as u128)
        .checked_mul(total_losing_pool as u128)
        .ok_or(MarketError::Overflow)?
        .checked_div(total_winning_pool as u128)
        .ok_or(MarketError::Overflow)? as u64;
    let total_payout = user_winning_bet
        .checked_add(winnings)
        .ok_or(MarketError::Overflow)?;

    // 从 PDA 转移到用户
    let market_account_info = ctx.accounts.market.to_account_info();
    let user_account_info = ctx.accounts.user.to_account_info();

    **market_account_info.try_borrow_mut_lamports()? -= total_payout;
    **user_account_info.try_borrow_mut_lamports()? += total_payout;

    let position = &mut ctx.accounts.user_position;
    position.claimed = true;

    Ok(())
}
```

相当直接：第一个防护是 `market.resolved` 和 `!position.claimed`。简单的检查，但很重要。没有它们，用户可以在结算前领取或多次领取。我们还通过检查 `user_winning_bet > 0` 来验证用户有获胜的下注。如果您只在失败方下注，就没有什么可领取的。

我们通过读取 `market.outcome` 来确定获胜方。因为市场已结算，这个选项应该是 `Some(true)` 或 `Some(false)`。然后我们选择用户的获胜金额以及总获胜和失败资金池。这就是我们支付所需的一切；我们不存储任何每注历史。

奖金公式是同注分彩：`奖金 = (用户下注 / 获胜池) * 失败池`。在实际代码中，我们使用 u128 数学来避免大数相乘时的溢出。整数除法向下取整结果，因此市场 PDA 中可能留下一些 lamports。这在整数算术中是正常的，并使程序具有确定性。

要转移 lamports，我们不能调用系统程序。市场 PDA 是程序拥有的，系统程序只从签名者账户移动 lamports。相反，我们直接改变市场和用户账户上的 lamports 字段。这是安全的，因为程序拥有市场账户，并且我们已经在账户约束中验证了用户和 seeds。

我们还依赖这些账户约束来确保 `user_position` PDA 与签名者匹配。

转移后，我们将 `position.claimed` 翻转为 true。这使得领取是幂等的：如果用户再次提交交易，它将在防护处失败。在这个版本中，我们不关闭持仓账户，但我们可以添加一个 `close = user` 约束，以便在领取后回收租金。

最后一个值得指出的细微差别：如果用户在双方都下注，只有获胜方获得支付。失败方保留在资金池中并分配给获胜者，如果他们也有获胜的下注，这包括他们自己。这即使在用户对冲时也保持数学一致性。总体总计仍然平衡，因为进入市场的所有 lamports 要么被支付，要么作为零头保留。

这就完成了链上流程。我们现在有了一个完整的循环：创建市场、下注、结算市场和领取奖金。接下来，我们将采用这个程序接口并生成一个 TypeScript 客户端，以便前端可以类型安全地调用这些指令。