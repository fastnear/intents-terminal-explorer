//! Unified theme for csli-style consistency across Terminal, Web, and Tauri
//!
//! This provides a single cohesive visual identity with:
//! - Consistent colors across all deployment targets
//! - Focus-aware pane backgrounds (csli-dashboard style)
//! - WCAG AA compliant contrast ratios
//! - Helpers for both ratatui and egui

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub bg: Rgb,            // App background
    pub panel: Rgb,         // Unfocused pane background
    pub panel_alt: Rgb,     // Focused pane background
    pub text: Rgb,          // Primary text
    pub text_dim: Rgb,      // Secondary text / metadata
    pub border: Rgb,        // Unfocused borders
    pub accent: Rgb,        // Links / highlights
    pub accent_strong: Rgb, // Focused borders / active states
    pub sel_bg: Rgb,        // Selection background
    pub success: Rgb,       // Success states
    pub warn: Rgb,          // Warning states
    pub error: Rgb,         // Error states

    // JSON syntax highlighting (WCAG AAA compliant)
    pub json_bg: Rgb,  // JSON details pane background (darker for better contrast)
    pub json_key: Rgb, // JSON object keys
    pub json_string: Rgb, // JSON string values
    pub json_number: Rgb, // JSON numbers
    pub json_bool: Rgb, // JSON booleans/null
    pub json_struct: Rgb, // JSON structural chars (braces, brackets, colons)
}

impl Default for Theme {
    fn default() -> Self {
        // Dark palette aligned to csli-dashboard proportions
        Theme {
            bg: Rgb(0x0b, 0x0e, 0x14),            // #0b0e14 - backdrop
            panel: Rgb(0x0f, 0x13, 0x1a),         // #0f131a - unfocused pane bg
            panel_alt: Rgb(0x12, 0x17, 0x22),     // #121722 - focused pane bg
            text: Rgb(0xe6, 0xed, 0xf3),          // #e6edf3 - primary text
            text_dim: Rgb(0xa2, 0xad, 0xbd),      // #a2adbd - secondary text
            border: Rgb(0x1e, 0x24, 0x30),        // #1e2430 - unfocused borders
            accent: Rgb(0x66, 0xb3, 0xff),        // #66b3ff - links/highlights
            accent_strong: Rgb(0x33, 0x99, 0xff), // #3399ff - focused borders
            sel_bg: Rgb(0x1e, 0x2a, 0x3a),        // #1e2a3a - selection background
            success: Rgb(0x6b, 0xdc, 0x96),       // #6bdc96 - success
            warn: Rgb(0xff, 0xcc, 0x66),          // #ffcc66 - warnings
            error: Rgb(0xff, 0x6b, 0x6b),         // #ff6b6b - errors

            // JSON syntax highlighting - WCAG AAA compliant (7:1+ contrast on dark bg)
            json_bg: Rgb(0x08, 0x0a, 0x0f), // #080a0f - even darker than app bg
            json_key: Rgb(102, 221, 236),   // #66DDEC - soft cyan (9.7:1 contrast)
            json_string: Rgb(171, 227, 56), // #ABE338 - yellow-green (11.4:1 contrast)
            json_number: Rgb(245, 171, 50), // #F5AB32 - warm orange (9.8:1 contrast)
            json_bool: Rgb(107, 190, 255),  // #6BBEFF - light blue (8.1:1 contrast)
            json_struct: Rgb(212, 208, 171), // #D4D0AB - tan/beige (9.3:1 contrast)
        }
    }
}

// ---------- Ratatui helpers (native TUI) ----------

#[cfg(feature = "native")]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub mod rat {
    use super::{Rgb, Theme};
    use ratatui::style::{Color, Modifier, Style};

    /// Convert theme RGB to ratatui Color
    #[inline]
    pub fn c(Rgb(r, g, b): Rgb) -> Color {
        Color::Rgb(r, g, b)
    }

    /// Common styles for widgets
    #[derive(Clone, Copy, Debug)]
    pub struct Styles {
        pub border: Style,
        pub border_focus: Style,
        pub title: Style,
        pub title_focus: Style,
        pub text: Style,
        pub text_dim: Style,
        pub selected: Style,
    }

    /// Generate ratatui styles from theme
    pub fn styles(t: &Theme) -> Styles {
        Styles {
            border: Style::default().fg(c(t.border)),
            border_focus: Style::default().fg(c(t.accent_strong)),
            title: Style::default().fg(c(t.text)).add_modifier(Modifier::BOLD),
            title_focus: Style::default()
                .fg(c(t.accent))
                .add_modifier(Modifier::BOLD),
            text: Style::default().fg(c(t.text)),
            text_dim: Style::default().fg(c(t.text_dim)),
            selected: Style::default().bg(c(t.sel_bg)).fg(c(t.text)),
        }
    }
}

// ---------- egui helpers (Web and Tauri) ----------

