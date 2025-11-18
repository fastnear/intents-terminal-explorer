// WASM-compatible transaction details fetch task (browser fetch API via reqwest-wasm)
#[cfg(target_arch = "wasm32")]
use crate::types::AppEvent;
#[cfg(target_arch = "wasm32")]
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

/// WASM-compatible background task for fetching transaction details from FastNEAR API
///
/// Unlike the native version, this:
/// - Uses browser Fetch API via reqwest (no blocking I/O)
/// - Spawns each request as a separate future (spawn_local)
/// - Returns immediately if API URL is empty or auth token missing
///
/// # Arguments
/// * `fetch_rx` - Channel receiving transaction hash requests
/// * `event_tx` - Channel for sending fetched transaction details back to app
/// * `api_url` - FastNEAR Explorer API endpoint URL
/// * `auth_token` - Optional FastNEAR auth token
#[cfg(target_arch = "wasm32")]
pub async fn run_tx_details_fetch_wasm(
    mut fetch_rx: UnboundedReceiver<String>,
    event_tx: UnboundedSender<AppEvent>,
    api_url: String,
    auth_token: Option<String>,
) {
    web_sys::console::log_1(
        &format!(
            "[TxDetailsFetch][WASM] Starting with API URL: {}, Auth: {}",
            api_url,
            if auth_token.is_some() { "present" } else { "missing" }
        )
        .into(),
    );

    while let Some(tx_hash) = fetch_rx.recv().await {
        let url = api_url.clone();
        let token = auth_token.clone();
        let tx = event_tx.clone();
        let hash = tx_hash.clone();

        web_sys::console::log_1(&format!("[TxDetailsFetch][WASM] Fetching tx: {}", hash).into());

        // Spawn each fetch as independent future (non-blocking)
        spawn_local(async move {
            match crate::fastnear_api::fetch_transaction_details(
                &url,
                &hash,
                5000, // 5 second timeout
                token.as_deref(),
            )
            .await
            {
                Ok(tx_data) => {
                    web_sys::console::log_1(
                        &format!("[TxDetailsFetch][WASM] ✅ Fetched tx details: {}", hash).into(),
                    );

                    // Convert to pretty JSON string
                    let json_str = crate::json_pretty::pretty_safe(&tx_data, 2, 100 * 1024);

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
