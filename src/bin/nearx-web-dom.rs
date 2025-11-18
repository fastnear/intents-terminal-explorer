#![cfg_attr(target_arch = "wasm32", no_main)]

//! DOM-based Web/Tauri frontend for NEARx / Ratacat.
//!
//! This binary is compiled to WASM and loaded from `web/app.js` via wasm-bindgen.
//! It exposes a minimal JSON-based API:
//!
//!   - `snapshot_json() -> String`
//!   - `handle_action_json(action_json: String) -> String`
//!
//! where `action_json` is a serialized [`UiAction`] and the return value is a
//! serialized [`UiSnapshot`].

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use tokio::sync::mpsc::{error::TryRecvError, unbounded_channel, UnboundedReceiver};
use web_time::{Duration, Instant};

use nearx::ui_snapshot::{apply_ui_action, UiAction, UiSnapshot};
use nearx::{App, AppEvent, Config, Source};

/// Wasm-exposed app wrapper. JS owns an instance of this and communicates via JSON.
#[wasm_bindgen]
pub struct WasmApp {
    app: App,
    event_rx: UnboundedReceiver<AppEvent>,
    last_tick: Instant,  // For on_tick() throttling
}

impl Default for WasmApp {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl WasmApp {
    /// Construct a new WasmApp and start RPC polling in the background.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmApp {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::default());

        // Bootstrap OAuth token from localStorage (if user previously logged in)
        nearx::auth::bootstrap_from_storage();

        // Channel for RPC -> App events.
        let (event_tx, event_rx) = unbounded_channel::<AppEvent>();

        // Same defaults as the TUI / existing web path.
        let fps: u32 = 60;
        let fps_choices: Vec<u32> = vec![20, 30, 60];
        let keep_blocks: usize = 100;
        let default_filter = "acct:intents.near".to_string();

        // Initialize archival fetch channel (WASM version)
        let (archival_tx, archival_rx) = unbounded_channel::<u64>();
        let archival_fetch_tx = Some(archival_tx);

        // Initialize tx_details_fetch channel (WASM version)
        let (tx_details_tx, tx_details_rx) = unbounded_channel::<String>();

        // Build config for the RPC poller.
        let cfg_default_filter = default_filter.clone();
        let cfg_fps = fps;
        let cfg_fps_choices = fps_choices.clone();
        let cfg_keep_blocks = keep_blocks;

        // Extract FastNEAR configuration before the closure
        let fastnear_api_url = "https://api.fastnear.com".to_string();
        let fastnear_auth_token = {
            let token = nearx::config::fastnear_token();
            if token.is_empty() { None } else { Some(token) }
        };

        spawn_local(async move {
            let config = Config {
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
                archival_rpc_url: Some("https://archival-rpc.mainnet.fastnear.com/".to_string()),
                fastnear_api_url: "https://api.fastnear.com".to_string(),
                rpc_timeout_ms: 8_000,
                rpc_retries: 2,
                fastnear_auth_token: {
                    let token = nearx::config::fastnear_token();
                    if token.is_empty() { None } else { Some(token) }
                },
                default_filter: cfg_default_filter,
                theme: nearx::theme::Theme::default(),
            };

            log::info!(
                "[WasmApp] RPC poller start - endpoint: {}",
                config.near_node_url
            );

            // Spawn WASM archival fetch task if archival URL configured
            if let Some(archival_url) = config.archival_rpc_url.clone() {
                let auth_token = config.fastnear_auth_token.clone();
                let archival_event_tx = event_tx.clone();

                spawn_local(async move {
                    nearx::archival_fetch_wasm::run_archival_fetch_wasm(
                        archival_rx,
                        archival_event_tx,
                        archival_url,
                        auth_token,
                    ).await;
                });

                log::info!("[WasmApp] Archival fetch task spawned");
            }

            // Spawn WASM tx_details_fetch task if auth token configured
            if config.fastnear_auth_token.is_some() {
                let api_url = config.fastnear_api_url.clone();
                let auth_token = config.fastnear_auth_token.clone();
                let tx_details_event_tx = event_tx.clone();

                spawn_local(async move {
                    nearx::tx_details_fetch_wasm::run_tx_details_fetch_wasm(
                        tx_details_rx,
                        tx_details_event_tx,
                        api_url,
                        auth_token,
                    ).await;
                });

                log::info!(
                    "[WasmApp] Tx details fetch task spawned - API: {}, Auth: {}",
                    config.fastnear_api_url,
                    if config.fastnear_auth_token.is_some() { "present" } else { "missing" }
                );
            }

            if let Err(e) = nearx::source_rpc::run_rpc(&config, event_tx).await {
                log::error!("[WasmApp] RPC poller error: {e}");
            }
        });

