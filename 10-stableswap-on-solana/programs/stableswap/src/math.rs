use crate::errors::StableSwapError;
use anchor_lang::prelude::*;

// ─── StableSwap 不变量 ───────────────────────────────────────────────────────
// 
// 对于双代币池，StableSwap 不变量公式为：
//
//   4·A·(x + y) + D  =  4·A·D + D³/(4·x·y)
//
// 公式推导参考：how_to_create_stableswap.md
// 其中：
//   A  = 放大系数
//   x  = token 0 的储备量
//   y  = token 1 的储备量
//   D  = 池子总价值（不变量）
//
// 当 A → ∞ 时，曲线趋近于常数和模型（x + y = D，几乎零滑点）
// 当 A → 0 时，曲线趋近于常数乘积模型（x·y = k，滑点较高）
//
// 当 n=2 时，通用 Curve 公式可化简为这个形式。
// 这里使用 Newton-Raphson 牛顿迭代法来求解 D 和 y。
// ─────────────────────────────────────────────────────────────────────────────

/// 根据两侧储备量和放大系数 A，计算 StableSwap 不变量 D。
///
/// 以 `D = x + y` 为初始值，使用 Newton-Raphson 迭代法求解。
///
/// # 错误
/// 如果任一储备量为零，则返回 [`StableSwapError::EmptyPool`]。
/// 如果迭代未能收敛，则返回 [`StableSwapError::ConvergenceFailed`]。
pub fn compute_d(reserve_a: u128, reserve_b: u128, amp: u128) -> Result<u128> {
    require!(reserve_a > 0 && reserve_b > 0, StableSwapError::EmptyPool);

    let s = reserve_a
        .checked_add(reserve_b)
        .ok_or(StableSwapError::MathOverflow)?;

    // A·n^n，其中 n=2，因此化简为 4·A
    let ann = amp
        .checked_mul(4)
        .ok_or(StableSwapError::MathOverflow)?;

    let mut d = s;

    for _ in 0..255u8 {
        let d_prev = d;

        // D_P = D³ / (4·x·y)，这里通过迭代方式累积计算：
        //   d_p = d; d_p = d_p·D / (2·x); d_p = d_p·D / (2·y)
        let dp = d
            .checked_mul(d)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_div(
                reserve_a
                    .checked_mul(2)
                    .ok_or(StableSwapError::MathOverflow)?,
            )
            .ok_or(StableSwapError::MathOverflow)?
            .checked_mul(d)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_div(
                reserve_b
                    .checked_mul(2)
                    .ok_or(StableSwapError::MathOverflow)?,
            )
            .ok_or(StableSwapError::MathOverflow)?;

        // 牛顿迭代步骤：
        // D = (ann·S + D_P·2) · D / ((ann - 1)·D + 3·D_P)
        let numerator = ann
            .checked_mul(s)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_add(dp.checked_mul(2).ok_or(StableSwapError::MathOverflow)?)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_mul(d)
            .ok_or(StableSwapError::MathOverflow)?;

        let denominator = ann
            .checked_sub(1)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_mul(d)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_add(dp.checked_mul(3).ok_or(StableSwapError::MathOverflow)?)
            .ok_or(StableSwapError::MathOverflow)?;

        d = numerator
            .checked_div(denominator)
            .ok_or(StableSwapError::MathOverflow)?;

        if d.abs_diff(d_prev) <= 1 {
            return Ok(d);
        }
    }

    Err(StableSwapError::ConvergenceFailed.into())
}

/// 给定 token i 的新储备量，计算 token j 的新储备量，
/// 使得不变量 D 保持不变。
///
/// 求解方程：`y² + c·y = b·y + c`（使用 Newton-Raphson）。
/// 其中 `c` 和 `b` 由 D 以及另一侧储备推导得到。
///
/// # 参数
/// * `reserve_other` - 当前另一种 token 的储备量
///   （已代入输入 token 更新后的数值）
/// * `d`             - 预先计算好的不变量
/// * `amp`           - 放大系数
pub fn compute_y(reserve_other: u128, d: u128, amp: u128) -> Result<u128> {
    require!(reserve_other > 0, StableSwapError::EmptyPool);

    // ann = 4·A（n=2）
    let ann = amp
        .checked_mul(4)
        .ok_or(StableSwapError::MathOverflow)?;

    // b = reserve_other + D/ann
    let b = reserve_other
        .checked_add(
            d.checked_div(ann).ok_or(StableSwapError::MathOverflow)?,
        )
        .ok_or(StableSwapError::MathOverflow)?;

    // c = D³ / (4·ann·reserve_other)
    let c = d
        .checked_mul(d)
        .ok_or(StableSwapError::MathOverflow)?
        .checked_mul(d)
        .ok_or(StableSwapError::MathOverflow)?
        .checked_div(
            reserve_other
                .checked_mul(4)
                .ok_or(StableSwapError::MathOverflow)?
                .checked_mul(ann)
                .ok_or(StableSwapError::MathOverflow)?,
        )
        .ok_or(StableSwapError::MathOverflow)?;

    // Newton-Raphson：y = (y² + c) / (2y + b - D)
    let mut y = d;

    for _ in 0..255u8 {
        let y_prev = y;

        let numerator = y
            .checked_mul(y)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_add(c)
            .ok_or(StableSwapError::MathOverflow)?;

        let denominator = y
            .checked_mul(2)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_add(b)
            .ok_or(StableSwapError::MathOverflow)?
            .checked_sub(d)
            .ok_or(StableSwapError::MathOverflow)?;

        y = numerator
            .checked_div(denominator)
            .ok_or(StableSwapError::MathOverflow)?;

        if y.abs_diff(y_prev) <= 1 {
            return Ok(y);
        }
    }

    Err(StableSwapError::ConvergenceFailed.into())
}

