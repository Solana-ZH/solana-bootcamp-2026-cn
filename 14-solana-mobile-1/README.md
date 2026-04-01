# 14 - Solana Mobile 入门

今天我们来开启一个特别的主题——Solana Mobile。我们相信这是一个属于华语 builder 的巨大机会。

## 为什么是 Solana Mobile？

过去几年，Web3 的用户体验一直停留在桌面端和网页端。但现实是——全球超过 55 亿人使用智能手机，移动互联网才是大多数人接触数字世界的第一入口。

而华语区，正是全球移动互联网的中心。

华语用户早在十年前就完成了移动支付的普及，我们的工程师深度参与了微信、支付宝、抖音这些改变世界的移动产品的构建。华语区有全球最顶尖的移动互联网工程师，这是我们独特的优势。

**Solana Mobile，正在把这个优势变成 Web3 的机会。**

## 什么是 Solana Mobile Stack？

Solana Mobile Stack（SMS）是一套专为 Android 移动端 dApp 开发设计的技术集合，核心由三个部分组成：

### ① Mobile Wallet Adapter（MWA）

MWA 是移动端 dApp 与钱包 App 之间的通信协议。类比 Web 端的 Wallet Adapter——你的 dApp 只需集成一次，就能兼容所有实现了 MWA 协议的钱包，包括 Phantom、Solflare、Ultimate 等。支持 React Native、Kotlin、Flutter 等主流框架。

### ② Seed Vault

系统级安全密钥托管服务，把私钥保存在设备上安全性最高的执行环境中——比如安全处理器或安全辅助芯片。对钱包应用来说，这是移动端密钥安全的底层保障。

### ③ Solana dApp Store

专为 Solana 生态设计的独立应用分发平台。绕开传统应用商店的限制——不抽分成、没有额外审核壁垒，让你的应用直接触达用户。目前已有超过 100 个应用上线。

```
Solana Mobile Stack
├── Mobile Wallet Adapter  →  dApp ↔ 钱包通信协议
├── Seed Vault             →  系统级安全密钥管理
└── dApp Store             →  独立应用分发平台
```

## 对开发者的支持

### Builder Grants

Solana Mobile 设有专属的 Builder Grants 计划，每个团队最高可获得 **$10,000** 的资金支持，同时附带官方市场宣发和上线协助。只要你的应用专注于移动端开发，都可以申请。

### 黑客松奖励

在各大 Colosseum 黑客松中，移动端项目是重点评审方向之一。Solana Mobile 官方黑客松奖金池高达 **$100,000**，获奖项目还能在 dApp Store 获得首发推荐位。这不只是奖金，更是直接触达真实用户的机会。

## 华语区的机会

我们非常相信一件事：下一批最出色的 Solana Mobile 应用，很可能诞生在华语区。

我们懂移动端，从产品设计到工程实现，华语区积累了全球最丰富的移动互联网经验。把这些能力迁移到 Web3，我们天然有优势。而现在正是早期——早进来的 builder，能拿到最多支持，也能抢占最好的位置。

## 相关资源

### 搭建开发环境

- [Solana Mobile 开发环境配置](https://docs.solanamobile.com/get-started/development-setup#expo-%2F-react-native)

### Solana 移动端模板

如果你想在已有的 Web2 移动应用基础上快速接入 Solana，官方开发者模板里有专门的移动端模板，已预集成 MWA、钱包连接、交易发送等常用功能：

- [Solana 官方开发者模板](https://solana.com/developers/templates)

### 参考项目

想看看真实的 Solana Mobile 应用是怎么构建的？官方提供了一批示例应用，覆盖 DeFi、NFT、游戏等多个场景：

- [示例应用概览](https://docs.solanamobile.com/sample-apps/sample_app_overview)
- [完整示例代码](https://github.com/solana-mobile/react-native-samples/tree/main)
