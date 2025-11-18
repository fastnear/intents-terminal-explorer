//! Unified theme for csli-style consistency across Terminal, Web, and Tauri
//!
//! This provides a single cohesive visual identity with:
//! - Consistent colors across all deployment targets
//! - Focus-aware pane backgrounds (csli-dashboard style)
//! - WCAG AA compliant contrast ratios
//! - Helpers for both ratatui and egui

pub mod tokens;

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
    pub hover_bg: Rgb,      // Hover background (Web/Tauri rows)
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
            panel_alt: Rgb(0x1a, 0x20, 0x30),     // #1a2030 - focused pane bg
            text: Rgb(0xe6, 0xed, 0xf3),          // #e6edf3 - primary text
            text_dim: Rgb(0xa2, 0xad, 0xbd),      // #a2adbd - secondary text
            border: Rgb(0x5d, 0x63, 0x6d),        // #5d636d - unfocused borders
            accent: Rgb(0x66, 0xb3, 0xff),        // #66b3ff - links/highlights
            accent_strong: Rgb(0xff, 0xcc, 0x00), // #ffcc00 - focused borders (yellow)
            sel_bg: Rgb(0x1e, 0x2a, 0x3a),        // #1e2a3a - selection background
            hover_bg: Rgb(0x15, 0x1b, 0x23),      // #151b23 - hover background
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

impl Rgb {
    /// Convert RGB to CSS hex color string
    pub fn to_css_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
}

impl Theme {
    /// Export theme as CSS custom properties for web/Tauri
    ///
    /// Returns (var_name, hex_value) pairs that should be set on document.documentElement.style
    pub fn to_css_vars(&self) -> Vec<(&'static str, String)> {
        vec![
            // Core colors
            ("--bg", self.bg.to_css_hex()),
            ("--panel", self.panel.to_css_hex()),
            ("--panel-alt", self.panel_alt.to_css_hex()),
            ("--fg", self.text.to_css_hex()),
            ("--fg-dim", self.text_dim.to_css_hex()),
            ("--border", self.border.to_css_hex()),
            ("--accent", self.accent.to_css_hex()),
            ("--accent-strong", self.accent_strong.to_css_hex()),
            ("--sel-bg", self.sel_bg.to_css_hex()),
            ("--hover-bg", self.hover_bg.to_css_hex()),
            ("--success", self.success.to_css_hex()),
            ("--warn", self.warn.to_css_hex()),
            ("--error", self.error.to_css_hex()),
            // JSON syntax highlighting
            ("--json-bg", self.json_bg.to_css_hex()),
            ("--json-key", self.json_key.to_css_hex()),
            ("--json-string", self.json_string.to_css_hex()),
            ("--json-number", self.json_number.to_css_hex()),
            ("--json-bool", self.json_bool.to_css_hex()),
            ("--json-struct", self.json_struct.to_css_hex()),
        ]
    }
}

// ---------- Ratatui helpers (native TUI) ----------

#[cfg(feature = "native")]
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub mod ratatui_helpers {
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
            "Primary text should meet WCAG AA on panel (got {ratio:.2}:1, need >=4.5:1)"
        );
    }

    #[test]
    fn wcag_aa_primary_text_on_bg() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text, t.bg);
        assert!(
            ratio >= 4.5,
            "Primary text should meet WCAG AA on bg (got {ratio:.2}:1, need >=4.5:1)"
        );
    }

    #[test]
    fn wcag_dim_text_readable() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text_dim, t.panel);
        // Allow slightly lower for secondary text (comfortable threshold)
        assert!(
            ratio >= 3.0,
            "Dim text should be readable (got {ratio:.2}:1, need >=3.0:1)"
        );
    }

    #[test]
    fn wcag_selection_contrast() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.text, t.sel_bg);
        assert!(
            ratio >= 4.5,
            "Selected text should meet WCAG AA (got {ratio:.2}:1, need >=4.5:1)"
        );
    }

    #[test]
    fn wcag_unfocused_border_visible() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.border, t.panel);
        assert!(
            ratio >= 3.0,
            "Unfocused border should be visible (got {ratio:.2}:1, need >=3.0:1)"
        );
    }

    #[test]
    fn wcag_focus_border_visible() {
        let t = Theme::default();
        let ratio = contrast_ratio(t.accent_strong, t.panel);
        // Focus borders should be clearly visible
        assert!(
            ratio >= 3.0,
            "Focus border should be visible (got {ratio:.2}:1, need >=3.0:1)"
        );
    }
}
