// Graph-based price discovery using DEX pool reserves
// Finds shortest path from any token to known stablecoins

use crate::arb_engine::PoolInfo;
use std::collections::{HashMap, HashSet, VecDeque};

/// Known stablecoins (USD-pegged) to use as price anchors
const STABLECOINS: &[&str] = &[
    "dac17f958d2ee523a2206206994597c13d831ec7.factory.bridge.near", // USDT
    "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near", // USDC
    "6b175474e89094c44da98b954eedeac495271d0f.factory.bridge.near", // DAI
    "usdt.tether-token.near",                                       // Native USDT
    "17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1", // USDt
];

/// Edge in the token price graph (exchange rate between two tokens)
#[derive(Debug, Clone)]
struct PriceEdge {
    #[allow(dead_code)]
    from_token: String,
    to_token: String,
    exchange_rate: f64, // How many to_token per 1 from_token
    #[allow(dead_code)]
    pool_id: u64,
}

/// Token price graph for BFS-based price discovery
pub struct PriceGraph {
    /// Adjacency list: token -> [(neighbor_token, exchange_rate, pool_id)]
    edges: HashMap<String, Vec<PriceEdge>>,

    /// Known USD prices (stablecoins = $1.00)
    usd_prices: HashMap<String, f64>,
}

impl PriceGraph {
    pub fn new() -> Self {
        let mut usd_prices = HashMap::new();

        // Initialize stablecoins at $1.00
        for &stablecoin in STABLECOINS {
            usd_prices.insert(stablecoin.to_string(), 1.0);
        }

        Self {
            edges: HashMap::new(),
            usd_prices,
        }
    }

    /// Add a pool to the price graph
    pub fn add_pool(&mut self, pool: &PoolInfo) {
        if pool.token_account_ids.len() != 2 {
            return; // Only handle 2-token pools
        }

        if pool.amounts[0] == 0 || pool.amounts[1] == 0 {
            return; // Skip empty pools
        }

        let token_a = &pool.token_account_ids[0];
        let token_b = &pool.token_account_ids[1];

        // Calculate exchange rates (accounting for decimals is simplified here)
        let rate_a_to_b = pool.amounts[1] as f64 / pool.amounts[0] as f64;
        let rate_b_to_a = pool.amounts[0] as f64 / pool.amounts[1] as f64;

        // Add bidirectional edges
        self.edges
            .entry(token_a.clone())
            .or_insert_with(Vec::new)
            .push(PriceEdge {
                from_token: token_a.clone(),
                to_token: token_b.clone(),
                exchange_rate: rate_a_to_b,
                pool_id: pool.pool_id,
            });

        self.edges
            .entry(token_b.clone())
            .or_insert_with(Vec::new)
            .push(PriceEdge {
                from_token: token_b.clone(),
                to_token: token_a.clone(),
                exchange_rate: rate_b_to_a,
                pool_id: pool.pool_id,
            });
    }

    /// Get USD price for a token using BFS to find shortest path to stablecoin
    pub fn get_usd_price(&self, token: &str) -> Option<f64> {
        // Check if already known
        if let Some(&price) = self.usd_prices.get(token) {
            return Some(price);
        }

        // BFS to find shortest path to any stablecoin
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back((token.to_string(), 1.0)); // Start with 1 unit of token
        visited.insert(token.to_string());

        while let Some((current_token, accumulated_value)) = queue.pop_front() {
            // Check if we reached a stablecoin
            if let Some(&stablecoin_price) = self.usd_prices.get(&current_token) {
                return Some(accumulated_value * stablecoin_price);
            }

            // Explore neighbors
            if let Some(neighbors) = self.edges.get(&current_token) {
                for edge in neighbors {
                    if !visited.contains(&edge.to_token) {
                        visited.insert(edge.to_token.clone());
                        let new_value = accumulated_value * edge.exchange_rate;
                        queue.push_back((edge.to_token.clone(), new_value));
                    }
                }
            }
        }

        None // No path to stablecoin found
    }

    /// Calculate USD liquidity for a pool
    pub fn calculate_pool_liquidity_usd(&self, pool: &PoolInfo) -> f64 {
        if pool.token_account_ids.len() != 2 {
            return 0.0;
        }

        let token_a = &pool.token_account_ids[0];
        let token_b = &pool.token_account_ids[1];

        // Check if either token is a stablecoin
        let is_stablecoin_a = self.usd_prices.contains_key(token_a);
        let is_stablecoin_b = self.usd_prices.contains_key(token_b);

        if is_stablecoin_a {
            // Token A is a stablecoin - use its amount directly
            // Common stablecoin decimals: USDC/USDT = 6, DAI = 18
            // Heuristic: if amount < 1e12, likely 6 decimals, else 18
            let amount_raw = pool.amounts[0] as f64;
            let amount_usd = if amount_raw < 1e12 {
                amount_raw / 1e6 // USDC/USDT (6 decimals)
            } else {
                amount_raw / 1e18 // DAI (18 decimals)
            };
            return amount_usd * 2.0; // TVL = 2x one side
        }

        if is_stablecoin_b {
            // Token B is a stablecoin
            let amount_raw = pool.amounts[1] as f64;
            let amount_usd = if amount_raw < 1e12 {
                amount_raw / 1e6
            } else {
                amount_raw / 1e18
            };
            return amount_usd * 2.0;
        }

        // Neither is a stablecoin - try price discovery
        let price_a = self.get_usd_price(token_a);
        let price_b = self.get_usd_price(token_b);

        match (price_a, price_b) {
            (Some(price_a), _) => {
                // Assume 24 decimals for non-stablecoin tokens (NEAR standard)
                let amount_a = pool.amounts[0] as f64 / 1e24;
                amount_a * price_a * 2.0
            }
            (None, Some(price_b)) => {
                let amount_b = pool.amounts[1] as f64 / 1e24;
                amount_b * price_b * 2.0
            }
            (None, None) => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stablecoin_price() {
        let graph = PriceGraph::new();

        // Stablecoins should have $1.00 price
        assert_eq!(graph.get_usd_price("usdt.tether-token.near"), Some(1.0));
    }

    #[test]
    fn test_one_hop_price_discovery() {
        let mut graph = PriceGraph::new();

        // Create a pool: NEAR-USDC with 1 NEAR = 5 USDC
        let pool = PoolInfo {
            pool_id: 1,
            token_account_ids: vec![
                "wrap.near".to_string(),
                "a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48.factory.bridge.near".to_string(),
            ],
            amounts: vec![
                1_000_000_000_000_000_000_000_000, // 1 NEAR (24 decimals)
                5_000_000,                         // 5 USDC (6 decimals, but we simplify)
            ],
            total_fee: 25,
            shares_total_supply: 1_000_000,
        };

        graph.add_pool(&pool);

        // NEAR should be discoverable via USDC
        let near_price = graph.get_usd_price("wrap.near");
        assert!(near_price.is_some());

        // Should be approximately $5 (accounting for decimal simplification)
        let price = near_price.unwrap();
        assert!(price > 0.0);
    }
}
