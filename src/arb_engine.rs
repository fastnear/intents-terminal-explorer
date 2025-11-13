// Ultra-fast arbitrage detection engine for Ref Finance pools
// Monitors pool state changes in real-time and identifies profitable opportunities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MIN_PROFIT_THRESHOLD: f64 = 0.003; // 0.3% minimum profit after fees
const REF_FINANCE_FEE: f64 = 0.0025; // 0.25% fee on Ref Finance
const TICK_MA_WINDOW: usize = 50; // Last 50 pool updates for MA

/// Pool state from Ref Finance contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub pool_id: u64,
    pub token_account_ids: Vec<String>,
    pub amounts: Vec<u128>,
    pub total_fee: u32,
    pub shares_total_supply: u128,
}

impl PoolInfo {
    /// Calculate price of token0 in terms of token1 (how many token1 per 1 token0)
    pub fn price(&self) -> f64 {
        if self.amounts.len() != 2 {
            return 0.0;
        }
        self.amounts[1] as f64 / self.amounts[0] as f64
    }

    /// Get liquidity depth for the pool
    pub fn liquidity_usd(&self) -> f64 {
        // Simplified: assume token1 is a stablecoin
        (self.amounts[1] as f64) / 1e24 // Assuming 24 decimals
    }
}

/// Snapshot of pool state at a specific moment
#[derive(Debug, Clone)]
pub struct PoolState {
    pub pool_id: u64,
    pub price: f64,
    pub liquidity: f64,
    pub timestamp: Instant,
    pub sequence: u64,
}

/// Fixed-size ring buffer for efficient tick-level moving average
#[derive(Debug, Clone)]
pub struct TickMA {
    window_size: usize,
    values: Vec<f64>,
    current_idx: usize,
    filled: bool,
    sum: f64,
}

impl TickMA {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            values: vec![0.0; window_size],
            current_idx: 0,
            filled: false,
            sum: 0.0,
        }
    }

    /// Update with new price tick (returns current MA value)
    #[inline(always)]
    pub fn update(&mut self, new_price: f64) -> f64 {
        if self.filled {
            // Remove oldest value from sum
            self.sum -= self.values[self.current_idx];
        }

        // Add new value
        self.values[self.current_idx] = new_price;
        self.sum += new_price;

        // Advance index
        self.current_idx = (self.current_idx + 1) % self.window_size;

        if !self.filled && self.current_idx == 0 {
            self.filled = true;
        }

        // Calculate MA
        let count = if self.filled {
            self.window_size
        } else {
            self.current_idx
        };
        if count > 0 {
            self.sum / count as f64
        } else {
            new_price
        }
    }

    #[inline(always)]
    pub fn value(&self) -> f64 {
        let count = if self.filled {
            self.window_size
        } else {
            self.current_idx
        };
        if count > 0 {
            self.sum / count as f64
        } else {
            0.0
        }
    }
}

/// Arbitrage opportunity detected
#[derive(Debug, Clone)]
pub struct ArbOpportunity {
    pub arb_type: ArbType,
    pub pool_a: u64,
    pub pool_b: u64,
    pub pool_c: Option<u64>, // For triangle arbitrage
    pub price_a: f64,
    pub price_b: f64,
    pub price_c: Option<f64>, // For triangle arbitrage
    pub spread: f64,
    pub ma_spread: f64,
    pub estimated_profit_pct: f64,
    pub liquidity_a: f64,
    pub liquidity_b: f64,
    pub liquidity_c: Option<f64>, // For triangle arbitrage
    pub confidence: f64,
    pub detected_at: Instant,
}

/// Type of arbitrage opportunity
#[derive(Debug, Clone, PartialEq)]
pub enum ArbType {
    TwoHop,   // Simple: Buy A, Sell B
    Triangle, // Complex: A→B→C→A
}

impl ArbOpportunity {
    /// Calculate expected profit for a given trade size
    pub fn calculate_profit(&self, size_usd: f64) -> f64 {
        match self.arb_type {
            ArbType::TwoHop => {
                // Simplified constant product model with fees
                let profit_before_fees = size_usd * self.spread;
                let fees = size_usd * REF_FINANCE_FEE * 2.0; // Two swaps
                profit_before_fees - fees
            }
            ArbType::Triangle => {
                // Three swaps in a cycle
                let profit_before_fees = size_usd * self.spread;
                let fees = size_usd * REF_FINANCE_FEE * 3.0; // Three swaps
                profit_before_fees - fees
            }
        }
    }

