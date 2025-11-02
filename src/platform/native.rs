//! Native platform implementation (uses tokio, copypasta, rusqlite)

// Re-export clipboard function
pub fn copy_to_clipboard(content: &str) -> bool {
    crate::clipboard::copy_to_clipboard(content)
}
