# NEARx Development - Architecture & Current Status

## Executive Summary

NEARx (formerly Ratacat) is a high-performance NEAR Protocol blockchain transaction viewer with **quad-mode deployment**: Terminal (TUI), Web (WASM), Desktop (Tauri), and Browser Extension integration.

**Current Status (2025-11-07)**:
- ✅ Native Terminal: Fully functional with mouse toggle, theme system, excellent performance
- ⚠️ Web Browser: Rendering broken - large blue area instead of UI, WASM errors in console
- ⚠️ Tauri Desktop: Similar issues to Web (shares WASM/egui pipeline)
- 🏗️ Theme parity work: Completed but Web/Tauri need rendering pipeline fixes

---

## Recent Work: Visual Parity Implementation (2025-11-07)

### Goal
Align Web and Tauri visual appearance with the TUI's flat, minimal aesthetic.

### Changes Made

#### 1. Enhanced `src/theme.rs::eg::apply()` (egui theme)
- **Flat panels**: No glossy chrome, minimal shadows
- **Subtle corner radius**: 4px windows, 3px widgets
- **Strong focus indicators**: Accent borders on hover, accent-strong on active
- **Compact spacing**: 8x6px items, 8x4px buttons (matches TUI density)
- **Selection emphasis**: Proper selection background + accent-strong stroke
- **State colors**: warn/error colors from theme
- **Monospace fonts**: System stack for code-like UI elements

**API fixes applied**:
- `Rounding` → `CornerRadius` (egui 0.32 API)
- `window_rounding` → `window_corner_radius`
- `f32` → `u8` for corner radius values

#### 2. Extended `web/theme.css`
- Added `--radius`, `--stroke`, `--spacing` variables
- Created `.nx-panel` class for DOM elements (toasts, overlays)
- Ensures any non-canvas UI elements match egui styling

#### 3. Fixed `src/bin/nearx-web.rs` frame
```rust
// Before:
.frame(egui::Frame::NONE.fill(egui::Color32::BLACK))

// After:
.frame(egui::Frame::default())  // Respects theme
```

### Build Verification
✅ All 3 targets compile successfully
✅ All 13 preflight checks pass
✅ No theme violations detected

---

## Critical Issue: Web/Tauri Rendering Broken

### Symptom
Large bright blue section dominates screen instead of proper 3-pane TUI layout.

### Evidence
- Screenshot shows ~70% of window as solid blue
- Small ratatui content visible in top portion
- User reported: "WASM-related error in browser dev console"

### Suspected Root Causes

#### 1. egui_ratatui Layout Issues
The ratatui terminal is being rendered but egui might be:
- Not allocating enough space for the terminal widget
- Incorrectly sizing the canvas
- Failing to paint the terminal texture

#### 2. SoftBackend Initialization
Current dimensions in `src/bin/nearx-web.rs:64-66`:
```rust
let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
    100,  // width in columns
    35,   // height in rows
    // ... fonts
);
```

**Problem**: Fixed size (100x35) might not match window size, causing:
- Clipping/overflow
- Blank areas filled with default color
- Layout math mismatch with actual pixels

#### 3. Frame Usage After Theme Changes
The `Frame::default()` change might interact poorly with egui's layout if:
- Default frame has padding/margins
- Inner/outer sizing calculations are wrong
- CentralPanel isn't expanding to fill window

#### 4. WASM Module Errors (Unreported)
User mentioned console errors but didn't share details. Could be:
- Font loading failures
- Texture allocation failures
- WebGL context issues
- Memory limits exceeded

---

## Unified Architecture Proposal

### Vision: Single Core, Multiple Transforms

