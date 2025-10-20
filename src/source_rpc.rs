use anyhow::Result;
use tokio::time::{sleep, Duration};
use crate::{types::AppEvent, config::Config, rpc_utils::{get_latest_block, fetch_block_with_txs}};
use tokio::sync::mpsc::UnboundedSender;

pub async fn run_rpc(cfg:&Config, tx: UnboundedSender<AppEvent>) -> Result<()> {
    let mut last_height: u64 = 0;
    loop {
        // non-overlapping loop, catch-up limited (guide's pattern).
        match get_latest_block(&cfg.near_node_url, cfg.rpc_timeout_ms, cfg.fastnear_auth_token.as_deref()).await {
            Ok(latest) => {
                let latest_h = latest["header"]["height"].as_u64().unwrap_or(0);
                if last_height == 0 { last_height = latest_h; }
                if latest_h > last_height {
                    let start = last_height + 1;
                    let end = (start + cfg.poll_max_catchup - 1).min(latest_h);
                    for h in start..=end {
                        if let Ok(row) = fetch_block_with_txs(
                            &cfg.near_node_url,
                            h,
                            cfg.rpc_timeout_ms,
                            cfg.poll_chunk_concurrency,
                            cfg.fastnear_auth_token.as_deref()
                        ).await {
                            let _ = tx.send(AppEvent::NewBlock(row));
                            last_height = h;
                        }
                    }
                }
            }
            Err(_) => { /* circuit opens implicitly by retry wait */ }
        }
        sleep(Duration::from_millis(cfg.poll_interval_ms)).await;
    }
}