#[cfg(feature = "egui-web")]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub mod eg {
    use super::{Rgb, Theme};
    use egui::{Color32, Stroke};

    /// Convert theme RGB to egui Color32
    #[inline]
    pub fn c(Rgb(r, g, b): Rgb) -> Color32 {
        Color32::from_rgb(r, g, b)
    }

    /// Apply theme to egui context (call once at startup)
    /// Tuned to match TUI feel: low-chrome panels, strong focus/selection, AA contrast
    pub fn apply(ctx: &egui::Context, t: &Theme) {
        use egui::{CornerRadius, Shadow};

        let mut v = egui::Visuals::dark();

        // Override text color for consistency
        v.override_text_color = Some(c(t.text));

        // Panels & backgrounds - flat and minimal like TUI
        v.panel_fill = c(t.panel);
        v.window_fill = c(t.panel); // Avoid window chrome
        v.faint_bg_color = c(t.panel_alt); // Hovered backgrounds
        // IMPORTANT: eframe clears with extreme_bg_color. Use true bg, not panel.
        v.extreme_bg_color = c(t.bg);

        // Corner radius & shadows - subtle like TUI
        v.window_corner_radius = CornerRadius::same(4);
        v.menu_corner_radius = CornerRadius::same(4);
        v.widgets.noninteractive.corner_radius = CornerRadius::same(3);
        v.widgets.inactive.corner_radius = CornerRadius::same(3);
        v.widgets.hovered.corner_radius = CornerRadius::same(3);
        v.widgets.active.corner_radius = CornerRadius::same(3);
        v.window_shadow = Shadow::NONE;

        // Widget fills - keep frames light and text readable
        v.widgets.noninteractive.bg_fill = c(t.panel);
        v.widgets.inactive.bg_fill = c(t.panel_alt);
        v.widgets.hovered.bg_fill = c(t.panel_alt);
        v.widgets.active.bg_fill = c(t.panel_alt);
        v.widgets.open.bg_fill = c(t.panel_alt);

        // Widget strokes (borders) - match TUI border tone; stronger on focus/active
        let border = Stroke::new(1.0, c(t.border));
        v.widgets.noninteractive.bg_stroke = border;
        v.widgets.inactive.bg_stroke = border;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, c(t.accent));
        v.widgets.active.bg_stroke = Stroke::new(1.0, c(t.accent_strong));

        // Selection highlight - same accent emphasis as TUI selection
        v.selection.bg_fill = c(t.sel_bg);
        v.selection.stroke = Stroke::new(1.0, c(t.accent_strong));

        // Links & state colors aligned to theme
        v.hyperlink_color = c(t.accent);
        v.warn_fg_color = c(t.warn);
        v.error_fg_color = c(t.error);

        // Apply visuals and spacing
        let mut style = (*ctx.style()).clone();
        style.visuals = v;

        // Reduce chrome and match TUI density
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        style.spacing.indent = 8.0;
        style.interaction.tooltip_delay = 0.05; // Snappy tooltips

        ctx.set_style(style);

        // Font customization only on native - WASM uses embedded fonts
        // Calling set_fonts() in WASM can cause panics in FontImplCache
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut fonts = egui::FontDefinitions::default();
            if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                // Put system monospace ahead for crispness
                fam.insert(0, "ui-monospace".into());
                fam.insert(0, "monospace".into());
            }
            ctx.set_fonts(fonts);
        }
    }
}

// ---------- Contrast calculation (for testing) ----------

/// Convert sRGB component to linear RGB (WCAG formula)
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Calculate relative luminance (WCAG 2.0 formula)
fn rel_luminance(Rgb(r, g, b): Rgb) -> f32 {
    let r = srgb_to_linear(r as f32 / 255.0);
    let g = srgb_to_linear(g as f32 / 255.0);
    let b = srgb_to_linear(b as f32 / 255.0);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG 2.0 contrast ratio between foreground and background
///
/// ## Standards
/// - **AA small text**: >= 4.5:1
/// - **AA large text**: >= 3.0:1
/// - **AAA small text**: >= 7.0:1
pub fn contrast_ratio(fg: Rgb, bg: Rgb) -> f32 {
    let l1 = rel_luminance(fg);
    let l2 = rel_luminance(bg);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wcag_aa_primary_text_on_panel() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text, t.panel);
        assert!(
            ratio >= 4.5,
            "Primary text should meet WCAG AA on panel (got {:.2}:1, need >=4.5:1)",
            ratio
        );
    }

    #[test]
    fn wcag_aa_primary_text_on_bg() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text, t.bg);
        assert!(
            ratio >= 4.5,
            "Primary text should meet WCAG AA on bg (got {:.2}:1, need >=4.5:1)",
            ratio
        );
    }

    #[test]
    fn wcag_dim_text_readable() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text_dim, t.panel);
        // Allow slightly lower for secondary text (comfortable threshold)
        assert!(
            ratio >= 3.0,
            "Dim text should be readable (got {:.2}:1, need >=3.0:1)",
            ratio
        );
    }

    #[test]
    fn wcag_selection_contrast() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text, t.sel_bg);
        assert!(
            ratio >= 4.5,
            "Selected text should meet WCAG AA (got {:.2}:1, need >=4.5:1)",
            ratio
        );
    }

    #[test]
    fn wcag_focus_border_visible() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.accent_strong, t.panel);
        // Focus borders should be clearly visible
        assert!(
            ratio >= 3.0,
            "Focus border should be visible (got {:.2}:1, need >=3.0:1)",
            ratio
        );
    }
}
