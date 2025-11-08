//! WebSocket data source for NEAR blockchain
//!
//! This module is only available on native targets (not WASM).

use crate::{
    config::Config,
    rpc_utils::fetch_block_with_txs,
    types::{AppEvent, WsPayload},
};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::UnboundedSender;
use tokio_tungstenite::connect_async;
use tungstenite::protocol::Message;

/// Detect NEAR network from block height
/// Mainnet blocks are > 100M, testnet blocks are < 100M
fn detect_network_from_height(height: u64) -> &'static str {
    if height > 100_000_000 {
        "mainnet"
    } else {
        "testnet"
    }
}

/// Get RPC URL for detected network
fn get_rpc_url_for_network(network: &str) -> &'static str {
    match network {
        "mainnet" => "https://rpc.mainnet.near.org",
        "testnet" => "https://rpc.testnet.fastnear.com",
        _ => "https://rpc.testnet.fastnear.com",
    }
}

pub async fn run_ws(cfg: &Config, tx: UnboundedSender<AppEvent>) -> Result<()> {
    let (ws, _) = connect_async(&cfg.ws_url).await?;
    let (mut ws_write, mut ws_read) = ws.split();

    // Optional: identify as Ratacat client
    let _ = ws_write
        .send(Message::Text(r#"{"ratacat":"hello"}"#.into()))
        .await;

    while let Some(msg) = ws_read.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(_) => break,
        };
        if !msg.is_text() {
            continue;
        }
        let text = msg.into_text().unwrap_or_default();
        if let Ok(payload) = serde_json::from_str::<WsPayload>(&text) {
            match payload {
                WsPayload::Block { data: height } if cfg.ws_fetch_blocks => {
                    // Hybrid mode: fetch full block data via RPC
                    let tx_clone = tx.clone();

                    // Auto-detect network from block height if URL not explicitly set
                    let url = if cfg.near_node_url_explicit {
                        cfg.near_node_url.clone()
                    } else {
                        let detected_network = detect_network_from_height(height);
                        let auto_url = get_rpc_url_for_network(detected_network);
                        // Silently auto-detect network (logging would break TUI)
                        auto_url.to_string()
                    };

                    let timeout = cfg.rpc_timeout_ms;
                    let concurrency = cfg.poll_chunk_concurrency;
                    let auth_token = cfg.fastnear_auth_token.clone();
                    tokio::spawn(async move {
                        match fetch_block_with_txs(
                            &url,
                            height,
                            timeout,
                            concurrency,
                            auth_token.as_deref(),
                        )
                        .await
                        {
                            Ok(row) => {
                                let _ = tx_clone.send(AppEvent::NewBlock(row));
                            }
                            Err(_e) => {
                                // Silently fail (logging would break TUI)
                                // Fallback: send empty block notification
                                let _ = tx_clone
                                    .send(AppEvent::FromWs(WsPayload::Block { data: height }));
                            }
                        }
                    });
                }
                _ => {
                    // Legacy mode or Tx payload: pass through unchanged
                    let _ = tx.send(AppEvent::FromWs(payload));
                }
            }
        }
    }
    Ok(())
}
