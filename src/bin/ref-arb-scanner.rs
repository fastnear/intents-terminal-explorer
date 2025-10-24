// Standalone arbitrage scanner for Ref Finance pools
// Capital-managed execution with Display/Simulate/Execute modes

use anyhow::Result;
use ratacat::arb_config::ArbConfig;
use ratacat::arb_engine::LightningArbEngine;
use ratacat::execution_engine::ExecutionEngine;
use ratacat::price_discovery::PriceGraph;
use ratacat::ref_finance_client::RefFinanceClient;
use ratacat::risk_manager::RiskManager;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("🚀 Starting Capital-Managed Arbitrage Scanner");

    // Load configuration (CLI > Env > File > Defaults)
    let config = ArbConfig::load()?;
    config.print_summary();

    // Initialize risk manager and execution engine
    let risk_manager = Arc::new(Mutex::new(RiskManager::new(config.clone())));
    let execution_engine = ExecutionEngine::new(config.clone());

    log::info!("📡 Connecting to RPC: {}", config.near_node_url);

    // Create client
    let client = RefFinanceClient::new(config.near_node_url.clone())
        .with_poll_interval(1000); // Poll every 1 second

    // Discover ALL pools (not just NEAR pairs)
    log::info!("🔍 Discovering all pools...");
    let num_pools = client.get_number_of_pools().await?;
    log::info!("📊 Found {} total pools on Ref Finance", num_pools);

    // Build price graph first
    log::info!("💰 Building USD price graph from all pools...");
    let mut price_graph = PriceGraph::new();
    let mut all_pools = Vec::new();

    // Fetch all pools in batches
    for batch_start in (0..num_pools).step_by(100) {
        let batch_size = 100.min(num_pools - batch_start);
        let pools = client.get_pools(batch_start, batch_size).await?;

        for pool in pools {
            price_graph.add_pool(&pool);
            all_pools.push(pool);
        }
    }
    log::info!("✅ Price graph built with {} pools", all_pools.len());

    // Filter for high-liquidity pools (using config threshold)
    let min_liquidity = config.min_pool_liquidity_usd;
    log::info!("💎 Filtering for pools with >${}k liquidity...", min_liquidity / 1000.0);
    let mut high_liq_pools = Vec::new();

    for pool in &all_pools {
        let liquidity_usd = price_graph.calculate_pool_liquidity_usd(&pool);
        if liquidity_usd > min_liquidity {
            high_liq_pools.push(pool.pool_id);
            if liquidity_usd > 1_000_000.0 {
                log::info!(
                    "  Pool {}: {} / {} (${:.2}M liquidity)",
                    pool.pool_id,
                    pool.token_account_ids.get(0).unwrap_or(&"?".to_string()),
                    pool.token_account_ids.get(1).unwrap_or(&"?".to_string()),
                    liquidity_usd / 1_000_000.0
                );
            } else {
                log::info!(
                    "  Pool {}: {} / {} (${:.0}k liquidity)",
                    pool.pool_id,
                    pool.token_account_ids.get(0).unwrap_or(&"?".to_string()),
                    pool.token_account_ids.get(1).unwrap_or(&"?".to_string()),
                    liquidity_usd / 1000.0
                );
            }
        }
    }

    log::info!("✅ Found {} pools with >${}k liquidity", high_liq_pools.len(), min_liquidity / 1000.0);

    if high_liq_pools.is_empty() {
        log::warn!("⚠️  No pools with >${}k liquidity found!", min_liquidity / 1000.0);
        return Ok(());
    }

    // Create arbitrage engine
    let engine = Arc::new(Mutex::new(LightningArbEngine::new()));

    // Register high-liquidity pools in engine
    log::info!("📊 Registering high-liquidity pools in arbitrage engine...");
    for pool in &all_pools {
        if high_liq_pools.contains(&pool.pool_id) {
            engine.lock().unwrap().register_pool(&pool);
        }
    }

    // Log sample pool data for debugging
    log::debug!("Sample pool liquidities:");
    for pool in all_pools.iter().filter(|p| high_liq_pools.contains(&p.pool_id)).take(5) {
        let liq = price_graph.calculate_pool_liquidity_usd(&pool);
        log::debug!("  Pool {}: ${:.2} | {} / {}",
            pool.pool_id, liq,
            pool.token_account_ids.get(0).unwrap_or(&"?".to_string()),
            pool.token_account_ids.get(1).unwrap_or(&"?".to_string())
        );
    }

    // Print stats
    {
        let stats = engine.lock().unwrap().stats();
        log::info!("🎯 Monitoring {} pools | {} 2-hop paths | {} triangle paths",
            stats.pools_tracked,
            stats.two_hop_paths,
            stats.triangle_paths
        );
    }

    let stats = engine.lock().unwrap().stats();
    if stats.two_hop_paths == 0 && stats.triangle_paths == 0 {
        log::warn!("⚠️  No arbitrage paths found! Need pools with matching token pairs.");
        log::info!("💡 Try monitoring more pools or different token pairs");
        return Ok(());
    }

    // Build pool lookup map for risk assessment
    let pool_map: HashMap<u64, _> = all_pools.iter()
        .filter(|p| high_liq_pools.contains(&p.pool_id))
        .map(|p| (p.pool_id, p))
        .collect();
    let pool_map = Arc::new(pool_map);

    // Start polling
    log::info!("⚡ Starting real-time monitoring... (Ctrl+C to stop)");
    log::info!("📈 Mode: {:?}", config.execution_mode);
    log::info!("💰 Capital: ${:.2} | Max Trade: ${:.2}\n",
        config.capital(),
        config.max_trade_size()
    );

    let engine_clone = Arc::clone(&engine);
    let risk_manager_clone = Arc::clone(&risk_manager);
    let pool_map_clone = Arc::clone(&pool_map);
    let _start_time = Instant::now();
    let mut last_stats_print = Instant::now();
    let mut opportunities_found = 0;

    client.poll_pools(high_liq_pools.clone(), move |pool_info| {
        // Update engine and check for opportunities
        let mut engine = engine_clone.lock().unwrap();

        if let Some(opp) = engine.on_pool_update(&pool_info) {
            opportunities_found += 1;

            log::debug!(
                "Opportunity #{}: optimal_size=${:.2}, liquidity_a=${:.2}, liquidity_b=${:.2}, confidence={:.1}%",
                opportunities_found,
                opp.optimal_size(),
                opp.liquidity_a,
                opp.liquidity_b,
                opp.confidence * 100.0
            );

            // Perform risk assessment
            let pool_map_ref: HashMap<u64, &_> = pool_map_clone.iter()
                .map(|(id, pool)| (*id, *pool))
                .collect();

            let risk_mgr = risk_manager_clone.lock().unwrap();
            let risk_assessment = risk_mgr.assess_trade(&opp, &pool_map_ref);

            // Process opportunity through execution engine
            let execution_result = if risk_assessment.approved {
                execution_engine.process_opportunity(&opp, &risk_assessment).ok()
            } else {
                None
            };

            // Print formatted opportunity
            execution_engine.print_opportunity(
                opportunities_found,
                &opp,
                &risk_assessment,
                execution_result.as_ref(),
                &risk_mgr,
            );
        }

        // Print stats every 30 seconds
        if last_stats_print.elapsed().as_secs() >= 30 {
            let stats = engine.stats();
            println!("📊 Stats: {} pools | {}+{} paths | {} opportunities | avg latency: {}μs",
                stats.pools_tracked,
                stats.two_hop_paths,
                stats.triangle_paths,
                stats.opportunities_detected,
                stats.avg_detection_latency_us
            );
            last_stats_print = Instant::now();
        }
    }).await?;

    Ok(())
}
