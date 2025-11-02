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
pub mod types;
pub mod util_text;
pub mod json_pretty;
pub mod json_syntax;
pub mod json_auto_parse;

// RPC utilities (same direct JSON-RPC implementation for both native and web)
pub mod rpc_utils;

pub mod app;
pub mod ui;
pub mod near_args;
pub mod filter;
pub mod theme;

// History module (has native-only implementation internally)
pub mod history;

// Clipboard module (native-only)
#[cfg(feature = "native")]
pub mod clipboard;

// Platform-specific modules
#[cfg(feature = "native")]
pub mod source_ws;

pub mod source_rpc;
pub mod archival_fetch;

#[cfg(feature = "native")]
pub mod credentials;

#[cfg(feature = "native")]
pub mod marks;

// Platform abstraction layer
pub mod platform;
pub mod copy_payload;
pub mod copy_api;

// NOTE: Arbitrage engine has been moved to separate workspace member: ref-arb-scanner/
// To build: cargo build -p ref-arb-scanner
// To run: cargo run -p ref-arb-scanner

// Re-export commonly used types
pub use app::{App, InputMode};
pub use types::{AppEvent, BlockRow, TxLite, Mark};
pub use config::{Config, Source};