        let app = App::new(
            fps,
            fps_choices,
            keep_blocks,
            default_filter,
            archival_fetch_tx,
            fastnear_api_url.clone(),
            fastnear_auth_token.clone(),
            if fastnear_auth_token.is_some() {
                Some(tx_details_tx)
            } else {
                None
            },
        );

        WasmApp {
            app,
            event_rx,
            last_tick: Instant::now(),
        }
    }

    /// Get current snapshot as JSON (Rust -> JS).
    #[wasm_bindgen]
    pub fn snapshot_json(&mut self) -> String {
        self.drain_events();
        let snap = UiSnapshot::from_app(&self.app);
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot: {e}");
            "{}".to_string()
        })
    }

    /// Apply an action (JSON-encoded UiAction) and return an updated snapshot.
    #[wasm_bindgen]
    pub fn handle_action_json(&mut self, action_json: String) -> String {
        self.drain_events();

        match serde_json::from_str::<UiAction>(&action_json) {
            Ok(action) => apply_ui_action(&mut self.app, action),
            Err(e) => {
                log::warn!("[WasmApp] Failed to deserialize UiAction ({e}): {action_json:?}");
            }
        }

        let snap = UiSnapshot::from_app(&self.app);
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot after action: {e}");
            "{}".to_string()
        })
    }

    /// Set Details pane viewport size (called by JS based on pane height).
    #[wasm_bindgen(js_name = "setDetailsViewportLines")]
    pub fn set_details_viewport_lines_js(&mut self, lines: u32) {
        self.app.set_details_viewport_lines(lines as usize);
    }

    /// Get clipboard content for the currently focused pane (called only on 'c' key).
    #[wasm_bindgen(js_name = "getClipboardContent")]
    pub fn get_clipboard_content(&mut self) -> String {
        self.drain_events();

        match self.app.pane() {
            0 => self.app.get_raw_block_json(),      // Blocks pane
            1 => self.app.get_raw_tx_json(),         // Transactions pane
            2 => self.app.details().to_string(),     // Details pane
            _ => String::new(),
        }
    }
}

/// wasm-bindgen startup hook - applies theme to DOM.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());

    // Apply theme CSS vars to :root for TUI-consistent styling
    let theme = nearx::theme::Theme::default();
    apply_theme_to_dom(&theme);
}

#[allow(unused_variables)]
fn apply_theme_to_dom(theme: &nearx::theme::Theme) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use web_sys::{window, HtmlElement};

        if let Some(window) = window() {
            if let Some(document) = window.document() {
                if let Some(root) = document.document_element() {
                    if let Some(html_root) = root.dyn_ref::<HtmlElement>() {
                        let style = html_root.style();
                        for (name, value) in theme.to_css_vars() {
                            let _ = style.set_property(name, &value);
                        }
                        log::info!(
                            "[theme] Applied {} CSS variables to :root",
                            theme.to_css_vars().len()
                        );
                    }
                }
            }
        }
    }
}

// On non-wasm targets this binary is not meant to run; provide a stub main
// so `cargo build --all` remains happy.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("nearx-web-dom is only supported on wasm32-unknown-unknown target.");
}

impl WasmApp {
    fn drain_events(&mut self) {
        // Drain all pending RPC events
        // let mut event_count = 0;
        loop {
            match self.event_rx.try_recv() {
                Ok(ev) => {
                    // event_count += 1;
                    self.app.on_event(ev);
                }
                Err(TryRecvError::Empty) => {
                    // Commented out to reduce console spam
                    // if event_count > 0 {
                    //     log::debug!("[drain_events] Processed {} events", event_count);
                    // }
                    break;
                }
                Err(TryRecvError::Disconnected) => {
                    log::warn!("[WasmApp] Event channel disconnected");
                    break;
                }
            }
        }

        // Periodic housekeeping: backfill chain-walking, etc.
        // Call on_tick() at most every 100ms to throttle archival requests
        let now = Instant::now();
        if now.duration_since(self.last_tick) >= Duration::from_millis(100) {
            self.app.on_tick(now);
            self.last_tick = now;
        }
    }
}
