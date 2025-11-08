use crate::{
    config::Config,
    rpc_utils::{fetch_block_with_txs, get_latest_block},
    types::AppEvent,
};
use anyhow::Result;
use tokio::sync::mpsc::UnboundedSender;

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{sleep, Duration};

#[cfg(target_arch = "wasm32")]
use web_time::Duration;

#[cfg(target_arch = "wasm32")]
async fn sleep(duration: Duration) {
    // Use gloo-timers for reliable WASM sleep
    gloo_timers::future::sleep(std::time::Duration::from_millis(duration.as_millis() as u64)).await;
}

pub async fn run_rpc(cfg: &Config, tx: UnboundedSender<AppEvent>) -> Result<()> {
    let mut last_height: u64 = 0;
    log::info!(
        "🚀 RPC polling loop started - endpoint: {}",
        cfg.near_node_url
    );

    loop {
        log::debug!("📡 RPC loop tick - polling for latest block...");

        // non-overlapping loop, catch-up limited (guide's pattern).
        match get_latest_block(
            &cfg.near_node_url,
            cfg.rpc_timeout_ms,
            cfg.fastnear_auth_token.as_deref(),
        )
        .await
        {
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
                    log::info!(
                        "📦 Fetching blocks {} to {} ({} blocks)",
                        start,
                        end,
                        end - start + 1
                    );

                    for h in start..=end {
                        if let Ok(row) = fetch_block_with_txs(
                            &cfg.near_node_url,
                            h,
                            cfg.rpc_timeout_ms,
                            cfg.poll_chunk_concurrency,
                            cfg.fastnear_auth_token.as_deref(),
                        )
                        .await
                        {
                            log::info!(
                                "🔔 Sending NewBlock event - height: {}, txs: {}",
                                h,
                                row.tx_count
                            );
                            let _ = tx.send(AppEvent::NewBlock(row));
                            last_height = h;
                        } else {
                            log::warn!("⚠️ Failed to fetch block {h}");
                        }
                    }
                } else {
                    log::debug!("💤 No new blocks (latest: {latest_h}, last: {last_height})");
                }
            }
            Err(e) => {
                log::error!("❌ RPC error: {e:?}");
            }
        }

        log::debug!("😴 Sleeping for {}ms...", cfg.poll_interval_ms);
        sleep(Duration::from_millis(cfg.poll_interval_ms)).await;
        log::debug!("⏰ Woke up from sleep!");
    }
}
