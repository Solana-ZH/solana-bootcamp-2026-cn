在我们写下第一行 Solana 代码之前, 需要先把开发环境配置好. 下面我们来看看需要安装哪些东西.

第一, Rust. Solana 程序是用 Rust 编写的, 因此我们需要 Rust 编译器以及 Rust 的包管理器 Cargo. 如果你之前从未写过 Rust, 不用担心, 我们会覆盖所有你需要了解的内容. 但工具链必须先装好.

第二, Solana CLI. 这是与 Solana 网络交互的命令行工具. 你会用它来创建钱包, 部署程序, 查询余额, 在 devnet 上空投 SOL, 以及做更多事情. 它是必不可少的.

第三, Anchor. Anchor 是一个让 Solana 开发变得轻松许多的框架. 它帮你处理大量的样板代码和安全检查, 否则这些都需要你手动编写. 如今大多数生产级 Solana 程序都是用 Anchor 构建的, 我们在整个训练营中也会使用它.

第四, Surfpool. 这是一个速度极快的本地测试验证节点. 你不必再等待交易在 devnet 上确认, 可以在本地测试并立即获得代码反馈. 在快速迭代时, 它能大幅提升你的开发效率.

演示将在 Linux 上进行, 但我会在下方的资源中附上其他操作系统的安装说明. 各平台的流程大同小异.

现在, 让我们开始吧.

- Rust: <https://rust-lang.org/tools/install/>
- Solana CLI + Anchor + Surfpool: <https://solana.com/docs/intro/installation>
