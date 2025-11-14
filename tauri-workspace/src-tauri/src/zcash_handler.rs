//! Zcash transaction handler - integrates auth, signing, and native messaging
//! This is the main orchestrator that ties together all Zcash functionality

use crate::zcash_auth;
use crate::zcash_native_msg::{NativeMessagingHandler, NativeResponse, TransactionRequest};
use crate::zcash_signer;
use std::sync::Arc;
use std::thread;
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Start the Zcash native messaging handler
/// This runs in a background thread and processes transaction requests from the browser extension
pub fn start_zcash_handler<R: Runtime>(app: AppHandle<R>) {
    log::info!("🚀 [Zcash Handler] Starting Zcash native messaging handler...");

    thread::spawn(move || {
        let handler = NativeMessagingHandler::new();

        // Start listening on STDIN for requests from browser extension
        let tx = handler.start_listener();
        let rx = handler.receiver();

        log::info!("🚀 [Zcash Handler] Listening for native messaging requests...");

        // Process incoming requests
        loop {
            match rx.recv() {
                Ok(request) => {
                    log::info!("📥 [Zcash Handler] Received request: {:?}", request);

                    match request.action.as_str() {
                        "signTransaction" => {
                            handle_sign_transaction(&app, request.params, request.session);
                        }
                        "ping" => {
                            log::info!("🏓 [Zcash Handler] Received ping");
                            let response = NativeResponse {
                                status: "pong".to_string(),
                                txid: None,
                                error: None,
                                session: request.session,
                            };
                            if let Err(e) = NativeMessagingHandler::send_response(&response) {
                                log::error!("🔴 [Zcash Handler] Failed to send pong: {}", e);
                            }
                        }
                        _ => {
                            log::warn!("⚠️ [Zcash Handler] Unknown action: {}", request.action);
                            let response = NativeResponse {
                                status: "error".to_string(),
                                txid: None,
                                error: Some(format!("Unknown action: {}", request.action)),
                                session: request.session,
                            };
                            if let Err(e) = NativeMessagingHandler::send_response(&response) {
                                log::error!("🔴 [Zcash Handler] Failed to send error response: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("🔴 [Zcash Handler] Error receiving request: {}", e);
                    break;
                }
            }
        }

        log::info!("🛑 [Zcash Handler] Native messaging handler stopped");
    });
}

/// Handle a transaction signing request
fn handle_sign_transaction<R: Runtime>(
    app: &AppHandle<R>,
    params: serde_json::Value,
    session: String,
) {
    log::info!("💰 [Zcash Handler] Processing transaction signing request...");

    // Parse transaction details
    let tx_request = match NativeMessagingHandler::parse_transaction(&params, session.clone()) {
        Ok(req) => req,
        Err(e) => {
            log::error!("🔴 [Zcash Handler] Failed to parse transaction: {}", e);
            let response = NativeResponse {
                status: "error".to_string(),
                txid: None,
                error: Some(e),
                session,
            };
            if let Err(err) = NativeMessagingHandler::send_response(&response) {
                log::error!("🔴 [Zcash Handler] Failed to send error response: {}", err);
            }
            return;
        }
    };

    log::info!("💰 [Zcash Handler] Transaction details:");
    log::info!("💰 [Zcash Handler]   To: {}", tx_request.to);
    log::info!("💰 [Zcash Handler]   Amount: {} ZEC", tx_request.amount);
    log::info!("💰 [Zcash Handler]   Memo: {}", tx_request.memo);

    // Authenticate user (Touch ID or PIN)
    let auth_reason = format!(
        "Approve Zcash transaction:\n\nSend {} ZEC to {}\n\nMemo: {}",
        tx_request.amount, tx_request.to, tx_request.memo
    );

    let auth_result = zcash_auth::authenticate(&auth_reason);

    if !auth_result.approved {
        log::warn!("❌ [Zcash Handler] User denied transaction");

        // Send denial response
        let response = NativeResponse {
            status: "denied".to_string(),
            txid: None,
            error: None,
            session: tx_request.session.clone(),
        };

        if let Err(e) = NativeMessagingHandler::send_response(&response) {
            log::error!("🔴 [Zcash Handler] Failed to send denial response: {}", e);
        }

        // Also trigger deep link callback
        send_deep_link_callback(app, "denied", None, &tx_request.session);

        return;
    }

    log::info!(
        "✅ [Zcash Handler] User approved transaction via {}",
        auth_result.method
    );

    // Sign the transaction
    match zcash_signer::sign_transaction(&tx_request) {
        Ok(signed_tx) => {
            log::info!("✅ [Zcash Handler] Transaction signed successfully");
            log::info!("✅ [Zcash Handler] TX ID: {}", signed_tx.txid);

            // Send approval response
            let response = NativeResponse {
                status: "approved".to_string(),
                txid: Some(signed_tx.txid.clone()),
                error: None,
                session: tx_request.session.clone(),
            };

            if let Err(e) = NativeMessagingHandler::send_response(&response) {
                log::error!("🔴 [Zcash Handler] Failed to send approval response: {}", e);
            }

            // Also trigger deep link callback
            send_deep_link_callback(app, "approved", Some(&signed_tx.txid), &tx_request.session);

            // Optional: Broadcast transaction
            if let Err(e) = zcash_signer::broadcast_transaction(&signed_tx) {
                log::error!("🔴 [Zcash Handler] Failed to broadcast transaction: {}", e);
            }
        }
        Err(e) => {
            log::error!("🔴 [Zcash Handler] Failed to sign transaction: {}", e);

            let response = NativeResponse {
                status: "error".to_string(),
                txid: None,
                error: Some(e),
                session: tx_request.session.clone(),
            };

            if let Err(err) = NativeMessagingHandler::send_response(&response) {
                log::error!("🔴 [Zcash Handler] Failed to send error response: {}", err);
            }

            send_deep_link_callback(app, "error", None, &tx_request.session);
        }
    }
}

/// Send a deep link callback to the browser
/// This allows the extension to intercept the result via webRequest API
fn send_deep_link_callback<R: Runtime>(
    app: &AppHandle<R>,
    status: &str,
    txid: Option<&str>,
    session: &str,
) {
    log::info!("🔗 [Zcash Handler] Sending deep link callback...");

    // Construct callback URL that extension will intercept
    let mut url = format!(
        "https://return.zwallet/txResult?status={}&session={}",
        status, session
    );

    if let Some(tx) = txid {
        url.push_str(&format!("&txid={}", tx));
    }

    log::info!("🔗 [Zcash Handler] Callback URL: {}", url);

    // Use opener plugin to open the URL in default browser
    // Extension will intercept this via webRequest listener
    if let Err(e) = tauri_plugin_opener::open_url(url, None::<&str>) {
        log::error!("🔴 [Zcash Handler] Failed to open callback URL: {}", e);
    } else {
        log::info!("🔗 [Zcash Handler] ✅ Callback URL opened successfully");
    }
}
