# Solana Escrow 程序教学讲稿

## 1.1 项目背景与目标

大家好, 欢迎来到本节课. 今天我们要从零开始, 在 solana 链上实现一个去中心化的代币交换程序, 也就是所谓的 escrow(托管) 程序. 它是区块链开发中最基本的模式之一.

在现实生活中, 当两个陌生人想要交换资产时, 比如我想用我的人民币换你的美元, 我们之间缺乏信任. 这时候我们会找一个双方都信任的第三方, 比如银行来做中间人: 我先把人民币给银行, 你把美元给银行, 银行确认双方都到位之后, 再把资产分别转给对方.

在链上, 这个银行的角色可以由一个智能合约来扮演, 这就是所谓托管的本质. 它是无需信任第三方, 完全透明, 由代码自动执行的原子交换.

## 1.2 我们的场景

在本项目中, 我们涉及到的角色和流程如下:

- alice(maker): 持有代币 a, 想用它换代币 b.
- bob(taker): 持有代币 b, 看到了 alice 发出的交换请求, 愿意以代币 b 换取代币 a.

整个流程分两步:

1. make_offer(发起报价): alice 把代币 a 托管到智能合约, 并声明一件事: 我愿意用 x 个代币 a 换 y 个代币 b.
2. take_offer(接受报价): bob 看到报价, 发送代币 b 给 alice, 同时智能合约自动把代币 a 释放给 bob.

在 take_offer 阶段, 这两步要么全部发生, 要么全部不发生, 这就是原子性.

## 1.3 技术选型

本项目使用以下技术栈:

- anchor 框架: solana 的最流行开发框架, 大幅简化账户验证, pda 管理和 cpi 调用.
- token 2022 program: solana 新一代代币标准, 向后兼容经典 token program, 同时支持更多扩展功能.
- typescript + mocha: 编写链下测试.

## 2.1 项目初始化

在开始写代码之前, 我们先初始化项目:

```bash
$ anchor init escrow
$ cd escrow
```

在 `programs/escrow/Cargo.toml` 中添加 `anchor-spl` 依赖:

```bash
$ cargo add anchor-spl
```

并在 cargo.toml 的 `[features]` 中启用 idl 构建支持:

```toml
# programs/escrow/Cargo.toml
[features]
idl-build = ["anchor-lang/idl-build", "anchor-spl/idl-build"]
```

同时为 anchor-lang 添加 init-if-needed feature, 因为后面我们会用到这个 feature.

```toml
anchor-lang = { version = "1.0.0-rc.2", features = ["init-if-needed"] }
```

在 javascript/typescript 侧安装辅助库:

```bash
$ yarn add @solana-developers/helpers
$ yarn add @solana/spl-token
$ yarn add mocha
```

## 2.2 顶部引入与程序 ID

现在我们打开 `programs/escrow/src/lib.rs`, 我会从零开始搭建这个程序. 首先是顶部的依赖库引入, 这里我们直接引入 anchor 的预导入模块和 anchor-spl 的 associated_token 和 token_interface 模块.

```rust
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
// token_interface 下的类型支持同时兼容经典 token program 和 token 2022 program, 这是本项目使用 interface 而非 program 的原因.
use anchor_spl::token_interface::*;

declare_id!("25Q841qjRsaGQzWSKh5kiEZ9qpXbWMzm3v4ytGXs6PzY");
```

## 2.2 数据结构: Offer 账户

在写指令之前, 先看我们需要存储什么数据:

```rust
#[account]           // anchor 宏, 标记这是一个链上账户结构, 自动添加 8 字节的 discriminator(账户类型标识符).
#[derive(InitSpace)] // 自动计算该结构体的存储空间大小, 方便后面 init 时计算 space.
pub struct Offer {
    pub maker:                  Pubkey,  // alice 的公钥
    pub mint_a:                 Pubkey,  // alice 提供的代币 mint
    pub mint_b:                 Pubkey,  // alice 想要的代币 mint
    pub offer_id:               u64,     // 唯一 id, 支持一个 maker 发多个报价
    pub token_a_offered_amount: u64,     // alice 愿意提供多少代币 a
    pub token_b_wanted_amount:  u64,     // alice 期望获得多少代币 b
    pub bump:                   u8,      // pda 的 canonical bump, 避免在后续 cpi 调用时重新计算, 节省计算资源.
}
```