```
┌─────────────────────────────────────────────────────────────┐
│                     Shared Core (Rust)                      │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ • App state (blocks, txs, selection, filter)          │ │
│  │ • Business logic (filtering, navigation, history)     │ │
│  │ • Unified Theme (Rgb values, contrast ratios)         │ │
│  │ • Data layer (RPC, archival fetch)                    │ │
│  └────────────────────────────────────────────────────────┘ │
└────────────────────────┬────────────────────────────────────┘
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
┌───────────────┐ ┌──────────────┐ ┌────────────────┐
│ Native TUI    │ │ Web/Tauri    │ │ Extension      │
│ Transform     │ │ Transform    │ │ Transform      │
├───────────────┤ ├──────────────┤ ├────────────────┤
│ • Crossterm   │ │ • egui       │ │ • Native msg   │
│ • Ratatui     │ │ • egui_ratatui│ │ • Deep links  │
│   direct      │ │ • SoftBackend│ │ • Relay        │
│ • SQLite      │ │ • WebGL      │ │                │
│ • Mouse opt-in│ │ • Mouse ON   │ │                │
│ • Theme::rat  │ │ • Theme::eg  │ │                │
└───────────────┘ └──────────────┘ └────────────────┘
```

### Core Principles

#### 1. Theme as Single Source of Truth
**Current State**: ✅ Working well
```rust
pub struct Theme {
    pub bg: Rgb,
    pub panel: Rgb,
    pub panel_alt: Rgb,
    pub text: Rgb,
    pub accent: Rgb,
    // ...
}
```

**Transform Layer**:
- `theme::rat` → ratatui `Color::Rgb`
- `theme::eg` → egui `Color32`
- `theme.css` → CSS variables

**Missing**: Web/Tauri need better integration with egui visuals.

#### 2. Input as Unified Events
**Current State**: ⚠️ Partially unified
- Native: Crossterm events → App methods
- Web: egui events → App methods
- Tauri: Same as Web

**Problems**:
- Mouse handling duplicated between TUI and Web binaries
- Tab consumption logic duplicated
- UiFlags partially applied

**Proposal**: Unified input abstraction
```rust
pub enum InputEvent {
    Key { code: KeyCode, modifiers: Modifiers },
    Mouse { kind: MouseKind, pos: (u16, u16) },
}

impl App {
    pub fn handle_input(&mut self, event: InputEvent) -> InputResult {
        // Unified logic for all targets
        // Returns commands for platform layer
    }
}
```

**Transform Layer**:
- Native: `crossterm::Event` → `InputEvent`
- Web: `egui::Event` → `InputEvent`

#### 3. Rendering as Target-Specific
**Current State**: ⚠️ Diverging implementations

**Native TUI**:
```rust
fn draw(f: &mut Frame, app: &App, marks: &[Mark]) {
    // Direct ratatui rendering
    // Full control over layout
}
```

**Web/Tauri**:
```rust
impl eframe::App for RatacatApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // egui layout
        // ratatui terminal as widget
        // Limited control
    }
}
```

**Problem**: Two completely different render paths with duplicated UI logic.

**Proposal**: Unified render core with target adapters
```rust
pub struct RenderState {
    pub blocks_area: Rect,
    pub txs_area: Rect,
    pub details_area: Rect,
    pub chrome_height: u16,
}

pub fn calculate_layout(total: Rect, app: &App) -> RenderState {
    // Unified layout math
    // Returns areas for all panes
}

// Native uses directly:
pub fn draw_native(f: &mut Frame, state: &RenderState, app: &App) {
    f.render_widget(blocks_widget, state.blocks_area);
    // ...
}

// Web wraps in egui:
pub fn draw_web(ui: &mut egui::Ui, terminal: &mut Terminal<...>, app: &App) {
    let state = calculate_layout(terminal.size(), app);
    terminal.draw(|f| draw_native(f, &state, app));
    ui.add(terminal.backend_mut());
}
```

#### 4. Clipboard as Platform Abstraction
**Current State**: ✅ Working well
```rust
// src/platform/mod.rs
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    #[cfg(feature = "native")]
    return native::copy_to_clipboard(text);

    #[cfg(target_arch = "wasm32")]
    return web::copy_to_clipboard(text);
}
```

**Transform Layer**:
- Native: copypasta crate
- Web: JavaScript bridge with 4-tier fallback
- Tauri: Tauri plugin (highest priority)

---

## Specific Issues to Fix

### 1. Web/Tauri Rendering (Critical)

**Task**: Fix large blue area, make ratatui content fill window

**Investigation Steps**:
1. Get WASM console errors from user (F12 → Console tab)
2. Check if SoftBackend is creating correct texture size
3. Verify egui_ratatui widget is getting full available space
4. Test with fixed window size vs. responsive sizing

