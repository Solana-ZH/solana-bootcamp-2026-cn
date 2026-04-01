# Solana 预测市场

一个使用 Anchor + framework-kit 在 Solana 上构建的最小化全栈预测市场示例。用户可以创建二元（是/否）市场，用 SOL 下注，并在手动结算后领取奖金。

## 快速开始

```shell
npm install
npm run setup   # 构建程序并生成客户端
npm run dev
```

打开 [http://localhost:3000](http://localhost:3000)，连接您的钱包，并在 devnet 上与预测市场交互。

## 功能特性

- **创建市场** - 提出任何是/否问题并设置结算截止时间
- **下注** - 在截止时间前用 SOL 对是或否结果下注
- **结算市场** - 市场创建者在截止时间后手动结算
- **领取奖金** - 获胜者从失败池中按比例获得奖金

## 技术栈

| 层级           | 技术                                      |
| -------------- | --------------------------------------- |
| 前端           | Next.js 16, React 19, TypeScript        |
| 样式           | Tailwind CSS v4                         |
| Solana 客户端  | `@solana/client`, `@solana/react-hooks` |
| 程序客户端     | Codama 生成, `@solana/kit`              |
| 程序           | Anchor 0.31 (Rust)                      |

## 项目结构

```
├── app/
│   ├── components/
│   │   ├── providers.tsx           # Solana 客户端设置
│   │   ├── create-market-form.tsx  # 市场创建界面
│   │   ├── market-card.tsx         # 市场下注/结算界面
│   │   └── markets-list.tsx        # 获取并显示所有市场
│   ├── generated/prediction_market/ # Codama 生成的客户端
│   └── page.tsx                    # 主页面
├── anchor/
│   └── programs/prediction_market/ # 预测市场程序 (Rust)
└── codama.json                     # Codama 客户端生成配置
```

## 工作原理

### 程序架构

**账户:**
- `Market` (PDA) - 存储问题、资金池、结算状态、创建者权限
- `UserPosition` (PDA) - 跟踪每个用户在每场市场的下注

**指令:**
1. `create_market` - 使用问题和截止时间初始化市场
2. `place_bet` - 将 SOL 转移到市场资金池（是或否）
3. `resolve_market` - 创建者在截止时间后设置获胜结果
4. `claim_winnings` - 获胜者按比例提取奖金

**奖金计算公式:**
```
奖金 = (用户下注 / 获胜池) * 失败池
总额 = 用户下注 + 奖金
```

### 安全性

- 基于 PDA 的资金池管理（无管理员密钥持有资金）
- 基于时间的下注窗口强制执行
- 截止时间后仅创建者可结算
- 通过位置标志防止重复领取
- 检查数学运算以防止溢出

## 自行部署

### 先决条件

- [Rust](https://rustup.rs/)
- [Solana CLI](https://solana.com/docs/intro/installation)
- [Anchor](https://www.anchor-lang.com/docs/installation)

### 步骤

1. **配置 Solana CLI 使用 devnet**
   ```bash
   solana config set --url devnet
   ```

2. **创建钱包并充值**
   ```bash
   solana-keygen new
   solana airdrop 2
   ```

3. **构建和部署**
   ```bash
   cd anchor
   anchor build
   anchor keys sync    # 更新源代码中的程序 ID
   anchor build        # 使用新 ID 重新构建
   anchor deploy
   cd ..
   npm run setup       # 重新生成客户端
   npm run dev
   ```

## 测试

程序包含在 `anchor/programs/prediction_market/src/tests.rs` 中的 LiteSVM 测试。

```bash
npm run anchor-build   # 先构建
npm run anchor-test    # 运行测试
```

## 了解更多

- [Solana 文档](https://solana.com/docs) - 核心概念
- [Anchor 文档](https://www.anchor-lang.com/docs) - 程序框架
- [framework-kit](https://github.com/solana-foundation/framework-kit) - React hooks
- [Codama](https://github.com/codama-idl/codama) - 客户端生成
- [solana-dev-skill](https://github.com/GuiBibeau/solana-dev-skill) - Claude Code 的 Solana 开发技能

## 版本声明

本节课程代码复制自 [solana-foundation/solana-bootcamp-2026](https://github.com/solana-foundation/solana-bootcamp-2026/tree/42ee75925d550be3bda8a53c572dc4cba99bb374/11-prediction-market)
