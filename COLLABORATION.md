# NEARx Development - Current Status & Architecture

## Executive Summary (2025-11-11 Update)

NEARx (formerly Ratacat) is a high-performance NEAR Protocol blockchain transaction viewer with **quad-mode deployment**: Terminal (TUI), Web (WASM), Desktop (Tauri), and Browser Extension integration.

**Current Status (All Targets Functional ✅)**:
- ✅ Native Terminal: Fully functional, excellent performance
- ✅ Web Browser: Working with relative WASM paths
- ✅ Tauri Desktop: Working with fixed CSP + relative paths
- ✅ Theme parity: Complete (unified tokens system)
- ✅ Keyboard/Mouse: Full parity across all targets

---

## Recent Critical Fixes (2025-11-11)

### 1. Tauri WASM Loading Regression (FIXED)

**Problem**: Dark screen after "Loading NEARx" disappeared, WASM preload warnings in console.

**Root Cause**: Trunk.toml configured with `public_url = "/"` (absolute paths) which work for web servers but fail with Tauri's custom protocol handler (`tauri://localhost/`).

**Solution**:
```toml
# Trunk.toml (line 13)
public_url = "./"  # Changed from "/" to "./"
```

**Why This Works**:
- Web servers: Both `/nearx-web.js` and `./nearx-web.js` resolve correctly
- Tauri protocol: `tauri://localhost/nearx-web.js` works, but `/nearx-web.js` tries to load from `tauri:///` (invalid)

**Files Modified**:
- `Trunk.toml` - Changed public_url to relative
- Rebuilt WASM with `trunk build --release`
- Tauri bundle now loads correctly

---

### 2. Tauri CSP Configuration (FIXED)

**Problem**: Console errors about IPC protocol being blocked, causing fallback to postMessage.

**Root Cause**: Content Security Policy missing required Tauri IPC directives.

**Solution**:
```json
// tauri.conf.json (line 24)
"csp": "default-src 'none'; script-src 'self'; ... connect-src 'self' ipc: http://ipc.localhost https://accounts.google.com ..."
```

**Added Directives**:
- `ipc:` - Tauri's IPC protocol
- `http://ipc.localhost` - Tauri's local IPC endpoint

