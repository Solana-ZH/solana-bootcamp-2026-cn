# 第四部分：前端架构

**时长:** 20 分钟

---

## 4.1 — 组件层次结构 (5 分钟 | ~300 字)

<!-- 树状图 - 主要是视觉内容 -->

好的，让我们放大看看前端结构。我们使用 Next.js App Router，所以顶层布局将所有内容包装在提供者中。这使得 Solana 连接和钱包状态在整个应用程序中可用。

心理模型很简单：一个带有提供者包装器的布局，然后是两个页面。主页是您创建和浏览市场的地方。活动页面是您查看过去持仓的地方。其他一切都挂在这两个根上。

保持树结构浅层有很大帮助。它清楚地显示了数据流动的位置和副作用存在的位置。您会看到任何与钱包相关或与 RPC 相关的内容都位于 `Providers` 内部，而其他所有内容都只是 UI 组件。

**图表: 组件树**

```
App (layout.tsx)
 └─ Providers (Solana 客户端 + 钱包)
     ├─ HomePage (page.tsx)
     │   ├─ CreateMarketForm
     │   └─ MarketsList
     │       └─ MarketCard (×N)
     │
     └─ ActivityPage (activity/page.tsx)
         └─ PositionsList
             └─ PositionCard (×N)
```

这并不花哨，而这正是重点。一个小的树结构使教程保持专注，并使得找到给定行为的位置变得容易。如果您以后需要添加更多页面，您已经有一个干净的起点。

一个实际细节：`Providers` 是一个客户端组件。钱包适配器依赖于 `window`，所以您保持这个边界清晰。它下面的所有内容都可以是客户端，而布局本身可以保持服务器渲染。这为您提供了良好的 Next.js 默认设置，而无需与钱包对抗。

在主页上，`CreateMarketForm` 只是一个带有问题输入和结算时间选择器的表单。它调用 `create_market` 指令，然后重置 UI。`MarketsList` 是读取端：它获取市场并为每个市场渲染一个 `MarketCard`。

在活动页面上，`PositionsList` 对用户持仓做同样的事情。每个 `PositionCard` 可以显示下注金额、结果以及持仓是否已被领取。这使得流程感觉完整，而无需添加一堆额外的复杂性。

`Providers` 也是我们设置集群和连接的地方。在这个仓库中，它是 devnet，但它可以是主网、本地网络或自定义 RPC。您通常在 `app/components/providers.tsx` 中设置这个，并保持集中化，以便每个组件获得相同的连接和钱包上下文。

我们还保持状态局部于需要它的组件。`MarketCard` 获取解码后的市场加上几个回调。没有全局存储，没有繁重的状态管理。对于教程来说，这保持了心理负担低。

---

## 4.2 — 数据获取模式 (8 分钟 | ~700 字)

<!-- markets-list.tsx 的代码讲解 -->

对于数据获取，我们保持简单和可预测。在本教程中，我们不使用索引器或数据库层。前端直接与 RPC 通信，拉取原始账户，并使用生成的客户端解码它们。

这主要发生在 `markets-list.tsx` 中。它调用 `getProgramAccounts`，按市场鉴别器过滤，然后将每个账户解码为可用的对象。就是这样。

**讲解: `app/components/markets-list.tsx`**

