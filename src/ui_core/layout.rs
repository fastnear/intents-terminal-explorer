use std::cmp;

/// LayoutSpec is renderer-agnostic. Web/Tauri use pixels; TUI uses rows/cols.
#[derive(Clone, Copy, Debug)]
pub struct LayoutSpec {
    /// 0.0..1.0 fraction of total height for the top strip (Blocks+Tx).
    pub top_ratio: f32,
    /// Minimum height per strip in pixels (web); ignored by TUI helpers.
    pub min_px: f32,
    /// Minimum rows per strip (TUI).
    pub min_rows: u16,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        Self {
            top_ratio: 0.52,
            min_px: 48.0,
            min_rows: 3,
        }
    }
}

/// Web/Tauri: split a vertical space by ratio with clamping.
pub fn split_pixels(total_y: f32, spec: LayoutSpec) -> (f32, f32) {
    let r = spec.top_ratio.clamp(0.05, 0.95);
    let top = (total_y * r).max(spec.min_px);
    let bottom = (total_y - top).max(spec.min_px);
    (top, bottom)
}

/// TUI: split rows by ratio with clamping.
pub fn split_rows(total_rows: u16, spec: LayoutSpec) -> (u16, u16) {
    let r = spec.top_ratio.clamp(0.05, 0.95);
    let mut top = ((total_rows as f32) * r).round() as i32;
    let mut bottom = (total_rows as i32) - top;
    let min = spec.min_rows as i32;
    if top < min {
        top = min;
        bottom = (total_rows as i32 - top).max(min);
    }
    if bottom < min {
        bottom = min;
        top = (total_rows as i32 - bottom).max(min);
    }
    (cmp::max(0, top) as u16, cmp::max(0, bottom) as u16)
}

/// Even horizontal split.
pub fn split_half_pixels(total_x: f32) -> (f32, f32) {
    let half = total_x / 2.0;
    (half, half)
}
