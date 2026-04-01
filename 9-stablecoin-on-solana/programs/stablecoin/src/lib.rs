use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{Token2022, mint_to, burn, MintTo, Burn},
    token_interface::{Mint, TokenAccount},
};

declare_id!("rYXfi25x9JMgau82aGMJMVUokq7JzueqehiJUmwR97Q");

#[program]
pub mod stablecoin {
    use super::*;

    /// 初始化稳定币的 mint 和配置账户
    /// 这里会创建一个新的 Token-2022 mint，并把程序 PDA 设置为 mint authority
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.admin = ctx.accounts.admin.key();
        config.mint = ctx.accounts.mint.key();
        config.paused = false;
        config.bump = ctx.bumps.config;
        config.mint_bump = ctx.bumps.mint;

        Ok(())
    }

    /// 为某个 minter 配置指定的铸造额度
    /// 只有 admin 可以调用这个指令
    /// 如果该 minter 已存在，则更新其 allowance
    pub fn configure_minter(ctx: Context<ConfigureMinter>, allowance: u64) -> Result<()> {
        let minter_config = &mut ctx.accounts.minter_config;

        // 如果该配置账户尚未初始化，则写入 minter 地址和初始状态
        if !minter_config.is_initialized {
            minter_config.minter = ctx.accounts.minter.key();
            minter_config.amount_minted = 0;
            minter_config.is_initialized = true;
            minter_config.bump = ctx.bumps.minter_config;
        }

        minter_config.allowance = allowance;

        msg!("Configured minter {} with allowance {}", ctx.accounts.minter.key(), allowance);

        Ok(())
    }

    /// 移除某个 minter 的授权
    /// 只有 admin 可以调用这个指令
    /// 这里不是删除 minter 这个账户本身，而是关闭该 minter 对应的配置账户 `minter_config`
    /// `minter_config` 被关闭后，rent 会退回给 admin，后续该 minter 也无法再通过授权校验进行 mint
    pub fn remove_minter(_ctx: Context<RemoveMinter>) -> Result<()> {
        msg!("Minter removed");
        Ok(())
    }

    /// 给用户铸造新的稳定币
    /// 只有已授权的 minter 可以调用这个指令
    /// 该 minter 必须还有足够的剩余额度
    pub fn mint_tokens(ctx: Context<MintTokens>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.config;

        // 检查当前是否处于暂停状态
        require!(!config.paused, StablecoinError::Paused);

        // 检查并更新 minter 的已铸造额度
        let minter_config = &mut ctx.accounts.minter_config;
        let remaining = minter_config.allowance.checked_sub(minter_config.amount_minted)
            .ok_or(StablecoinError::ExceedsAllowance)?;
        require!(amount <= remaining, StablecoinError::ExceedsAllowance);

        minter_config.amount_minted = minter_config.amount_minted.checked_add(amount)
            .ok_or(StablecoinError::Overflow)?;

        // 为 mint authority PDA 组装签名 seeds
        let signer_seeds: &[&[&[u8]]] = &[&[b"config", &[config.bump]]];

        // 通过 Token-2022 CPI 把代币铸造到目标账户
        mint_to(
            CpiContext::new_with_signer(
                anchor_spl::token_2022::ID,
                MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.destination.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        msg!("Minted {} tokens to {}", amount, ctx.accounts.destination.key());

        Ok(())
    }

    /// 从调用者自己的代币账户中销毁稳定币
    /// 任何人都可以销毁自己持有的代币
    /// 在真实稳定币场景中，这通常对应用户赎回法币时的销毁操作
    pub fn burn_tokens(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
        burn(
            CpiContext::new(
                anchor_spl::token_2022::ID,
                Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    from: ctx.accounts.token_account.to_account_info(),
                    authority: ctx.accounts.owner.to_account_info(),
                },
            ),
            amount,
        )?;

        msg!("Burned {} tokens from {}", amount, ctx.accounts.token_account.key());

        Ok(())
    }

    /// 暂停所有 mint 操作
    /// 只有 admin 可以调用这个指令
    pub fn pause(ctx: Context<Pause>) -> Result<()> {
        ctx.accounts.config.paused = true;
        msg!("Stablecoin paused");
        Ok(())
    }

    /// 恢复 mint 操作
    /// 只有 admin 可以调用这个指令
    pub fn unpause(ctx: Context<Unpause>) -> Result<()> {
        ctx.accounts.config.paused = false;
        msg!("Stablecoin unpaused");
        Ok(())
    }
}

// ============================================================================
// 账户结构
// ============================================================================

/// 存储稳定币全局配置的账户
#[account]
#[derive(InitSpace)]
pub struct Config {
    /// 管理员是谁
    pub admin: Pubkey,
    /// 稳定币 mint 地址
    pub mint: Pubkey,
    /// 是否暂停
    pub paused: bool,
    /// PDA bump 信息，后续 PDA 作为 mint authority 签名时会使用
    pub bump: u8,
    /// mint PDA 对应的 bump seed。
    /// 用于后续账户约束中重新校验是 `[b"mint"]` 推导出的 PDA
    pub mint_bump: u8,
}

