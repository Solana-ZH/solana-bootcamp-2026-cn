**~7 分钟**

# 第 4.2 步: Sunspot 验证程序

## 目标

部署 Sunspot 验证程序, 用于在链上验证 ZK 证明.

---

* 告诉 Sunspot 验证程序模板的位置
* 根据验证密钥生成验证程序(即一个 Solana 程序)
* 将验证程序部署到 devnet

---

### 设置 GNARK_VERIFIER_KEY 环境变量

在你安装 Sunspot 的目录中, 可以找到 `gnark-solana/crates/verifier-bin`, 需要将其导出为环境变量:

```bash
export GNARK_VERIFIER_BIN="$HOME/.sunspot/gnark-solana/crates/verifier-bin"
```

---

## 生成验证程序

```bash
cd circuit
sunspot deploy target/withdrawal.vk
```

> Sunspot 使用 Gnark 将 Noir 电路(编译为 CCS, 可定制约束系统格式)转换为 Groth16 验证程序. `.vk` 文件包含专属于我们电路的密码学参数.

这将生成:
- `target/withdrawal.so` - 编译后的 Solana 程序
- `target/withdrawal-keypair.json` - 部署用密钥对(类似程序的钱包, 让我们能部署到特定地址, 并在之后重新部署到相同地址)

---


## 部署到 Devnet

确保你有足够的 SOL:

```bash
solana config set --url devnet
solana balance
# 如需充值: solana airdrop 2
```

部署:

```bash
solana program deploy circuit/target/withdrawal.so
```

**复制 Program ID:**

```
Program Id: Amugr8yL9EQVAgGwqds9gCmjzs8fh6H3wjJ3eB4pBhXV
```

在后端步骤中会用到它.

---

## 你部署了什么

该验证程序:
- 内置了你的验证密钥
- 只接受来自你的特定电路的证明
- 使用 BN254 椭圆曲线配对运算(约 140 万计算单元)
- 无状态, 无需任何账户, 只需通过 CPI 调用

---

## Solana 深入: 为什么选择 Groth16?

并非所有证明系统都适合 Solana. 以下是 Groth16 成为理想选择的原因:

**极小的证明体积:** Groth16 证明约 256 字节. Solana 交易最大 1232 字节, 因此小体积的证明至关重要. STARKs 证明可达 50-200KB, 根本放不进一笔交易.

**快速验证:** Groth16 验证使用椭圆曲线配对运算, 计算量大, 但可预测. 在 Solana 上约消耗 140 万计算单元(CU). 默认交易预算为 20 万 CU, 但可通过 `SetComputeUnitLimit` 申请最高 140 万 CU. 我们的验证刚好在限额内.

**权衡之处:** Groth16 需要可信设置(即我们生成的 `.pk` 和 `.vk` 文件). 如果有人知道设置时使用的随机数, 就可以伪造证明. 生产环境中应使用多方计算仪式, 确保随机数被彻底销毁. Sunspot 在开发阶段处理了这个问题.

**为什么需要单独的验证程序?**

验证程序约有 100KB 的编译代码, 太大, 无法包含在主程序中. 单独部署的好处是:

- 多个程序可以共享同一个验证程序
- 无需重新部署主程序即可升级验证程序
- 验证逻辑可单独进行审计

"通过 CPI 调用外部验证程序"这一模式正在成为 Solana 上 ZK 应用的标准做法.

---

## 下一步

现在我们将更新主程序, 在提款时调用这个验证程序.

继续 [第 6 步: 验证 CPI](./step-6-verification-cpi.md).
