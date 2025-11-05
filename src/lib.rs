//! Ratacat - NEAR Blockchain Transaction Viewer
//!
//! This library provides the core functionality for Ratacat, a high-performance
//! terminal UI (and web UI) for monitoring NEAR Protocol blockchain transactions.
//!
//! ## Architecture
//!
//! Ratacat is built to work in two modes:
//! - **Native**: Terminal UI using crossterm and ratatui
//! - **Web**: Browser UI using egui, eframe, and egui_ratatui
//!
//! ## Usage
//!
//! For native builds:
//! ```bash
//! cargo build --features native
//! ```
//!
//! For web builds:
//! ```bash
//! trunk build --features web
//! ```

// Core modules (available on all platforms)
pub mod config;
pub mod constants;
pub mod json_auto_parse;
pub mod json_pretty;
pub mod json_syntax;
pub mod types;
pub mod util_text;

// RPC utilities (same direct JSON-RPC implementation for both native and web)
pub mod rpc_utils;

pub mod app;
pub mod ui;
pub use ui::pane_layout;
pub mod filter;
pub mod near_args;
pub mod theme;

// History module (has native-only implementation internally)
pub mod history;

// Platform-specific modules
#[cfg(feature = "native")]
pub mod source_ws;

pub mod archival_fetch;
pub mod source_rpc;

#[cfg(feature = "native")]
pub mod credentials;

#[cfg(feature = "native")]
pub mod marks;

// Platform abstraction layer
pub mod copy_api;
pub mod copy_payload;
pub mod platform;
pub mod spawn;

// WASM/JS interop bridge (has WASM implementation + non-WASM stubs internally)
pub mod webshim;

// Re-export commonly used types
pub use app::{App, InputMode};
pub use config::{Config, Source};
pub use types::{AppEvent, BlockRow, Mark, TxLite};
