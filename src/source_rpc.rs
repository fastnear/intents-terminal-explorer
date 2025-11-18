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

    // Get effective auth token with priority: User token (from auth module) → Config token → None
    let get_token = || -> Option<String> {
        // Try user token first (from authenticated login via auth module)
        if let Some(token) = crate::auth::token_string() {
            log::debug!("🔑 Using user FastNEAR token (from auth)");
            return Some(token);
        }
        // Fall back to config token (from env or URL param)
        if let Some(ref token) = cfg.fastnear_auth_token {
            log::debug!("🔑 Using config FastNEAR token (env/URL)");
            Some(token.clone())
        } else {
            log::debug!("⚠️ No FastNEAR token available (may hit rate limits)");
            None
        }
    };

    loop {
        log::debug!("📡 RPC loop tick - polling for latest block...");

        let token = get_token();

        // non-overlapping loop, catch-up limited (guide's pattern).
        match get_latest_block(&cfg.near_node_url, cfg.rpc_timeout_ms, token.as_deref()).await {
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
                        let token = get_token(); // Refresh token for each block fetch
                        if let Ok(row) = fetch_block_with_txs(
                            &cfg.near_node_url,
                            h,
                            cfg.rpc_timeout_ms,
                            cfg.poll_chunk_concurrency,
                            token.as_deref(),
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

                            // Yield briefly to allow UI to process events and stay responsive
                            #[cfg(not(target_arch = "wasm32"))]
                            tokio::task::yield_now().await;
                            #[cfg(target_arch = "wasm32")]
                            sleep(Duration::from_millis(1)).await;
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
