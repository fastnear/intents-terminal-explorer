//! Native platform implementation (uses tokio, copypasta, rusqlite)

use crate::history::History as HistoryImpl;
use copypasta::{ClipboardContext, ClipboardProvider};

// Re-export commonly used history types
#[allow(unused_imports)]
pub use crate::history::{BlockPersist, HistoryHit, TxPersist};

/// Copy text to clipboard using copypasta
pub fn copy_to_clipboard(content: &str) -> bool {
    match ClipboardContext::new() {
        Ok(mut ctx) => ctx.set_contents(content.to_string()).is_ok(),
        Err(_) => false,
    }
}

// Re-export History type
pub type History = HistoryImpl;
