use anyhow::Result;
use crate::{types::AppEvent, config::Config, rpc_utils::{get_latest_block, fetch_block_with_txs}};
use crate::platform::{Duration, Instant, sleep};
use tokio::sync::mpsc::UnboundedSender;

pub async fn run_rpc(cfg:&Config, tx: UnboundedSender<AppEvent>) -> Result<()> {
    let mut last_height: u64 = 0;
    log::info!("🚀 RPC polling loop started - endpoint: {}", cfg.near_node_url);

    // ═══════════════════════════════════════════════════════════════
    // HTTP CLIENT WARM-UP: Pre-warm DNS/TLS/HTTP connection pool
    // ═══════════════════════════════════════════════════════════════
    log::info!("🔥 Warming up HTTP client (DNS/TLS/connection pool)...");
    match get_latest_block(&cfg.near_node_url, cfg.rpc_timeout_ms, cfg.fastnear_auth_token.as_deref()).await {
        Ok(_) => log::info!("✅ HTTP client warmed up successfully"),
        Err(e) => log::warn!("⚠️  HTTP warm-up failed (continuing anyway): {}", e),
    }

    let mut last_poll_time = Instant::now();

    loop {
        log::debug!("📡 RPC loop tick - polling for latest block...");

        // non-overlapping loop, catch-up limited (guide's pattern).
        match get_latest_block(&cfg.near_node_url, cfg.rpc_timeout_ms, cfg.fastnear_auth_token.as_deref()).await {
            Ok(latest) => {
                let latest_h = latest["header"]["height"].as_u64().unwrap_or(0);
                log::debug!("✅ Got latest block height: {latest_h}");

                if last_height == 0 {
                    last_height = latest_h;
                    log::info!("🏁 Starting from block height: {last_height}");
                }

                if latest_h > last_height {
                    let start = last_height + 1;
                    let end = (start + cfg.poll_max_catchup - 1).min(latest_h);
                    log::info!("📦 Fetching blocks {} to {} ({} blocks)", start, end, end - start + 1);

                    for h in start..=end {
                        if let Ok(row) = fetch_block_with_txs(
                            &cfg.near_node_url,
                            h,
                            cfg.rpc_timeout_ms,
                            cfg.poll_chunk_concurrency,
                            cfg.fastnear_auth_token.as_deref()
                        ).await {
                            log::info!("🔔 Sending NewBlock event - height: {}, txs: {}", h, row.tx_count);
                            let _ = tx.send(AppEvent::NewBlock(row));
                            last_height = h;

                            // Yield to UI between blocks for responsiveness (200ms)
                            sleep(Duration::from_millis(200)).await;
                        } else {
                            log::warn!("⚠️ Failed to fetch block {h}");
                        }
                    }

                    // We just did catch-up work - skip sleep and immediately check for more
                    last_poll_time = Instant::now();
                    continue;
                } else {
                    log::debug!("💤 No new blocks (latest: {latest_h}, last: {last_height})");
                }
            }
            Err(e) => {
                log::error!("❌ RPC error: {e:?}");
            }
        }

        // Smart sleep: only wait remaining time in poll interval
        let elapsed = last_poll_time.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;
        let remaining_ms = cfg.poll_interval_ms.saturating_sub(elapsed_ms);

        if remaining_ms > 0 {
            log::debug!("😴 Sleeping for {}ms (elapsed: {}ms)...", remaining_ms, elapsed_ms);
            sleep(Duration::from_millis(remaining_ms)).await;
        } else {
            log::debug!("⏭️  Skipping sleep - already past interval (elapsed: {}ms)", elapsed_ms);
        }

        last_poll_time = Instant::now();
        log::debug!("⏰ Woke up from sleep!");
    }
}
