//! UI Feature Toggles
//!
//! This module provides opt-in/opt-out toggles for enhanced UI behaviors
//! introduced for Web/Tauri targets. All features are safe defaults that
//! can be disabled if they cause issues.

/// UI feature flags for controlling enhanced behaviors
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiFlags {
    /// Consume Tab/Shift+Tab after cycling panes so egui can't hijack focus.
    ///
    /// When enabled, after Tab switches panes, we consume the key event to prevent
    /// egui from moving focus to toolbar widgets or other UI elements.
    ///
    /// Default: `true` (recommended for Web/Tauri)
    pub consume_tab: bool,

    /// Snap egui pixels_per_point to devicePixelRatio (for crisp canvas).
    ///
    /// When enabled, aligns the egui rendering scale to the device's pixel ratio
    /// (snapped to nearest 0.5) to avoid fractional resampling blur on the canvas.
    ///
    /// Default: `true` (recommended for Web/Tauri)
    pub dpr_snap: bool,

    /// Map mouse/trackpad clicks to pane focus + row select.
    ///
    /// When enabled, clicking in the terminal UI will focus the appropriate pane
    /// and select the row under the cursor.
    ///
    /// Default: `true` on wasm32 (Web/Tauri), `false` on native (TUI)
    pub mouse_map: bool,

    /// Double-click in Details toggles fullscreen overlay.
    ///
    /// When enabled, double-clicking in the details pane toggles the fullscreen
    /// overlay mode.
    ///
    /// Default: `true` on wasm32 (Web/Tauri), `false` on native (TUI)
    pub dblclick_details: bool,
}

impl Default for UiFlags {
    fn default() -> Self {
        // Defaults depend on target:
        // - wasm32 (Web/Tauri): mouse on, dblclick on, crisp DPR, consume Tab
        // - native (TUI): keyboard-only parity, keep mouse/dblclick N/A
        #[cfg(target_arch = "wasm32")]
        {
            UiFlags {
                mouse_map: true,
                consume_tab: true,
                dpr_snap: true,
                dblclick_details: true,
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            UiFlags {
                mouse_map: false,
                consume_tab: true,
                dpr_snap: true,
                dblclick_details: false,
            }
        }
    }
}

impl UiFlags {
    /// Create flags with all features enabled (for testing new behaviors)
    pub fn all_enabled() -> Self {
        UiFlags {
            consume_tab: true,
            dpr_snap: true,
            mouse_map: true,
            dblclick_details: true,
        }
    }

    /// Create flags with all features disabled (for maximum stability)
    pub fn all_disabled() -> Self {
        UiFlags {
            consume_tab: false,
            dpr_snap: false,
            mouse_map: false,
            dblclick_details: false,
        }
    }

    /// Create flags with only keyboard features enabled
    pub fn keyboard_only() -> Self {
        UiFlags {
            consume_tab: true,
            dpr_snap: false,
            mouse_map: false,
            dblclick_details: false,
        }
    }
}
