//! Platform abstraction layer for native and web targets
//!
//! This module provides a unified interface for platform-specific functionality
//! like clipboard access, persistent storage, and async runtime.

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::*;

#[cfg(feature = "web")]
mod web;
#[cfg(feature = "web")]
pub use web::*;

// Re-export types that are common across platforms
pub use crate::history::{BlockPersist, TxPersist, HistoryHit};
