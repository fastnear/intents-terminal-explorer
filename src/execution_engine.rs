// Execution engine for arbitrage opportunities
// Supports Display, Simulate, and Execute modes

use crate::arb_config::{ArbConfig, ExecutionMode};
use crate::arb_engine::{ArbOpportunity, ArbType};
use crate::risk_manager::{RiskAssessment, RiskManager};
use anyhow::Result;

/// Execution engine handles displaying, simulating, and executing trades
pub struct ExecutionEngine {
    config: ArbConfig,
}

/// Result of executing/simulating a trade
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub mode: ExecutionMode,
    pub success: bool,
    pub message: String,
    pub estimated_pnl: f64,
    pub gas_cost_usd: f64,
    pub net_pnl: f64,
}

impl ExecutionEngine {
    pub fn new(config: ArbConfig) -> Self {
        Self { config }
    }

    /// Process an arbitrage opportunity according to execution mode
    pub fn process_opportunity(
        &self,
        opportunity: &ArbOpportunity,
        risk_assessment: &RiskAssessment,
    ) -> Result<ExecutionResult> {
        match self.config.execution_mode {
            ExecutionMode::Display => self.display_opportunity(opportunity, risk_assessment),
            ExecutionMode::Simulate => self.simulate_trade(opportunity, risk_assessment),
            ExecutionMode::Execute => self.execute_trade(opportunity, risk_assessment),
        }
    }

    /// Display mode: just show the opportunity
    fn display_opportunity(
        &self,
        _opportunity: &ArbOpportunity,
        _risk_assessment: &RiskAssessment,
    ) -> Result<ExecutionResult> {
        Ok(ExecutionResult {
            mode: ExecutionMode::Display,
            success: true,
            message: "Opportunity displayed".to_string(),
            estimated_pnl: 0.0,
            gas_cost_usd: 0.0,
            net_pnl: 0.0,
        })
    }

    /// Simulate mode: build transaction and estimate outcome
    fn simulate_trade(
        &self,
        opportunity: &ArbOpportunity,
        risk_assessment: &RiskAssessment,
    ) -> Result<ExecutionResult> {
        let trade_size = risk_assessment.max_trade_size;

        // Calculate expected profit
        let gross_profit = opportunity.calculate_profit(trade_size);

        // Estimate gas costs (rough approximation)
        let num_swaps = match opportunity.arb_type {
            ArbType::TwoHop => 2,
            ArbType::Triangle => 3,
        };

        // Each swap costs ~5 TGas, NEAR price ~$5, 1 TGas = 1e12 Gas
        // Cost per swap ≈ 5 TGas × $5/NEAR × 1e-12 = $0.025
        let gas_cost_per_swap = 0.025;
        let total_gas_cost = gas_cost_per_swap * num_swaps as f64;

        let net_profit = gross_profit - total_gas_cost;

        Ok(ExecutionResult {
            mode: ExecutionMode::Simulate,
            success: net_profit > 0.0,
            message: format!(
                "Simulated {} swaps with ${:.2} trade size",
                num_swaps, trade_size
            ),
            estimated_pnl: gross_profit,
            gas_cost_usd: total_gas_cost,
            net_pnl: net_profit,
        })
    }

    /// Execute mode: submit transaction to blockchain
    /// NOTE: This is a placeholder - full implementation requires NEAR account signing
    fn execute_trade(
        &self,
        opportunity: &ArbOpportunity,
        risk_assessment: &RiskAssessment,
    ) -> Result<ExecutionResult> {
        // Check if NEAR account is configured
        if self.config.near_account.is_none() {
            anyhow::bail!("NEAR_ACCOUNT required for execute mode");
        }

        // TODO: Implement actual NEAR transaction submission
        // This would require:
        // 1. Load NEAR account credentials
        // 2. Build swap transaction(s) for Ref Finance
        // 3. Sign transaction with account key
        // 4. Submit to NEAR RPC
        // 5. Wait for confirmation
        // 6. Return actual result

        log::warn!("⚠️  Live execution not yet implemented - would execute:");
        log::warn!("   Trade size: ${:.2}", risk_assessment.max_trade_size);
        log::warn!(
            "   Expected profit: ${:.2}",
            opportunity.calculate_profit(risk_assessment.max_trade_size)
        );

        Ok(ExecutionResult {
            mode: ExecutionMode::Execute,
            success: false,
            message: "Live execution not yet implemented".to_string(),
            estimated_pnl: 0.0,
            gas_cost_usd: 0.0,
            net_pnl: 0.0,
        })
    }

