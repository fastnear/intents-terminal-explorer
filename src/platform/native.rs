//! Native platform implementation (uses tokio, copypasta, rusqlite)

use copypasta::{ClipboardContext, ClipboardProvider};

/// Copy text to system clipboard using copypasta
pub fn copy_to_clipboard(content: &str) -> bool {
    match ClipboardContext::new() {
        Ok(mut ctx) => ctx.set_contents(content.to_string()).is_ok(),
        Err(_) => false,
    }
}
