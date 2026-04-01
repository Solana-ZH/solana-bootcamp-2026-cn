use anchor_lang::prelude::*;

/// 一个双代币的 StableSwap 流动性池。
///
/// 适用于预期价格接近 1:1 的资产对（例如 USDC/USDT）。
/// 使用 Curve StableSwap 不变量，在平衡交换时提供极低滑点，
/// 同时也支持非平衡仓位。
#[account]
pub struct Pool {
    /// 池子的创建者 / 管理员权限账户
    pub admin: Pubkey,
    /// Token A 的 mint 地址
    pub token_mint_a: Pubkey,
    /// 保存池子中 Token A 储备的 vault（ATA）
    pub vault_a: Pubkey,
    /// Token B 的 mint 地址
    pub token_mint_b: Pubkey,
    /// 保存池子中 Token B 储备的 vault（ATA）
    pub vault_b: Pubkey,
    /// LP token 的 mint，存入时增发，提取时销毁
    pub lp_mint: Pubkey,
    /// StableSwap 放大系数（A）。
    /// 用于控制曲线的“稳定性”，值越高，接近平价时的滑点越低。
    pub amplification: u64,
    /// 交换手续费，单位为基点（1 bps = 0.01%）
    pub fee_bps: u16,
    /// PDA bump seed
    pub bump: u8,
}

impl Pool {
    pub const LEN: usize = 8   // 账户判别器
        + 32   // admin
        + 32   // token_mint_a
        + 32   // vault_a
        + 32   // token_mint_b
        + 32   // vault_b
        + 32   // lp_mint
        + 8    // amplification
        + 2    // fee_bps
        + 1;   // bump
}