/// minter 的配置账户
/// 每个被授权的 minter 都有自己独立的配置和额度
#[account]
#[derive(InitSpace)]
pub struct MinterConfig {
    /// minter 的公钥
    pub minter: Pubkey,
    /// 该 minter 总共最多可以铸造的数量
    pub allowance: u64,
    /// 该 minter 已经铸造过的数量
    pub amount_minted: u64,
    /// 该账户是否已经初始化
    pub is_initialized: bool,
    /// `minter_config` PDA 对应的 bump seed。
    /// 用于后续账户约束中重新校验当前传入的 `minter_config`
    /// 是否确实是 `[b"minter", minter_pubkey]` 推导出的 PDA。
    pub bump: u8,
}

// ============================================================================
// 指令上下文
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// 存储稳定币配置的 config 账户
    #[account(
        init,
        payer = admin,
        space = 8 + Config::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    /// Token-2022 稳定币 mint 账户
    /// 这里把 config PDA 同时设置为 mint authority 和 freeze authority
    #[account(
        init,
        payer = admin,
        mint::decimals = 6,
        mint::authority = config,
        mint::freeze_authority = config,
        seeds = [b"mint"],
        bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ConfigureMinter<'info> {
    /// 只有 admin 可以配置 minter
    #[account(
        mut,
        constraint = admin.key() == config.admin @ StablecoinError::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// 被配置的 minter
    /// CHECK: 这里只需要它的地址来推导 PDA；任何将被授权 mint 的账户都可以
    pub minter: UncheckedAccount<'info>,

    /// 该 minter 对应的配置账户
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + MinterConfig::INIT_SPACE,
        seeds = [b"minter", minter.key().as_ref()],
        bump
    )]
    pub minter_config: Account<'info, MinterConfig>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RemoveMinter<'info> {
    /// 只有 admin 可以移除 minter
    #[account(
        mut,
        constraint = admin.key() == config.admin @ StablecoinError::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// 要被移除授权的 minter
    /// CHECK: 这里只使用它的公钥来定位对应的 `minter_config` PDA，不检查账户内部数据
    pub minter: UncheckedAccount<'info>,

    /// 要关闭的 minter 配置账户
    /// 通过 `seeds = [b"minter", minter.key().as_ref()]` 约束，确保它确实属于这个 minter
    /// 通过 `close = admin` 在指令成功后自动关闭该账户，并把 rent 退回给 admin
    /// minter 被“移除”的本质，就是这个授权配置账户被关闭，而不是 `minter` 账户本身被删除
    #[account(
        mut,
        close = admin,
        seeds = [b"minter", minter.key().as_ref()],
        bump = minter_config.bump
    )]
    pub minter_config: Account<'info, MinterConfig>,
}

#[derive(Accounts)]
pub struct MintTokens<'info> {
    /// 发起本次 mint 的 minter
    #[account(mut)]
    pub minter: Signer<'info>,

    /// 全局配置账户
    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// 该 minter 对应的配置账户，用来校验其是否已被授权
    #[account(
        mut,
        seeds = [b"minter", minter.key().as_ref()],
        bump = minter_config.bump,
        constraint = minter_config.is_initialized @ StablecoinError::NotMinter
    )]
    pub minter_config: Account<'info, MinterConfig>,

    /// Token-2022 稳定币 mint 账户
    #[account(
        mut,
        seeds = [b"mint"],
        bump = config.mint_bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// 接收铸造代币的 Token-2022 ATA 账户
    #[account(
        init_if_needed,
        payer = minter,
        associated_token::mint = mint,
        associated_token::authority = destination_owner,
        associated_token::token_program = token_program,
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: 目标代币账户的 owner，只用于创建/校验 ATA
    pub destination_owner: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    /// 发起销毁操作的代币账户 owner
    pub owner: Signer<'info>,

    /// 全局配置账户
    #[account(
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,

    /// Token-2022 稳定币 mint 账户
    #[account(
        mut,
        seeds = [b"mint"],
        bump = config.mint_bump
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    /// 要从中销毁代币的 Token-2022 代币账户
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program,
    )]
    pub token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, Token2022>,
}

#[derive(Accounts)]
pub struct Pause<'info> {
    /// 只有 admin 可以暂停
    #[account(
        constraint = admin.key() == config.admin @ StablecoinError::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct Unpause<'info> {
    /// 只有 admin 可以恢复
    #[account(
        constraint = admin.key() == config.admin @ StablecoinError::Unauthorized
    )]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"config"],
        bump = config.bump
    )]
    pub config: Account<'info, Config>,
}

// ============================================================================
// 错误码
// ============================================================================

#[error_code]
pub enum StablecoinError {
    #[msg("You are not authorized to perform this action")]
    Unauthorized,
    #[msg("Minting is currently paused")]
    Paused,
    #[msg("Mint amount exceeds minter's remaining allowance")]
    ExceedsAllowance,
    #[msg("Account is not an authorized minter")]
    NotMinter,
    #[msg("Arithmetic overflow")]
    Overflow,
}
