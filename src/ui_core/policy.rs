/// Input policy shared by targets.
#[derive(Clone, Copy, Debug)]
pub struct InputPolicy {
    /// If true, Tab/Shift+Tab cycle panes (and Tab is consumed).
    pub tab_cycles_panes: bool,
    /// If true, Tab can focus text inputs. For NEARx web/tauri alpha: false.
    pub tab_focus_inputs: bool,
}

impl Default for InputPolicy {
    fn default() -> Self {
        Self {
            tab_cycles_panes: true,
            tab_focus_inputs: false,
        }
    }
}

// Web/Tauri default: panes only.
#[cfg(target_arch = "wasm32")]
pub fn default_policy() -> InputPolicy {
    InputPolicy::default()
}

// Native TUI default: also panes only.
#[cfg(not(target_arch = "wasm32"))]
pub fn default_policy() -> InputPolicy {
    InputPolicy::default()
}
