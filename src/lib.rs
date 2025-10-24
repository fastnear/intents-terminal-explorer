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
pub mod json_auto_parse;

// RPC utilities (same direct JSON-RPC implementation for both native and web)
pub mod rpc_utils;

pub mod app;
pub mod ui;
pub mod near_args;
pub mod filter;

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

// Arbitrage engine (native-only for now)
#[cfg(feature = "native")]
pub mod arb_engine;

#[cfg(feature = "native")]
pub mod ref_finance_client;

#[cfg(feature = "native")]
pub mod price_discovery;

#[cfg(feature = "native")]
pub mod arb_config;

#[cfg(feature = "native")]
pub mod slippage;

#[cfg(feature = "native")]
pub mod risk_manager;

#[cfg(feature = "native")]
pub mod execution_engine;

// Re-export commonly used types
pub use app::{App, InputMode};
pub use types::{AppEvent, BlockRow, TxLite, Mark};
pub use config::{Config, Source};
