// Capital management and risk configuration for arbitrage execution
// Priority: CLI args > Environment variables > Config file > Defaults

use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Execution mode for arbitrage opportunities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Display opportunities only, no execution
    Display,
    /// Simulate trades (build transactions, estimate outcomes)
    Simulate,
    /// Execute trades live on-chain
    Execute,
}

impl std::str::FromStr for ExecutionMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "display" => Ok(ExecutionMode::Display),
            "simulate" => Ok(ExecutionMode::Simulate),
            "execute" => Ok(ExecutionMode::Execute),
            _ => anyhow::bail!(
                "Invalid execution mode: {}. Must be 'display', 'simulate', or 'execute'",
                s
            ),
        }
    }
}

/// Arbitrage configuration with capital management and risk controls
#[derive(Debug, Clone, Parser)]
#[command(name = "ref-arb-scanner")]
#[command(about = "Ultra-fast arbitrage scanner with capital management")]
pub struct ArbConfig {
    /// Total capital available for arbitrage (USD)
    #[arg(long, env = "CAPITAL_USD")]
    pub capital_usd: Option<f64>,

    /// Maximum trade size per opportunity (USD)
    #[arg(long, env = "MAX_TRADE_SIZE_USD")]
    pub max_trade_size_usd: Option<f64>,

    /// Minimum profit percentage to execute (e.g., 0.3 for 0.3%)
    #[arg(long, env = "MIN_PROFIT_PCT")]
    pub min_profit_pct: Option<f64>,

    /// Maximum slippage tolerance percentage (e.g., 1.0 for 1%)
    #[arg(long, env = "MAX_SLIPPAGE_PCT")]
    pub max_slippage_pct: Option<f64>,

    /// Maximum pool exposure percentage (e.g., 10 for 10% of pool liquidity)
    #[arg(long, env = "MAX_POOL_EXPOSURE_PCT")]
    pub max_pool_exposure_pct: Option<f64>,

    /// Minimum pool liquidity to consider (USD)
    #[arg(long, env = "MIN_POOL_LIQUIDITY_USD", default_value = "100000")]
    pub min_pool_liquidity_usd: f64,

    /// Execution mode: display, simulate, or execute
    #[arg(long, env = "EXECUTION_MODE", default_value = "display")]
    pub execution_mode: ExecutionMode,

    /// NEAR account for signing transactions (required for execute mode)
    #[arg(long, env = "NEAR_ACCOUNT")]
    pub near_account: Option<String>,

    /// Gas buffer in TGas for transaction safety margin
    #[arg(long, env = "GAS_BUFFER_TGAS", default_value = "50")]
    pub gas_buffer_tgas: u64,

    /// NEAR RPC URL
    #[arg(
        long,
        env = "NEAR_NODE_URL",
        default_value = "https://rpc.mainnet.fastnear.com/"
    )]
    pub near_node_url: String,

    /// FastNEAR API token for rate limiting
    #[arg(long, env = "FASTNEAR_AUTH_TOKEN")]
    pub fastnear_auth_token: Option<String>,

    /// Optional config file path (TOML format)
    #[arg(long, env = "ARB_CONFIG_FILE")]
    pub config_file: Option<PathBuf>,

    /// Require manual confirmation before each trade
    #[arg(long, env = "REQUIRE_CONFIRMATION")]
    pub require_confirmation: bool,
}

