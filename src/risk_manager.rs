// Capital allocation and risk management for arbitrage execution
// Tracks capital usage, validates trades, enforces risk limits

use crate::arb_config::ArbConfig;
use crate::arb_engine::{ArbOpportunity, PoolInfo};
use crate::slippage;
use anyhow::Result;
use std::collections::HashMap;

/// Risk assessment result
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub approved: bool,
    pub max_trade_size: f64,
    pub estimated_slippage: f64,
    pub pool_exposure_pct: f64,
    pub capital_available: f64,
    pub rejection_reason: Option<String>,
}

/// Capital and risk management
pub struct RiskManager {
    config: ArbConfig,
    total_capital: f64,
    allocated_capital: f64,
    completed_trades: usize,
    total_pnl: f64,

    /// Track per-pool exposure to avoid over-concentration
    #[allow(dead_code)]
    pool_exposure: HashMap<u64, f64>,
}

impl RiskManager {
    pub fn new(config: ArbConfig) -> Self {
        let total_capital = config.capital();

        Self {
            config,
            total_capital,
            allocated_capital: 0.0,
            completed_trades: 0,
            total_pnl: 0.0,
            pool_exposure: HashMap::new(),
        }
    }

    /// Assess if a trade is allowed based on all risk constraints
    pub fn assess_trade(
        &self,
        opportunity: &ArbOpportunity,
        pools: &HashMap<u64, &PoolInfo>,
    ) -> RiskAssessment {
        // Start with Kelly criterion optimal size
        let kelly_size = opportunity.optimal_size();

        // Check capital availability
        let available_capital = self.total_capital - self.allocated_capital;
        if available_capital <= 0.0 {
            return RiskAssessment {
                approved: false,
                max_trade_size: 0.0,
                estimated_slippage: 0.0,
                pool_exposure_pct: 0.0,
                capital_available: available_capital,
                rejection_reason: Some("No capital available".to_string()),
            };
        }

        // Apply per-trade size limit
        let max_trade = self.config.max_trade_size().min(available_capital);
        let trade_size = kelly_size.min(max_trade);

        // Check minimum profit threshold
        let profit_pct = opportunity.estimated_profit_pct * 100.0;
        if profit_pct < self.config.min_profit() {
            return RiskAssessment {
                approved: false,
                max_trade_size: trade_size,
                estimated_slippage: 0.0,
                pool_exposure_pct: 0.0,
                capital_available: available_capital,
                rejection_reason: Some(format!(
                    "Profit {:.2}% below minimum {:.2}%",
                    profit_pct,
                    self.config.min_profit()
                )),
            };
        }

        // Estimate slippage for the trade
        let pool_ids = vec![opportunity.pool_a, opportunity.pool_b];
        let pool_refs: Vec<&PoolInfo> = pool_ids
            .iter()
            .filter_map(|id| pools.get(id).copied())
            .collect();

        if pool_refs.is_empty() {
            return RiskAssessment {
                approved: false,
                max_trade_size: trade_size,
                estimated_slippage: 0.0,
                pool_exposure_pct: 0.0,
                capital_available: available_capital,
                rejection_reason: Some("Pool data not available".to_string()),
            };
        }

        let (_, estimated_slippage) = slippage::calculate_multihop_slippage(&pool_refs, trade_size);

        // Check slippage limit
        if estimated_slippage > self.config.max_slippage() {
            return RiskAssessment {
                approved: false,
                max_trade_size: trade_size,
                estimated_slippage,
                pool_exposure_pct: 0.0,
                capital_available: available_capital,
                rejection_reason: Some(format!(
                    "Slippage {:.2}% exceeds limit {:.2}%",
                    estimated_slippage,
                    self.config.max_slippage()
                )),
            };
        }

        // Check pool exposure limit
        let pool_exposure_pct = self.calculate_pool_exposure(opportunity, pools, trade_size);
        if pool_exposure_pct > self.config.max_pool_exposure() {
            return RiskAssessment {
                approved: false,
                max_trade_size: trade_size,
                estimated_slippage,
                pool_exposure_pct,
                capital_available: available_capital,
                rejection_reason: Some(format!(
                    "Pool exposure {:.1}% exceeds limit {:.1}%",
                    pool_exposure_pct,
                    self.config.max_pool_exposure()
                )),
            };
        }

        // All checks passed
        RiskAssessment {
            approved: true,
            max_trade_size: trade_size,
            estimated_slippage,
            pool_exposure_pct,
            capital_available: available_capital,
            rejection_reason: None,
        }
    }

