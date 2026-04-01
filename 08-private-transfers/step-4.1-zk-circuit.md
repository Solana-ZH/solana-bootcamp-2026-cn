**~5 分钟**

# 第 4.1 步: ZK 电路

## 目标

创建 Noir 电路, 用于证明存款所有权, 并生成证明密钥和验证密钥.

---

在这一步中, 我们将浏览电路代码并解释其含义. 这些电路将在后端运行以生成证明, 然后我们可以在链上进行验证.

---

## 安装 Nargo

Nargo 是 Noir 的编译器.

noir-lang.org

```
$ curl -L https://raw.githubusercontent.com/noir-lang/noirup/refs/heads/main/install | bash
$ noirup -v 1.0.0-beta.18
```

---

## 浏览电路

提款电路

Noir 的工作方式是"如果这段计算成功, 则生成一个证明", 因此通常在电路末尾会有某种断言, 比如这里的 `assert(computed_root === root)`.

## 测试电路

`nargo test`

---

## 编译电路

```bash
cd circuit
nargo compile
```

这会生成 `target/withdrawal.json`, 编译后的电路文件.

---

## 安装 Sunspot

Sunspot 可以将 Noir 电路转换为可在 Solana 上验证的 Groth16 证明.

需要 **Go 1.24+**:

```bash
go version  # 应显示 1.24+
```

如有需要, 请从 [go.dev/dl](https://go.dev/dl/) 安装.

安装 Sunspot:

```bash
git clone https://github.com/reilabs/sunspot.git
cd sunspot/go
go build -o sunspot .

# 添加到 PATH
sudo mv sunspot /usr/local/bin/

# 验证安装
sunspot help
```

---

## 生成证明密钥

Groth16 证明需要进行**可信设置**, 这是一次性流程, 会生成专属于你的电路的密码学密钥.

**为什么需要这个? **

Groth16 通过在设置阶段将电路结构"烘焙"进专用密钥中, 实现了极小的证明体积(约 200 字节)和快速验证.
设置后会产生两个密钥:

- **证明密钥(Proving Key, pk)**: 包含生成证明所需的密码学参数.

- **验证密钥(Verification Key, vk)**: 一个小型密钥(约 1KB), 包含足够多的信息用于验证证明. 这是部署到链上的内容.

"可信"是指设置过程中使用的随机数, 如果有人知道这个随机数, 就可以伪造证明. Sunspot 在开发环境中安全处理了这个问题, 生产系统则使用多方计算仪式, 确保随机数在使用后被销毁.

```bash
cd circuit

# 转换为 CCS 格式
sunspot compile target/withdrawal.json

# 生成证明密钥和验证密钥
sunspot setup target/withdrawal.ccs
```

**输出文件: **

| 文件 | 大小 | 用途 |
|------|------|---------|
| `target/withdrawal.ccs` | ~100KB | CCS 格式的电路 |
| `target/withdrawal.pk` | ~2MB | 证明密钥(用于生成证明) |
| `target/withdrawal.vk` | ~1KB | 验证密钥(用于链上验证程序) |

---
---

## 你构建了什么

- **提款电路** - 在不透露具体存款的情况下证明你拥有某笔存款
- **证明密钥** - 后端将用它来生成证明
- **验证密钥** - 将被部署为链上验证程序

---

## 下一步

我们有了电路和密钥. 现在需要向 Solana 部署一个验证程序.

继续 [第 5 步: Sunspot 验证程序](./step-5-sunspot-verifier.md).
