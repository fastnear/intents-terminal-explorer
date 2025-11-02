//! Platform abstraction (clipboard, shared bits).

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::copy_to_clipboard;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::copy_to_clipboard;

// Re-export types used by UI code (kept for compatibility)
pub use crate::history::{BlockPersist, TxPersist, HistoryHit};