    /// Calculate what % of pool liquidity this trade represents
    fn calculate_pool_exposure(
        &self,
        opportunity: &ArbOpportunity,
        pools: &HashMap<u64, &PoolInfo>,
        trade_size: f64,
    ) -> f64 {
        // Find the pool with minimum liquidity (bottleneck)
        let pool_ids = vec![opportunity.pool_a, opportunity.pool_b];
        let min_liquidity = pool_ids
            .iter()
            .filter_map(|id| pools.get(id))
            .map(|pool| {
                if pool.amounts.len() != 2 {
                    f64::MAX
                } else {
                    // Simplified: use smaller of the two reserves
                    let r0 = pool.amounts[0] as f64 / 1e24;
                    let r1 = pool.amounts[1] as f64 / 1e24;
                    r0.min(r1)
                }
            })
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(f64::MAX);

        if min_liquidity == f64::MAX || min_liquidity == 0.0 {
            return 100.0; // No valid pool data
        }

        (trade_size / min_liquidity) * 100.0
    }

    /// Allocate capital for a trade (reserve it)
    pub fn allocate_capital(&mut self, amount: f64) -> Result<()> {
        if amount <= 0.0 {
            anyhow::bail!("Cannot allocate non-positive amount: {}", amount);
        }

        let available = self.total_capital - self.allocated_capital;
        if amount > available {
            anyhow::bail!(
                "Insufficient capital: requested ${:.2}, available ${:.2}",
                amount,
                available
            );
        }

        self.allocated_capital += amount;
        Ok(())
    }

    /// Release capital after trade completion
    pub fn release_capital(&mut self, amount: f64, pnl: f64) {
        self.allocated_capital = (self.allocated_capital - amount).max(0.0);
        self.total_pnl += pnl;
        self.completed_trades += 1;
    }

    /// Get current capital statistics
    pub fn stats(&self) -> CapitalStats {
        CapitalStats {
            total_capital: self.total_capital,
            allocated_capital: self.allocated_capital,
            available_capital: self.total_capital - self.allocated_capital,
            completed_trades: self.completed_trades,
            total_pnl: self.total_pnl,
            avg_pnl_per_trade: if self.completed_trades > 0 {
                self.total_pnl / self.completed_trades as f64
            } else {
                0.0
            },
        }
    }

    /// Print capital summary
    pub fn print_summary(&self) {
        let stats = self.stats();
        log::info!("💰 Capital Summary:");
        log::info!("  Total: ${:.2}", stats.total_capital);
        log::info!(
            "  Allocated: ${:.2} ({:.1}%)",
            stats.allocated_capital,
            (stats.allocated_capital / stats.total_capital) * 100.0
        );
        log::info!("  Available: ${:.2}", stats.available_capital);

        if self.completed_trades > 0 {
            log::info!("📊 Performance:");
            log::info!("  Trades: {}", stats.completed_trades);
            log::info!("  Total P&L: ${:.2}", stats.total_pnl);
            log::info!("  Avg P&L/Trade: ${:.2}", stats.avg_pnl_per_trade);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CapitalStats {
    pub total_capital: f64,
    pub allocated_capital: f64,
    pub available_capital: f64,
    pub completed_trades: usize,
    pub total_pnl: f64,
    pub avg_pnl_per_trade: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use crate::arb_engine::ArbType;
    #[allow(unused_imports)]
    use std::time::Instant;

    fn create_test_config() -> ArbConfig {
        ArbConfig {
            capital_usd: Some(10_000.0),
            max_trade_size_usd: Some(1_000.0),
            min_profit_pct: Some(0.5),
            max_slippage_pct: Some(1.0),
            max_pool_exposure_pct: Some(10.0),
            min_pool_liquidity_usd: 100_000.0,
            execution_mode: crate::arb_config::ExecutionMode::Display,
            near_account: None,
            gas_buffer_tgas: 50,
            near_node_url: "https://rpc.mainnet.fastnear.com/".to_string(),
            fastnear_auth_token: None,
            config_file: None,
            require_confirmation: false,
        }
    }

    #[test]
    fn test_capital_allocation() {
        let config = create_test_config();
        let mut manager = RiskManager::new(config);

        // Allocate half the capital
        assert!(manager.allocate_capital(5_000.0).is_ok());
        assert_eq!(manager.stats().allocated_capital, 5_000.0);
        assert_eq!(manager.stats().available_capital, 5_000.0);

        // Try to over-allocate
        assert!(manager.allocate_capital(6_000.0).is_err());

        // Release capital
        manager.release_capital(5_000.0, 150.0); // $150 profit
        assert_eq!(manager.stats().allocated_capital, 0.0);
        assert_eq!(manager.stats().total_pnl, 150.0);
    }
}
