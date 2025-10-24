use anyhow::Result;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::{types::AppEvent, config::Config, rpc_utils::fetch_block_with_txs};

/// Background task that fetches historical blocks from archival RPC endpoint
/// Receives block height requests and fetches them on demand
pub async fn run_archival_fetch(
    cfg: Config,
    mut fetch_rx: UnboundedReceiver<u64>,
    block_tx: tokio::sync::mpsc::UnboundedSender<AppEvent>,
) -> Result<()> {
    // Must have archival URL configured
    let archival_url = match &cfg.archival_rpc_url {
        Some(url) => url.clone(),
        None => return Ok(()), // No archival URL, exit immediately
    };

    eprintln!("[Archival] Starting archival fetch task with URL: {archival_url}");

    while let Some(height) = fetch_rx.recv().await {
        eprintln!("[Archival] Received request to fetch block #{height}");

        match fetch_block_with_txs(
            &archival_url,
            height,
            cfg.rpc_timeout_ms,
            cfg.poll_chunk_concurrency,
            cfg.fastnear_auth_token.as_deref(),
        ).await {
            Ok(block) => {
                eprintln!("[Archival] Successfully fetched block #{} ({} txs)", height, block.tx_count);
                // Send block via existing event channel
                if let Err(e) = block_tx.send(AppEvent::NewBlock(block)) {
                    eprintln!("[Archival] Failed to send block: {e}");
                    break;
                }
            }
            Err(e) => {
                eprintln!("[Archival] Failed to fetch block #{height}: {e}");
                // TODO: Send error event to App so it can show toast notification
                // For now, just log the error
            }
        }
    }

    eprintln!("[Archival] Archival fetch task shutting down");
    Ok(())
}
