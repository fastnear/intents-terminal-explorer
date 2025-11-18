# NEARx / Ratacat

**Version 0.4.5+** - Production-ready NEAR blockchain explorer with quad-mode architecture

Fast, keyboard-driven interface sharing a single Rust core across four deployment targets:

- **Terminal UI** (Ratatui) - Native TUI with full features
- **Web UI** (DOM + WASM) - Pure DOM, no canvas/WebGL
- **Desktop App** (Tauri v2) - Native app with deep link support
- **Browser Extension** - 1Password-style integration (WIP)

---

## ✨ Recent Features (v0.4.0-0.4.5)

### 🔐 OAuth & Authentication (v0.4.2)
- Google OAuth + Magic link authentication
- Secure token management with localStorage persistence
- XSS-hardened CSP headers for Web/Tauri

### 🎨 Pure DOM Frontend (v0.4.3)
- Complete egui removal - zero canvas dependencies
- JSON bridge pattern: `UiSnapshot` (state) + `UiAction` (commands)
- Native browser UX: text selection, scrolling, accessibility

### 📋 Unified Clipboard System (v0.4.1)
- 4-tier fallback: Tauri plugin → Extension relay → Navigator API → execCommand
- Platform abstraction eliminates code duplication
- Works across all targets (TUI, Web, Tauri, extension)

### 📜 Two-List Block Architecture (v0.4.4)
- Seamless infinite scrolling through blockchain history
- Automatic backfill placeholders with loading states
- Consistent UX across TUI and Web/Tauri

### ⌨️ Keyboard Shortcuts Overlay (v0.4.5+)
- Press `?` to show comprehensive help modal (Web/Tauri)
- Centralized state management in `App` struct
- Infrastructure ready for TUI help screen (future)

### 🔍 Fullscreen Dual-Mode Navigation (v0.4.3+)
- **Scroll Mode**: Browse massive JSON content
- **Navigate Mode**: Arrow keys navigate rows while viewing JSON
- `Tab` toggles modes, `Space` toggles fullscreen

---

## Quick Start

### Terminal (Native TUI)

```bash
# Development build
cargo run --bin nearx --features native

# Release build
cargo build --release --bin nearx --features native
./target/release/nearx
```

**Configuration**: Copy `.env.example` to `.env` and customize (optional)

**Keyboard shortcuts**: `/` filter • `Tab` switch panes • `Space` fullscreen • `c` copy JSON • `?` help (TUI: see CLAUDE.md)

### Web (WASM + DOM)

```bash
# Dev server (http://localhost:8000)
make dev

# Production build
make web-release

# Serve (Python example)
cd web && python -m http.server 8000
```

**Token Configuration** (optional):
```bash
export FASTNEAR_API_TOKEN_WEB="your-token-here"
make dev
```

### Desktop (Tauri v2)

```bash
cd tauri-workspace

# Development
cargo tauri dev

# Production build
cargo tauri build
```

**Deep Links**: Supports `nearx://` protocol (e.g., `nearx://v1/tx/ABC123`)

---

## Architecture Overview

**Tri-Target Design** with shared Rust core:

```
┌─────────────────────────────────────────────────────────┐
│                    Shared Rust Core                     │
│  • App state (blocks, txs, filters)                     │
│  • RPC polling & WebSocket support                      │
│  • JSON bridge (UiSnapshot ↔ UiAction)                  │
└─────────────────────────────────────────────────────────┘
                          ↓
      ┌───────────────────┼───────────────────┐
      ↓                   ↓                   ↓
┌──────────┐      ┌──────────────┐      ┌──────────┐
│   TUI    │      │   Web/Tauri  │      │ Browser  │
│ (native) │      │ (DOM + WASM) │      │   Ext    │
│ Ratatui  │      │ JSON bridge  │      │  (WIP)   │
└──────────┘      └──────────────┘      └──────────┘
```

**JSON Bridge** (Web/Tauri):
- **Rust → UI**: `UiSnapshot` (serialized state)
- **UI → Rust**: `UiAction` (user commands)
- **Benefits**: No canvas, native DOM, perfect accessibility

---

## Configuration

### Environment Variables

Copy `.env.example` to `.env` and customize:

```bash
# Data source (ws or rpc)
SOURCE=rpc
NEAR_NODE_URL=https://rpc.mainnet.fastnear.com/

# FastNEAR API token (recommended to avoid rate limits)
FASTNEAR_API_TOKEN=your-token-here

# Archival RPC (optional, enables unlimited history)
ARCHIVAL_RPC_URL=https://archival-rpc.mainnet.fastnear.com/

# Performance
RENDER_FPS=30
KEEP_BLOCKS=100
```

### CLI Override

All settings can be overridden via CLI arguments:

```bash
./nearx --source rpc --render-fps 60 --keep-blocks 200
```

**See `.env.example` for all 25+ configuration options.**

---

## Troubleshooting

### Terminal Build Errors

**Error**: `winit not supported on this platform`
- **Fix**: Use `--features native` flag explicitly

**Error**: `zstd-sys` or `secp256k1-sys` errors (Web builds)
- **Fix**: Use `--no-default-features --features dom-web`

### Web Build Errors

**Error**: `wasm-bindgen version mismatch`
- **Fix**: Reinstall CLI: `cargo install wasm-bindgen-cli --locked --force`

**Error**: Connection refused in browser console
- **Fix**: Check `NEAR_NODE_URL` in `.env` or pass via URL parameter

### Runtime Issues

**High CPU usage**:
```bash
RENDER_FPS=20 cargo run --bin nearx --features native
```

**RPC timeouts**:
```bash
RPC_TIMEOUT_MS=15000 POLL_CHUNK_CONCURRENCY=2 cargo run --bin nearx --features native
```

---

## Documentation

- **`CLAUDE.md`** - **Primary source of truth**: Architecture, design decisions, full feature documentation (5000+ lines)
- **`QUICK_START.md`** - Build/test matrix for all targets
- **`COLLABORATION.md`** - Development guidelines and workflow

**For comprehensive details**, see `CLAUDE.md` sections:
- Quad-Mode Architecture
- OAuth & Authentication
- Security (CSP, XSS hardening)
- JSON Bridge Pattern
- Performance Characteristics

---

## Development

### Build Matrix

| Target | Command | Features | Output |
|--------|---------|----------|--------|
| TUI | `cargo build --bin nearx --features native` | Full (SQLite, WebSocket) | Native binary |
| Web | `make web` | DOM + WASM | `web/pkg/` |
| Tauri | `cd tauri-workspace && cargo tauri build` | DOM + native plugins | `.app`/`.exe` |

### Testing

```bash
# Unit tests
cargo test --features native

# E2E tests (Tauri)
cd e2e-tests
npm test

# Check all targets compile
cargo check --bin nearx --features native
cargo check --bin nearx-web-dom --target wasm32-unknown-unknown --features dom-web
cd tauri-workspace && cargo check
```

---

**License**: MIT • Built with Ratatui, Tokio, Tauri, Rust for NEAR Protocol
