// WASM-compatible transaction details fetch task (browser fetch API via reqwest-wasm)
#[cfg(target_arch = "wasm32")]
use crate::types::AppEvent;
#[cfg(target_arch = "wasm32")]
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// WASM-compatible background task for fetching transaction details via NEAR RPC
///
/// Unlike the native version, this:
/// - Uses browser Fetch API via reqwest (no blocking I/O)
/// - Spawns each request as a separate future (spawn_local)
/// - Falls back from live RPC to archival RPC if transaction not found
/// - Requires Bearer token authentication for FastNEAR RPC endpoints
///
/// # Arguments
/// * `fetch_rx` - Channel receiving (tx_hash, signer_id) tuples
/// * `event_tx` - Channel for sending fetched transaction details back to app
/// * `rpc_url` - Live NEAR RPC endpoint URL
/// * `archival_rpc_url` - Optional archival NEAR RPC endpoint URL for fallback
/// * `auth_token` - Optional Bearer token for RPC authentication
#[cfg(target_arch = "wasm32")]
pub async fn run_tx_details_fetch_wasm(
    mut fetch_rx: UnboundedReceiver<(String, String)>,
    event_tx: UnboundedSender<AppEvent>,
    rpc_url: String,
    archival_rpc_url: Option<String>,
    auth_token: Option<String>,
) {
    web_sys::console::log_1(
        &format!(
            "[TxDetailsFetch][WASM] Starting with live RPC: {}, archival RPC: {}",
            rpc_url,
            archival_rpc_url.as_deref().unwrap_or("none")
        )
        .into(),
    );

    while let Some((tx_hash, signer_id)) = fetch_rx.recv().await {
        let live_url = rpc_url.clone();
        let archival_url = archival_rpc_url.clone();
        let token = auth_token.clone();
        let tx = event_tx.clone();
        let hash = tx_hash.clone();
        let signer = signer_id.clone();

        web_sys::console::log_1(&format!("[TxDetailsFetch][WASM] Fetching tx: {} (signer: {})", hash, signer).into());

        // Spawn each fetch as independent future (non-blocking)
        spawn_local(async move {
            match crate::fastnear_api::fetch_transaction_details(
                &live_url,
                archival_url.as_deref(),
                &hash,
                &signer,
                5000, // 5 second timeout
                token.as_deref(),
            )
            .await
            {
                Ok(tx_data) => {
                    web_sys::console::log_1(
                        &format!("[TxDetailsFetch][WASM] ✅ Fetched tx details: {}", hash).into(),
                    );

                    // Auto-parse nested JSON (including EVENT_JSON: logs)
                    let parsed_data = crate::json_auto_parse::auto_parse_nested_json(tx_data, 5, 0);

                    // Convert to pretty JSON string
                    let json_str = crate::json_pretty::pretty_safe(&parsed_data, 2, 100 * 1024);

                    // Send back to the app
                    let _ = tx.send(AppEvent::FetchedTxDetails {
                        tx_hash: hash,
                        json_data: json_str,
                    });
                }
                Err(e) => {
                    web_sys::console::error_1(
                        &format!("[TxDetailsFetch][WASM] ❌ Failed to fetch tx {}: {}", hash, e)
                            .into(),
                    );
                }
            }
        });
    }

    web_sys::console::log_1(&"[TxDetailsFetch][WASM] Channel closed, shutting down".into());
}