```typescript
// 从程序中获取所有市场账户
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

那么这里发生了什么？`getProgramAccounts` 返回程序拥有的每个账户。memcmp 过滤器是我们缩小范围的方式。Anchor 在每个账户类型前添加一个8字节的鉴别器，因此通过过滤该鉴别器，我们只获得 `Market` 账户，跳过 `UserPosition` 账户。

一旦我们有了原始账户数据，我们就用 `getMarketDecoder()` 解码它。这来自代码生成层，这意味着布局与 Rust 结构完全匹配。我们不手动解析字节，也不冒细微错误的风险。

在实践中，您会将其包装在 try/catch 中并设置加载状态。如果 RPC 调用失败或解码器抛出错误，您可以显示一个简单的"重试"按钮。在教程中，我们可以保持轻量，但在真实应用程序中，您需要一些针对不稳定 RPC 的护栏。

每3秒轮询一次是一个有意识的权衡。它简单可靠，但不是实时的。对于生产环境，您可能会切换到 WebSockets 或索引器，但对于教程来说，轮询方法保持代码简短且易于理解。

在客户端解码的一个很好的副作用是我们可以即时计算派生字段。例如，我们可以从 `yes_pool` 和 `no_pool` 计算隐含概率，或将总流动性显示为 `yes_pool + no_pool`。这些都不需要存在于链上。

如果您想添加缓存，React Query 或 SWR 是自然的下一步。但再次强调，对于教程，我们保持最小化和可读性。

还要注意 `useEffect` 中的清理。这很重要。如果您离开页面，您不希望多个间隔叠加。简单的 `clearInterval` 保持安全。

在更大的应用程序中，您可能将获取逻辑提取到自定义钩子中，如 `useMarkets`，并在页面间共享。但对于本教程，将逻辑保持在组件附近使其更容易理解。

在活动页面上，模式类似，但针对 `UserPosition` 账户。您可以在客户端获取所有持仓并过滤，或者为用户公钥添加 memcmp 过滤器。偏移量有点棘手，因为账户首先有一个鉴别器和一个市场公钥，但一旦计算出来，您就可以高效过滤。

您还可以在客户端对市场进行排序和分组。例如，您可以通过比较 `resolution_time` 与 `Date.now() / 1000` 来拆分活跃与已结算的市场。这保持 UI 清洁，而无需添加额外的链上字段。

如果市场数量增长，`getProgramAccounts` 将开始感觉沉重。那时您会切换到索引器或至少缓存结果。但对于工作坊规模的演示来说，这完全没问题。

还有一个小的细节：资金池存储为 lamports，因此 UI 在显示值时应转换为 SOL。这只是 `lamports / LAMPORTS_PER_SOL`，但保持该转换集中化避免了差一错误，并使 UI 保持一致。

**图表: 鉴别器过滤**

```
程序拥有许多账户：
┌──────────────────┐
│ [鉴别器: 市场]   │ ← 匹配过滤器 ✓
│ 问题: "..."      │
└──────────────────┘
┌──────────────────┐
│ [鉴别器: 持仓]   │ ← 不匹配 ✗
│ 用户: 0x...      │
└──────────────────┘
┌──────────────────┐
│ [鉴别器: 市场]   │ ← 匹配过滤器 ✓
│ 问题: "..."      │
└──────────────────┘
```

这个小过滤器做了很多工作。它保持 RPC 负载较小，并使解码器逻辑专注于单一账户类型。

---

## 4.3 — 交易流程 (7 分钟 | ~500 字)

<!-- 代码 + 往返图 -->

现在让我们看看下注的快乐路径。模式对于创建、结算和领取都是相同的：使用生成的客户端构建指令，用钱包发送它，然后让 UI 在下一次轮询时更新。

**讲解: `app/components/market-card.tsx` 中的下注**

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

  // 3. UI 在下一次轮询时更新 (3秒)
};
```

这里的关键是指令构建器是完全类型化的。如果您为 `amount` 传递了错误的类型或忘记了必需的账户，TypeScript 会立即告诉您。这节省了大量时间。

在 `sendTransaction` 之后，您可以显示一个 toast，设置一个本地"待处理"标志，或乐观地更新 UI。在本教程中，我们保持简单，并依赖轮询来刷新。这使 UI 与链上状态保持一致，而无需构建一堆额外的状态管理。

还要注意 `BigInt` 转换。Rust 中的 u64 值映射到 TypeScript 中的 `bigint`，因此我们总是将 `solAmount` 转换为 lamports，然后转换为 `BigInt`。这避免了大数字的精度问题。

如果您想要更强的用户体验，可以等待确认。钱包适配器通常会立即给您签名，然后您可以调用 `connection.confirmTransaction(signature)`，并仅在它落地后清除待处理状态。这为您提供了更准确的加载指示器。

相同的模式适用于其他指令。`create_market` 使用问题和结算时间构建不同的指令，`resolve_market` 只需要创建者签名，`claim_winnings` 使用用户的持仓 PDA。一旦您学会了一个指令的流程，其余的感觉就很熟悉了。

在实践中，您还会希望将处理程序包装在 try/catch 中。如果钱包拒绝签名或程序抛出错误，您可以显示友好的错误消息。生成的客户端已经为您提供了类型化的错误，因此您可以将它们映射到消息，如"下注已关闭"，而不是通用的失败。

对于 `create_market`，主要的 UI 工作是将用户的日期输入转换为 Unix 时间戳。简单的 `Math.floor(date.getTime() / 1000)` 使其与程序的 `resolution_time` 对齐。这是一个小细节，但如果弄错了，您将立即遇到"ResolutionTimeInPast"错误。

您还可以添加小的用户体验改进，如在下注按钮交易待处理时禁用它，或在卡片上显示当前资金池大小。这些不会改变架构，但它们使应用程序感觉更流畅。

如果在测试期间遇到"交易太大"或"未找到区块哈希"，通常是 devnet 或钱包计时问题。使用新的区块哈希重试可以修复它。对于更重的程序，您可能会添加计算预算指令，但这个程序很小，所以不需要。

您可以使用的另一个模式是乐观 UI。您可以暂时更新卡片中的资金池总计，然后在下次轮询时协调。这是可选的，但它使 UI 感觉快速，即使 RPC 很慢。

**图表: 完整往返流程**

```
用户点击         构建           签名 &           程序          账户
"下注是"     →   指令    →     发送      →    执行     →    更新
                        │                               │
                   生成的               验证 + 状态变更
                   客户端               (checked_add, 时间检查)
```

这个流程在整个应用程序中重复。一旦您为 `place_bet` 学会了它，其他所有内容都只是相同模式的变化。