## 2.3 错误码

之后我们可以先定义一下错误码. anchor 的 `#[error_code]` 宏让我们可以定义自定义错误, 并在约束检查失败时返回有意义的错误信息. 我们需要定义三个错误.

```rust
#[error_code]
pub enum EscrowError {
    // 当 bob take_offer 时传入的 maker 与 offer 记录不符
    #[msg("Maker account does not match Offer.maker")]
    MakerMismatch,
    // 当 bob take_offer 时传入的 mint 与 offer 记录不符
    #[msg("Mint account does not match Offer mints")]
    MintMismatch,
    // make_offer 时 mint_a 和 mint_b 是同一个代币
    #[msg("mint_a and mint_b must be different")]
    SameMint,
}
```

## 2.4 MakeOffer 账户约束结构

之后我们写 MakeOffer 账户约束结构. 这个结构主要做两件事: make offer 指令在执行的时候会读写哪些账号, 以及这些账号之间的关联关系.

```rust
#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct MakeOffer<'info> {
    // 交易发起人, 必须是签名者 signer, 且地址可变以便支付租金.
    #[account(mut)]
    pub maker: Signer<'info>,

    // 接下来写两种代币的 mint 账户. 分别命名为 mint_a 和 mint_b.
    #[account(mint::token_program = token_program)] // 约束确保该 mint 账户属于传入的 token_program, 防止假冒的 mint.
    pub mint_a: InterfaceAccount<'info, Mint>,
    #[account(mint::token_program = token_program)] // 约束确保该 mint 账户属于传入的 token_program, 防止假冒的 mint.
    pub mint_b: InterfaceAccount<'info, Mint>,      // 使用 interface_account 而非 account 是为了同时支持 token program 和 token 2022 program.

    // 接下来是 alice 持有代币 a 的 ata 账户.
    // mut 表示该账户的余额会被修改.
    // 总共有三个约束, 它们组合起来构成对 ata 地址的完整验证, 防止传入伪造账户.
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    // offer 账户是本项目的核心账户设计. 它里面存储的数据格式就是我们上面定义的 offer 结构体.
    #[account(
        init,
        payer = maker,                 // 首次创建, 由 payer = maker 支付租金.
        space = 8 + Offer::INIT_SPACE, // 8 字节 discriminator 加上结构体大小.
        seeds = [b"offer", maker.key().as_ref(), &offer_id.to_le_bytes()], // pda 的派生种子, 其中 offer_id 是用户传入的唯一 id, 因此同一个 maker 可以同时存在多个 offer.
        bump // anchor 自动计算并保存 canonical bump.
    )]
    pub offer: Account<'info, Offer>,
    // seeds 这里有一个关键设计点: 为什么 offer_id 用 to_le_bytes()? 因为 pda seeds 要求 &[u8], 而 u64 必须序列化为字节数组. 用小端序是 solana 生态的惯例.

    // alice 会把代币托管, 因此我们需要使用一个以 offer pda 为 authority 的 mint_a 的 ata 账户, 这就是托管仓库.
    // 这里要注意该账号 authority 是 pda 而不是 maker, 这意味着只有持有 pda 签名能力的程序代码才能动用这里的代币, maker 无法直接取走, 保证了双方的安全性.
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program
    )]
    pub offer_token_account: InterfaceAccount<'info, TokenAccount>,

    // 最后这里我们加上一些系统程序和原生程序.
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}
```

## 2.5 MakeOffer 指令逻辑

指令会接收三个参数, 分别是 alice 自定义的 offer_id, alice 愿意提供的代币 a 数量, 以及 alice 想要获得的代币 b 数量.

```rust
pub fn make_offer(
    ctx: Context<MakeOffer>,
    offer_id: u64,
    token_a_offered_amount: u64,
    token_b_wanted_amount: u64,
) -> Result<()> {
    // 首先用 require_keys_neq! 宏检查 mint_a 和 mint_b 不是同一个代币. 如果相同, 则直接返回 SameMint 错误. 早期检验可以避免无意义的后续操作.
    require_keys_neq!(
        ctx.accounts.mint_a.key(),
        ctx.accounts.mint_b.key(),
        EscrowError::SameMint
    );
    Ok(())
}
```