    /// Optimal trade size (Kelly criterion approximation)
    pub fn optimal_size(&self) -> f64 {
        let min_liquidity = match self.arb_type {
            ArbType::TwoHop => self.liquidity_a.min(self.liquidity_b),
            ArbType::Triangle => {
                let liq_c = self.liquidity_c.unwrap_or(f64::MAX);
                self.liquidity_a.min(self.liquidity_b).min(liq_c)
            }
        };
        let max_size = min_liquidity * 0.1; // Max 10% of liquidity
        let kelly_size = max_size * self.confidence;
        kelly_size.min(max_size)
    }
}

/// Pool tracker with tick-level moving averages
#[derive(Debug)]
pub struct PoolTracker {
    pub pool_id: u64,
    pub token_pair: (String, String),
    pub current_price: f64,
    pub current_liquidity: f64,
    pub ma_50: TickMA,
    pub last_update: Instant,
    pub update_count: u64,
    pub pool_info: Option<PoolInfo>, // Store full pool metadata
}

impl PoolTracker {
    pub fn new(pool_id: u64, token_pair: (String, String)) -> Self {
        Self {
            pool_id,
            token_pair,
            current_price: 0.0,
            current_liquidity: 0.0,
            ma_50: TickMA::new(TICK_MA_WINDOW),
            last_update: Instant::now(),
            update_count: 0,
            pool_info: None,
        }
    }

    /// Update pool state and moving average
    #[inline(always)]
    pub fn update(&mut self, pool_info: &PoolInfo) -> f64 {
        self.current_price = pool_info.price();
        self.current_liquidity = pool_info.liquidity_usd();
        self.last_update = Instant::now();
        self.update_count += 1;
        self.pool_info = Some(pool_info.clone()); // Store full metadata

        if self.current_liquidity == 0.0 {
            log::warn!(
                "Pool {} has ZERO liquidity (price: {:.6})",
                self.pool_id,
                self.current_price
            );
        }

        self.ma_50.update(self.current_price)
    }
}

/// Main arbitrage detection engine
pub struct LightningArbEngine {
    /// Track all pools by ID
    pools: HashMap<u64, PoolTracker>,

    /// Pre-compiled 2-hop arbitrage paths (same token pair, different pools)
    two_hop_paths: Vec<(u64, u64)>,

    /// Pre-compiled 3-hop triangle arbitrage paths (A→B→C→A)
    triangle_paths: Vec<TrianglePath>,

    /// Token pair index for fast triangle path discovery
    token_pair_index: HashMap<(String, String), Vec<u64>>, // (token0, token1) -> [pool_ids]

    /// Statistics
    opportunities_detected: u64,
    last_opportunity: Option<Instant>,

    /// Performance tracking
    detection_latencies: Vec<Duration>,
}

/// Triangle arbitrage path (3 pools forming a cycle)
#[derive(Debug, Clone)]
pub struct TrianglePath {
    pub pool_ab: u64, // Pool: A → B
    pub pool_bc: u64, // Pool: B → C
    pub pool_ca: u64, // Pool: C → A
    pub token_a: String,
    pub token_b: String,
    pub token_c: String,
}

