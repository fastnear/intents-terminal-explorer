//! Clipboard support for native platforms
//!
//! Note: This module is only compiled for native targets.
//! Web targets use web-sys clipboard API via platform::web module.

#[cfg(feature = "native")]
use copypasta::{ClipboardContext, ClipboardProvider};

#[cfg(feature = "native")]
pub fn copy_to_clipboard(s: &str) -> bool {
    match ClipboardContext::new() {
        Ok(mut ctx) => ctx.set_contents(s.to_string()).is_ok(),
        Err(_) => false,
    }
}