接下来的步骤, 会将 alice 提供的代币 a 转移到托管账户. 这里我们需要执行跨程序调用, 也就是 cpi 来调用 token program 的转账函数. 这是 solana 开发中的一个重要模式.

```rust
// 将 mint_a 存入托管账户.
// 我们创建一个 anchor-spl 提供的 cpi 账户集合, 指定 from/to/mint/authority.
let transfer_cpi_accounts = TransferChecked {
    from: ctx.accounts.maker_token_account_a.to_account_info(),
    to: ctx.accounts.offer_token_account.to_account_info(),
    mint: ctx.accounts.mint_a.to_account_info(),
    // authority 是 maker, 因为此时代币还在 maker 手里, 由 maker 签名授权.
    authority: ctx.accounts.maker.to_account_info(),
};

// 构建 CPI 上下文.
let cpi_context = CpiContext::new(*ctx.accounts.token_program.key, transfer_cpi_accounts);

let decimals = ctx.accounts.mint_a.decimals;
// transfer_checked 是带精度检查的转账函数, decimals 参数是 token 2022 的安全要求, 防止精度欺诈攻击.
transfer_checked(cpi_context, token_a_offered_amount, decimals)?;
```

最后我们要保存 offer 数据. 之前我们在账户约束中已经指定了 offer 账户的空间和 pda 派生方式, 现在我们只需要将数据写入这个账户即可. Anchor 提供了 set_inner 方法, 可以一次性设置整个结构体的数据.

```rust
// 3) 保存 offer 数据
ctx.accounts.offer.set_inner(Offer {
    maker: ctx.accounts.maker.key(),
    mint_a: ctx.accounts.mint_a.key(),
    mint_b: ctx.accounts.mint_b.key(),
    offer_id,
    token_a_offered_amount,
    token_b_wanted_amount,
    bump: ctx.bumps.offer,
});
```

## 2.6 TakeOffer 账户约束结构

Bob 现在看到了 alice 发出的请求, 它想和 alice 完成交换, 为此我们为它实现 take_offer 指令. 这个指令的账户约束结构如下:

```rust
#[derive(Accounts)]
pub struct TakeOffer<'info> {
    // taker 必须是签名者, 因为他需要授权转出代币 b.
    #[account(mut)]
    pub taker: Signer<'info>,

    /// maker 无需签名, 也就是说即使 alice 本人不在场也可以完成最后的代币交换.
    /// 使用 SystemAccount<'info> 而非 Signer, 表示不需要签名, 只需要其地址用于接收代币和租金.
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    // 接下来几个账号和 make offer 类似, 分别是 mint_a / mint_b / offer 和 offer_token_account: 但是不同的地方在于这里我们使用 Box 包装一下, 将数据分配到堆上, 避免超出 solana 栈大小 4kb 的限制. 这在账户数量较多时是必要的. 如果我们这里不加 Box, 编译的时候会报 warning, 所以我们就加上吧.
    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    // offer 账户的约束比较多:
    #[account(
        mut,
        close = maker, // 指令成功执行后, anchor 自动关闭 offer 账户, 并将其中的租金退还给 maker.
        has_one = maker @ EscrowError::MakerMismatch, // 验证 offer.maker == maker, 防止传入错误的 maker 地址来截取资金.
        has_one = mint_a @ EscrowError::MintMismatch, // 验证 mint_a 地址与 Offer 中记录的一致, 防止代币替换攻击.
        has_one = mint_b @ EscrowError::MintMismatch, // 验证 mint_b 地址与 Offer 中记录的一致, 防止代币替换攻击.
        seeds = [b"offer", offer.maker.as_ref(), &offer.offer_id.to_le_bytes()], // 验证传入的 offer 账户地址确实是合法的 PDA
        bump = offer.bump                             // 使用存储的 bump 而非重新推导, 节省计算.
    )]
    pub offer: Box<Account<'info, Offer>>,

    // taker_token_account_a 指 taker 接收代币 a 的账户:
    #[account(
        init_if_needed, // init_if_needed: 如果 bob 还没有代币 a 的 ata, 则自动创建, 由 taker 支付创建费用.
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_token_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // maker_token_account_b 指 alice 接收代币 b 的账户:
    #[account(
        init_if_needed, // 同样使用 init_if_needed, alice 可能没有代币 b 的 ata, 创建费由 taker(bob)承担. 这是合理的设计, 因为是 bob 在发起交换.
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // 以 offer pda 为 authority 的 mint_a ata(也就是 托管仓库).
    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program
    )]
    pub offer_token_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
}
```

