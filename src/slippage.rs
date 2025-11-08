// Constant product AMM slippage calculations
// Uses x * y = k formula to estimate price impact

use crate::arb_engine::PoolInfo;

/// Calculate slippage for a swap using constant product formula
/// Returns (output_amount, slippage_pct)
pub fn calculate_swap_slippage(
    pool: &PoolInfo,
    input_token_idx: usize,
    input_amount: f64,
) -> (f64, f64) {
    if pool.amounts.len() != 2 || input_token_idx > 1 {
        return (0.0, 100.0); // Invalid
    }

    let output_token_idx = 1 - input_token_idx;

    // Convert to f64 for calculation
    let reserve_in = pool.amounts[input_token_idx] as f64;
    let reserve_out = pool.amounts[output_token_idx] as f64;

    if reserve_in == 0.0 || reserve_out == 0.0 {
        return (0.0, 100.0); // Empty pool
    }

    // Apply fee (e.g., 0.25% = 0.0025)
    let fee_rate = pool.total_fee as f64 / 10000.0;
    let input_after_fee = input_amount * (1.0 - fee_rate);

    // Constant product formula: (x + Δx) * (y - Δy) = x * y
    // Solving for Δy: Δy = (y * Δx) / (x + Δx)
    let output_amount = (reserve_out * input_after_fee) / (reserve_in + input_after_fee);

    // Calculate price before and after swap
    let price_before = reserve_out / reserve_in;
    let price_after = (reserve_out - output_amount) / (reserve_in + input_amount);

    // Slippage = (price_before - price_after) / price_before
    let slippage_pct = ((price_before - price_after) / price_before).abs() * 100.0;

    (output_amount, slippage_pct)
}

/// Calculate maximum trade size for a given slippage tolerance
/// Returns the input amount that would cause exactly `max_slippage_pct` slippage
pub fn calculate_max_trade_size(
    pool: &PoolInfo,
    input_token_idx: usize,
    max_slippage_pct: f64,
) -> f64 {
    if pool.amounts.len() != 2 || input_token_idx > 1 {
        return 0.0;
    }

    let reserve_in = pool.amounts[input_token_idx] as f64;
    let reserve_out = pool.amounts[1 - input_token_idx] as f64;

    if reserve_in == 0.0 || reserve_out == 0.0 {
        return 0.0;
    }

    // Approximate using simplified formula
    // For small slippage: max_input ≈ reserve_in * slippage / (1 - slippage)
    let slippage_ratio = max_slippage_pct / 100.0;
    let max_input = reserve_in * slippage_ratio / (1.0 - slippage_ratio);

    max_input.max(0.0)
}

/// Calculate price impact for a multi-hop swap (2 or 3 pools)
/// Returns (final_output, total_slippage_pct)
pub fn calculate_multihop_slippage(pools: &[&PoolInfo], input_amount: f64) -> (f64, f64) {
    if pools.is_empty() {
        return (0.0, 100.0);
    }

    let mut current_amount = input_amount;
    let mut total_slippage = 0.0;

    for (i, pool) in pools.iter().enumerate() {
        // For each hop, input token is determined by the path
        // First hop: input is token 0, second hop: depends on pool structure
        // Simplified: assume alternating for now
        let input_idx = i % 2;

        let (output, slippage) = calculate_swap_slippage(pool, input_idx, current_amount);
        current_amount = output;
        total_slippage += slippage;

        if output == 0.0 {
            return (0.0, 100.0); // Swap failed
        }
    }

    (current_amount, total_slippage)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool(reserve_a: u128, reserve_b: u128) -> PoolInfo {
        PoolInfo {
            pool_id: 1,
            token_account_ids: vec!["token_a".to_string(), "token_b".to_string()],
            amounts: vec![reserve_a, reserve_b],
            total_fee: 25, // 0.25%
            shares_total_supply: 1_000_000,
        }
    }

    #[test]
    fn test_small_swap_low_slippage() {
        // Pool with 1M tokens on each side
        let pool = create_test_pool(1_000_000, 1_000_000);

        // Swap 1000 tokens (0.1% of pool)
        let (output, slippage) = calculate_swap_slippage(&pool, 0, 1000.0);

        // Should get close to 1000 tokens out (minus fee)
        assert!(output > 990.0 && output < 1000.0);
        // Slippage should be very low (<0.2%)
        assert!(slippage < 0.2);
    }

    #[test]
    fn test_large_swap_high_slippage() {
        let pool = create_test_pool(1_000_000, 1_000_000);

        // Swap 100k tokens (10% of pool)
        let (output, slippage) = calculate_swap_slippage(&pool, 0, 100_000.0);

        // Should get less than 100k out due to slippage
        assert!(output < 100_000.0);
        // Slippage should be significant (>5%)
        assert!(slippage > 5.0);
    }

    #[test]
    fn test_max_trade_size() {
        let pool = create_test_pool(1_000_000, 1_000_000);

        // Find max trade size for 1% slippage
        let max_size = calculate_max_trade_size(&pool, 0, 1.0);

        // Should be around 1% of pool reserves
        assert!(max_size > 9_000.0 && max_size < 11_000.0);

        // Verify it actually produces ~1% slippage
        let (_, slippage) = calculate_swap_slippage(&pool, 0, max_size);
        assert!((slippage - 1.0).abs() < 0.5); // Within 0.5% of target
    }
}
