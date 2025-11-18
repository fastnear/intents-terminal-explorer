use crate::theme::Theme;
use ratatui::text::Line;
use serde_json::Value;

/// Format JSON with syntax highlighting (colored Spans)
pub fn pretty_colored(v: &Value, _space: usize, theme: &Theme) -> Vec<Line<'static>> {
    // Use the new structured renderer
    crate::json_renderer::render_json(v, theme)
}

/// Format JSON as plain text (no colors)
pub fn pretty(v: &Value, _space: usize) -> String {
    // For now, continue using serde_json for plain text formatting
    // Our new renderer ensures proper formatting when colors are needed
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "{}".to_string())
}

/// Format JSON with truncation for massive payloads (prevents stack overflow)
///
/// Limits output to `max_bytes` to prevent UI rendering issues and memory exhaustion.
/// This is essential for raw block JSON which can exceed 10MB for blocks with many transactions.
///
/// # Arguments
/// * `v` - JSON value to format
/// * `space` - Indentation spaces per level (typically 2)
/// * `max_bytes` - Maximum output size in bytes (e.g., 100 * 1024 for 100KB)
///
/// # Returns
/// Formatted JSON string, truncated with "... (truncated)" footer if oversized
pub fn pretty_safe(v: &Value, space: usize, max_bytes: usize) -> String {
    let formatted = pretty(v, space);

    if formatted.len() > max_bytes {
        let truncated = &formatted[..max_bytes];
        // Find last complete line to avoid cutting mid-JSON
        let last_newline = truncated.rfind('\n').unwrap_or(max_bytes);
        let clean_truncate = &formatted[..last_newline];

        format!(
            "{}\n\n... (truncated - {} total bytes, showing first {} KB)\n",
            clean_truncate,
            formatted.len(),
            max_bytes / 1024
        )
    } else {
        formatted
    }
}