## 2.7 take_offer 指令逻辑

```rust
pub fn take_offer(ctx: Context<TakeOffer>) -> Result<()> {
    // 第一步: bob 将 mint_b 转给 alice.
    {
        let transfer_cpi_accounts = TransferChecked {
            from: ctx.accounts.taker_token_account_b.to_account_info(),
            to: ctx.accounts.maker_token_account_b.to_account_info(),
            mint: ctx.accounts.mint_b.to_account_info(),
            authority: ctx.accounts.taker.to_account_info(), // authority 是 taker(bob), 由 bob 签名授权.
        };

        let cpi_context = CpiContext::new(*ctx.accounts.token_program.key, transfer_cpi_accounts);

        let decimals = ctx.accounts.mint_b.decimals;
        // 转账金额来自 offer 账户中存储的 token_b_wanted_amount.
        transfer_checked(cpi_context, ctx.accounts.offer.token_b_wanted_amount, decimals)?;
    }
}
```

```rust
// 第二步, 托管仓库将代币 mint_a 转给 bob.
//
// 这是本项目最关键的技术点: pda 签名.
// 因为 offer_token_account 的 authority 是 offer pda, 而 pda 没有私钥, 不能像普通账户那样签名. Solana 允许程序代表自己派生出的 pda 进行签名.

// 要完成这一步首先需要定义 signer_seeds, 它是用来重建 pda 签名的种子数组:
// - 最外层 &[...]: 多个签名者的集合(这里只有一个 pda 签名者)
// - 第二层 &[...]: 这一个签名者的全部 seeds
// - 末尾 &[ctx.accounts.offer.bump]: canonical bump, 用于推导出正确的 pda 地址
// > 注意: seeds 中的 ctx.accounts.offer.offer_id 是从链上账户读取的, 而不是重新从参数中读取, 因此是可信的.
let signer_seeds: &[&[&[u8]]] = &[&[
    b"offer",
    ctx.accounts.offer.maker.as_ref(),
    &ctx.accounts.offer.offer_id.to_le_bytes(),
    &[ctx.accounts.offer.bump],
]];

{
    // 从托管账户将代币 a 转给 bob.
    let transfer_cpi_accounts = TransferChecked {
        from: ctx.accounts.offer_token_account.to_account_info(),
        to: ctx.accounts.taker_token_account_a.to_account_info(),
        mint: ctx.accounts.mint_a.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(), // authority 是 offer 账户, 代表 offer pda 持有资金.
    };

    let cpi_context = CpiContext::new(*ctx.accounts.token_program.key, transfer_cpi_accounts)
       .with_signer(signer_seeds); // .with_signer 将 pda 签名附加到 cpi 上下文, 让 token program 认可这次转账.

    let decimals = ctx.accounts.mint_a.decimals;
    transfer_checked(cpi_context, ctx.accounts.offer.token_a_offered_amount, decimals)?; // 执行转账
}
```

```rust
// 第三步: 关闭托管代币账户, 将其持有的租金退还给 maker(alice).
// 所谓空账户的关闭就是将其 lamports 清零并转给目标账户, 同时将数据清空, 使其不再占用链上空间.
// 这一步同样需要 pda 签名, 因为 offer_token_account 的 authority 是 pda.
{
    let close_cpi_accounts = CloseAccount {
        account: ctx.accounts.offer_token_account.to_account_info(),
        destination: ctx.accounts.maker.to_account_info(),
        authority: ctx.accounts.offer.to_account_info(),
    };

    let cpi_context = CpiContext::new(*ctx.accounts.token_program.key, close_cpi_accounts)
       .with_signer(signer_seeds);

    close_account(cpi_context)?;
}
```

## 3.1 测试准备: 账户和初始状态

先定义一些我们会使用到的账号:

