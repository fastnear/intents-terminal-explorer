//! UI snapshot and action types for DOM-based rendering
//!
//! This module provides a clean boundary between the headless App core
//! and the DOM frontend, enabling the wasm façade to serialize state as JSON.

use serde::{Deserialize, Serialize};

/// Snapshot of App state for DOM rendering (one-way data flow: Rust → JS)
#[derive(Debug, Clone, Serialize)]
pub struct UiSnapshot {
    /// Which pane has focus (0=Blocks, 1=Txs, 2=Details)
    pub focused_pane: usize,

    /// Filter bar state
    pub filter: FilterState,

    /// Blocks pane
    pub blocks: BlocksPane,

    /// Transactions pane
    pub txs: TxsPane,

    /// Details pane
    pub details: DetailsPane,

    /// Toast notification (transient message)
    pub toast: Option<String>,

    /// Loading state for archival fetch
    pub loading_block: Option<u64>,

    /// Auth state
    pub auth: AuthState,
}

#[derive(Debug, Clone, Serialize)]
pub struct FilterState {
    /// Current filter text (may not be applied yet if debouncing)
    pub text: String,
    /// Whether filter input has focus
    pub focused: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlocksPane {
    /// List of blocks to display (respects filter)
    pub rows: Vec<BlockRow>,
    /// Index of selected block in the rows list (None if no blocks)
    pub selected_index: Option<usize>,
    /// Total blocks in buffer (before filter)
    pub total_count: usize,
    /// Whether we're viewing a cached block (not in rolling buffer)
    pub viewing_cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockRow {
    pub height: u64,
    pub tx_count: usize,
    pub time_utc: String,
    /// Whether this block is available for navigation
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TxsPane {
    /// List of transactions to display (respects filter)
    pub rows: Vec<TxRow>,
    /// Index of selected tx in the rows list
    pub selected_index: usize,
    /// Total txs in block (before filter)
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TxRow {
    pub hash: String,
    pub signer_id: String,
    pub action_summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetailsPane {
    /// Pretty-printed JSON content (already syntax highlighted on Rust side)
    pub json: String,
    /// Whether details pane is in fullscreen mode
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthState {
    pub signed_in: bool,
    pub email: Option<String>,
}

/// User actions from DOM (data flow: JS → Rust)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum UiAction {
    // ----- Focus & Navigation -----
    /// Focus a specific pane (click)
    FocusPane { pane: usize },
    /// Cycle to next pane (Tab)
    NextPane,
    /// Cycle to previous pane (Shift+Tab)
    PrevPane,

    // ----- Block selection -----
    /// Select block by index in filtered list
    SelectBlock { index: usize },
    /// Navigate block up/down
    BlockUp,
    BlockDown,
    BlockPageUp,
    BlockPageDown,
    BlockHome,
    BlockEnd,

    // ----- Transaction selection -----
    /// Select tx by index in filtered list
    SelectTx { index: usize },
    /// Navigate tx up/down
    TxUp,
    TxDown,
    TxPageUp,
    TxPageDown,
    TxHome,
    TxEnd,

    // ----- Filter -----
    /// Update filter text (not yet applied)
    UpdateFilterText { text: String },
    /// Apply filter (Enter key)
    ApplyFilter { text: String },
    /// Focus filter input
    FocusFilter,

    // ----- Details -----
    /// Toggle fullscreen mode (Space or double-click)
    ToggleDetailsFullscreen,

    // ----- Copy -----
    /// Copy focused pane's JSON to clipboard
    CopyFocusedJson,

    // ----- Auth -----
    /// Sign in with Google
    SignInGoogle,
    /// Sign out
    SignOut,
}
