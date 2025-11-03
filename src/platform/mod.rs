//! Platform abstraction (clipboard, shared bits).

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::copy_to_clipboard;

#[cfg(target_arch = "wasm32")]
mod runtime_wasm;
#[cfg(target_arch = "wasm32")]
pub use runtime_wasm::{Duration, Instant, init_logging, install_panic_hook, sleep};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::copy_to_clipboard;

#[cfg(not(target_arch = "wasm32"))]
mod runtime_native;
#[cfg(not(target_arch = "wasm32"))]
pub use runtime_native::{Duration, Instant, init_logging, install_panic_hook, sleep};

// Re-export types used by UI code (kept for compatibility)
pub use crate::history::{BlockPersist, TxPersist, HistoryHit};