```ts
const provider = anchor.AnchorProvider.env();
anchor.setProvider(provider);
const user = (provider.wallet as anchor.Wallet).payer;
const payer = user;
const connection = provider.connection;
const program = anchor.workspace.Escrow as Program<Escrow>;

let alice: anchor.web3.Keypair;
let bob: anchor.web3.Keypair;
let tokenMintA: anchor.web3.Keypair;
let tokenMintB: anchor.web3.Keypair;
let makerTokenAccountA: anchor.web3.PublicKey;
let makerTokenAccountB: anchor.web3.PublicKey;
let takerTokenAccountA: anchor.web3.PublicKey;
let takerTokenAccountB: anchor.web3.PublicKey;
let offer: anchor.web3.PublicKey;
let offerTokenAccount: anchor.web3.PublicKey;

const tokenAOfferedAmount = new BN(1_000_000);
const tokenBWantedAmount = new BN(1_000_000);
```

然后创建这些账号. 这一步主要会使用 @solana-developers/helpers 里的 createAccountsMintsAndTokenAccounts 函数.

```ts
before(
  "Creates Alice and Bob accounts, 2 token mints, and associated token accounts for both tokens for both users",
  async () => {
    const usersMintsAndTokenAccounts =
      await createAccountsMintsAndTokenAccounts(
        [
          // Alice's token balances
          [
            1_000_000_000,  // 1_000_000_000 of token A
            0,              // 0 of token B
          ],
          // Bob's token balances
          [
            0,              // 0 of token A
            1_000_000_000,  // 1_000_000_000 of token B
          ],
        ],
        1_000_000_000,
        connection,
        payer
      );

    const users = usersMintsAndTokenAccounts.users;
    alice = users[0];
    bob = users[1];

    const mints = usersMintsAndTokenAccounts.mints;
    tokenMintA = mints[0];
    tokenMintB = mints[1];

    const tokenAccounts = usersMintsAndTokenAccounts.tokenAccounts;
    makerTokenAccountA = tokenAccounts[0][0];
    makerTokenAccountB = tokenAccounts[0][1];
    takerTokenAccountA = tokenAccounts[1][0];
    takerTokenAccountB = tokenAccounts[1][1];
  }
);
```

`createAccountsMintsAndTokenAccounts` 是 `@solana-developers/helpers` 提供的一站式辅助函数, 一次调用完成:

1. 创建 alice, bob 两个 keypair 并空投 sol.
2. 创建两个 token mint(代币 a 和代币 b).
3. 为每个用户创建两种代币的 ata, 并按传入的金额铸造代币.

初始状态:

- alice: 持有 1000 million 代币 a, 0 代币 b
- bob: 持有 0 代币 a, 1000 million 代币 b

## 3.2 测试一: make_offer

```typescript
it("Puts the tokens Alice offers into the vault when Alice makes an offer", async () => {
  // 生成随机 offer_id, 防止与历史测试的 pda 冲突
  // import { randomBytes } from "node:crypto";
  // import { BN } from "bn.js";
  const offerId = new BN(randomBytes(8));

  // 在链下预计算 pda 地址, 之后传给合约
  offer = anchor.web3.PublicKey.findProgramAddressSync(
    [
      Buffer.from("offer"),
      alice.publicKey.toBuffer(),
      offerId.toArrayLike(Buffer, "le", 8),
    ],
    program.programId
  )[0];

  // import { TOKEN_2022_PROGRAM_ID, type TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";
  // const TOKEN_PROGRAM: typeof TOKEN_2022_PROGRAM_ID | typeof TOKEN_PROGRAM_ID = TOKEN_2022_PROGRAM_ID;
  offerTokenAccount = getAssociatedTokenAddressSync(
    tokenMintA.publicKey,
    offer,
    true,   // allowOwnerOffCurve = true, 允许 pda 作为 authority
    TOKEN_PROGRAM
  );
});
```

这里展示了 solana 开发中的一个重要模式: 在链下计算 pda 地址, 然后把它作为账户传给交易.

- `findProgramAddressSync` 的 seeds 必须与 rust 侧完全一致: `offer` + `alice.publicKey` + `offerId`(小端 8 字节).
- `getAssociatedTokenAddressSync` 计算 ata 地址, 第三个参数 `true` 表示 owner 可以是 off-curve(即 pda), 这是必须设置的, 否则会抛出错误.

调用程序的 make_offer 指令, 传入参数和账户:

```typescript
const transactionSignature = await program.methods
  .makeOffer(offerId, tokenAOfferedAmount, tokenBWantedAmount)
  .accounts({
    maker: alice.publicKey,
    mintA: tokenMintA.publicKey,
    mintB: tokenMintB.publicKey,
    makerTokenAccountA,
    offer,
    offerTokenAccount,
    tokenProgram: TOKEN_PROGRAM,
  })
  .signers([alice])
  .rpc();

await confirmTransaction(connection, transactionSignature);
```

