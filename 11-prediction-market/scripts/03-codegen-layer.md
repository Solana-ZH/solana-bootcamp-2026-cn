# 第三部分：代码生成层

**时长:** 15 分钟

---

## 3.1 — 流水线 (5 分钟 | ~350 字)

<!-- 图表: Rust → IDL → TypeScript -->

好的，第三部分全是关于代码生成层。这是我们的 Rust 程序和 TypeScript 前端之间的桥梁。没有它，您将需要手动编写 PDA、指令数据和账户解码器。这很慢，容易出错，老实说也不有趣。

核心理念很简单：Rust 程序是真相来源，IDL 是合约，生成的客户端是我们在 UI 中实际使用的东西。当程序更改时，我们重新生成客户端，使一切保持同步。无需猜测字节布局，无需从文档复制账户顺序，没有静默不匹配。

**图表: 代码生成流程**

```
┌─────────────┐      ┌─────────────┐      ┌─────────────┐
│   lib.rs    │      │  IDL.json   │      │ TypeScript  │
│   (Rust)    │─────▶│ (Schema)    │─────▶│  Client     │
└─────────────┘      └─────────────┘      └─────────────┘
     anchor build         codama:js

   真相来源             中间表示          前端导入的内容
```

**命令:**
```bash
npm run anchor-build   # Rust → IDL
npm run codama:js      # IDL → TypeScript
```

`anchor-build` 编译程序并生成 IDL JSON。该文件是一个模式：指令、账户、类型和错误。然后 `codama:js` 读取 IDL 并输出一个 TypeScript 客户端，包含指令构建器、PDA 助手、账户解码器和错误类型。所有这些都放在 `app/generated/prediction_market/` 中。

快速工作流程说明：每当您更改 Rust 账户、指令或错误时，都应重新运行这两个命令。这使客户端保持最新。将生成的代码视为只读。您不手动编辑它；您重新生成它。

如果您在 UI 中看到奇怪的不匹配，首先要检查的是 IDL 和生成的客户端是否是最新的。最快的修复通常是：重新构建、重新生成和重新加载。这消除了整个"为什么这个反序列化出错"的 bug 类别。

所以流水线简短而无聊，这是好事。它让您专注于实际逻辑而不是字节布局。

还有一个说明：IDL 在 TypeScript 之外也很有用。其他团队可以使用它生成不同语言的客户端。这使您的链上程序更具可移植性，而无需您做额外工作。

如果您想检查程序暴露了什么，请打开 `anchor/target/idl/` 中的 IDL。您可以看到完整的指令列表、账户布局和确切的字段名称。当 UI 中感觉不对劲时，这是一个很好的调试工具。

老实说，生成的代码本身就是一个很好的学习资源。您可以打开任何指令文件，准确查看哪些账户是必需的，哪些是可写的，以及期望哪个签名者。这就像一个活的规范。

---

## 3.2 — 生成代码讲解 (10 分钟 | ~800 字)

<!-- 并排显示 IDL 与 TypeScript -->

让我们看看生成的客户端实际给了我们什么。我们将使用 `place_bet` 作为示例，因为它涉及所有内容：指令参数、PDA 和序列化。

**从 IDL 到 TypeScript:**

```json
// IDL 片段 (anchor/target/idl/prediction_market.json)
{
  "name": "placeBet",
  "args": [
    { "name": "amount", "type": "u64" },
    { "name": "betYes", "type": "bool" }
  ]
}
```

```typescript
// 生成: app/generated/prediction_market/instructions/placeBet.ts
export async function getPlaceBetInstructionAsync(
  input: PlaceBetAsyncInput
): Promise<PlaceBetInstruction> {
  // 从 seeds 自动推导 PDA
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

左边，IDL 只是声明指令名称和参数。右边，Codama 将其转换为我们可以从 UI 调用的函数。我们传递普通输入，它为我们完成所有困难的工作。

首先，它推导 PDA。它知道 `Market` 和 `UserPosition` 的 seeds，因为它们都在 IDL 中，所以它可以使用与程序相同的 seeds 调用 `findMarketPda` 和 `findPositionPda`。这意味着我们永远不必在 JavaScript 中重新实现该逻辑。

其次，它将参数序列化为字节。Rust 在底层使用 Borsh，顺序很重要。生成的编码器确保字节与程序完全匹配。对于 u64 值，您会注意到 TypeScript 中一切都是 `bigint`，这是一件好事。它防止您溢出普通的 JS 数字。

第三，它以正确的顺序构建账户元数据，带有正确的可写和签名者标志。如果您手动编写指令，这是最容易出错的地方之一。生成的客户端使您免于整个这类 bug。

生成的包还包括账户解码器。例如，`getMarketDecoder()` 知道如何从原始字节读取 `Market` 账户，包括 8 字节的 Anchor 鉴别器。这意味着您可以使用 `getProgramAccounts` 获取账户，然后使用与程序相同的布局解码它们。再次强调，无需手动布局计算。

您还会看到错误和类型的助手。如果程序返回 `MarketError::BettingClosed`，生成的客户端可以将其映射到前端的类型化错误。这使得更容易显示真实的错误消息，而不是"交易失败"。

另一个好处是生成的客户端同时暴露编码器和解码器。这意味着您可以编写小型测试来编码指令数据并与预期字节进行比较，或者解码从 RPC 捕获的账户 blob。这是一种轻量级的方式来验证您的前端和程序是否对齐。

所以在实践中，生成的客户端几乎自动地将链上更改转换为前端更改。您更新 Rust，运行两个命令，UI 就针对新类型进行编译。这是一个巨大的生产力提升，并保持代码库的诚实性。

这就是为什么我们在流水线上花时间。它消除了整个脆弱的胶水代码层，让您快速前进而不会破坏东西。

一个小工作流程提示：决定是否要提交生成的客户端。对于教程，我通常提交它，以便任何人都可以克隆和运行而无需额外步骤。在更大的团队中，您可能改为在 CI 中重新生成。无论哪种方式，都将生成的文件夹视为构建输出，而不是手写代码。

生成的输出中有一些额外的好处值得一提。您将获得程序 ID 作为常量，这防止您的前端漂移到错误的地址。您还将获得类型化的账户接口，因此如果您悬停在解码的 `Market` 对象上，您会看到确切的字段及其类型。这使得意外将 u64 视为普通数字变得更难。

指令构建器通常有同步和异步版本。当涉及 PDA 时，异步版本很方便，因为它们需要推导地址。同步版本在测试或脚本中可能很有用，您已经计算了地址。

您还会获得账户鉴别器。Anchor 使用账户的前 8 个字节来标识类型，生成的客户端知道这些鉴别器。这意味着您可以安全地过滤程序账户，并且只解码您期望的账户。

人们经常错过的一件事：客户端编码器和解码器是纯函数。这意味着您可以在隔离环境中测试它们。如果您想检查新字段，解码本地固定装置或编码样本并与程序期望的内容进行比较。这是一个很好的、紧密的反馈循环。

所以当您听到"代码生成"时，不要认为它是一个可有可无的东西。它是一个护栏。它在您迭代时保持链上和链下层的锁定。

**关键洞察:** 生成的客户端处理两个难题：
1. **PDA 推导** - 从 seeds 计算确定性地址
2. **序列化** - 将参数编码为与 Rust 的 Borsh 格式匹配的字节

您永远不需要手动编写这些。