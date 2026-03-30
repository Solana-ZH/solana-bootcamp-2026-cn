# 第 0 步: 介绍

## 目标

打开初始代码, 了解它目前的功能, 并明确我们将要添加哪些内容.

---

## 项目结构

**主要查看: ** `anchor/programs/private_transfers/src/lib.rs`

**同时了解: ** `circuits/` 目录中的电路代码, 以及 `backend/` 和 `frontend` 中的后端与前端代码

---

## 运行前端

先看看 UI 的样子:

```bash
# 终端 1 - 启动后端
cd backend
bun install
bun run dev

# 终端 2 - 启动前端
cd frontend
bun install
bun run dev
```

打开 `http://localhost:3000`.

你将看到:
- **连接钱包(Connect Wallet)** - 连接你的 Phantom/Solflare 钱包
- **存款区(Deposit Section)** - 输入金额, 获取存款凭据
- **提款区(Withdraw Section)** - 粘贴存款凭据, 取回资金

前端已经连接好了后端 API 并能构建交易, 我们只需要在程序中添加隐私功能.

---

# 打开程序


打开 `anchor/programs/private_transfers/src/lib.rs`.

这是一个基础的托管风格程序, 先了解它的工作原理.

---

## 构建程序

确认初始代码能够成功编译:

```bash
cd anchor
anchor build
```

你应该看到编译成功的输出.

---

## 下一步

让我们从使用 commitment 隐藏存款详情开始.

继续 [第 1 步: 隐藏存款](./step-1-hiding-deposits.md).