/// Configuration loaded from TOML file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub capital: CapitalConfig,

    #[serde(default)]
    pub risk: RiskConfig,

    #[serde(default)]
    pub execution: ExecutionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapitalConfig {
    pub total_usd: Option<f64>,
    pub max_trade_size_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RiskConfig {
    pub min_profit_pct: Option<f64>,
    pub max_slippage_pct: Option<f64>,
    pub max_pool_exposure_pct: Option<f64>,
    pub min_pool_liquidity_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionConfig {
    pub mode: Option<String>,
    pub near_account: Option<String>,
    pub gas_buffer_tgas: Option<u64>,
    pub require_confirmation: Option<bool>,
}

impl ArbConfig {
    /// Load configuration with full priority chain: CLI > Env > File > Defaults
    pub fn load() -> Result<Self> {
        let mut config = Self::parse();

        // Load from config file if specified
        if let Some(ref config_file) = config.config_file {
            log::info!("📄 Loading configuration from {}", config_file.display());
            let file_config = Self::load_from_file(config_file)?;
            config.merge_with_file(file_config);
        }

        // Apply defaults for any missing values
        config.apply_defaults();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Load configuration from TOML file
    fn load_from_file(path: &PathBuf) -> Result<ConfigFile> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        toml::from_str(&contents)
            .with_context(|| format!("Failed to parse TOML config: {}", path.display()))
    }

    /// Merge with file configuration (only if CLI/env not set)
    fn merge_with_file(&mut self, file: ConfigFile) {
        if self.capital_usd.is_none() {
            self.capital_usd = file.capital.total_usd;
        }
        if self.max_trade_size_usd.is_none() {
            self.max_trade_size_usd = file.capital.max_trade_size_usd;
        }
        if self.min_profit_pct.is_none() {
            self.min_profit_pct = file.risk.min_profit_pct;
        }
        if self.max_slippage_pct.is_none() {
            self.max_slippage_pct = file.risk.max_slippage_pct;
        }
        if self.max_pool_exposure_pct.is_none() {
            self.max_pool_exposure_pct = file.risk.max_pool_exposure_pct;
        }
        if self.near_account.is_none() {
            self.near_account = file.execution.near_account;
        }
        if let Some(mode_str) = file.execution.mode {
            if let Ok(mode) = mode_str.parse::<ExecutionMode>() {
                self.execution_mode = mode;
            }
        }
    }

    /// Apply default values for missing configuration
    fn apply_defaults(&mut self) {
        if self.capital_usd.is_none() {
            self.capital_usd = Some(10_000.0); // $10k default
        }
        if self.max_trade_size_usd.is_none() {
            self.max_trade_size_usd = Some(1_000.0); // $1k default
        }
        if self.min_profit_pct.is_none() {
            self.min_profit_pct = Some(0.3); // 0.3% minimum profit
        }
        if self.max_slippage_pct.is_none() {
            self.max_slippage_pct = Some(1.0); // 1% max slippage
        }
        if self.max_pool_exposure_pct.is_none() {
            self.max_pool_exposure_pct = Some(10.0); // 10% max of pool liquidity
        }
    }

    /// Validate configuration values
    fn validate(&self) -> Result<()> {
        // Capital validation
        let capital = self.capital_usd.unwrap();
        if capital <= 0.0 {
            anyhow::bail!("CAPITAL_USD must be > 0, got {}", capital);
        }

        let max_trade = self.max_trade_size_usd.unwrap();
        if max_trade <= 0.0 {
            anyhow::bail!("MAX_TRADE_SIZE_USD must be > 0, got {}", max_trade);
        }
        if max_trade > capital {
            anyhow::bail!(
                "MAX_TRADE_SIZE_USD (${}) cannot exceed CAPITAL_USD (${})",
                max_trade,
                capital
            );
        }

        // Risk validation
        let min_profit = self.min_profit_pct.unwrap();
        if min_profit < 0.0 || min_profit > 100.0 {
            anyhow::bail!("MIN_PROFIT_PCT must be in [0, 100], got {}", min_profit);
        }

        let max_slippage = self.max_slippage_pct.unwrap();
        if max_slippage < 0.0 || max_slippage > 50.0 {
            anyhow::bail!("MAX_SLIPPAGE_PCT must be in [0, 50], got {}", max_slippage);
        }

        let max_exposure = self.max_pool_exposure_pct.unwrap();
        if max_exposure < 0.0 || max_exposure > 100.0 {
            anyhow::bail!(
                "MAX_POOL_EXPOSURE_PCT must be in [0, 100], got {}",
                max_exposure
            );
        }

        if self.min_pool_liquidity_usd <= 0.0 {
            anyhow::bail!(
                "MIN_POOL_LIQUIDITY_USD must be > 0, got {}",
                self.min_pool_liquidity_usd
            );
        }

        // Execution validation
        if self.execution_mode == ExecutionMode::Execute && self.near_account.is_none() {
            anyhow::bail!("NEAR_ACCOUNT is required when EXECUTION_MODE=execute");
        }

        Ok(())
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        log::info!("⚙️  Configuration:");
        log::info!("  💰 Capital: ${:.2}", self.capital_usd.unwrap());
        log::info!(
            "  📊 Max Trade Size: ${:.2}",
            self.max_trade_size_usd.unwrap()
        );
        log::info!("  📈 Min Profit: {:.2}%", self.min_profit_pct.unwrap());
        log::info!("  ⚠️  Max Slippage: {:.2}%", self.max_slippage_pct.unwrap());
        log::info!(
            "  🏊 Max Pool Exposure: {:.1}%",
            self.max_pool_exposure_pct.unwrap()
        );
        log::info!(
            "  💧 Min Pool Liquidity: ${:.0}",
            self.min_pool_liquidity_usd
        );
        log::info!("  🎬 Execution Mode: {:?}", self.execution_mode);

        if let Some(ref account) = self.near_account {
            log::info!("  🔑 NEAR Account: {}", account);
        }

        if self.require_confirmation {
            log::info!("  ✋ Manual Confirmation: ENABLED");
        }
    }

    /// Get capital as unwrapped value (guaranteed to exist after validation)
    pub fn capital(&self) -> f64 {
        self.capital_usd.unwrap()
    }

    /// Get max trade size as unwrapped value
    pub fn max_trade_size(&self) -> f64 {
        self.max_trade_size_usd.unwrap()
    }

    /// Get min profit as unwrapped value
    pub fn min_profit(&self) -> f64 {
        self.min_profit_pct.unwrap()
    }

    /// Get max slippage as unwrapped value
    pub fn max_slippage(&self) -> f64 {
        self.max_slippage_pct.unwrap()
    }

    /// Get max pool exposure as unwrapped value
    pub fn max_pool_exposure(&self) -> f64 {
        self.max_pool_exposure_pct.unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_parsing() {
        assert_eq!(
            "display".parse::<ExecutionMode>().unwrap(),
            ExecutionMode::Display
        );
        assert_eq!(
            "simulate".parse::<ExecutionMode>().unwrap(),
            ExecutionMode::Simulate
        );
        assert_eq!(
            "execute".parse::<ExecutionMode>().unwrap(),
            ExecutionMode::Execute
        );
        assert!("invalid".parse::<ExecutionMode>().is_err());
    }

    #[test]
    fn test_default_config() {
        std::env::set_var("CAPITAL_USD", "50000");
        std::env::set_var("MAX_TRADE_SIZE_USD", "5000");

        let config = ArbConfig::load().unwrap();
        assert_eq!(config.capital(), 50000.0);
        assert_eq!(config.max_trade_size(), 5000.0);
    }
}
