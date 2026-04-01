# StableSwap AMM

这是一个基于 [Anchor](https://www.anchor-lang.com/) 构建在 Solana 上的双代币流动性池，专门针对稳定币交易对进行了优化。它实现了 [Curve StableSwap invariant](https://curve.fi/files/stableswap-paper.pdf)，因此在交换价格接近的资产（例如 USDC/USDT）时，相比恒定乘积 AMM，可以显著降低滑点。

## 工作原理

标准 AMM（如 Uniswap 风格的 `x·y = k`）即使面对中等规模的交易，也会产生明显价格冲击。对于那些应当接近 `1:1` 交易的资产，StableSwap invariant：

```text
4·A·(x + y) + D  =  4·A·D + D³ / (4·x·y)
```

会把流动性集中在锚定价格附近，因此在储备平衡时可以实现接近零滑点的交换，同时当价格偏离锚定时，曲线仍能自动回到更稳健的状态。

**放大系数 A（amplification parameter）** 控制这种权衡：

- **高 A（100–2000）**：曲线在 peg 附近几乎是平的，滑点极低，适合 USDC/USDT
- **低 A（1–10）**：更接近恒定乘积，对脱锚场景的适应性更强

### 示例

在 `A = 100` 时，交换相当于池子规模 `10%` 的资产：

- **StableSwap**：输出效率约为 `99.9%`
- **恒定乘积**：输出效率约为 `90.9%`

## Program ID

```text
CorabfeniSyoc4aLcJe7t9b3RaFX5tzVWXdewU1xuA6B
```

## 前置要求

- [Rust](https://rustup.rs/)
- [Solana CLI](https://solana.com/developers/guides/getstarted/setup-local-development)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) v1.0.0-rc.2
- [Node.js](https://nodejs.org/) + [Yarn](https://yarnpkg.com/)

## 构建

```bash
anchor build
```

## 测试

```bash
yarn install
anchor test
```

测试会在本地 validator 上覆盖完整的池子生命周期，包括：

- 池子初始化
- 初始与后续流动性添加
- A→B 和 B→A 交换
- 交换和提取流动性时的滑点保护
- 移除流动性
- StableSwap 与恒定乘积 AMM 的效率对比

## 指令

### `initialize_pool`

为 token A / token B 交易对创建一个新的池子。

| 参数 | 类型 | 说明 |
|------|------|------|
| `amplification` | `u64` | A 参数（1–1,000,000）。对稳定币通常使用 100–2000。 |
| `fee_bps` | `u16` | 交换手续费，单位为 basis points（例如 `4` = `0.04%`）。 |

该指令会自动创建：

- pool PDA
- LP mint
- 两个 vault ATA

### `add_liquidity`

向池子中存入 token A 和/或 token B，以获得代表池子份额的 LP token。

| 参数 | 类型 | 说明 |
|------|------|------|
| `amount_a` | `u64` | 要存入的 token A 数量。 |
| `amount_b` | `u64` | 要存入的 token B 数量。 |
| `min_lp_out` | `u64` | 至少应收到的 LP token 数量（滑点保护）。 |

首次存款会建立初始价格。后续存款可以是不平衡的，但 LP token 的铸造数量将按池子 invariant `D` 的变化比例计算。

### `swap`

使用 StableSwap invariant 将一种代币交换为另一种。

| 参数 | 类型 | 说明 |
|------|------|------|
| `amount_in` | `u64` | 输入代币数量。 |
| `min_amount_out` | `u64` | 至少应收到的输出代币数量（滑点保护）。 |
| `a_to_b` | `bool` | `true` 表示 A→B，`false` 表示 B→A。 |

### `remove_liquidity`

销毁 LP token，并按比例提取两种底层资产。

| 参数 | 类型 | 说明 |
|------|------|------|
| `lp_amount` | `u64` | 要销毁的 LP token 数量。 |
| `min_a` | `u64` | 至少应收到的 token A 数量（滑点保护）。 |
| `min_b` | `u64` | 至少应收到的 token B 数量（滑点保护）。 |

## 账户结构

```text
Pool PDA  seeds: ["pool", mint_a, mint_b]
├── token_mint_a   — token A 的 mint 地址
├── token_mint_b   — token B 的 mint 地址
├── vault_a        — 池子持有 token A 的 ATA（owner 为 pool PDA）
├── vault_b        — 池子持有 token B 的 ATA（owner 为 pool PDA）
├── lp_mint        — LP token mint（authority = pool PDA）
├── amplification  — A 参数
└── fee_bps        — 交换手续费
```

## 安全性

- **防 LP inflation attack**：首次存款会锁定 `MINIMUM_LIQUIDITY = 1000` 作为虚拟死股份，避免攻击者通过捐赠极小数量资产来操纵 LP 价格
- **滑点保护**：所有指令都支持用户指定最小输出值，不满足则回滚
- **溢出保护**：所有数学运算都使用 `checked_*`；Newton–Raphson 迭代最多执行 255 次
