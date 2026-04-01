use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};
use crate::constants::MINIMUM_LIQUIDITY;
use crate::errors::StableSwapError;
use crate::state::Pool;

/// 从 StableSwap 池移除流动性所需的账户。
#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
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

    /// LP token 的 mint（销毁时总供应量会减少）。
    #[account(
        mut,
        constraint = lp_mint.key() == pool.lp_mint @ StableSwapError::InvalidMint,
    )]
    pub lp_mint: Account<'info, Mint>,

    /// 提取者的 Token A 账户（接收 Token A）。
    #[account(mut)]
    pub user_token_a: Account<'info, TokenAccount>,

    /// 提取者的 Token B 账户（接收 Token B）。
    #[account(mut)]
    pub user_token_b: Account<'info, TokenAccount>,

    /// 提取者的 LP token 账户（LP token 从这里被销毁）。
    #[account(mut)]
    pub user_lp_token: Account<'info, TokenAccount>,

    /// 提取者签名账户。
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// 销毁 LP token，按比例提取 Token A 和 Token B。
///
/// # 参数
/// * `lp_amount` — 要销毁的 LP token 数量。
/// * `min_a`     — 最少应收到的 Token A 数量（滑点保护）。
/// * `min_b`     — 最少应收到的 Token B 数量（滑点保护）。
pub fn remove_liquidity_handler(
    ctx: Context<RemoveLiquidity>,
    lp_amount: u64,
    min_a: u64,
    min_b: u64,
) -> Result<()> {
    require!(lp_amount > 0, StableSwapError::ZeroAmount);

    let pool = &ctx.accounts.pool;
    let reserve_a = ctx.accounts.vault_a.amount as u128;
    let reserve_b = ctx.accounts.vault_b.amount as u128;
    // 链上实际供应量不包含 `MINIMUM_LIQUIDITY` 这部分虚拟份额。
    // 这里将其加回，才能正确计算按比例提取的数量。
    let actual_supply = ctx.accounts.lp_mint.supply;
    let adjusted_supply = (actual_supply as u128)
        .checked_add(MINIMUM_LIQUIDITY as u128)
        .ok_or(StableSwapError::MathOverflow)?;

    require!(adjusted_supply > 0, StableSwapError::EmptyPool);
    require!(
        lp_amount as u128 <= actual_supply as u128,
        StableSwapError::InsufficientLiquidity
    );

    // 按比例提取：amount_i = reserve_i * lp_amount / adjusted_supply
    let amount_a = (reserve_a
        .checked_mul(lp_amount as u128)
        .ok_or(StableSwapError::MathOverflow)?)
    .checked_div(adjusted_supply)
    .ok_or(StableSwapError::MathOverflow)? as u64;

    let amount_b = (reserve_b
        .checked_mul(lp_amount as u128)
        .ok_or(StableSwapError::MathOverflow)?)
    .checked_div(adjusted_supply)
    .ok_or(StableSwapError::MathOverflow)? as u64;

    require!(amount_a >= min_a, StableSwapError::SlippageExceeded);
    require!(amount_b >= min_b, StableSwapError::SlippageExceeded);

    // 从用户账户销毁 LP token
    token::burn(
        CpiContext::new(
            anchor_spl::token::ID,
            Burn {
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        lp_amount,
    )?;

    let seeds: &[&[u8]] = &[
        b"pool",
        pool.token_mint_a.as_ref(),
        pool.token_mint_b.as_ref(),
        &[pool.bump],
    ];

    // 将 Token A 转给用户
    if amount_a > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.vault_a.to_account_info(),
                    to: ctx.accounts.user_token_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[seeds],
            ),
            amount_a,
        )?;
    }

    // 将 Token B 转给用户
    if amount_b > 0 {
        token::transfer(
            CpiContext::new_with_signer(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.vault_b.to_account_info(),
                    to: ctx.accounts.user_token_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[seeds],
            ),
            amount_b,
        )?;
    }

    msg!(
        "Removed liquidity: lp_burned={} a_out={} b_out={}",
        lp_amount,
        amount_a,
        amount_b
    );
    Ok(())
}
