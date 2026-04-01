use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("CorabfeniSyoc4aLcJe7t9b3RaFX5tzVWXdewU1xuA6B");

/// StableSwap AMM：针对稳定币交易对优化的双代币流动性池。
///
/// ## 背景
///
/// 常数乘积型 AMM（如 Uniswap 的 x·y=k）即使在中等规模交易下，
/// 也会产生明显的价格影响。对于理论上应接近等值交易的资产
/// （如 USDC/USDT、mSOL/stSOL 等），Curve StableSwap 不变量：
///
///   4·A·(x + y) + D  =  4·A·D + D³/(4·x·y)
///
/// 可以显著降低滑点，同时在脱锚时仍具备自平衡能力。
/// 放大参数 A 用来控制这种权衡：
///   - 高 A（100-2000）：更稳定，在平价附近曲线近乎平坦
///   - 低 A（1-10）：更接近常数乘积模型，对脱锚情况更稳健
#[program]
pub mod stableswap {
    use super::*;

    /// 创建一个新的双代币 StableSwap 池。
    ///
    /// 初始化池子状态、LP mint 以及两个 token vault。
    ///
    /// # 参数
    /// * `amplification` — A 参数（1–1,000,000），常用范围为 100–2000。
    /// * `fee_bps`        — 交换手续费，单位为基点（例如 4 = 0.04%）。
    pub fn initialize_pool(
        ctx: Context<InitializePool>,
        amplification: u64,
        fee_bps: u16,
    ) -> Result<()> {
        instructions::initialize_pool::initialize_pool_handler(ctx, amplification, fee_bps)
    }

    /// 存入 Token A 和/或 Token B 以获得 LP token。
    ///
    /// LP token 代表池子的按比例份额。
    /// 按照当前池子比例同时存入两种代币效率最高，
    /// 但也支持非平衡存款。
    ///
    /// # 参数
    /// * `amount_a`   — 要存入的 Token A 数量。
    /// * `amount_b`   — 要存入的 Token B 数量。
    /// * `min_lp_out` — 最少应收到的 LP token 数量（滑点保护）。
    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        amount_a: u64,
        amount_b: u64,
        min_lp_out: u64,
    ) -> Result<()> {
        instructions::add_liquidity::add_liquidity_handler(ctx, amount_a, amount_b, min_lp_out)
    }

    /// 销毁 LP token，按比例提取两种代币。
    ///
    /// # 参数
    /// * `lp_amount` — 要销毁的 LP token 数量。
    /// * `min_a`     — 最少应收到的 Token A 数量（滑点保护）。
    /// * `min_b`     — 最少应收到的 Token B 数量（滑点保护）。
    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        lp_amount: u64,
        min_a: u64,
        min_b: u64,
    ) -> Result<()> {
        instructions::remove_liquidity::remove_liquidity_handler(ctx, lp_amount, min_a, min_b)
    }

    /// 将 Token A 交换为 Token B，或将 Token B 交换为 Token A。
    ///
    /// 当两种代币价格接近平价时（稳定币常见场景），
    /// 使用 StableSwap 不变量可以实现极低滑点。
    ///
    /// # 参数
    /// * `amount_in`      — 输入卖出的数量。
    /// * `min_amount_out` — 最少应收到的输出数量（滑点保护）。
    /// * `a_to_b`         — `true` 表示 A→B，`false` 表示 B→A。
    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        a_to_b: bool,
    ) -> Result<()> {
        instructions::swap::swap_handler(ctx, amount_in, min_amount_out, a_to_b)
    }
}
