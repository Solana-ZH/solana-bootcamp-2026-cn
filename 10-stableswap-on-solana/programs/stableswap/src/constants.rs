/// 首次存入时锁定的虚拟份额，用于防止 LP 通胀攻击。
/// 等价于 Uniswap V2 中的 `MINIMUM_LIQUIDITY`。
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

/// 放大系数 A 的最大值。
/// A 越高，价格曲线越平缓，滑点更低，但灵活性也更差。
/// Curve Finance 通常对稳定币交易对使用 100-2000。
pub const MAX_AMP: u64 = 1_000_000;

/// 交换手续费上限，单位为基点（100% = 10_000 bps）。
pub const MAX_FEE_BPS: u16 = 10_000;

/// 计算不变量时牛顿法的最大迭代次数。
pub const MAX_ITERATIONS: u8 = 255;