**Potential Fixes**:
- Make SoftBackend dimensions responsive to window size
- Use `ui.available_size()` to calculate columns/rows dynamically
- Ensure CentralPanel has no margins/padding
- Check if Clear widget is working correctly

**Code Location**: `src/bin/nearx-web.rs:47-82` (RatacatApp::new)

### 2. Dynamic Terminal Sizing (High Priority)

**Current**: Fixed 100x35 columns/rows
```rust
let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
    100,  // Fixed
    35,   // Fixed
    // ...
);
```

**Needed**: Responsive sizing
```rust
fn calculate_terminal_size(window_size: egui::Vec2, font_size: (f32, f32)) -> (u16, u16) {
    let cols = (window_size.x / font_size.0).floor() as u16;
    let rows = (window_size.y / font_size.1).floor() as u16;
    (cols, rows)
}

// On window resize:
let (cols, rows) = calculate_terminal_size(ui.available_size(), (9.0, 18.0));
// Recreate terminal with new size
```

**Complexity**: SoftBackend might not support dynamic resize

**Alternative**: Use larger fixed size (e.g., 200x60) and let egui scale down

### 3. UiFlags Consistency (Medium Priority)

**Current State**: Flags exist but not consistently applied

```rust
pub struct UiFlags {
    pub consume_tab: bool,        // ✅ Applied in Web
    pub dpr_snap: bool,           // ✅ Applied in Web
    pub mouse_map: bool,          // ⚠️ Default false, needs testing
    pub dblclick_details: bool,   // ⚠️ Default false, needs testing
}
```

**Target-Specific Defaults** (from `src/flags.rs:18-25`):
```rust
#[cfg(target_arch = "wasm32")]
impl Default for UiFlags {
    fn default() -> Self {
        Self {
            consume_tab: true,
            dpr_snap: true,
            mouse_map: true,   // Should be ON for Web/Tauri
            dblclick_details: true,  // Should be ON for Web/Tauri
        }
    }
}
```

**Issue**: These defaults exist but mouse_map/dblclick_details features might not be fully wired up.

**Testing Needed**:
1. Verify mouse click selects panes/rows
2. Verify double-click toggles details fullscreen
3. Check if scroll wheel works

### 4. Font Rendering Sharpness (Low Priority)

**Current**: Using embedded_graphics_unicodefonts (9x18 bitmap)

**Observation**: Web might look blurry compared to native terminal

**Options**:
1. Keep bitmap fonts (current, crisp but retro)
2. Switch to egui's native text rendering (modern but loses terminal feel)
3. Hybrid: Use egui text for chrome, ratatui for content

**Decision**: Defer until rendering is working at all

### 5. Theme Application Timing (Medium Priority)

**Current**: Applied once on startup + when theme changes
```rust
let cur_theme = *self.app.borrow().theme();
if self.last_egui_theme != Some(cur_theme) {
    nearx::theme::eg::apply(ctx, &cur_theme);
    self.last_egui_theme = Some(cur_theme);
}
```

**Issue**: Frame background might not update if set before theme applies

**Fix**: Apply theme before creating frame, or use theme colors directly in frame construction
```rust
// Apply theme first
nearx::theme::eg::apply(ctx, &cur_theme);

// Then create panel with themed background
egui::CentralPanel::default()
    .frame(egui::Frame {
        fill: egui::Color32::from_rgb(
            cur_theme.bg.0,
            cur_theme.bg.1,
            cur_theme.bg.2
        ),
        ..Default::default()
    })
    .show(ctx, |ui| { /* ... */ });
```

---

## Transform Architecture Examples

### Example 1: Keyboard Input Transform

**Core Event**:
```rust
pub enum KeyAction {
    NextPane,
    PrevPane,
    ScrollUp(u16),
    ScrollDown(u16),
    Copy,
    ToggleFullscreen,
    // ...
}
```

**Native Transform**:
```rust
// src/bin/nearx.rs
match crossterm_event {
    Event::Key(KeyEvent { code: KeyCode::Tab, modifiers: KeyModifiers::SHIFT, .. }) => {
        app.handle_key_action(KeyAction::PrevPane);
    }
    Event::Key(KeyEvent { code: KeyCode::Tab, .. }) => {
        app.handle_key_action(KeyAction::NextPane);
    }
    // ...
}
```

