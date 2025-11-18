//! Design tokens shared across targets. No IO, no platform deps.
//! Units:
//! - EGUI: pixels (f32)
//! - Ratatui: cells (u16) + border types

/// Layout spec: top (blocks+txs) vs bottom (details).
#[derive(Copy, Clone, Debug)]
pub struct LayoutSpec {
    /// Portion of height allocated to the top strip (0.0..=1.0).
    pub top_ratio: f32,
    /// Gap between stacked areas (EGUI, px).
    pub gap_px: f32,
    /// Gap between stacked areas (Ratatui, cells).
    pub gap_cells: u16,
}

/// Visual tokens for frames and lists.
#[derive(Copy, Clone, Debug)]
pub struct VisualTokens {
    /// Focused border stroke width (EGUI, px).
    pub focus_stroke_px: f32,
    /// Unfocused border stroke width (EGUI, px).
    pub unfocus_stroke_px: f32,
    /// Window corner radius (EGUI, px).
    pub window_radius_px: u8,
    /// Widget corner radius (EGUI, px).
    pub widget_radius_px: u8,
    /// Approx row height for virtualized lists (EGUI, px).
    pub row_height_px: f32,
    /// Monospace code font (JSON/details) in CSS px/egui points.
    pub code_font_px: f32,
    /// UI/body font for inputs and labels.
    pub ui_font_px: f32,
    /// Preferred number of rows when the filter is expanded/active.
    pub filter_rows: usize,
    /// Preferred number of rows when the filter is collapsed/idle.
    pub filter_rows_collapsed: usize,
}

/// Ratatui-specific approximations (borders are character-based).
#[derive(Copy, Clone, Debug)]
pub struct RatTokens {
    /// Whether focused panes use thick border characters.
    pub focused_thick_border: bool,
    /// Inter-pane gap (cells).
    pub gap_cells: u16,
}

#[derive(Copy, Clone, Debug)]
pub struct Tokens {
    pub layout: LayoutSpec,
    pub visuals: VisualTokens,
    pub rat: RatTokens,
}

impl Default for Tokens {
    fn default() -> Self {
        Self {
            layout: LayoutSpec {
                top_ratio: 0.52, // keep parity with csli-dashboard feel
                gap_px: 6.0,
                gap_cells: 1,
            },
            visuals: VisualTokens {
                focus_stroke_px: 2.0,
                unfocus_stroke_px: 1.0,
                window_radius_px: 4,
                widget_radius_px: 3,
                row_height_px: 22.0, // monospace ~13–14pt at default dpi
                // Web felt small; bump monospace a bit for retina.
                code_font_px: 16.0,
                ui_font_px: 19.0,
                filter_rows: 3,
                filter_rows_collapsed: 1,
            },
            rat: RatTokens {
                focused_thick_border: true,
                gap_cells: 1,
            },
        }
    }
}

/// Global accessor used by both targets.
#[inline]
pub fn tokens() -> Tokens {
    Tokens::default()
}

/// A11y audit (non-fatal): ensure core contrasts are sane (log-only).
#[allow(dead_code)]
pub fn audit_theme_for_contrast(rgb_fg: (u8, u8, u8), rgb_bg: (u8, u8, u8), label: &str) {
    fn lum(c: (u8, u8, u8)) -> f32 {
        let to_l = |x: u8| {
            let xf = (x as f32) / 255.0;
            if xf <= 0.03928 {
                xf / 12.92
            } else {
                ((xf + 0.055) / 1.055).powf(2.4)
            }
        };
        let (r, g, b) = c;
        0.2126 * to_l(r) + 0.7152 * to_l(g) + 0.0722 * to_l(b)
    }
    let l1 = lum(rgb_fg);
    let l2 = lum(rgb_bg);
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    let ratio = (hi + 0.05) / (lo + 0.05);
    if ratio < 4.5 {
        log::warn!(
            "⚠️ [theme] {} contrast {:.2}:1 below WCAG AA 4.5:1",
            label,
            ratio
        );
    } else {
        log::debug!("✅ [theme] {} contrast {:.2}:1", label, ratio);
    }
}
