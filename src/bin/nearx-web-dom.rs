//! NEARx Web (DOM-based)
//!
//! Pure DOM rendering using wasm-bindgen. The Rust app acts as a headless
//! state machine, exposing snapshots via JSON strings.

#![cfg_attr(target_arch = "wasm32", no_main)]

use wasm_bindgen::prelude::*;
use nearx::{App, Config, Source, ui_snapshot::{UiSnapshot, UiAction}};

#[cfg(target_arch = "wasm32")]
use web_time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

/// WASM façade wrapping the headless App
#[wasm_bindgen]
pub struct WasmApp {
    app: App,
    event_rx: tokio::sync::mpsc::UnboundedReceiver<nearx::types::AppEvent>,
}

#[wasm_bindgen]
impl WasmApp {
    /// Create a new WasmApp instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WasmApp, JsValue> {
        // Install panic hook for human-readable errors in DevTools
        #[cfg(target_arch = "wasm32")]
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        // Initialize logger
        #[cfg(target_arch = "wasm32")]
        wasm_logger::init(wasm_logger::Config::default());

        // Create event channel for RPC -> UI communication
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

        // Initialize App with defaults
        let fps = 30;
        let fps_choices = vec![20, 30, 60];
        let keep_blocks = 100;
        let default_filter = String::new();
        let archival_fetch_tx = None;

        let app = App::new(
            fps,
            fps_choices.clone(),
            keep_blocks,
            default_filter.clone(),
            archival_fetch_tx,
        );

        // Build RPC config with mainnet defaults
        let config = Config {
            source: Source::Rpc,
            ws_url: String::new(),
            ws_fetch_blocks: false,
            render_fps: fps,
            render_fps_choices: fps_choices,
            poll_interval_ms: 1000,
            poll_max_catchup: 5,
            poll_chunk_concurrency: 4,
            keep_blocks,
            near_node_url: "https://rpc.mainnet.fastnear.com/".to_string(),
            near_node_url_explicit: false,
            archival_rpc_url: None,
            rpc_timeout_ms: 8000,
            rpc_retries: 2,
            fastnear_auth_token: nearx::config::fastnear_token(),
            default_filter,
            theme: nearx::theme::Theme::default(),
        };

        // Spawn RPC poller in background
        let event_tx_clone = event_tx.clone();
        let config_clone = config.clone();
        wasm_bindgen_futures::spawn_local(async move {
            log::info!("🚀 Starting RPC poller ({})", config_clone.near_node_url);
            match nearx::source_rpc::run_rpc(&config_clone, event_tx_clone).await {
                Ok(_) => log::info!("✅ RPC poller completed"),
                Err(e) => log::error!("❌ RPC poller error: {}", e),
            }
        });

        Ok(WasmApp { app, event_rx })
    }

    /// Get current UI state as JSON snapshot
    #[wasm_bindgen]
    pub fn snapshot_json(&mut self) -> String {
        // Process pending RPC events (non-blocking drain)
        while let Ok(ev) = self.event_rx.try_recv() {
            self.app.on_event(ev);
        }

        let snap: UiSnapshot = self.app.ui_snapshot();
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot: {}", e);
            "{}".to_string()
        })
    }

    /// Handle a UI action from DOM (returns updated snapshot)
    #[wasm_bindgen]
    pub fn handle_action_json(&mut self, action_json: String) -> String {
        // Process pending RPC events first
        while let Ok(ev) = self.event_rx.try_recv() {
            self.app.on_event(ev);
        }

        // Parse and apply action
        match serde_json::from_str::<UiAction>(&action_json) {
            Ok(action) => {
                self.app.handle_ui_action(action);
            }
            Err(e) => {
                log::error!("Failed to deserialize UiAction: {:?}", e);
            }
        }

        // Return updated snapshot
        let snap: UiSnapshot = self.app.ui_snapshot();
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot after action: {}", e);
            "{}".to_string()
        })
    }

    /// Request repaint from DOM (call this on a timer for live updates)
    #[wasm_bindgen]
    pub fn tick(&mut self) -> String {
        self.snapshot_json()
    }
}

// WASM entry point (auto-called by wasm-bindgen)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    // Initialization happens in WasmApp::new()
    log::info!("✅ NEARx Web (DOM) module loaded");
}
