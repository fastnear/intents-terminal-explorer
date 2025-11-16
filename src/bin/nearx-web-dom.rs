#![cfg_attr(target_arch = "wasm32", no_main)]

// DOM-based Web/Tauri frontend for NEARx.
//
// JS side:
//   import init, { WasmApp } from "./nearx-web-dom.js";
//   const app = new WasmApp();
//   const snap = JSON.parse(app.snapshot_json());
//   const snap2 = JSON.parse(app.handle_action_json(JSON.stringify({ type: "SetFilter", text: "foo" })));

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

use serde::{Deserialize, Serialize};

use tokio::sync::mpsc::{error::TryRecvError, unbounded_channel, UnboundedReceiver};

use nearx::{App, AppEvent, Config, Source};

/// One row in the Blocks pane (filtered view).
#[derive(Debug, Clone, Serialize)]
pub struct UiBlockRow {
    pub index: usize,          // index within filtered list
    pub height: u64,           // block height
    pub hash: String,          // block hash
    pub when: String,          // human-readable "when"
    pub tx_count: usize,       // total tx in block
    pub owned_tx_count: usize, // tx in block involving owned accounts
    pub is_selected: bool,     // row is currently selected
}

/// One row in the Transactions pane (filtered view).
#[derive(Debug, Clone, Serialize)]
pub struct UiTxRow {
    pub index: usize,     // index within filtered list
    pub hash: String,     // tx hash
    pub signer_id: String,
    pub receiver_id: String,
    pub is_selected: bool,
    pub is_owned: bool, // tx involves an owned account (signer/receiver)
}

/// Complete UI snapshot for DOM rendering.
#[derive(Debug, Clone, Serialize)]
pub struct UiSnapshot {
    /// 0 = Blocks, 1 = Txs, 2 = Details
    pub pane: u8,

    /// Current filter query (text)
    pub filter_query: String,

    /// Owned-only filter toggle
    pub owned_only_filter: bool,

    /// Blocks pane rows (filtered)
    pub blocks: Vec<UiBlockRow>,
    /// Total number of blocks in buffer (unfiltered)
    pub blocks_total: usize,

    /// Height of currently-selected block (if any)
    pub selected_block_height: Option<u64>,

    /// Transactions pane rows (filtered)
    pub txs: Vec<UiTxRow>,
    /// Total number of txs in current block (unfiltered)
    pub txs_total: usize,

    /// Pretty-printed JSON in the details pane
    pub details: String,

    /// Details fullscreen flag (Space toggles)
    pub details_fullscreen: bool,

    /// Toast notification (if any)
    pub toast: Option<String>,

    // === TUI-matching additions ===
    /// Current FPS
    pub fps: u32,

    /// Dynamic title for blocks pane (e.g., "Blocks (5 / 100)")
    pub blocks_title: String,

    /// Dynamic title for txs pane (e.g., "Txs (own: 2 of 5)")
    pub txs_title: String,

    /// Dynamic title for details pane (e.g., "Press 'c' to copy")
    pub details_title: String,

    /// Loading state (archival fetch in progress)
    pub loading_block: Option<u64>,

    /// Viewing cached block (not in main buffer)
    pub is_viewing_cached: bool,

    /// Count of pinned marks (for footer badge)
    pub pinned_marks_count: usize,
}

