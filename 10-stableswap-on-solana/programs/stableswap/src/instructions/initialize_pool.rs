use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use crate::constants::{MAX_AMP, MAX_FEE_BPS};
use crate::errors::StableSwapError;
use crate::state::Pool;

/// 初始化新的 StableSwap 池所需的账户。
#[derive(Accounts)]
pub struct InitializePool<'info> {
    /// 池子的创建者，负责支付账户创建费用。
    #[account(mut)]
    pub admin: Signer<'info>,

    /// Token A 的 mint（例如 USDC）。
    pub token_mint_a: Account<'info, Mint>,

    /// Token B 的 mint（例如 USDT）。
    pub token_mint_b: Account<'info, Mint>,

    /// 池子的 PDA，由 `[b"pool", token_mint_a, token_mint_b]` 推导得到。
    #[account(
        init,
        payer = admin,
        space = Pool::LEN,
        seeds = [b"pool", token_mint_a.key().as_ref(), token_mint_b.key().as_ref()],
        bump,
    )]
    pub pool: Account<'info, Pool>,

    /// LP token 的 mint，其 authority 设为池子的 PDA。
    #[account(
        init,
        payer = admin,
        mint::decimals = 6,
        mint::authority = pool,
    )]
    pub lp_mint: Account<'info, Mint>,

    /// 池子的 Token A vault（由池子 PDA 持有的 ATA）。
    #[account(
        init,
        payer = admin,
        associated_token::mint = token_mint_a,
        associated_token::authority = pool,
    )]
    pub vault_a: Account<'info, TokenAccount>,

    /// 池子的 Token B vault（由池子 PDA 持有的 ATA）。
    #[account(
        init,
        payer = admin,
        associated_token::mint = token_mint_b,
        associated_token::authority = pool,
    )]
    pub vault_b: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

/// 初始化一个新的双代币 StableSwap 池。
///
/// # 参数
/// * `amplification` — A 参数。对于稳定资产对，常见范围为 100–2000。
///   A 越高，两种 token 越接近平价时滑点越低。
/// * `fee_bps` — 交换手续费，单位为基点（例如 4 = 0.04%）。
pub fn initialize_pool_handler(
    ctx: Context<InitializePool>,
    amplification: u64,
    fee_bps: u16,
) -> Result<()> {
    require!(
        amplification > 0 && amplification <= MAX_AMP,
        StableSwapError::InvalidAmplification
    );
    require!(fee_bps <= MAX_FEE_BPS, StableSwapError::InvalidFee);
    require!(
        ctx.accounts.token_mint_a.key() != ctx.accounts.token_mint_b.key(),
        StableSwapError::InvalidMint
    );
    require!(
        ctx.accounts.token_mint_a.decimals == ctx.accounts.token_mint_b.decimals,
        StableSwapError::InvalidMintDecimals
    );

    ctx.accounts.pool.set_inner(Pool {
        admin: ctx.accounts.admin.key(),
        token_mint_a: ctx.accounts.token_mint_a.key(),
        vault_a: ctx.accounts.vault_a.key(),
        token_mint_b: ctx.accounts.token_mint_b.key(),
        vault_b: ctx.accounts.vault_b.key(),
        lp_mint: ctx.accounts.lp_mint.key(),
        amplification,
        fee_bps,
        bump: ctx.bumps.pool,
    });

    msg!(
        "StableSwap pool initialized: A={}, fee={}bps",
        amplification,
        fee_bps
    );
    Ok(())
}
