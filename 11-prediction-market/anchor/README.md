# Anchor Vault 程序

此模板包含一个使用 [Anchor](https://www.anchor-lang.com/) 构建的简单SOL保险库程序。

## 预部署的程序

保险库程序已部署在 **devnet** 上，地址为：

```
F4jZpgbtTb6RWNWq6v35fUeiAsRJMrDczVPv9U23yXjB
```

您可以通过将钱包连接到devnet立即与之交互。

## 部署您自己的程序

要部署您自己版本的程序：

### 1. 生成新的程序密钥对

```bash
cd anchor
solana-keygen new -o target/deploy/vault-keypair.json
```

### 2. 获取新的程序ID

```bash
solana address -k target/deploy/vault-keypair.json
```

### 3. 更新程序ID

在这些文件中更新程序ID：

- `anchor/Anchor.toml` - 更新 `[programs.devnet]` 下的 `vault = "..."`
- `anchor/programs/vault/src/lib.rs` - 更新 `declare_id!("...")`

### 4. 构建和部署

```bash
# 构建程序
anchor build

# 获取devnet SOL用于部署（需要约2 SOL）
solana airdrop 2 --url devnet

# 部署到devnet
anchor deploy --provider.cluster devnet
```

### 5. 重新生成TypeScript客户端

```bash
cd ..
npm run codama:js
```

这将使用您的新程序ID更新 `app/generated/vault/` 中的生成客户端代码。

## 程序概述

保险库程序允许用户：

- **存款**：将SOL发送到个人保险库PDA（程序派生地址）
- **取款**：从您的保险库中取出所有SOL

每个用户都会获得一个从其钱包地址派生的自己的保险库。

## 测试

运行Anchor测试：

```bash
anchor test --skip-deploy
```