**Web Transform**:
```rust
// src/bin/nearx-web.rs
match egui_event {
    egui::Event::Key { key: egui::Key::Tab, modifiers, .. } => {
        let action = if modifiers.shift {
            KeyAction::PrevPane
        } else {
            KeyAction::NextPane
        };
        app.handle_key_action(action);

        // Platform-specific: Consume Tab for egui
        if app.ui_flags().consume_tab {
            ctx.input_mut(|i| i.consume_key(...));
        }
    }
}
```

### Example 2: Mouse Input Transform

**Core Event**:
```rust
pub enum MouseAction {
    SelectPane(PaneIndex),
    SelectRow { pane: PaneIndex, row: usize },
    Scroll { delta: i32 },
    DoubleClickDetails,
}
```

**Native Transform**:
```rust
// src/bin/nearx.rs
match crossterm_event {
    Event::Mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, .. }) => {
        let pane = determine_pane_from_position(column, row, terminal.size()?);
        app.handle_mouse_action(MouseAction::SelectPane(pane));

        if pane != PaneIndex::Details {
            let row_idx = (row - 2).max(0) as usize;
            app.handle_mouse_action(MouseAction::SelectRow { pane, row: row_idx });
        }
    }
}
```

**Web Transform**:
```rust
// src/bin/nearx-web.rs
if flags.mouse_map && resp.hovered() {
    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
        // Convert pixels to cells
        let col = ((pos.x - rect.min.x) / cell_w).floor() as i32;
        let row = ((pos.y - rect.min.y) / cell_h).floor() as i32;

        let pane = determine_pane_from_position(col, row, terminal.size()?);
        app.handle_mouse_action(MouseAction::SelectPane(pane));

        // Platform-specific: Double-click on details
        if flags.dblclick_details && ui.input(|i| i.pointer.button_double_clicked(...)) {
            app.handle_mouse_action(MouseAction::DoubleClickDetails);
        }
    }
}
```

### Example 3: Theme Transform

**Core Definition**:
```rust
// src/theme.rs
pub struct Theme {
    pub bg: Rgb,
    pub panel: Rgb,
    pub accent: Rgb,
    // ...
}

impl Theme {
    pub fn contrast_ratio(&self, fg: Rgb, bg: Rgb) -> f32 {
        // WCAG calculation
    }
}
```

**Native Transform**:
```rust
// src/theme.rs (mod rat)
#[inline]
pub fn c(Rgb(r, g, b): Rgb) -> ratatui::style::Color {
    Color::Rgb(r, g, b)
}

pub fn styles(t: &Theme) -> Styles {
    Styles {
        border: Style::default().fg(c(t.border)),
        border_focus: Style::default().fg(c(t.accent_strong)),
        // ...
    }
}
```

**Web Transform**:
```rust
// src/theme.rs (mod eg)
#[inline]
pub fn c(Rgb(r, g, b): Rgb) -> egui::Color32 {
    Color32::from_rgb(r, g, b)
}

pub fn apply(ctx: &egui::Context, t: &Theme) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = c(t.panel);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, c(t.accent));
    // ... full egui visuals mapping
    ctx.set_style(style);
}
```

**CSS Transform**:
```css
/* web/theme.css - generated or manually synced */
:root {
  --bg: #0b0e14;      /* Theme.bg */
  --panel: #0f131a;   /* Theme.panel */
  --accent: #66b3ff;  /* Theme.accent */
  /* ... */
}
```

---

## Testing Strategy

### Web/Tauri Rendering Fix

**Step 1: Get Error Details**
```bash
# User runs:
trunk serve --open
# Then in browser: F12 → Console tab
# Copy all errors/warnings mentioning WASM, canvas, or texture
```

**Step 2: Minimal Reproduction**
```rust
// Simplify RatacatApp::new to minimal working state
let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
    80, 24,  // Standard terminal size
    mono_9x18_atlas(),
    None, None,
);
// Test if even blank terminal renders correctly
```

