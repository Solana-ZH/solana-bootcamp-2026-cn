# AI Best Practice for Solana

在 Solana 开发中如何使用 AI 的实践参考，通过高质量上下文、最新文档和可复用技能（Skills），提升 AI 生成代码的准确性与可维护性，应用 AI 的同时，应该了解：

- AI 代理可以显著提升项目开发效率，但不能替代对架构、协议和工程质量的判断。
- 区块链生态更新快，直接依赖模型内置知识通常不够，需要主动提供最新资料。
- 在 Solana 开发中，应优先使用官方或社区维护的技能、最佳实践和工具链说明。

## 推荐 AI 工具

### CLI

- Claude Code: https://claude.com/product/claude-code
- Codex CLI: `npm i -g @openai/codex`
- Codex App: https://openai.com/zh-Hans-CN/codex/

### IDE

- Cursor: https://cursor.com/cn/home
- Antigravity: https://antigravity.google/
- TRAE: https://www.trae.ai/

### 常用模型

- Claude Sonnet / Opus
- GPT-5.4
- Gemini 3.1 Pro

## 推荐资源

### Solana AI 与 Skills

- Awesome Solana AI: https://github.com/solana-foundation/awesome-solana-ai
- **Solana Dev Skill**: https://github.com/solana-foundation/solana-dev-skill
- Solana Skills: https://solana.com/zh/skills
- AI on Solana: https://solana.com/zh/solutions/ai
- Solana MCP: https://github.com/solana-foundation/solana-mcp-official
- X402 on Solana: https://solana.com/zh/x402

## 使用建议

- 在开始生成代码前，先为 AI 提供项目目标、约束、技术栈和参考实现。
- 对于 Solana 项目，优先补充最新的框架文档、Skill、IDL、测试方式和安全约束。
- 将 AI 视为实现助手，而不是决策主体；关键设计、接口边界和验证流程仍需人工把关。
- 持续学习相关最佳实践，可以显著提升提示质量、审查能力和开发效率。
