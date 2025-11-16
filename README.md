# NEARx — Quick Start (alpha)

### Run targets
```bash
# Terminal (TUI)
cargo run --bin nearx --features native

# Web (DOM frontend) - builds and serves at http://localhost:8000
make dev

# Tauri (Desktop) - automatically builds frontend
cd tauri-workspace && cargo tauri dev
```

### Build targets
```bash
# TUI release
cargo build --release --features native --bin nearx

# Web release (DOM frontend)
make web-release

# Tauri bundle (automatically builds frontend)
cd tauri-workspace && cargo tauri build
```

### Preflight (alpha)
```bash
./tools/preflight.sh
npm run e2e
```

---

# Ratacat - NEAR Blockchain Transaction Viewer

High-performance terminal UI for monitoring NEAR Protocol transactions in real-time. Runs as native terminal app, web browser app, or Tauri desktop app.

Built with [Ratatui](https://ratatui.rs) and Rust.

---

## Quick Start

**Clone and run:**

```bash
# 1. Clone the repo
git clone <repo-url>
cd ratacat

# 2. Run the desktop app (macOS/Linux/Windows)
cd tauri-workspace
cargo tauri dev
```

That's it! The app will open with live blockchain data from NEAR mainnet.

**Note:** Tauri automatically builds the DOM-based web frontend (`web/`) via Makefile.

**For deep link testing (macOS only):**
```bash
./tauri-dev.sh test
```

---

## Screenshots

### 3-Pane Dashboard
![Ratacat showing the full 3-pane layout - blocks list, transaction hashes, and detailed transaction view while monitoring intents.near on NEAR mainnet](static/selection.png)

Main interface: blocks on the left, transaction hashes in the middle, full transaction details on the right. Filter bar shows active filtering.

### Fullscreen Details
![Ratacat in fullscreen mode showing detailed JSON transaction data for intents.near on NEAR mainnet](static/full-screen.png)

Press `Spacebar` to toggle fullscreen mode for maximum vertical space to inspect transaction payloads.

---

## Keyboard & Mouse

See **[docs/KEYMAP.md](docs/KEYMAP.md)** for complete shortcuts:
- **Tab/Shift+Tab**: Switch panes
- **Spacebar**: Toggle fullscreen details
- **c**: Copy focused content
- **Ctrl+F**: Search history
- **Mouse**: Click to select, scroll to navigate, double-click details for fullscreen

---

## Features

### Core
- **3-Pane Dashboard**: Blocks → Transaction Hashes → Transaction Details
- **Real-time Monitoring**: WebSocket (development) or RPC polling (production)
- **Smart Filtering**: Filter by account, action type, method name, or free text
- **Fullscreen Details**: Toggle with `Spacebar` for maximum inspection area
- **Archival Navigation**: Explore unlimited blockchain history (with `ARCHIVAL_RPC_URL`)

### Data & Search
- **Function Call Decoding**: Three-tier decoding (JSON → Text → Binary) with auto-parsing of nested JSON
- **SQLite History**: Non-blocking persistence for all transactions (native only)
- **Jump Marks**: Bookmark important blocks/transactions (`m` to set, `'` to jump)
- **History Search**: Full-text search with `Ctrl+F`
- **Owned Account Tracking**: Auto-detect your NEAR accounts from credentials, filter with `Ctrl+U`

### Performance
- **FPS Control**: Runtime adjustable (default 30 FPS, toggle with `Ctrl+O`)
- **Smart Caching**: ±12 blocks preserved around selection after aging out
- **Non-blocking I/O**: Async data fetching keeps UI responsive
- **Clipboard Integration**: Copy transaction details with `c`

### Multi-Platform
- **Native Terminal**: Full-featured TUI with SQLite, WebSocket support
- **Web Browser**: Pure DOM UI with WASM core, runs in any modern browser (no canvas/WebGL required)
- **Tauri Desktop**: Native desktop app with DOM frontend, deep link support (`nearx://` URLs)

---

## Configuration

Configuration is loaded with priority: **CLI args > Environment variables > Defaults**

### Quick Configuration

```bash
# Watch specific accounts
WATCH_ACCOUNTS=alice.near,bob.near cargo tauri dev

# Use testnet
NEAR_NODE_URL=https://rpc.testnet.fastnear.com/ cargo tauri dev

# Disable filtering (show all transactions)
DEFAULT_FILTER= cargo tauri dev

# Add authentication (avoid rate limits)
FASTNEAR_AUTH_TOKEN=your_token cargo tauri dev
```

### Configuration File

Copy `.env.example` to `.env` and customize:

```bash
cp .env.example .env
vim .env
```

See `.env.example` for all available options (RPC endpoints, polling intervals, rendering settings, etc.).

---

## Building from Source

### Tauri Desktop (Recommended)

```bash
# Development (automatically builds frontend)
cd tauri-workspace
cargo tauri dev

# Production build (automatically builds frontend)
cd tauri-workspace
cargo tauri build
```

**Note:** Tauri automatically builds the DOM-based web frontend from `web/` using the Makefile.

### Native Terminal

```bash
# Build release
cargo build --release --features native

# Run
./target/release/nearx

# With options
FASTNEAR_AUTH_TOKEN=your_token ./target/release/nearx
```

### Web Browser (DOM Frontend)

```bash
# One-time setup (if not already installed)
cargo install wasm-bindgen-cli --locked
rustup target add wasm32-unknown-unknown

# Development (builds and serves at http://localhost:8000)
make dev

# Production build (output in web/pkg/)
make web-release
```

**Architecture:** Pure DOM-based UI with JSON bridge to WASM core using wasm-bindgen. No canvas or WebGL - just native HTML/CSS/JavaScript for maximum compatibility.

---

## Testing

### Web E2E Smoke Tests (Playwright)

End-to-end tests verify the Web target works without WASM panics, keyboard/mouse input functions correctly, and clipboard copy operates.

**Prerequisites:**
- Node.js/npm installed
- Web target dependencies (`wasm-bindgen-cli`, `wasm32-unknown-unknown` target)

**Setup (one-time):**

```bash
# Install dependencies
npm install

# Install Playwright browsers
npm run e2e:install
```

**Run tests:**

```bash
# Run tests headless (default)
npm run e2e

# Run with visible browser (watch test execution)
npm run e2e:headed

# Interactive debug mode (step through tests)
npm run e2e:debug

# Strict mode: require valid JSON in clipboard (when RPC data is flowing)
NEARX_E2E_REQUIRE_DATA=1 npm run e2e
```

**What it tests:**
- ✅ No WASM runtime errors or panics
- ✅ Canvas renders and is visible
- ✅ Keyboard: Tab/Shift+Tab cycling works
- ✅ Mouse: Click into Blocks/Tx/Details regions
- ✅ Copy: Press 'c' key, verify clipboard readable
- ✅ Optional: Clipboard contains valid JSON (strict mode)

**Port usage:**
- E2E tests run on `http://127.0.0.1:5173` (separate test server)
- Development server runs on `http://localhost:8000` (via `make dev`)
- This separation allows running tests while development server is active

**Test configuration:**
- `playwright.config.ts` - Playwright settings
- `e2e/smoke.spec.ts` - Smoke test suite

---

## Development

### General Development (UI, features, logic)

```bash
cd tauri-workspace
cargo tauri dev
```
- Fast hot-reload
- Debug logging enabled
- DevTools: `Cmd+Option+I` (macOS) or `F12` (Windows/Linux)
- **Note:** Deep links won't work in dev mode (see below)

### For Deep Link Testing (macOS only)

macOS caches URL scheme registrations, so `cargo tauri dev` often runs old code when opened via deep links. Use the helper script instead:

```bash
# Build debug bundle and register for deep links
./tauri-dev.sh

# Or build, register, AND test with sample URL
./tauri-dev.sh test

# Clean up old registrations only
./tauri-dev.sh clean

# Show help
./tauri-dev.sh --help
```

**What the script does:**
1. Kills old app instances
2. Builds fresh debug .app bundle (includes symbols, faster than release)
3. Clears macOS Launch Services cache
4. Copies bundle to /Applications
5. Registers the app from /Applications for `nearx://` URLs
6. Optionally tests with `nearx://v1/tx/ABC123`

**Manual Deep Link Testing:**
```bash
# After running tauri-dev.sh, test with:
open 'nearx://v1/tx/ABC123'

# Monitor logs:
tail -f ~/Library/Logs/com.fastnear.nearx/NEARx.log
```

### Key Features
- Deep link handler for `nearx://v1/tx/HASH` URLs
- Single-instance enforcement (prevents duplicate launches)
- Native performance with desktop integration
- Comprehensive debug logging waterfall for deep link tracing

### Configuration

Set environment variables or copy `.env.example` to `.env`:

```bash
cp .env.example .env
```

Key settings:
- `NEAR_NODE_URL`: RPC endpoint (default: `https://rpc.mainnet.fastnear.com/`)
- `FASTNEAR_AUTH_TOKEN`: Authentication token to avoid rate limits
- `WATCH_ACCOUNTS`: Comma-separated account list (default: `intents.near`)
- `ARCHIVAL_RPC_URL`: Archival RPC for unlimited history navigation

---

## Architecture

**Quad-Mode Design**: Write once, run everywhere
- **Native Terminal**: Crossterm backend with SQLite persistence
- **Web Browser**: Pure DOM UI with JSON bridge to WASM core (headless App pattern)
- **Tauri Desktop**: Same DOM frontend with native desktop integration
- **Shared Core**: Same `App` state engine across all targets

**DOM Frontend Architecture** (Web + Tauri):
- **Rust (WASM)**: `WasmApp` exposes headless `App` via JSON snapshots (`UiSnapshot`) and actions (`UiAction`)
- **JavaScript**: DOM renderer consumes snapshots, dispatches user actions
- **Data Flow**: RPC events → App state → JSON snapshot → DOM render → User action → App update
- **Native UX**: Pure HTML/CSS/JavaScript, no canvas or WebGL

**Key Technologies**:
- [Ratatui](https://ratatui.rs) - Terminal UI framework
- [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) - Rust/JavaScript bridge (direct, no bundler)
- [Tauri v2](https://tauri.app) - Desktop app framework
- Makefile + Python HTTP server - Simple, standard build tooling

---

## Project Structure

```
ratacat/
├── src/
│   ├── bin/
│   │   ├── nearx.rs          # Native terminal binary
│   │   ├── nearx-web-dom.rs  # DOM frontend binary (WASM, JSON bridge)
│   │   └── ratacat-proxy.rs  # RPC proxy (development)
│   ├── app.rs                # Application state (shared)
│   ├── ui.rs                 # Ratatui rendering (terminal only)
│   ├── theme.rs              # Unified theme system (runtime CSS vars)
│   └── ...                   # Other shared modules
├── tauri-workspace/
│   └── src-tauri/            # Tauri desktop app
├── web/
│   ├── index.html            # DOM frontend entry point
│   ├── app.js                # DOM renderer (snapshot → render → action)
│   ├── theme.css             # Theme variables (injected by theme.rs)
│   ├── platform.js           # Unified clipboard bridge
│   ├── auth.js               # OAuth popup manager
│   ├── router_shim.js        # Hash router for auth callbacks
│   └── pkg/                  # WASM build output (gitignored)
├── Makefile                  # Web build automation
├── tauri-dev.sh              # Deep link testing helper (macOS)
└── .env.example              # Configuration template
```

---

## Contributing

See **[COLLABORATION.md](COLLABORATION.md)** for detailed development guidelines.

Key points:
- All UI changes should work across all targets (native, web, Tauri)
- Test with `cargo check --all-targets --all-features`
- Run formatters: `cargo fmt` and `cargo clippy`
- For deep link changes, test with `./tauri-dev.sh test`

---

## Documentation

- **[CLAUDE.md](CLAUDE.md)** - Comprehensive technical documentation
- **[COLLABORATION.md](COLLABORATION.md)** - Development guidelines
- **[docs/KEYMAP.md](docs/KEYMAP.md)** - Keyboard and mouse shortcuts
- **[.env.example](.env.example)** - Configuration options

---

## License

[Your License Here]

---

Built with ❤️ using Ratatui, Tokio, and Rust. Designed for NEAR Protocol monitoring.