impl LightningArbEngine {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            two_hop_paths: Vec::new(),
            triangle_paths: Vec::new(),
            token_pair_index: HashMap::new(),
            opportunities_detected: 0,
            last_opportunity: None,
            detection_latencies: Vec::with_capacity(1000),
        }
    }

    /// Register a pool for tracking
    pub fn register_pool(&mut self, pool_info: &PoolInfo) {
        if pool_info.token_account_ids.len() != 2 {
            return; // Only track 2-token pools for now
        }

        let token_pair = (
            pool_info.token_account_ids[0].clone(),
            pool_info.token_account_ids[1].clone(),
        );

        // Index pool by token pair (both directions)
        self.token_pair_index
            .entry(token_pair.clone())
            .or_insert_with(Vec::new)
            .push(pool_info.pool_id);

        // Also index reverse direction
        let reverse_pair = (token_pair.1.clone(), token_pair.0.clone());
        self.token_pair_index
            .entry(reverse_pair)
            .or_insert_with(Vec::new)
            .push(pool_info.pool_id);

        let mut tracker = PoolTracker::new(pool_info.pool_id, token_pair.clone());
        tracker.update(pool_info); // Initialize with current pool data
        self.pools.insert(pool_info.pool_id, tracker);

        // Find matching pools for arbitrage paths
        self.update_arb_paths(&token_pair, pool_info.pool_id);
    }

    /// Update arbitrage paths when new pool is registered
    fn update_arb_paths(&mut self, token_pair: &(String, String), new_pool_id: u64) {
        // 2-hop: Find other pools with same token pair
        for (pool_id, tracker) in &self.pools {
            if *pool_id != new_pool_id && tracker.token_pair == *token_pair {
                self.two_hop_paths.push((new_pool_id, *pool_id));
            }
        }

        // Triangle: Find potential 3-hop cycles
        self.discover_triangle_paths(token_pair, new_pool_id);
    }

    /// Discover triangle arbitrage paths involving this pool
    fn discover_triangle_paths(&mut self, token_pair: &(String, String), pool_id: u64) {
        let (token_a, token_b) = token_pair;

        // Find pools that connect B → C (for various C)
        for (candidate_pair, candidate_pools) in &self.token_pair_index {
            if candidate_pair.0 != *token_b {
                continue; // Not starting from token_b
            }
            let token_c = &candidate_pair.1;

            if token_c == token_a || token_c == token_b {
                continue; // Would create a degenerate cycle
            }

            // Now find pools that complete the cycle: C → A
            let return_key = (token_c.clone(), token_a.clone());
            if let Some(return_pools) = self.token_pair_index.get(&return_key) {
                // We found a triangle! A→B→C→A
                for &pool_bc in candidate_pools {
                    for &pool_ca in return_pools {
                        if pool_bc != pool_id && pool_ca != pool_id && pool_bc != pool_ca {
                            self.triangle_paths.push(TrianglePath {
                                pool_ab: pool_id,
                                pool_bc,
                                pool_ca,
                                token_a: token_a.clone(),
                                token_b: token_b.clone(),
                                token_c: token_c.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Update pool state and scan for opportunities
    #[inline(always)]
    pub fn on_pool_update(&mut self, pool_info: &PoolInfo) -> Option<ArbOpportunity> {
        let start = Instant::now();

        // Update pool tracker
        let tracker = self.pools.get_mut(&pool_info.pool_id)?;
        let old_price = tracker.current_price;
        let _ma_value = tracker.update(pool_info);
        let new_price = tracker.current_price;

        // Skip if price didn't move significantly (> 0.05%)
        if old_price > 0.0 && (new_price - old_price).abs() / old_price < 0.0005 {
            return None;
        }

        // Scan for opportunities on paths involving this pool
        let opp = self.scan_opportunities(pool_info.pool_id);

        // Track latency
        let latency = start.elapsed();
        if self.detection_latencies.len() < 1000 {
            self.detection_latencies.push(latency);
        }

        opp
    }

    /// Scan for arbitrage opportunities
    #[inline(always)]
    fn scan_opportunities(&mut self, changed_pool: u64) -> Option<ArbOpportunity> {
        let mut best_opp: Option<ArbOpportunity> = None;
        let mut max_spread = 0.0;

        for (pool_a, pool_b) in &self.two_hop_paths {
            // Skip paths not involving changed pool
            if *pool_a != changed_pool && *pool_b != changed_pool {
                continue;
            }

            let tracker_a = self.pools.get(pool_a)?;
            let tracker_b = self.pools.get(pool_b)?;

            // Calculate spread
            let price_a = tracker_a.current_price;
            let price_b = tracker_b.current_price;

            if price_a <= 0.0 || price_b <= 0.0 {
                continue;
            }

            let spread = (price_a - price_b).abs() / price_a.min(price_b);

            // Calculate MA spread for comparison
            let ma_a = tracker_a.ma_50.value();
            let ma_b = tracker_b.ma_50.value();
            let ma_spread = if ma_a > 0.0 && ma_b > 0.0 {
                (ma_a - ma_b).abs() / ma_a.min(ma_b)
            } else {
                spread
            };

            // Check if spread is anomalous (> 2x MA spread)
            let is_anomaly = spread > ma_spread * 2.0;

            // Check if profitable after fees
            let profit_pct = spread - (REF_FINANCE_FEE * 2.0);

            if profit_pct > MIN_PROFIT_THRESHOLD && is_anomaly && spread > max_spread {
                max_spread = spread;

                let confidence = (spread / ma_spread).min(1.0)
                    * (tracker_a.current_liquidity.min(tracker_b.current_liquidity) / 10000.0)
                        .min(1.0);

                best_opp = Some(ArbOpportunity {
                    arb_type: ArbType::TwoHop,
                    pool_a: *pool_a,
                    pool_b: *pool_b,
                    pool_c: None,
                    price_a,
                    price_b,
                    price_c: None,
                    spread,
                    ma_spread,
                    estimated_profit_pct: profit_pct,
                    liquidity_a: tracker_a.current_liquidity,
                    liquidity_b: tracker_b.current_liquidity,
                    liquidity_c: None,
                    confidence,
                    detected_at: Instant::now(),
                });

                self.opportunities_detected += 1;
                self.last_opportunity = Some(Instant::now());
            }
        }

        // Check triangle arbitrage opportunities
        for triangle in &self.triangle_paths {
            // Skip if changed pool is not part of this triangle
            if triangle.pool_ab != changed_pool
                && triangle.pool_bc != changed_pool
                && triangle.pool_ca != changed_pool
            {
                continue;
            }

            let tracker_ab = match self.pools.get(&triangle.pool_ab) {
                Some(t) => t,
                None => continue,
            };
            let tracker_bc = match self.pools.get(&triangle.pool_bc) {
                Some(t) => t,
                None => continue,
            };
            let tracker_ca = match self.pools.get(&triangle.pool_ca) {
                Some(t) => t,
                None => continue,
            };

            // Calculate compound rate: Start with 1 unit of A, end with X units of A
            // A→B: get price_ab units of B
            // B→C: get price_ab * price_bc units of C
            // C→A: get price_ab * price_bc * price_ca units of A
            let price_ab = tracker_ab.current_price;
            let price_bc = tracker_bc.current_price;
            let price_ca = tracker_ca.current_price;

            if price_ab <= 0.0 || price_bc <= 0.0 || price_ca <= 0.0 {
                continue;
            }

            let compound_rate = price_ab * price_bc * price_ca;
            let spread = (compound_rate - 1.0).abs();

            // Need > 0.75% profit after 3 swaps (3 * 0.25% = 0.75% fees)
            let profit_pct = spread - (REF_FINANCE_FEE * 3.0);

            if profit_pct > MIN_PROFIT_THRESHOLD && spread > max_spread {
                max_spread = spread;

                let min_liquidity = tracker_ab
                    .current_liquidity
                    .min(tracker_bc.current_liquidity)
                    .min(tracker_ca.current_liquidity);

                let confidence = (spread / (REF_FINANCE_FEE * 3.0)).min(1.0)
                    * (min_liquidity / 10000.0).min(1.0);

                log::debug!(
                    "Triangle opportunity detected: pools {}→{}→{} | liquidity: ${:.2}, ${:.2}, ${:.2} | min: ${:.2} | confidence: {:.1}% | spread: {:.2}% | profit: {:.2}%",
                    triangle.pool_ab, triangle.pool_bc, triangle.pool_ca,
                    tracker_ab.current_liquidity, tracker_bc.current_liquidity, tracker_ca.current_liquidity,
                    min_liquidity, confidence * 100.0, spread * 100.0, profit_pct * 100.0
                );

                best_opp = Some(ArbOpportunity {
                    arb_type: ArbType::Triangle,
                    pool_a: triangle.pool_ab,
                    pool_b: triangle.pool_bc,
                    pool_c: Some(triangle.pool_ca),
                    price_a: price_ab,
                    price_b: price_bc,
                    price_c: Some(price_ca),
                    spread,
                    ma_spread: 0.0, // TODO: Track MA for compound rates
                    estimated_profit_pct: profit_pct,
                    liquidity_a: tracker_ab.current_liquidity,
                    liquidity_b: tracker_bc.current_liquidity,
                    liquidity_c: Some(tracker_ca.current_liquidity),
                    confidence,
                    detected_at: Instant::now(),
                });

                self.opportunities_detected += 1;
                self.last_opportunity = Some(Instant::now());
            }
        }

        best_opp
    }

    /// Get performance stats
    pub fn stats(&self) -> ArbStats {
        let avg_latency = if !self.detection_latencies.is_empty() {
            self.detection_latencies.iter().sum::<Duration>()
                / self.detection_latencies.len() as u32
        } else {
            Duration::ZERO
        };

        ArbStats {
            pools_tracked: self.pools.len(),
            two_hop_paths: self.two_hop_paths.len(),
            triangle_paths: self.triangle_paths.len(),
            opportunities_detected: self.opportunities_detected,
            avg_detection_latency_us: avg_latency.as_micros() as u64,
        }
    }

    /// Get pool metadata by pool ID
    pub fn get_pool_info(&self, pool_id: u64) -> Option<&PoolInfo> {
        self.pools.get(&pool_id)?.pool_info.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct ArbStats {
    pub pools_tracked: usize,
    pub two_hop_paths: usize,
    pub triangle_paths: usize,
    pub opportunities_detected: u64,
    pub avg_detection_latency_us: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_ma() {
        let mut ma = TickMA::new(3);

        assert_eq!(ma.update(10.0), 10.0); // [10]
        assert_eq!(ma.update(20.0), 15.0); // [10, 20]
        assert_eq!(ma.update(30.0), 20.0); // [10, 20, 30]
        assert_eq!(ma.update(40.0), 30.0); // [40, 20, 30] (wrapped)
    }

    #[test]
    fn test_pool_price_calculation() {
        let pool = PoolInfo {
            pool_id: 1,
            token_account_ids: vec!["token0".to_string(), "token1".to_string()],
            amounts: vec![1_000_000_000, 2_000_000_000], // 1:2 ratio
            total_fee: 25,
            shares_total_supply: 1_000_000,
        };

        assert_eq!(pool.price(), 2.0);
    }

    #[test]
    fn test_arb_opportunity_detection() {
        let mut engine = LightningArbEngine::new();

        // Register two pools with same token pair
        let pool1 = PoolInfo {
            pool_id: 1,
            token_account_ids: vec!["near".to_string(), "usdc".to_string()],
            amounts: vec![
                1_000_000_000_000_000_000_000_000,
                5_000_000_000_000_000_000_000_000,
            ], // 1 NEAR = 5 USDC
            total_fee: 25,
            shares_total_supply: 1_000_000,
        };

        let pool2 = PoolInfo {
            pool_id: 2,
            token_account_ids: vec!["near".to_string(), "usdc".to_string()],
            amounts: vec![
                1_000_000_000_000_000_000_000_000,
                5_100_000_000_000_000_000_000_000,
            ], // 1 NEAR = 5.1 USDC (2% spread!)
            total_fee: 25,
            shares_total_supply: 1_000_000,
        };

        engine.register_pool(&pool1);
        engine.register_pool(&pool2);

        // Update pool states to build MA baseline
        for _ in 0..10 {
            engine.on_pool_update(&pool1);
            engine.on_pool_update(&pool2);
        }

        // Now update with divergent price - should detect opportunity
        let pool2_diverged = PoolInfo {
            pool_id: 2,
            amounts: vec![
                1_000_000_000_000_000_000_000_000,
                5_500_000_000_000_000_000_000_000,
            ], // 1 NEAR = 5.5 USDC (10% spread!)
            ..pool2
        };

        let opp = engine.on_pool_update(&pool2_diverged);
        assert!(opp.is_some());

        if let Some(opportunity) = opp {
            assert!(opportunity.spread > 0.05); // > 5% spread
            assert!(opportunity.estimated_profit_pct > MIN_PROFIT_THRESHOLD);
        }
    }
}