**Step 3: Incremental Fixes**
1. Fix any WASM module errors
2. Ensure SoftBackend creates valid texture
3. Verify egui_ratatui widget renders texture
4. Add responsive sizing
5. Apply theme properly

### Cross-Platform Input Testing

**Test Matrix**:
| Action | Native (TUI) | Web (Browser) | Tauri (Desktop) |
|--------|-------------|---------------|-----------------|
| Tab cycles panes | ✅ Ctrl+M mouse | ⚠️ Test needed | ⚠️ Test needed |
| Click selects pane | ✅ (opt-in) | ⚠️ Should work | ⚠️ Should work |
| Double-click details | ❌ N/A | ⚠️ Test needed | ⚠️ Test needed |
| Scroll wheel | ✅ (opt-in) | ⚠️ Test needed | ⚠️ Test needed |
| Space fullscreen | ✅ Works | ⚠️ Test needed | ⚠️ Test needed |
| Copy (c key) | ✅ Works | ⚠️ Test needed | ⚠️ Test needed |

---

## Immediate Next Steps

### Priority 1: Fix Web Rendering (Blocking)
1. User provides console errors
2. Investigate SoftBackend sizing
3. Test with different terminal dimensions
4. Verify egui_ratatui integration

### Priority 2: Test Mouse/Keyboard on Web
1. Verify UiFlags defaults are actually applied
2. Test mouse click → pane selection
3. Test double-click → fullscreen toggle
4. Test scroll wheel
5. Document any broken behaviors

### Priority 3: Unified Input Layer (Refactoring)
1. Create `InputEvent` enum in core
2. Implement `App::handle_input_event(&mut self, event: InputEvent)`
3. Update Native binary to use new API
4. Update Web binary to use new API
5. Verify behavior unchanged

### Priority 4: Documentation
1. Update README with current status
2. Add troubleshooting section for Web rendering
3. Document UiFlags and target-specific defaults
4. Create ARCHITECTURE.md with transform diagrams

---

## Long-Term Vision

### Unified Core API
```rust
// High-level goal
pub struct NEARx {
    app: App,
    platform: Box<dyn PlatformAdapter>,
}

trait PlatformAdapter {
    fn render(&mut self, state: &RenderState) -> Result<()>;
    fn poll_events(&mut self) -> Vec<InputEvent>;
    fn copy_to_clipboard(&self, text: &str) -> Result<()>;
}

// Native
struct CrosstermAdapter { /* ... */ }
impl PlatformAdapter for CrosstermAdapter { /* ... */ }

// Web/Tauri
struct EguiAdapter { /* ... */ }
impl PlatformAdapter for EguiAdapter { /* ... */ }
```

### Benefits of Unified Approach
1. **Single UI Logic**: All layout/rendering rules in one place
2. **Consistent Behavior**: Input handling guaranteed same across targets
3. **Easier Testing**: Mock PlatformAdapter for unit tests
4. **Theme Compliance**: Impossible to use wrong colors (enforced at compile time)
5. **New Targets Easy**: Just implement PlatformAdapter trait

---

## Conclusion

NEARx has excellent foundational architecture with successful platform abstractions for theme, clipboard, and configuration. The current challenge is completing the Web/Tauri rendering pipeline to achieve feature parity with the native TUI.

**Key Strengths**:
- ✅ Unified theme system (Rgb → ratatui/egui/CSS)
- ✅ Clean platform abstraction (clipboard, storage)
- ✅ Strong native TUI experience
- ✅ Comprehensive documentation (CLAUDE.md, COLLABORATION.md, KEYMAP.md)

**Current Gaps**:
- ⚠️ Web/Tauri rendering broken (large blue area)
- ⚠️ Input handling duplicated between binaries
- ⚠️ UiFlags not fully tested on Web/Tauri
- ⚠️ No responsive terminal sizing

**Path Forward**:
1. Fix Web rendering (get console errors, debug SoftBackend)
2. Test all interactions on Web/Tauri (mouse, keyboard, copy)
3. Refactor input handling to unified core
4. Implement responsive terminal sizing
5. Document transform architecture for future contributors

The vision is clear: **Single Core, Multiple Transforms**. We're 70% there—just need to fix the rendering pipeline and complete the Web/Tauri experience.
