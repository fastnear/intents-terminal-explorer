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
pub mod json_auto_parse;
pub mod json_pretty;
pub mod json_syntax;
pub mod types;
pub mod util_text;

// RPC utilities (same direct JSON-RPC implementation for both native and web)
pub mod rpc_utils;

// Theme system (available on all platforms, with platform-specific helpers)
pub mod theme;

// UI core (layout and input policy - available on all platforms)
pub mod ui_core;

// csli-style pane frame helper (native-only)
#[cfg(feature = "native")]
pub mod pane_frame;

pub mod app;
pub mod filter;
pub mod near_args;
pub mod ui;

// Deep link router (available on all platforms)
pub mod router;

// UI feature flags (available on all platforms)
pub mod flags;

// Debug logging system (available on all platforms)
pub mod debug;

// UI snapshot types for DOM-based rendering (all platforms)
pub mod ui_snapshot;

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
pub mod platform;

// Authentication module (web/Tauri JavaScript bridge)
pub mod auth;

// Network utilities (429 backoff for native builds)
#[cfg(feature = "native")]
pub mod net;

// WASM-specific JavaScript bridge (web/Tauri only)
pub mod webshim;

// WASM-facing exports (JS -> Rust) are only built on wasm32.
// Keep surface tight to avoid pulling wasm_bindgen into native targets.
#[cfg(target_arch = "wasm32")]
pub mod wasm_api;

// Utility modules (shared across all targets)
pub mod util;

// Copy functionality (shared across all targets)
pub mod copy_api;
pub mod copy_payload;

// egui helpers (only available when egui is enabled)
// Temporarily disabled - needs API updates for egui 0.32
// #[cfg(feature = "egui-web")]
// pub mod egui_extra;

// Arbitrage engine modules - MOVED to ref-arb-scanner workspace
// These modules are no longer part of the main crate since ref-arb-scanner
// has been extracted to a separate workspace member.
// See: ref-arb-scanner/ directory
//
// #[cfg(feature = "native")]
// pub mod arb_engine;
// #[cfg(feature = "native")]
// pub mod ref_finance_client;
// #[cfg(feature = "native")]
// pub mod price_discovery;
// #[cfg(feature = "native")]
// pub mod arb_config;
// #[cfg(feature = "native")]
// pub mod slippage;
// #[cfg(feature = "native")]
// pub mod risk_manager;
// #[cfg(feature = "native")]
// pub mod execution_engine;

// Re-export commonly used types
pub use app::{App, BlockLite, InputMode};
pub use config::{Config, Source};
pub use types::{AppEvent, BlockRow, Mark, TxLite};