Anchor 客户端的调用方式非常直观:

- `.methods.makeOffer(...)`: 指定指令名和参数
- `.accounts({...})`: 传入账户地址, anchor 会自动推导(resolve)未传入的账户(如 `associatedTokenProgram`, `systemProgram`)
- `.signers([alice])`: 传入额外的签名者(provider wallet 会自动签名, 但 alice 需要额外添加)
- `.rpc()`: 发送交易并返回签名
- `confirmTransaction`: 等待交易被链上确认

```typescript
// 验证托管仓库中含有正确数量的代币 a
// import { assert } from "chai";
const vaultBalanceResponse = await connection.getTokenAccountBalance(offerTokenAccount);
const vaultBalance = new BN(vaultBalanceResponse.value.amount);
assert(vaultBalance.eq(tokenAOfferedAmount));

// 验证 offer 账户中数据正确
const offerAccount = await program.account.offer.fetch(offer);

assert(offerAccount.maker.equals(alice.publicKey));
assert(offerAccount.mintA.equals(tokenMintA.publicKey));
assert(offerAccount.mintB.equals(tokenMintB.publicKey));
assert(offerAccount.tokenBWantedAmount.eq(tokenBWantedAmount));
```

断言验证两件事:

1. 托管账户余额: `offerTokenAccount` 中确实存入了 1 million 个代币 a.
2. Offer 账户数据: `program.account.offer.fetch(offer)` 从链上读取 offer pda 的数据, 验证每个字段都正确保存了.

### 3.3 测试二: take_offer

```typescript
it("Puts the tokens from the vault into Bob's account, and gives Alice Bob's tokens, when Bob takes an offer", async () => {
  const transactionSignature = await program.methods
    .takeOffer()
    .accounts({
      taker: bob.publicKey,
      maker: alice.publicKey,
      mintA: tokenMintA.publicKey,
      mintB: tokenMintB.publicKey,
      takerTokenAccountA,
      takerTokenAccountB,
      makerTokenAccountB,
      offer,
      offerTokenAccount,
      tokenProgram: TOKEN_PROGRAM,
    })
    .signers([bob])
    .rpc();
  await confirmTransaction(connection, transactionSignature);
});
```

`take_offer` 指令不需要额外参数(所有条件都在 offer 账户中存储), 因此 `.methods.takeOffer()` 不传参数. 账户中的 `offer` 和 `offerTokenAccount` 正是在上一个测试中保存的地址, 体现了两个测试的依赖关系.

```typescript
// 验证 bob 账户中收到了代币 a
const bobTokenAccountBalanceAfterResponse =
  await connection.getTokenAccountBalance(takerTokenAccountA);
const bobTokenAccountBalanceAfter = new BN(
  bobTokenAccountBalanceAfterResponse.value.amount
);
assert(bobTokenAccountBalanceAfter.eq(tokenAOfferedAmount));

// 验证 alice 账户中收到了代币 b
const aliceTokenAccountBalanceAfterResponse =
  await connection.getTokenAccountBalance(makerTokenAccountB);
const aliceTokenAccountBalanceAfter = new BN(
  aliceTokenAccountBalanceAfterResponse.value.amount
);
assert(aliceTokenAccountBalanceAfter.eq(tokenBWantedAmount));
```

最终验证交换结果:

- bob 的代币 a 账户增加了 1 million
- alice 的代币 b 账户增加了 1 million

这两个断言共同确认了代币在双方之间成功进行了原子交换.

## 3.4 运行测试

```bash
anchor test
```

看到如下输出即为成功:

```
escrow
  ✔ Puts the tokens Alice offers into the vault when Alice makes an offer (xxxx ms)
  ✔ Puts the tokens from the vault into Bob's account, and gives Alice Bob's tokens, when Bob takes an offer (xxxx ms)

2 passing (xxxx)
```

## 总结

我们从背景到实现完整地走了一遍 solana escrow 程序, 这套设计模式在 solana defi 协议中非常常见, 掌握之后你就具备了构建 amm, nft 市场等更复杂交换场景的基础. 感谢收看, 下节课见!
