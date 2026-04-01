use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};
use crate::errors::StableSwapError;
use crate::math::calculate_swap_output;
use crate::state::Pool;

/// 执行交换所需的账户。
#[derive(Accounts)]
pub struct Swap<'info> {
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

    /// 用户的输入 token 账户（会被扣款）。
    #[account(mut)]
    pub user_input: Account<'info, TokenAccount>,

    /// 用户的输出 token 账户（会被入账）。
    #[account(mut)]
    pub user_output: Account<'info, TokenAccount>,

    /// 发起交换的用户签名账户。
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

/// 使用 StableSwap 不变量执行代币交换。
///
/// # 参数
/// * `amount_in`     — 要卖出的输入 token 数量。
/// * `min_amount_out` — 最少应收到的输出 token 数量（滑点保护）。
/// * `a_to_b`        — `true` 表示 A→B，`false` 表示 B→A。
pub fn swap_handler(
    ctx: Context<Swap>,
    amount_in: u64,
    min_amount_out: u64,
    a_to_b: bool,
) -> Result<()> {
    require!(amount_in > 0, StableSwapError::ZeroAmount);

    let pool = &ctx.accounts.pool;
    let reserve_a = ctx.accounts.vault_a.amount as u128;
    let reserve_b = ctx.accounts.vault_b.amount as u128;
    let amp = pool.amplification as u128;
    let fee_bps = pool.fee_bps;

    // 两侧储备量都必须大于零，交换才有效
    require!(reserve_a > 0 && reserve_b > 0, StableSwapError::EmptyPool);

    // 确定输入侧和输出侧
    let (reserve_in, reserve_out) = if a_to_b {
        (reserve_a, reserve_b)
    } else {
        (reserve_b, reserve_a)
    };

    let (amount_out, fee) = calculate_swap_output(
        reserve_in,
        reserve_out,
        amount_in as u128,
        amp,
        fee_bps,
    )?;

    require!(
        amount_out >= min_amount_out as u128,
        StableSwapError::SlippageExceeded
    );

    let seeds: &[&[u8]] = &[
        b"pool",
        pool.token_mint_a.as_ref(),
        pool.token_mint_b.as_ref(),
        &[pool.bump],
    ];

    if a_to_b {
        // 转入 A：user → vault_a
        token::transfer(
            CpiContext::new(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.user_input.to_account_info(),
                    to: ctx.accounts.vault_a.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_in,
        )?;

        // 转出 B：vault_b → user
        token::transfer(
            CpiContext::new_with_signer(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.vault_b.to_account_info(),
                    to: ctx.accounts.user_output.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[seeds],
            ),
            amount_out as u64,
        )?;
    } else {
        // 转入 B：user → vault_b
        token::transfer(
            CpiContext::new(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.user_input.to_account_info(),
                    to: ctx.accounts.vault_b.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount_in,
        )?;

        // 转出 A：vault_a → user
        token::transfer(
            CpiContext::new_with_signer(
                anchor_spl::token::ID,
                Transfer {
                    from: ctx.accounts.vault_a.to_account_info(),
                    to: ctx.accounts.user_output.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                &[seeds],
            ),
            amount_out as u64,
        )?;
    }

    msg!(
        "Swap {}: {} in → {} out (fee: {})",
        if a_to_b { "A→B" } else { "B→A" },
        amount_in,
        amount_out,
        fee
    );
    Ok(())
}