**Reference**: [Tauri v2 CSP Documentation](https://v2.tauri.app/concept/security/#content-security-policy)

---

### 3. Tauri Plugin Configuration (FIXED)

**Problem**: App crashed immediately on launch with `PluginInitialization("opener", "Error deserializing 'plugins.opener'")`.

**Root Cause**: Invalid `opener` plugin configuration with unsupported `scope` field.

**Solution**: Removed invalid configuration block:
```json
// REMOVED FROM tauri.conf.json:
"opener": {
  "scope": [  // This field doesn't exist in opener plugin
    "https://accounts.google.com/*",
    ...
  ]
}
```

**Result**: Opener plugin works fine without explicit configuration.

---

### 4. Theme Tokens Centralization (IMPLEMENTED)

**Problem**: Visual design tokens (border thickness, layout ratios, spacing) hardcoded and duplicated across `src/ui.rs` (TUI) and `src/bin/nearx-web.rs` (Web/Tauri).

**Solution**: Created `src/theme/tokens.rs` as single source of truth.

**Architecture**:
```
src/theme/tokens.rs
├── LayoutSpec (top_ratio: 0.52, gaps)
├── VisualTokens (focus_stroke_px: 2.0, row_height_px: 22.0, radii)
└── RatTokens (focused_thick_border: true)
```

**Implementation**:

1. **TUI (`src/ui.rs`)**:
   - Layout ratio: Changed from hardcoded 30/70 to token-driven 52/48
   - Focused borders: Use `BorderType::Thick` when `tokens().rat.focused_thick_border` is true
   - Applied to all three panes: Blocks, Transactions, Details

2. **Web/Tauri (`src/bin/nearx-web.rs`)**:
   - Currently renders pure ratatui (no separate egui chrome)
   - Uses same `src/ui.rs` rendering code
   - Benefits from same token-driven design

**Visual Consistency**:
- TUI thick border = Web 2px stroke (both from tokens)
- 52/48 layout split matches csli-dashboard feel
- All spacing and gaps consistent

**Accessibility**:
- Included `audit_theme_for_contrast()` helper for WCAG AA compliance checking
- Can be called from theme application code to log contrast ratios

---

## Architecture Overview

### Quad-Mode Deployment

```
┌─────────────────────────────────────────────────────────────┐
│              NEARx Deployment Architecture                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Terminal (TUI)    Web (WASM)      Tauri Desktop            │
│  ├─ Crossterm     ├─ egui         ├─ Same as Web           │
│  ├─ SQLite        ├─ RPC only     ├─ Deep links            │
│  └─ WS + RPC      └─ In-memory    └─ Single instance       │
│                                                              │
│              ▼                                               │
│  ┌────────────────────────────────────────────────────┐    │
│  │          Shared Rust Core                          │    │
│  │  • App state (src/app.rs)                          │    │
│  │  • UI rendering (src/ui.rs - ratatui)              │    │
│  │  • Theme tokens (src/theme/tokens.rs) ◄── NEW     │    │
│  │  • RPC client (src/source_rpc.rs)                  │    │
│  │  • Filter (src/filter.rs)                          │    │
│  └────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

### Key Files

**Binaries**:
- `src/bin/nearx.rs` - Native terminal (Crossterm backend)
- `src/bin/nearx-web.rs` - Web/Tauri (egui_ratatui bridge)
- `tauri-workspace/src-tauri/src/main.rs` - Tauri wrapper

**Shared Core**:
- `src/app.rs` - Application state machine
- `src/ui.rs` - Ratatui UI rendering (used by ALL targets)
- `src/theme/` - Theme system
  - `tokens.rs` - Design tokens (NEW)
  - `mod.rs` - Theme definitions
  - `rat.rs` - Ratatui helpers
  - `eg.rs` - egui helpers

**Configuration**:
- `Trunk.toml` - Web build config (public_url = "./")
- `tauri-workspace/src-tauri/tauri.conf.json` - Tauri config (CSP, deep links)
- `.env.example` - Runtime configuration template

---

## Development Workflows

### 1. General UI Development (Recommended)

```bash
cd tauri-workspace
cargo tauri dev
```

**Features**:
- Hot reload on Rust file changes
- DevTools auto-open (Cmd+Option+I / F12)
- Debug logging enabled
- Fast iteration cycle

**When to Use**:
- UI layout changes
- Feature development
- Theme tweaks
- General debugging

**Limitations**:
- Deep links won't work (macOS Launch Services caches URL schemes)
- Use dev-deep-links.sh for deep link testing instead

---

### 2. Deep Link Testing (macOS Only)

```bash
./tauri-dev.sh test
```

**What It Does** (6 steps):
1. Kills any running instances
2. Builds debug binary (`cargo build` in tauri-workspace/src-tauri)
3. Clears macOS Launch Services cache
4. Creates .app bundle with Info.plist
5. Copies bundle to /Applications
6. Registers with Launch Services
7. Tests with `nearx://v1/tx/ABC123` (optional)

**Why Needed**:
- macOS caches URL scheme registrations
- `cargo tauri dev` doesn't update Launch Services
- Fresh bundle required for deep link testing

**When to Use**:
- Testing deep link handling
- After changing CFBundleURLTypes in Info.plist
- When deep links open wrong app version

**Script Location**: Project root (`./tauri-dev.sh`)

---

### 3. Web Development

```bash
trunk serve --open
# Opens at http://127.0.0.1:8083
```

**Features**:
- Auto-reload on changes
- No bundle/registration overhead
- Easy browser DevTools access

**When to Use**:
- Web-specific features (auth callbacks, etc.)
- Testing CORS/CSP
- Performance profiling

---

### 4. Native Terminal Development

```bash
cargo run --bin nearx --features native
```

**Features**:
- Full TUI experience
- SQLite persistence
- WebSocket + RPC support
- Fastest iteration for terminal-specific features

**When to Use**:
- Terminal-specific features
- SQLite history debugging
- Credential watching
- Performance optimization

---

## Build Commands Reference

### All Targets

```bash
# Check compilation (no binary)
cargo check --features native --bin nearx
cargo check --target wasm32-unknown-unknown --no-default-features --features egui-web --bin nearx-web

# Full builds
cargo build --release --features native --bin nearx              # Terminal
trunk build --release                                            # Web
cd tauri-workspace && cargo tauri build                          # Tauri
```

### Quick Iteration

```bash
# Fastest: Native terminal
cargo run --bin nearx --features native

# Fast: Web with hot reload
trunk serve

# Medium: Tauri with hot reload
cd tauri-workspace && cargo tauri dev

# Slow: Deep link testing (requires full rebuild + registration)
./tauri-dev.sh
```

---

## Common Issues & Solutions

### Issue 1: Tauri Shows Dark Screen After Loading

**Symptoms**:
- "Loading NEARx" disappears but UI doesn't render
- Console shows WASM preload warnings
- Deep link bridge initializes but nothing visible

**Solution**:
```bash
# 1. Verify Trunk.toml has relative paths
grep "public_url" Trunk.toml
# Should show: public_url = "./"

# 2. Rebuild WASM
trunk build --release

# 3. Rebuild Tauri
./tauri-dev.sh
```

---

### Issue 2: CSP Errors in Console

**Symptoms**:
```
Refused to connect to ipc://localhost/...
IPC custom protocol failed, falling back to postMessage
```

**Solution**:
Check `tauri.conf.json` CSP includes:
```json
"connect-src 'self' ipc: http://ipc.localhost ..."
```

---

### Issue 3: Deep Links Don't Work

**Symptoms**:
- Clicking `nearx://` link does nothing
- Opens wrong app version
- Opens browser instead of app

**Solution**:
```bash
# 1. Clean old registrations
./tauri-dev.sh clean

# 2. Rebuild and register
./tauri-dev.sh

# 3. Test
open 'nearx://v1/tx/ABC123'

# 4. Verify registration
mdfind "kMDItemCFBundleIdentifier == 'com.fastnear.nearx'"
# Should show: /Applications/NEARx.app
```

---

### Issue 4: Keyboard/Mouse Not Working

**Current Status**: ✅ Working across all targets

**If Regresses**:
1. Check browser console for errors
2. Verify no panic messages in logs
3. Test in `cargo tauri dev` mode (better error messages)
4. Check if egui wants_keyboard_input() is true

**Known Pattern**:
- Mouse: Click events must not hold app borrows during egui context access
- Keyboard: Key events collected first, then processed (avoid nested borrows)

---

## Theme System

### Tokens Structure

```rust
// src/theme/tokens.rs
pub struct Tokens {
    pub layout: LayoutSpec {
        top_ratio: 0.52,     // 52% top (Blocks+Txs), 48% bottom (Details)
        gap_px: 6.0,         // Gap between panels (egui)
        gap_cells: 1,        // Gap between panels (ratatui)
    },
    pub visuals: VisualTokens {
        focus_stroke_px: 2.0,      // Focused border width (egui)
        unfocus_stroke_px: 1.0,    // Unfocused border width (egui)
        window_radius_px: 4,       // Window corners
        widget_radius_px: 3,       // Widget corners
        row_height_px: 22.0,       // Virtual scroll row height
    },
    pub rat: RatTokens {
        focused_thick_border: true,  // Use BorderType::Thick when focused
        gap_cells: 1,
    },
}
```

### Usage Example

```rust
// In src/ui.rs (TUI)
let top_ratio = (tokens::tokens().layout.top_ratio * 100.0).round() as u16;
let rows = Layout::default()
    .constraints([Constraint::Percentage(top_ratio), ...])
    .split(area);

// Border thickness
if focused && tokens::tokens().rat.focused_thick_border {
    block.border_type(BorderType::Thick)
} else {
    block.border_type(BorderType::Rounded)
}
```

---

## Testing Checklist

### Before Committing

```bash
# 1. Check all targets compile
cargo check --features native --bin nearx
cargo check --target wasm32-unknown-unknown --no-default-features --features egui-web --bin nearx-web
cd tauri-workspace && cargo check

# 2. Format
cargo fmt

# 3. Clippy
cargo clippy --features native
cargo clippy --target wasm32-unknown-unknown --no-default-features --features egui-web

# 4. Run preflight
./tools/preflight.sh
```

### Manual Testing Matrix

| Feature | Terminal | Web | Tauri | Notes |
|---------|----------|-----|-------|-------|
| Arrow keys | ✅ | ✅ | ✅ | Navigate lists |
| Tab cycling | ✅ | ✅ | ✅ | Focus panes |
| Space toggle | ✅ | ✅ | ✅ | Fullscreen details |
| Copy ('c') | ✅ | ✅ | ✅ | Clipboard |
| Mouse click | N/A | ✅ | ✅ | Focus + select |
| Mouse scroll | N/A | ✅ | ✅ | Navigate lists |
| Double-click | N/A | ✅ | ✅ | Fullscreen toggle |
| Deep links | N/A | N/A | ✅ | `nearx://v1/tx/HASH` |
| Theme borders | ✅ | ✅ | ✅ | Thick when focused |
| Layout ratio | ✅ | ✅ | ✅ | 52/48 split |

---

## Configuration

### Environment Variables

```bash
# RPC endpoint
NEAR_NODE_URL=https://rpc.mainnet.fastnear.com/

# Authentication
FASTNEAR_AUTH_TOKEN=your_token_here

# Filtering
WATCH_ACCOUNTS=alice.near,bob.near
DEFAULT_FILTER="acct:intents.near"

# Performance
RENDER_FPS=30
KEEP_BLOCKS=100

# Archival (optional)
ARCHIVAL_RPC_URL=https://archival-rpc.mainnet.fastnear.com
```

See `.env.example` for complete documentation.

---

## Known Limitations

### Web/Tauri
- ⚠️ No SQLite persistence (in-memory only)
- ⚠️ No WebSocket support (RPC polling only)
- ⚠️ No credential watching
- ⚠️ WASM preload warning (cosmetic, doesn't affect functionality)

### Native Terminal
- ⚠️ Mouse text selection requires modifier key (Option/Alt/Shift depending on terminal)

### Tauri
- ⚠️ Deep links require `./tauri-dev.sh` for testing (not `cargo tauri dev`)
- ⚠️ Bundle identifier must not end in `.app` (Apple reserved)

---

## Next Steps / Roadmap

### High Priority
1. ✅ ~~Fix Tauri WASM loading~~ (DONE 2025-11-11)
2. ✅ ~~Centralize theme tokens~~ (DONE 2025-11-11)
3. ✅ ~~Fix CSP for Tauri IPC~~ (DONE 2025-11-11)
4. ⬜ E2E tests for keyboard/mouse interactions
5. ⬜ OAuth integration testing

### Medium Priority
1. ⬜ WASM preload warning fix (cosmetic)
2. ⬜ Performance profiling (Web vs Native)
3. ⬜ Contrast audit integration (call in debug builds)
4. ⬜ Documentation for browser extension integration

### Low Priority
1. ⬜ Windows/Linux deep link testing
2. ⬜ Code signing automation
3. ⬜ Auto-updater integration
4. ⬜ DMG installer with drag-to-Applications

---

## Files Modified (This Session - 2025-11-11)

1. **Trunk.toml** - Changed `public_url` from `/` to `./`
2. **tauri.conf.json** - Added `ipc:` and `http://ipc.localhost` to CSP, removed invalid opener config
3. **src/theme/tokens.rs** - Created (NEW)
4. **src/theme.rs** - Added `pub mod tokens;`
5. **src/ui.rs** - Updated to use tokens for layout ratio and border thickness
6. **dist-egui/** - Rebuilt with relative WASM paths

---

## Contact & Support

**Project Repository**: [GitHub URL]
**Documentation**: See `CLAUDE.md` for comprehensive technical details
**Bug Reports**: GitHub Issues
**Development Chat**: [Link if applicable]

---

**Last Updated**: 2025-11-11 21:15 UTC
**Status**: ✅ All targets functional, theme tokens centralized, ready for PE review
