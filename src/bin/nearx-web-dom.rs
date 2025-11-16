#![cfg_attr(target_arch = "wasm32", no_main)]

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

use serde_json;

use tokio::sync::mpsc::{error::TryRecvError, unbounded_channel, UnboundedReceiver};

use nearx::{App, AppEvent, Config, Source};
use nearx::ui_snapshot::{UiSnapshot, UiAction, apply_ui_action};

/// Wasm-exposed app wrapper (held by JS).
#[wasm_bindgen]
pub struct WasmApp {
    app: App,
    event_rx: UnboundedReceiver<AppEvent>,
}

#[wasm_bindgen]
impl WasmApp {
    /// Construct a new WasmApp and start RPC polling in the background.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmApp {
        console_error_panic_hook::set_once();
        let _ = wasm_logger::init(wasm_logger::Config::default());

        let fps: u32 = 60;
        let fps_choices: Vec<u32> = vec![20, 30, 60];
        let keep_blocks: usize = 100;
        let default_filter = "acct:intents.near".to_string();

        let (event_tx, event_rx) = unbounded_channel::<AppEvent>();

        // Spawn RPC poller on wasm executor.
        let cfg_default_filter = default_filter.clone();
        let cfg_fps = fps;
        let cfg_fps_choices = fps_choices.clone();
        let cfg_keep_blocks = keep_blocks;

        spawn_local(async move {
            let cfg = Config {
                source: Source::Rpc,
                ws_url: "".to_string(),
                ws_fetch_blocks: false,
                render_fps: cfg_fps,
                render_fps_choices: cfg_fps_choices,
                poll_interval_ms: 1000,
                poll_max_catchup: 5,
                poll_chunk_concurrency: 4,
                keep_blocks: cfg_keep_blocks,
                near_node_url: "https://rpc.mainnet.fastnear.com/".to_string(),
                near_node_url_explicit: false,
                archival_rpc_url: None,
                rpc_timeout_ms: 8000,
                rpc_retries: 2,
                fastnear_auth_token: nearx::config::fastnear_token(),
                default_filter: cfg_default_filter,
                theme: nearx::theme::Theme::default(),
            };

            log::info!(
                "[WasmApp] RPC poller start - endpoint: {}",
                cfg.near_node_url
            );

            if let Err(e) = nearx::source_rpc::run_rpc(&cfg, event_tx).await {
                log::error!("[WasmApp] RPC poller error: {e}");
            }
        });

        let archival_fetch_tx = None;

        let app = App::new(
            fps,
            fps_choices,
            keep_blocks,
            default_filter,
            archival_fetch_tx,
        );

        WasmApp { app, event_rx }
    }

    /// Get current snapshot as JSON.
    #[wasm_bindgen]
    pub fn snapshot_json(&mut self) -> String {
        self.drain_events();
        let snap = UiSnapshot::from_app(&self.app);
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot: {e}");
            "{}".to_string()
        })
    }

    /// Apply an action and return updated snapshot JSON.
    #[wasm_bindgen]
    pub fn handle_action_json(&mut self, action_json: String) -> String {
        self.drain_events();

        match serde_json::from_str::<UiAction>(&action_json) {
            Ok(action) => apply_ui_action(&mut self.app, action),
            Err(e) => log::warn!(
                "[WasmApp] Failed to deserialize UiAction: {e} (payload={action_json})"
            ),
        }

        let snap = UiSnapshot::from_app(&self.app);
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot after action: {e}");
            "{}".to_string()
        })
    }
}

/// wasm-bindgen startup hook (theme injection).
#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    // Inject theme CSS vars from Rust theme
    if let Some(win) = window() {
        if let Some(doc) = win.document() {
            if let Some(root) = doc.document_element() {
                if let Some(html_root) = root.dyn_ref::<web_sys::HtmlElement>() {
                    let theme = nearx::theme::Theme::default();
                    for (name, value) in theme.to_css_vars() {
                        if let Err(e) = html_root.style().set_property(name, &value) {
                            log::warn!("[theme] Failed to set CSS var {}: {:?}", name, e);
                        }
                    }
                    log::info!("[theme] CSS variables injected from theme.rs");
                }
            }
        }
    }
}

// Non-wasm: stub main so `cargo build` is happy.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("nearx-web-dom is wasm-only; build with --target wasm32-unknown-unknown");
}

impl WasmApp {
    fn drain_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(ev) => self.app.on_event(ev),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::warn!("[WasmApp] Event channel disconnected");
                    break;
                }
            }
        }
    }
}
