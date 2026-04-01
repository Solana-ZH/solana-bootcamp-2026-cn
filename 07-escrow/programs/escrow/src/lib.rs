use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};

declare_id!("25Q841qjRsaGQzWSKh5kiEZ9qpXbWMzm3v4ytGXs6PzY");

#[program]
pub mod escrow {
    use super::*;

    /// maker 将 mint_a 存入 escrow（以 offer PDA 为 authority 的 ATA），
    /// 创建一个「给我这些 mint_b，我就把 mint_a 给你」的 Offer
    pub fn make_offer(
        ctx: Context<MakeOffer>,
        offer_id: u64,
        token_a_offered_amount: u64,
        token_b_wanted_amount: u64,
    ) -> Result<()> {
        require_keys_neq!(
            ctx.accounts.mint_a.key(),
            ctx.accounts.mint_b.key(),
            EscrowError::SameMint
        );

        // 1) maker -> offer_token_account：将 mint_a 存入托管账户
        let transfer_cpi_accounts = TransferChecked {
            from: ctx.accounts.maker_token_account_a.to_account_info(),
            to: ctx.accounts.offer_token_account.to_account_info(),
            mint: ctx.accounts.mint_a.to_account_info(),
            authority: ctx.accounts.maker.to_account_info(),
        };

        let cpi_context = CpiContext::new(*ctx.accounts.token_program.key, transfer_cpi_accounts);

        let decimals = ctx.accounts.mint_a.decimals;
        transfer_checked(cpi_context, token_a_offered_amount, decimals)?;

        // 2) 保存 Offer 数据
        ctx.accounts.offer.set_inner(Offer {
            maker: ctx.accounts.maker.key(),
            mint_a: ctx.accounts.mint_a.key(),
            mint_b: ctx.accounts.mint_b.key(),
            offer_id,
            token_a_offered_amount,
            token_b_wanted_amount,
            bump: ctx.bumps.offer,
        });

        Ok(())
    }

    /// taker 向 maker 支付 mint_b，并从 escrow 中取回 mint_a
    pub fn take_offer(ctx: Context<TakeOffer>) -> Result<()> {
        // 1) taker -> maker：发送 mint_b（wanted amount）
        {
            let transfer_cpi_accounts = TransferChecked {
                from: ctx.accounts.taker_token_account_b.to_account_info(),
                to: ctx.accounts.maker_token_account_b.to_account_info(),
                mint: ctx.accounts.mint_b.to_account_info(),
                authority: ctx.accounts.taker.to_account_info(),
            };

            let cpi_context =
                CpiContext::new(*ctx.accounts.token_program.key, transfer_cpi_accounts);

            let decimals = ctx.accounts.mint_b.decimals;
            transfer_checked(
                cpi_context,
                ctx.accounts.offer.token_b_wanted_amount,
                decimals,
            )?;
        }

        // 2) escrow(offer_token_account) -> taker：发送 mint_a（offered amount）
        // 因为 offer PDA 是 offer_token_account 的 authority，所以用 offer PDA 签名驱动
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"offer",
            ctx.accounts.offer.maker.as_ref(),
            &ctx.accounts.offer.offer_id.to_le_bytes(),
            &[ctx.accounts.offer.bump],
        ]];

        {
            let transfer_cpi_accounts = TransferChecked {
                from: ctx.accounts.offer_token_account.to_account_info(),
                to: ctx.accounts.taker_token_account_a.to_account_info(),
                mint: ctx.accounts.mint_a.to_account_info(),
                authority: ctx.accounts.offer.to_account_info(),
            };

            let cpi_context =
                CpiContext::new(*ctx.accounts.token_program.key, transfer_cpi_accounts)
                    .with_signer(signer_seeds);

            let decimals = ctx.accounts.mint_a.decimals;
            transfer_checked(
                cpi_context,
                ctx.accounts.offer.token_a_offered_amount,
                decimals,
            )?;
        }

        // 3) 关闭 escrow 的 token account（将租金退还给 maker）
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

        // 4) offer 账户由 Accounts 中的 close = maker 自动关闭
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(offer_id: u64)]
pub struct MakeOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: InterfaceAccount<'info, Mint>,

    #[account(mint::token_program = token_program)]
    pub mint_b: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_token_account_a: InterfaceAccount<'info, TokenAccount>,

    // Offer PDA（seeds 中包含 maker + offer_id，以支持创建多个 Offer）
    #[account(
        init,
        payer = maker,
        space = 8 + Offer::INIT_SPACE,
        seeds = [b"offer", maker.key().as_ref(), &offer_id.to_le_bytes()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    // 以 Offer PDA 为 authority 的 mint_a ATA（escrow 托管仓库）
    #[account(
        init,
        payer = maker,
        associated_token::mint = mint_a,
        associated_token::authority = offer,
        associated_token::token_program = token_program
    )]
    pub offer_token_account: InterfaceAccount<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TakeOffer<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    /// maker 无需签名（即使本人不在场也可以 take）
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(mint::token_program = token_program)]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,

    #[account(mint::token_program = token_program)]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    // Offer（此处指定 close = maker，take 成功时自动关闭 offer）
    #[account(
        mut,
        close = maker,
        has_one = maker @ EscrowError::MakerMismatch,
        has_one = mint_a @ EscrowError::MintMismatch,
        has_one = mint_b @ EscrowError::MintMismatch,
        seeds = [b"offer", offer.maker.as_ref(), &offer.offer_id.to_le_bytes()],
        bump = offer.bump
    )]
    pub offer: Box<Account<'info, Offer>>,

    // taker 接收 mint_a 的 ATA（若不存在则由 taker 作为 payer 创建）
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_token_account_a: Box<InterfaceAccount<'info, TokenAccount>>,

    // taker 用于支付的 mint_b ATA
    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program
    )]
    pub taker_token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // maker 接收 mint_b 的 ATA（若不存在则由 taker 作为 payer 创建）
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program
    )]
    pub maker_token_account_b: Box<InterfaceAccount<'info, TokenAccount>>,

    // escrow 托管仓库（以 Offer PDA 为 authority 的 mint_a ATA）
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

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub maker: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub offer_id: u64,
    pub token_a_offered_amount: u64,
    pub token_b_wanted_amount: u64,
    pub bump: u8,
}

#[error_code]
pub enum EscrowError {
    #[msg("Maker account does not match Offer.maker")]
    MakerMismatch,
    #[msg("Mint account does not match Offer mints")]
    MintMismatch,
    #[msg("mint_a and mint_b must be different")]
    SameMint,
}