impl UiSnapshot {
    pub fn from_app(app: &App) -> Self {
        // Blocks (filtered)
        let (blocks_filtered, selected_block_idx_opt, blocks_total) = app.filtered_blocks();

        let blocks: Vec<UiBlockRow> = blocks_filtered
            .into_iter()
            .enumerate()
            .map(|(idx, b)| UiBlockRow {
                index: idx,
                height: b.height,
                hash: b.hash.clone(),
                when: b.when.clone(),
                tx_count: b.transactions.len(),
                owned_tx_count: app.owned_count(b.height),
                is_selected: selected_block_idx_opt == Some(idx),
            })
            .collect();

        let selected_block_height = app.selected_block_height();

        // Txs (filtered for current block)
        let (txs_vec, selected_tx_idx, txs_total) = app.txs();
        let txs: Vec<UiTxRow> = txs_vec
            .into_iter()
            .enumerate()
            .map(|(idx, tx)| {
                let signer = tx.signer_id.clone().unwrap_or_default();
                let receiver = tx.receiver_id.clone().unwrap_or_default();
                UiTxRow {
                    index: idx,
                    hash: tx.hash.clone(),
                    signer_id: signer,
                    receiver_id: receiver,
                    is_selected: idx == selected_tx_idx,
                    is_owned: app.is_owned_tx(&tx),
                }
            })
            .collect();

        let toast = app.toast_message().map(|s| s.to_string());

        // Dynamic titles (match TUI logic from ui.rs)
        let blocks_title = if app.is_viewing_cached_block() {
            "Blocks (cached) · ← Recent".to_string()
        } else if blocks.len() < blocks_total {
            format!("Blocks ({} / {})", blocks.len(), blocks_total)
        } else {
            "Blocks".to_string()
        };

        let owned_count = selected_block_height
            .map(|height| app.owned_count(height))
            .unwrap_or(0);
        let txs_title = if app.owned_only_filter() {
            format!("Txs (own: {} of {})", owned_count.min(txs_total), txs_total)
        } else if txs.len() < txs_total {
            format!("Txs ({} / {})", txs.len(), txs_total)
        } else {
            format!("Txs ({})", txs.len())
        };

        let pane_focused = app.pane() == 2;
        let details_title = if pane_focused {
            if app.details_fullscreen() {
                "Transaction details - Press 'c' to copy • Spacebar exits fullscreen".to_string()
            } else {
                "Transaction details - Press 'c' to copy • Spacebar to expand".to_string()
            }
        } else {
            "Transaction details".to_string()
        };

        UiSnapshot {
            pane: app.pane() as u8,
            filter_query: app.filter_query().to_string(),
            owned_only_filter: app.owned_only_filter(),
            blocks,
            blocks_total,
            selected_block_height,
            txs,
            txs_total,
            details: app.details_pretty_string(),
            details_fullscreen: app.details_fullscreen(),
            toast,
            fps: app.fps(),
            blocks_title,
            txs_title,
            details_title,
            loading_block: app.loading_block(),
            is_viewing_cached: app.is_viewing_cached_block(),
            pinned_marks_count: 0, // TODO: marks not available in web yet
        }
    }
}

/// Actions that JS can send to Rust.
///
/// These are intentionally small and high-level; JS owns DOM details,
/// Rust owns navigation and selection logic.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum UiAction {
    /// Update filter text (immediate apply).
    SetFilter { text: String },

    /// Focus a pane directly: 0 = Blocks, 1 = Txs, 2 = Details.
    FocusPane { pane: u8 },

    /// Select a block row by index in the filtered list.
    SelectBlock { index: usize },

    /// Select a tx row by index in the filtered list.
    SelectTx { index: usize },

    /// Toggle owned-only filter.
    ToggleOwnedOnly,

    /// Toggle details fullscreen mode.
    ToggleDetailsFullscreen,

    /// Keyboard navigation (Arrow keys, PageUp/Down, Tab, Enter, Space, j/k/h/l, etc.).
    Key {
        code: String,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    },
}

/// Initialize theme CSS variables on page load.
///
/// This runs once when WASM loads, injecting theme.rs colors into document root.
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

/// Wasm-exposed app wrapper.
///
/// Holds the core App and an event receiver for RPC events.
#[wasm_bindgen]
pub struct WasmApp {
    app: App,
    event_rx: UnboundedReceiver<AppEvent>,
}

