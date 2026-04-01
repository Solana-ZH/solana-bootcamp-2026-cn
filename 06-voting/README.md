# 投票程序

一个使用 [Anchor](https://www.anchor-lang.com/) 在 Solana 上构建的链上投票程序。演示了如何使用 PDA 存储结构化状态、通过指令传递类型化参数以及强制执行时间门控访问规则。

## 概述

任何人都可以创建一个包含名称、描述和投票窗口的投票。投票创建者随后添加候选选项。在投票窗口期间，任何钱包都可以为候选者投票；窗口外的投票将在链上被拒绝。

## 程序 ID

```
65KHV8cXwJ8apTKMqnpSdhdHkHhRySatgKMwnxm6C3gG
```

## 先决条件

- [Rust](https://rustup.rs/)
- [Solana CLI](https://solana.com/developers/guides/getstarted/setup-local-development)
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) v1.0.0-rc.2
- [Node.js](https://nodejs.org/) + [Yarn](https://yarnpkg.com/)

## 构建

```bash
cd anchor
anchor build
```

## 测试

测试使用 TypeScript 编写，使用 [@anchor-lang/core](https://www.npmjs.com/package/@anchor-lang/core) 并在本地验证器上使用 Jest 运行。

```bash
cd anchor
yarn install
yarn jest
```

## 指令

### `initialize_poll`

创建一个新的投票账户。

| 参数          | 类型     | 描述                                  |
|---------------|----------|---------------------------------------|
| `poll_id`     | u64      | 用作 PDA seed 的唯一标识符            |
| `start_time`  | u64      | 投票开始的 Unix 时间戳                |
| `end_time`    | u64      | 投票结束的 Unix 时间戳                |
| `name`        | String   | 投票名称（最多 32 个字符）            |
| `description` | String   | 投票描述（最多 280 个字符）           |

### `initialize_candidate`

向现有投票添加候选选项。

| 参数        | 类型     | 描述                                  |
|-------------|----------|---------------------------------------|
| `poll_id`   | u64      | 要添加候选者的投票 ID                 |
| `candidate` | String   | 候选者名称，也用作 PDA seed           |

### `vote`

为候选者投票。如果当前时间不在投票窗口内，则操作将被回滚。

| 参数        | 类型     | 描述                          |
|-------------|----------|-------------------------------|
| `poll_id`   | u64      | 投票 ID                       |
| `candidate` | String   | 要投票的候选者名称            |

## 账户

### `PollAccount` — PDA seeds: `["poll", poll_id (小端序 u64)]`

| 字段                  | 类型     | 描述                                  |
|-----------------------|----------|---------------------------------------|
| `poll_name`           | String   | 投票名称（最多 32 个字符）            |
| `poll_description`    | String   | 描述（最多 280 个字符）               |
| `poll_voting_start`   | u64      | 投票开始的 Unix 时间戳                |
| `poll_voting_end`     | u64      | 投票结束的 Unix 时间戳                |
| `poll_option_index`   | u64      | 当前已添加的候选者数量                |

### `CandidateAccount` — PDA seeds: `[poll_id (小端序 u64), candidate_name]`

| 字段                | 类型     | 描述                          |
|---------------------|----------|-------------------------------|
| `candidate_name`    | String   | 候选者名称（最多 32 个字符）  |
| `candidate_votes`   | u64      | 收到的总票数                  |

## 错误代码

| 代码                | 消息                     |
|---------------------|--------------------------|
| `VotingNotStarted`  | 投票尚未开始             |
| `VotingEnded`       | 投票已结束               |

## 版本声明

本节课程代码复制自 [solana-foundation/solana-bootcamp-2026](https://github.com/solana-foundation/solana-bootcamp-2026/tree/42ee75925d550be3bda8a53c572dc4cba99bb374/03-voting)