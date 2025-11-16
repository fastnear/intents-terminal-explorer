//! Web platform implementation (uses web-sys, in-memory storage)

// Allow dead code when native feature is enabled (Tauri uses native, not web)
#![cfg_attr(feature = "native", allow(dead_code))]

use crate::history::{BlockPersist, HistoryHit};
use anyhow::Result;

/// Clipboard support for web using web-sys
pub fn copy_to_clipboard(_content: &str) -> bool {
    // Clipboard is now handled by web/platform.js via JavaScript bridge
    // This stub exists for API compatibility but isn't used
    false
}

/// In-memory history implementation for web
/// (SQLite not available in WASM, IndexedDB would be future enhancement)
pub struct History {
    // For initial web version, we skip persistence
    // Future: could use IndexedDB or localStorage
}

impl History {
    pub fn start(_db_path: &str) -> Result<Self> {
        log::info!("History persistence disabled for web build (in-memory only)");
        Ok(History {})
    }

    pub fn persist_block(&self, _block: BlockPersist) {
        // No-op for web version
    }

    pub async fn search(&self, _query: String, _limit: usize) -> Vec<HistoryHit> {
        // Return empty results for web version
        // Future: could implement in-memory search over recent blocks
        vec![]
    }

    pub async fn get_tx(&self, _hash: String) -> Option<String> {
        // Not available in web version
        None
    }
}