    /// Print formatted opportunity with risk assessment
    pub fn print_opportunity(
        &self,
        opportunity_num: usize,
        opportunity: &ArbOpportunity,
        risk_assessment: &RiskAssessment,
        execution_result: Option<&ExecutionResult>,
        risk_manager: &RiskManager,
    ) {
        let status = if risk_assessment.approved {
            "[EXECUTABLE]"
        } else {
            "[REJECTED]"
        };

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🎯 ARBITRAGE OPPORTUNITY #{} {}", opportunity_num, status);
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        match opportunity.arb_type {
            ArbType::TwoHop => {
                println!("📊 Type: 2-Hop Arbitrage");
                println!("🔀 Pools: {} ↔ {}", opportunity.pool_a, opportunity.pool_b);
            }
            ArbType::Triangle => {
                println!("📊 Type: Triangle Arbitrage");
                println!(
                    "🔺 Pools: {} → {} → {}",
                    opportunity.pool_a,
                    opportunity.pool_b,
                    opportunity.pool_c.unwrap_or(0)
                );
            }
        }

        println!("\n💰 Financial Analysis:");
        println!(
            "  • Optimal Trade Size: ${:.2} (Kelly criterion)",
            opportunity.optimal_size()
        );
        println!(
            "  • Capital Constraint: ${:.2} ✓ (within ${:.0} limit)",
            risk_assessment.max_trade_size,
            self.config.max_trade_size()
        );

        if let Some(result) = execution_result {
            println!("  • Expected Profit: ${:.2}", result.estimated_pnl);
            println!("  • Gas Cost: ${:.2}", result.gas_cost_usd);
            println!(
                "  • Net Profit: ${:.2} ({:.2}%)",
                result.net_pnl,
                (result.net_pnl / risk_assessment.max_trade_size) * 100.0
            );
        }

        println!("\n⚠️  Risk Assessment:");
        println!("  • Confidence: {:.1}%", opportunity.confidence * 100.0);
        println!(
            "  • Slippage Estimate: {:.2}%{}",
            risk_assessment.estimated_slippage,
            if risk_assessment.estimated_slippage <= self.config.max_slippage() {
                " ✓"
            } else {
                " ✗"
            }
        );
        println!(
            "  • Pool Exposure: {:.1}%{}",
            risk_assessment.pool_exposure_pct,
            if risk_assessment.pool_exposure_pct <= self.config.max_pool_exposure() {
                " ✓"
            } else {
                " ✗"
            }
        );

        if !risk_assessment.approved {
            println!(
                "\n❌ Rejection Reason: {}",
                risk_assessment.rejection_reason.as_ref().unwrap()
            );
        }

        let stats = risk_manager.stats();
        println!("\n📊 Capital Tracker:");
        println!("  • Total Capital: ${:.2}", stats.total_capital);
        println!(
            "  • Allocated: ${:.2} ({:.1}%)",
            stats.allocated_capital,
            (stats.allocated_capital / stats.total_capital) * 100.0
        );
        println!("  • Available: ${:.2}", stats.available_capital);

        if stats.completed_trades > 0 {
            println!("  • Completed Trades: {}", stats.completed_trades);
            println!("  • Total P&L: ${:.2}", stats.total_pnl);
        }

        println!();
    }
}
