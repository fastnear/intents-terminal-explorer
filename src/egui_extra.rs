//! Helper widgets for egui that extend standard functionality
//!
//! This module provides custom UI widgets that solve specific UX challenges,
//! particularly around keyboard focus management.

use egui::{self, *};

/// Creates a clickable button that does **not** participate in Tab focus.
///
/// This is useful for UI elements that should be clickable with the mouse but
/// shouldn't steal keyboard focus from the main application flow. For example,
/// a "Settings" button that would interfere with pane navigation if it could
/// receive Tab focus.
///
/// # Example
/// ```ignore
/// if egui_extra::non_tabbable_button(ui, "Settings").clicked() {
///     // Handle settings click
/// }
/// ```
pub fn non_tabbable_button(ui: &mut Ui, text: impl Into<WidgetText>) -> Response {
    let text: WidgetText = text.into();
    let galley = text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Button);
    let pad = vec2(8.0, 4.0);
    let size = galley.size() + pad * 2.0;

    // Allocate space but only respond to clicks (not keyboard focus)
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());

    // Get visual state (hover, click, etc.)
    let visuals = ui.style().interact(&resp);

    // Render the button if visible
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 4.0, visuals.bg_fill);
        ui.painter().rect_stroke(rect, 4.0, visuals.bg_stroke);
        ui.painter().galley(rect.min + pad, galley, visuals.text_color());
    }

    resp
}
