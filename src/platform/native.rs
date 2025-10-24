//! Native platform implementation (uses tokio, copypasta, rusqlite)

use crate::history::History as HistoryImpl;

// Re-export commonly used history types
#[allow(unused_imports)]
pub use crate::history::{BlockPersist, TxPersist, HistoryHit};

// Re-export clipboard function
pub fn copy_to_clipboard(content: &str) -> bool {
    crate::clipboard::copy_to_clipboard(content)
}

// Re-export History type
pub type History = HistoryImpl;
