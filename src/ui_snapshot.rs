//! Shared UI snapshot + action types for all frontends.
//!
//! - `UiSnapshot` is a DOM/JSON-friendly view of `App`,
//!   used by the web/Tauri frontends.
//! - `UiAction` is a high-level, frontend-agnostic input
//!   (filter change, pane focus, selection, key presses, copy).
//! - `apply_ui_action` is the single place where these actions
//!   are translated into `App` mutations.
//!
//! The TUI, web DOM frontend, and any future GUI should all go
//! through these types where possible to keep behavior in lockstep.

use serde::{Deserialize, Serialize};

use crate::{App, BlockRow, TxLite};

/// One row in the Blocks pane (filtered view).
#[derive(Debug, Clone, Serialize)]
pub struct UiBlockRow {
    pub index: usize,
    pub height: u64,
    pub hash: String,
    pub when: String,
    pub tx_count: usize,
    pub owned_tx_count: usize,
    pub is_selected: bool,
}

/// One row in the Transactions pane (filtered view).
#[derive(Debug, Clone, Serialize)]
pub struct UiTxRow {
    pub index: usize,
    pub hash: String,
    pub signer_id: String,
    pub receiver_id: String,
    pub is_selected: bool,
    pub is_owned: bool,
}

/// DOM-/JSON-friendly snapshot of `App` state (Rust → UI).
#[derive(Debug, Clone, Serialize)]
pub struct UiSnapshot {
    /// 0 = Blocks, 1 = Txs, 2 = Details
    pub pane: u8,

    pub filter_query: String,
    pub owned_only_filter: bool,

    pub blocks: Vec<UiBlockRow>,
    pub blocks_total: usize,
    pub selected_block_height: Option<u64>,

    pub txs: Vec<UiTxRow>,
    pub txs_total: usize,

    pub details: String,
    pub details_fullscreen: bool,

    pub toast: Option<String>,
}

impl UiSnapshot {
    /// Build a snapshot from the current app state.
    pub fn from_app(app: &App) -> Self {
        let pane = app.pane() as u8;

        // Blocks (filtered view).
        let (blocks_filtered, selected_block_idx_opt, blocks_total) = app.filtered_blocks();
        let blocks: Vec<UiBlockRow> = blocks_filtered
            .into_iter()
            .enumerate()
            .map(|(idx, b)| UiSnapshot::block_row_from(idx, &b, app, selected_block_idx_opt))
            .collect();

        let selected_block_height = app.selected_block_height();

        // Transactions (filtered for current block).
        let (txs_vec, selected_tx_idx, txs_total) = app.txs();
        let txs: Vec<UiTxRow> = txs_vec
            .into_iter()
            .enumerate()
            .map(|(idx, tx)| UiSnapshot::tx_row_from(idx, &tx, selected_tx_idx, app))
            .collect();

        let details = app.details_pretty_string();
        let details_fullscreen = app.details_fullscreen();
        let toast = app.toast_message().map(|s| s.to_string());

        UiSnapshot {
            pane,
            filter_query: app.filter_query().to_string(),
            owned_only_filter: app.owned_only_filter(),
            blocks,
            blocks_total,
            selected_block_height,
            txs,
            txs_total,
            details,
            details_fullscreen,
            toast,
        }
    }

    fn block_row_from(
        index: usize,
        b: &BlockRow,
        app: &App,
        selected_block_idx_opt: Option<usize>,
    ) -> UiBlockRow {
        UiBlockRow {
            index,
            height: b.height,
            hash: b.hash.clone(),
            when: b.when.clone(),
            tx_count: b.tx_count,
            owned_tx_count: app.owned_count(b.height),
            is_selected: selected_block_idx_opt == Some(index),
        }
    }

    fn tx_row_from(
        index: usize,
        tx: &TxLite,
        selected_tx_idx: usize,
        app: &App,
    ) -> UiTxRow {
        let signer = tx.signer_id.clone().unwrap_or_default();
        let receiver = tx.receiver_id.clone().unwrap_or_default();
        UiTxRow {
            index,
            hash: tx.hash.clone(),
            signer_id: signer,
            receiver_id: receiver,
            is_selected: index == selected_tx_idx,
            is_owned: app.is_owned_tx(tx),
        }
    }
}

/// Frontend-agnostic high-level UI actions (UI → Rust).
///
/// These are what your TUI, web, and Tauri frontends should send into the core.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum UiAction {
    /// Update the filter query (applied immediately).
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

    /// Keyboard navigation (Arrow keys, PageUp/Down, Tab, Vim keys, etc.).
    Key {
        code: String,
        ctrl: bool,
        alt: bool,
        shift: bool,
        meta: bool,
    },

    /// Copy JSON / focused data (pane-aware).
    CopyFocusedJson,
}

/// Apply a UI action to the core `App`.
///
/// This is where navigation, selection, filters, and copy semantics live.
/// All frontends should call this for behavior consistency.
pub fn apply_ui_action(app: &mut App, action: UiAction) {
    match action {
        UiAction::SetFilter { text } => {
            app.set_filter_query(text);
        }
        UiAction::FocusPane { pane } => {
            app.set_pane_direct(pane as usize);
        }
        UiAction::SelectBlock { index } => {
            app.select_block_clamped(index);
        }
        UiAction::SelectTx { index } => {
            app.select_tx_clamped(index);
        }
        UiAction::ToggleOwnedOnly => {
            app.toggle_owned_filter();
        }
        UiAction::ToggleDetailsFullscreen => {
            app.toggle_details_fullscreen();
        }
        UiAction::Key {
            code,
            ctrl,
            alt: _,
            shift,
            meta,
        } => handle_key(app, &code, ctrl || meta, shift),
        UiAction::CopyFocusedJson => handle_copy(app),
    }
}

fn handle_key(app: &mut App, code: &str, ctrl: bool, shift: bool) {
    match code {
        // Arrow navigation.
        "ArrowUp" => app.up(),
        "ArrowDown" => app.down(),
        "ArrowLeft" => app.left(),
        "ArrowRight" => app.right(),

        // Vim-style navigation.
        "j" | "J" => app.down(),
        "k" | "K" => app.up(),
        "h" | "H" => app.left(),
        "l" | "L" => app.right(),

        // Paging in details.
        "PageUp" => app.page_up(20),
        "PageDown" => app.page_down(20),

        // Home/End.
        "Home" => app.home(),
        "End" => app.end(),

        // Tab / Shift+Tab for pane cycling.
        "Tab" if !shift => app.next_pane(),
        "Tab" if shift => app.prev_pane(),

        // Enter: open selected tx into details.
        "Enter" => app.select_tx(),

        // Space: toggle details fullscreen.
        " " => app.toggle_details_fullscreen(),

        // Ctrl+U: toggle owned-only filter.
        "u" | "U" if ctrl => app.toggle_owned_filter(),

        // Quit is a no-op for web/Tauri; TUI can layer its own logic.
        "q" | "Q" => {}

        // Everything else: ignore.
        _ => {}
    }
}

fn handle_copy(app: &mut App) {
    // If you want this gated by a feature, you can wrap this in #[cfg(feature = "clipboard")] etc.
    if crate::copy_api::copy_current(app) {
        let msg = match app.pane() {
            0 => "Copied block".to_string(),
            1 => "Copied transaction".to_string(),
            2 => "Copied details".to_string(),
            _ => "Copied".to_string(),
        };
        app.show_toast(msg);
    } else {
        app.show_toast("Copy failed".to_string());
    }
}