/// 计算一次交换在扣除手续费后的输出数量。
///
/// # 返回值
/// `(amount_out, fee_amount)`
pub fn calculate_swap_output(
    reserve_in: u128,
    reserve_out: u128,
    amount_in: u128,
    amp: u128,
    fee_bps: u16,
) -> Result<(u128, u128)> {
    require!(amount_in > 0, StableSwapError::ZeroAmount);

    let d = compute_d(reserve_in, reserve_out, amp)?;

    let new_reserve_in = reserve_in
        .checked_add(amount_in)
        .ok_or(StableSwapError::MathOverflow)?;

    let new_reserve_out = compute_y(new_reserve_in, d, amp)?;

    let amount_out_before_fee = reserve_out
        .checked_sub(new_reserve_out)
        .ok_or(StableSwapError::MathOverflow)?;

    let fee = amount_out_before_fee
        .checked_mul(fee_bps as u128)
        .ok_or(StableSwapError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(StableSwapError::MathOverflow)?;

    let amount_out = amount_out_before_fee
        .checked_sub(fee)
        .ok_or(StableSwapError::MathOverflow)?;

    Ok((amount_out, fee))
}

/// 计算一次存款应增发多少 LP token。
///
/// 在**首次存款**时（`lp_supply == 0`），会增发
/// `D - MINIMUM_LIQUIDITY` 个 token，并锁定 `MINIMUM_LIQUIDITY`
/// 作为虚拟死份额，以防止首个存款者通胀攻击。
///
/// 在**后续存款**时，按照 D 的变化比例计算：
///   mint = lp_supply * (D_after - D_before) / D_before
pub fn calculate_lp_mint_amount(
    reserve_a_before: u128,
    reserve_b_before: u128,
    reserve_a_after: u128,
    reserve_b_after: u128,
    lp_supply: u128,
    amp: u128,
    minimum_liquidity: u64,
) -> Result<u64> {
    if lp_supply == 0 {
        let d = compute_d(reserve_a_after, reserve_b_after, amp)?;
        require!(
            d > minimum_liquidity as u128,
            StableSwapError::InsufficientInitialLiquidity
        );
        let lp_to_mint = (d - minimum_liquidity as u128)
            .min(u64::MAX as u128) as u64;
        return Ok(lp_to_mint);
    }

    let d_before = compute_d(reserve_a_before, reserve_b_before, amp)?;
    let d_after = compute_d(reserve_a_after, reserve_b_after, amp)?;

    require!(d_after >= d_before, StableSwapError::InsufficientLiquidity);

    let d_diff = d_after
        .checked_sub(d_before)
        .ok_or(StableSwapError::MathOverflow)?;

    let lp_to_mint = lp_supply
        .checked_mul(d_diff)
        .ok_or(StableSwapError::MathOverflow)?
        .checked_div(d_before)
        .ok_or(StableSwapError::MathOverflow)?
        .min(u64::MAX as u128) as u64;

    Ok(lp_to_mint)
}

// ─── 测试 ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 平衡池：D 应接近两侧储备量之和。
    #[test]
    fn test_compute_d_balanced() {
        let reserve = 1_000_000_000u128; // 1000 USDC（6 位小数）
        let d = compute_d(reserve, reserve, 100).unwrap();
        // 对于平衡池，D 应约等于 2 * reserve
        assert!(d > 1_900_000_000u128);
        assert!(d < 2_100_000_000u128);
    }

    /// 在 1M/1M 的池子中交换 1 USDC，输出应接近 1 USDC。
    #[test]
    fn test_swap_low_slippage() {
        let reserve = 1_000_000_000_000u128; // 1M USDC（6 位小数）
        let amount_in = 1_000_000u128;       // 1 USDC
        let (out, _fee) = calculate_swap_output(reserve, reserve, amount_in, 100, 4).unwrap();
        // 当 A=100，且交易量相对大池子极小时，应几乎接近 1:1
        assert!(out > 990_000u128);
        assert!(out <= amount_in);
    }

    /// 大额交换也应保持较低滑点（稳定币池的优势）。
    #[test]
    fn test_swap_large_amount_low_slippage() {
        let reserve = 1_000_000_000_000u128; // 1M USDC
        let amount_in = 100_000_000_000u128; // 100k USDC 交换（池子 10%）
        let (out, _fee) = calculate_swap_output(reserve, reserve, amount_in, 100, 4).unwrap();
        // StableSwap 对于池子 10% 的交换，输出比例应高于 99% 左右
        let ratio = out * 100 / amount_in;
        assert!(ratio > 98, "预期输出比例 >98%，实际为 {}%", ratio);
    }

    /// 首次存款时的 LP 增发。
    #[test]
    fn test_lp_mint_first_deposit() {
        let amount = 1_000_000_000u128; // 每种 token 各 1000
        let lp = calculate_lp_mint_amount(0, 0, amount, amount, 0, 100, 1_000).unwrap();
        // LP ≈ D - MINIMUM_LIQUIDITY ≈ 2_000_000_000 - 1_000
        assert!(lp > 1_999_000_000u64);
    }

    /// 后续存款时，LP 数量应按比例增长。
    #[test]
    fn test_lp_mint_subsequent_deposit() {
        let reserve = 1_000_000_000u128;
        let lp_supply = 2_000_000_000u128;
        // 储备翻倍时，LP 供应量也应近似翻倍
        let lp = calculate_lp_mint_amount(
            reserve,
            reserve,
            reserve * 2,
            reserve * 2,
            lp_supply,
            100,
            1_000,
        )
        .unwrap();
        // 新增 LP 应与当前供应量大致相当
        assert!(lp > 1_900_000_000u64);
        assert!(lp < 2_100_000_000u64);
    }
}
