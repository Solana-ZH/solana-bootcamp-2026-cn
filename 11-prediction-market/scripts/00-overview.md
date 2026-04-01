# 视频讲座脚本：全栈 Solana 预测市场

**总时长:** 90 分钟
**总字数:** ~7,350 字
**语速:** 150 字/分钟（根据视觉效果调整）

---

## 部分划分

| 文件 | 部分 | 时长 | 字数 |
|------|------|----------|-------|
| `01-why-and-what.md` | 为什么 & 是什么 | 15 分钟 | ~1,300 |
| `02-on-chain-program.md` | 链上程序 | 30 分钟 | ~2,350 |
| `03-codegen-layer.md` | 代码生成层 | 15 分钟 | ~1,150 |
| `04-frontend-architecture.md` | 前端架构 | 20 分钟 | ~1,500 |
| `05-security-tradeoffs.md` | 安全性与权衡 | 5 分钟 | ~550 |
| `06-recap.md` | 回顾总结 | 5 分钟 | ~500 |

---

## 制作清单

- [ ] 首先录制第二部分（核心内容）
- [ ] 录制前创建图表素材
- [ ] 测试代码片段在 1080p 分辨率下的可读性
- [ ] 首次录制后检查语速

---

## 文件参考

| 主题 | 文件路径 |
|-------|-----------|
| 账户结构 | `anchor/programs/prediction_market/src/state.rs` |
| 程序指令 | `anchor/programs/prediction_market/src/lib.rs` |
| 错误定义 | `anchor/programs/prediction_market/src/errors.rs` |
| 生成的客户端 | `app/generated/prediction_market/` |
| 市场列表组件 | `app/components/markets-list.tsx` |
| 市场卡片组件 | `app/components/market-card.tsx` |
| 钱包提供者 | `app/components/providers.tsx` |
| Codama 配置 | `codama.json` |
