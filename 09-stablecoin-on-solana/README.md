# Stablecoin 程序

这是一个基于 [Anchor](https://www.anchor-lang.com/) 构建的 Solana 稳定币程序，演示了如何发行和管理一个带有受控铸币、额度管理以及紧急暂停功能的 Token-2022（Token Extensions）代币。

## 概览

该程序创建了一个 Token-2022 mint，其权限由程序自身拥有的 PDA 控制，因此不会由某一个私钥直接掌握发行权。管理员账户负责管理一组被授权的铸币人，每个铸币人都有独立的额度上限，用来限制其累计可铸造的代币数量。这种模式类似现实中的稳定币（例如 USDC）如何将管理员角色与具体执行铸币的操作角色分离。

## 功能特性

- **Token-2022 mint**：使用 Token Extensions 程序（`TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`）
- **受控铸币**：只有被授权的铸币人才能创建新代币
- **按铸币人划分额度**：每个铸币人都有自己的累计铸币上限
- **紧急暂停**：管理员可以立即暂停全部铸币行为
- **代币销毁**：任何用户都可以销毁自己持有的代币（例如用于法币赎回）
- **回收租金**：移除某个铸币人时，会关闭其配置账户并将租金返还给管理员

## 程序 ID

```text
rYXfi25x9JMgau82aGMJMVUokq7JzueqehiJUmwR97Q
```

## 前置要求

- [Rust](https://rustup.rs/)（所需版本见 `rust-toolchain.toml`）
- [Solana CLI](https://solana.com/developers/guides/getstarted/setup-local-development)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) 0.32.1

## 构建

```bash
anchor build
```

## 测试

测试使用 [LiteSVM](https://github.com/LiteSVM/litesvm) —— 一个进程内运行的 Solana VM，无需启动本地 validator，因此测试更快且结果更稳定。

```bash
cargo test -p stablecoin
```

全部 18 个测试都应通过。

## 指令

### `initialize`

创建稳定币 mint 和配置账户。其他任何指令都必须在它之后才能调用，并且它只能执行一次。

- 创建 `Config` PDA，种子为 `["config"]`
- 创建 Token-2022 mint PDA，种子为 `["mint"]`
- 将 `Config` PDA 设置为 mint authority 和 freeze authority

### `configure_minter`

授权一个新的铸币人，或更新已有铸币人的额度。仅管理员可调用。

```text
allowance: u64  — 该铸币人的累计铸币上限
```

### `remove_minter`

撤销某个铸币人的授权。会关闭该 `MinterConfig` 账户，并将租金返还给管理员。仅管理员可调用。

### `mint_tokens`

向任意目标代币账户铸币（若目标 ATA 不存在则自动创建）。在以下情况下会回滚：

- 程序处于暂停状态
- 调用者没有对应的 `MinterConfig`
- 请求的数量超过该铸币人的剩余额度（`allowance - amount_minted`）

### `burn_tokens`

从调用者自己的代币账户中销毁代币。任何人都可以调用该指令。

### `pause` / `unpause`

切换 `Config` 账户上的全局 `paused` 标志。当 `paused = true` 时，所有 `mint_tokens` 调用都会失败。仅管理员可调用。

## 账户结构

### `Config` — PDA seeds: `["config"]`

| 字段        | 类型     | 说明                                                             |
| ----------- | -------- | ---------------------------------------------------------------- |
| `admin`     | `Pubkey` | 可以管理铸币人并执行暂停/恢复                                    |
| `mint`      | `Pubkey` | Token-2022 mint 地址                                             |
| `paused`    | `bool`   | 为 `true` 时禁止铸币                                             |
| `bump`      | `u8`     | PDA bump seed                                                    |
| `mint_bump` | `u8`     | `mint` PDA 的 bump，用于后续账户约束校验该 `mint` 是否为预期 PDA |

同时作为 mint::authority

### `MinterConfig` — PDA seeds: `["minter", minter_pubkey]`

| 字段             | 类型     | 说明                                                                   |
| ---------------- | -------- | ---------------------------------------------------------------------- |
| `minter`         | `Pubkey` | 被授权的铸币人公钥                                                     |
| `allowance`      | `u64`    | 该铸币人一生中最多可铸造的代币数量                                     |
| `amount_minted`  | `u64`    | 该铸币人已累计铸造的代币数量                                           |
| `is_initialized` | `bool`   | 在首次调用 `configure_minter` 时设为 true                              |
| `bump`           | `u8`     | `minter_config` PDA 的 bump，用于后续账户约束校验它是否属于该 `minter` |

## Token-2022 说明

该 mint 是通过 **Token Extensions 程序**（`Token-2022`）创建的。这个 mint 对应的关联代币账户（ATA）与传统 SPL Token 使用不同的派生路径，因为 Token Program ID 也会被纳入 seeds：

```text
ATA = find_program_address(
    [wallet, TOKEN_2022_PROGRAM_ID, mint],
    ATA_PROGRAM_ID
)
```

所有 CPI 调用（mint、burn）都直接指向 `anchor_spl::token_2022::ID`，这是 Anchor v1.0.0-rc.2 下所要求的用法。

## 许可证

MIT
