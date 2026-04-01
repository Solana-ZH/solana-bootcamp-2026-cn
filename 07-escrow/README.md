# Anchor Escrow

一个使用 [Anchor](https://www.anchor-lang.com/) 在 Solana 上实现的无需信任代币交换(Escrow)程序. 它演示了如何把 PDA 用作托管金库, 如何使用 `transfer_checked` 做 CPI, 以及如何使用 token interface, 从而同时兼容传统 SPL Token 与 Token-2022 mint.

## 为什么要做 Escrow?

Escrow 是入门 Solana 开发时非常值得练手的项目之一, 覆盖了大量核心能力:

- 创建和管理作为托管代币金库的 PDA
- 通过 `transfer_checked` 发起跨程序调用(CPI)
- 使用 token interface(`TokenInterface`)同时支持 SPL Token 与 Token-2022
- 关闭账户并将租金返还给指定账户
- 使用 `has_one` 约束在链上强制执行关键不变量

这些模式是更复杂 DeFi 程序(如 AMM, 借贷协议, 订单簿)的基础.

## 概览

两方参与者, 即 **maker** 与 **taker**, 无需彼此信任, 也不需要第三方担保, 就可以完成代币交换. maker 先把代币 A 存入程序控制的金库, 并声明希望换回多少代币 B. 任意持有代币 B 的 taker 都可以原子化完成交换.

```
maker 发起报价(make_offer)
    maker 的 mint_a -> offer_token_account(由 offer PDA 控制)

taker 接受报价(take_offer)
    taker 的 mint_b -> maker 的 mint_b ATA
    offer_token_account 的 mint_a -> taker 的 mint_a ATA
    offer_token_account 关闭(租金返还 maker)
    offer 账户关闭(close = maker)
```

## Program ID

```
25Q841qjRsaGQzWSKh5kiEZ9qpXbWMzm3v4ytGXs6PzY
```

## 前置依赖

- [Rust](https://rustup.rs/)
- [Solana CLI](https://solana.com/developers/guides/getstarted/setup-local-development)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation)
- [Node.js](https://nodejs.org/) + [Yarn](https://yarnpkg.com/)

## 构建

```bash
$ anchor build
```

## 测试

```bash
$ anchor test
```

## 指令

### `make_offer`

创建一个报价(Offer). maker 会把 `token_a_offered_amount` 数量的代币 A 存入托管账户, 并记录自己希望收到的代币 B 数量.

|           参数           | 类型 |                             说明                             |
| ------------------------ | ---- | ------------------------------------------------------------ |
| `offer_id`               | u64  | 报价 ID, 用于和 maker 一起推导 offer PDA, 可并行创建多个报价 |
| `token_a_offered_amount` | u64  | maker 提供的代币 A 数量                                      |
| `token_b_wanted_amount`  | u64  | maker 希望收到的代币 B 数量                                  |

约束说明:

- `mint_a` 与 `mint_b` 不能相同, 否则抛出 `SameMint`

### `take_offer`

原子化完成交换:

1. 从 taker 向 maker 转账 `offer.token_b_wanted_amount` 数量的代币 B
2. 从 `offer_token_account` 向 taker 转账 `offer.token_a_offered_amount` 数量的代币 A(由 offer PDA 签名)
3. 关闭 `offer_token_account`(租金返还给 maker)
4. `offer` 账户通过 `close = maker` 自动关闭(租金返还给 maker)

## 账户

### `Offer` — PDA seeds: `["offer", maker_pubkey, offer_id (little-endian u64)]`

|           字段           |  类型  |            说明             |
| ------------------------ | ------ | --------------------------- |
| `maker`                  | Pubkey | 创建报价的钱包地址          |
| `mint_a`                 | Pubkey | maker 提供的代币 mint       |
| `mint_b`                 | Pubkey | maker 希望收到的代币 mint   |
| `offer_id`               | u64    | 报价 ID                     |
| `token_a_offered_amount` | u64    | 报价中提供的代币 A 数量     |
| `token_b_wanted_amount`  | u64    | 报价中希望收到的代币 B 数量 |
| `bump`                   | u8     | PDA bump 种子               |

### `offer_token_account`

由 `Offer` PDA 持有的 mint_a ATA, 用于托管 maker 存入的代币 A. 在 `take_offer` 完成时关闭.

## Token Interface

本程序使用 `TokenInterface` / `InterfaceAccount<Mint>` / `InterfaceAccount<TokenAccount>`, 而不是绑定具体 SPL Token 类型. 这意味着同一套程序代码无需修改即可同时支持 **legacy SPL Token** 与 **Token-2022** mint.

## 错误码

|      代码       |                   消息                   |
| --------------- | ---------------------------------------- |
| `MakerMismatch` | Maker account does not match Offer.maker |
| `MintMismatch`  | Mint account does not match Offer mints  |
| `SameMint`      | mint_a and mint_b must be different      |
