# HelloSolar

一个基于 Next.js 的起步项目，集成了 Tailwind CSS、`@solana/react-hooks`，以及一个 Anchor 金库程序示例。

## 快速开始

```shell
npx -y create-solana-dapp@latest -t solana-foundation/templates/kit/HelloSolar
```

```shell
npm install   # 自动构建程序并生成客户端
npm run dev
```

打开 [http://localhost:3000](http://localhost:3000)，连接你的钱包，并在 devnet 上与金库进行交互。

## 包含内容

- 通过 `@solana/react-hooks` 实现的**钱包连接**，支持自动发现
- **SOL 金库程序**：可在个人 PDA 金库中存入和提取 SOL
- **Codama 生成的客户端**：基于 `@solana/kit` 提供类型安全的程序交互
- 支持亮色/暗色模式的 **Tailwind CSS v4**

## 技术栈

| 层级          | 技术                                    |
| ------------- | --------------------------------------- |
| 前端          | Next.js 16、React 19、TypeScript        |
| 样式          | Tailwind CSS v4                         |
| Solana 客户端 | `@solana/client`、`@solana/react-hooks` |
| 程序客户端    | Codama 生成、`@solana/kit`              |
| 链上程序      | Anchor（Rust）                          |

## 项目结构

```text
├── app/
│   ├── components/
│   │   ├── providers.tsx      # Solana 客户端配置
│   │   └── vault-card.tsx     # 金库存取款界面
│   ├── generated/vault/       # Codama 生成的程序客户端
│   └── page.tsx               # 主页面
├── anchor/                    # Anchor 工作区
│   └── programs/vault/        # 金库程序（Rust）
└── codama.json                # Codama 客户端生成配置
```

## 部署你自己的金库

仓库内附带的金库程序已经部署到 devnet。如果你想部署自己的版本，请按以下步骤操作：

### 前置要求

- [Rust](https://rustup.rs/)
- [Solana CLI](https://solana.com/docs/intro/installation)
- [Anchor](https://www.anchor-lang.com/docs/installation)

### 步骤

1. **将 Solana CLI 配置到 devnet**

   ```bash
   solana config set --url devnet
   ```

2. **创建钱包（如有需要）并领取测试币**

   ```bash
   solana-keygen new
   solana airdrop 2
   ```

3. **构建并部署程序**

   ```bash
   cd anchor
   anchor build
   anchor keys sync    # 更新源码中的程序 ID
   anchor build        # 使用新的 ID 重新构建
   anchor deploy
   cd ..
   ```

4. **重新生成客户端并重启项目**

   ```bash
   npm run setup   # 重新构建程序并生成客户端
   npm run dev
   ```

## 测试

测试使用 [LiteSVM](https://github.com/LiteSVM/litesvm)，这是一个轻量且快速的 Solana 虚拟机，适合测试场景。

```bash
npm run anchor-build   # 先构建程序
npm run anchor-test    # 运行测试
```

测试文件位于 `anchor/programs/vault/src/tests.rs`，并会自动使用 `declare_id!` 中定义的程序 ID。

## 重新生成客户端

如果你修改了链上程序，请重新生成 TypeScript 客户端：

```bash
npm run setup   # 或者：npm run anchor-build && npm run codama:js
```

这里使用 [Codama](https://github.com/codama-idl/codama) 从 Anchor IDL 生成类型安全的客户端。

## 了解更多

- [Solana Docs](https://solana.com/docs) - 核心概念与使用指南
- [Anchor Docs](https://www.anchor-lang.com/docs) - 链上程序开发框架
- [Deploying Programs](https://solana.com/docs/programs/deploying) - 程序部署指南
- [framework-kit](https://github.com/solana-foundation/framework-kit) - 此项目使用的 React Hooks 来源
- [Codama](https://github.com/codama-idl/codama) - 从 IDL 生成客户端