#[wasm_bindgen]
impl WasmApp {
    /// Construct a new WasmApp and start RPC polling in the background.
    #[wasm_bindgen(constructor)]
    #[allow(clippy::new_without_default)]
    pub fn new() -> WasmApp {
        // Channel for RPC -> App events.
        let (event_tx, event_rx) = unbounded_channel::<AppEvent>();

        // Same defaults as nearx-web.rs
        let fps = 60;
        let fps_choices = vec![20, 30, 60];
        let keep_blocks = 100;
        let default_filter = "acct:intents.near".to_string();
        let archival_fetch_tx = None;

        // Build RPC config (mainnet).
        let config = Config {
            source: Source::Rpc,
            ws_url: "".to_string(),
            ws_fetch_blocks: false,
            render_fps: fps,
            render_fps_choices: fps_choices.clone(),
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
            default_filter: default_filter.clone(),
            theme: nearx::theme::Theme::default(),
        };

        // Spawn RPC poller in the background.
        let config_clone = config.clone();
        let event_tx_clone = event_tx.clone();
        spawn_local(async move {
            log::info!(
                "🚀 [WasmApp] Starting RPC poller ({})",
                config_clone.near_node_url
            );
            match nearx::source_rpc::run_rpc(&config_clone, event_tx_clone).await {
                Ok(_) => log::info!("[WasmApp] RPC poller completed"),
                Err(e) => log::error!("[WasmApp] RPC poller error: {e}"),
            }
        });

        // Initialize App with the same defaults.
        let app = App::new(
            fps,
            fps_choices,
            keep_blocks,
            default_filter,
            archival_fetch_tx,
        );

        WasmApp { app, event_rx }
    }

    /// Get a fresh snapshot as JSON (drains pending events first).
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
            Ok(action) => self.apply_action(action),
            Err(e) => {
                log::warn!(
                    "Failed to deserialize UiAction ({e}): {action_json:?}"
                );
            }
        }

        let snap = UiSnapshot::from_app(&self.app);
        serde_json::to_string(&snap).unwrap_or_else(|e| {
            log::error!("Failed to serialize UiSnapshot after action: {e}");
            "{}".to_string()
        })
    }
}

// Native builds: just provide a stub main so `cargo build --all-features` doesn't explode.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    eprintln!("nearx-web-dom is only supported on wasm32 (browser) target.");
}

impl WasmApp {
    fn drain_events(&mut self) {
        loop {
            match self.event_rx.try_recv() {
                Ok(ev) => {
                    self.app.on_event(ev);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    log::warn!("[WasmApp] Event channel disconnected");
                    break;
                }
            }
        }
    }

    fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::SetFilter { text } => {
                self.app.set_filter_query(text);
            }
            UiAction::FocusPane { pane } => {
                self.app.set_pane_direct(pane as usize);
            }
            UiAction::SelectBlock { index } => {
                self.app.select_block_clamped(index);
            }
            UiAction::SelectTx { index } => {
                self.app.select_tx_clamped(index);
            }
            UiAction::ToggleOwnedOnly => {
                self.app.toggle_owned_filter();
            }
            UiAction::ToggleDetailsFullscreen => {
                self.app.toggle_details_fullscreen();
            }
            UiAction::Key {
                code,
                ctrl,
                alt: _,
                shift,
                meta,
            } => {
                self.handle_key(code, ctrl || meta, shift);
            }
        }
    }

    fn handle_key(&mut self, code: String, ctrl: bool, shift: bool) {
        match code.as_str() {
            // Arrow navigation (same semantics as TUI).
            "ArrowUp" => self.app.up(),
            "ArrowDown" => self.app.down(),
            "ArrowLeft" => self.app.left(),
            "ArrowRight" => self.app.right(),

            // Vim-style aliases (j/k/h/l) for convenience.
            "j" | "J" => self.app.down(),
            "k" | "K" => self.app.up(),
            "h" | "H" => self.app.left(),
            "l" | "L" => self.app.right(),

            // Paging in details pane.
            "PageUp" => self.app.page_up(20),
            "PageDown" => self.app.page_down(20),

            // Home/End in details pane.
            "Home" => self.app.home(),
            "End" => self.app.end(),

            // Tab / Shift+Tab pane cycling.
            "Tab" if !shift => self.app.next_pane(),
            "Tab" if shift => self.app.prev_pane(),

            // Enter: open selected tx into details (same as TUI).
            "Enter" => self.app.select_tx(),

            // Space: toggle details fullscreen.
            " " => self.app.toggle_details_fullscreen(),

            // Ctrl+U: toggle owned-only filter (keyboard path).
            "u" | "U" if ctrl => self.app.toggle_owned_filter(),

            // Escape: let DOM handle filter focus; no-op here.
            "Escape" => {}

            // Everything else currently ignored.
            _ => {}
        }
    }
}
