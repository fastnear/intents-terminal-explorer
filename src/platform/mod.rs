//! Platform abstraction layer for native and web targets
//!
//! This module provides a unified interface for platform-specific functionality
//! like clipboard access, persistent storage, and async runtime.

// Import modules based on features
#[cfg(feature = "native")]
mod native;

#[cfg(feature = "egui-web")]
mod web;

// Export platform-specific implementations with priority:
// 1. Native takes precedence when both native and web features are enabled (e.g., Tauri)
// 2. Web is used only when native is not available (e.g., WASM-only builds)

#[cfg(feature = "native")]
pub use native::{copy_to_clipboard, History};

#[cfg(all(feature = "egui-web", not(feature = "native")))]
pub use web::{copy_to_clipboard, History};

// Re-export types that are common across platforms
pub use crate::history::{BlockPersist, HistoryHit, TxPersist};

/// Open a NEARx deep link (`nearx://…`) using the OS, to hand off to the desktop app.
/// Returns true if the command was launched successfully.
///
/// This is useful for:
/// - TUI: Open link in desktop app via OS protocol handler
/// - Web (trunk): Hand off to desktop app if installed
/// - Native: Not typically needed (already in desktop app)
#[cfg(not(target_arch = "wasm32"))]
pub fn open_deep_link(route_or_url: &str) -> bool {
    use std::process::Command;

    let url = if route_or_url.to_ascii_lowercase().starts_with("nearx://") {
        route_or_url.to_string()
    } else {
        format!("nearx://{}", route_or_url.trim_start_matches('/'))
    };

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(&url).spawn().is_ok()
    }

    #[cfg(target_os = "windows")]
    {
        // Use cmd start with empty title arg
        Command::new("cmd")
            .args(&["/C", "start", "", &url])
            .spawn()
            .is_ok()
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(&url).spawn().is_ok()
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(unused_variables)]
pub fn open_deep_link(_route_or_url: &str) -> bool {
    // In the browser, we can't spawn processes; use web/open_desktop.js helper instead.
    // JavaScript code should call: window.NEARx.openInDesktop('v1/tx/ABC123')
    false
}
