use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount, Transfer};
use crate::constants::MINIMUM_LIQUIDITY;
use crate::errors::StableSwapError;
use crate::math::calculate_lp_mint_amount;
use crate::state::Pool;

/// 向 StableSwap 池添加流动性所需的账户。
#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    /// Token A 的 mint，与 `token_mint_b` 一起用于推导池子的 PDA。
    pub token_mint_a: Box<Account<'info, Mint>>,

    /// Token B 的 mint，与 `token_mint_a` 一起用于推导池子的 PDA。
    pub token_mint_b: Box<Account<'info, Mint>>,

    /// 池子的 PDA，可由 `[b"pool", token_mint_a, token_mint_b]` 自动解析。
    #[account(
        mut,
        seeds = [b"pool", token_mint_a.key().as_ref(), token_mint_b.key().as_ref()],
        bump = pool.bump,
    )]
    pub pool: Account<'info, Pool>,

    /// 池子的 Token A vault。
    #[account(
        mut,
        constraint = vault_a.key() == pool.vault_a @ StableSwapError::InvalidVault,
    )]
    pub vault_a: Account<'info, TokenAccount>,

    /// 池子的 Token B vault。
    #[account(
        mut,
        constraint = vault_b.key() == pool.vault_b @ StableSwapError::InvalidVault,
    )]
    pub vault_b: Account<'info, TokenAccount>,

    /// LP token 的 mint。
    #[account(
        mut,
        constraint = lp_mint.key() == pool.lp_mint @ StableSwapError::InvalidMint,
    )]
    pub lp_mint: Account<'info, Mint>,

    /// 存款者的 Token A 账户。
    #[account(mut)]
    pub user_token_a: Account<'info, TokenAccount>,

    /// 存款者的 Token B 账户。
    #[account(mut)]
    pub user_token_b: Account<'info, TokenAccount>,

    /// 存款者的 LP token 账户（接收新铸造的 LP token）。
    #[account(mut)]
    pub user_lp_token: Account<'info, TokenAccount>,

    /// 存款者签名账户。
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// 向池子存入 Token A 和 Token B，以换取 LP token。
///
/// # 参数
/// * `amount_a`     — 要存入的 Token A 数量。
/// * `amount_b`     — 要存入的 Token B 数量。
/// * `min_lp_out`   — 最少应收到的 LP token 数量（滑点保护）。
pub fn add_liquidity_handler(
    ctx: Context<AddLiquidity>,
    amount_a: u64,
    amount_b: u64,
    min_lp_out: u64,
) -> Result<()> {
    require!(amount_a > 0 || amount_b > 0, StableSwapError::ZeroAmount);

    let pool = &ctx.accounts.pool;
    let reserve_a = ctx.accounts.vault_a.amount as u128;
    let reserve_b = ctx.accounts.vault_b.amount as u128;
    let lp_supply = ctx.accounts.lp_mint.supply as u128;
    let amp = pool.amplification as u128;

    let new_reserve_a = reserve_a + amount_a as u128;
    let new_reserve_b = reserve_b + amount_b as u128;

    let lp_to_mint = calculate_lp_mint_amount(
        reserve_a,
        reserve_b,
        new_reserve_a,
        new_reserve_b,
        lp_supply,
        amp,
        MINIMUM_LIQUIDITY,
    )?;

    require!(lp_to_mint >= min_lp_out, StableSwapError::SlippageExceeded);
    require!(lp_to_mint > 0, StableSwapError::ZeroAmount);

    // 将 Token A 从用户账户转入 vault
    if amount_a > 0 {
        token::transfer(
            CpiContext::new(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.user_token_a.to_account_info(),
                    to: ctx.accounts.vault_a.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_a,
        )?;
    }

    // 将 Token B 从用户账户转入 vault
    if amount_b > 0 {
        token::transfer(
            CpiContext::new(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.user_token_b.to_account_info(),
                    to: ctx.accounts.vault_b.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_b,
        )?;
    }

    // 给用户铸造 LP token（mint authority 为池子的 PDA）
    let seeds: &[&[u8]] = &[
        b"pool",
        pool.token_mint_a.as_ref(),
        pool.token_mint_b.as_ref(),
        &[pool.bump],
    ];
    token::mint_to(
        CpiContext::new_with_signer(
            anchor_spl::token::ID,
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[seeds],
        ),
        lp_to_mint,
    )?;

    msg!(
        "Added liquidity: a={} b={} lp_minted={}",
        amount_a,
        amount_b,
        lp_to_mint
    );
    Ok(())
}
