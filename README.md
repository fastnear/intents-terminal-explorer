# Ratacat - NEAR Blockchain Transaction Viewer

High-performance terminal UI for monitoring NEAR Protocol transactions in real-time. Runs as native terminal app, web browser app, Tauri desktop app, or browser extension integration.

Built with [Ratatui](https://ratatui.rs) and Rust.

---

## Quick Start

### Native Terminal (Recommended)

```bash
# Clone and build
git clone <repo>
cd ratacat
cargo build --release --features native

# Run (defaults: mainnet RPC, filters to intents.near)
./target/release/nearx

# With authentication (recommended to avoid rate limits)
FASTNEAR_AUTH_TOKEN=your_token ./target/release/nearx

# Monitor different accounts
WATCH_ACCOUNTS=alice.near,bob.near ./target/release/nearx

# Or disable filtering entirely
DEFAULT_FILTER= ./target/release/nearx

# Use testnet
NEAR_NODE_URL=https://rpc.testnet.fastnear.com/ ./target/release/nearx
```

### Web Browser

```bash
# One-time setup
cargo install --locked trunk
rustup target add wasm32-unknown-unknown

# Run locally
trunk serve  # Opens at http://127.0.0.1:8083

# Build for deployment
trunk build --release  # Output in dist-egui/
```

### Tauri Desktop App

```bash
cd tauri-workspace
cargo tauri dev
# or: cargo tauri build    # packages (DMG/EXE/AppImage) with signing if configured
```

### Keyboard & Mouse

See **[docs/KEYMAP.md](docs/KEYMAP.md)** for standardized shortcuts across TUI / Web / Tauri:
- Tab/Shift+Tab, Space, c-copy, scrolling
- Mouse row select (Web/Tauri default ON, TUI Ctrl+M toggle)
- Mouse wheel scrolling (Web/Tauri) - scroll through Blocks/Tx/Details
- Double-click details (Web/Tauri only)

---

## Screenshots

### 3-Pane Dashboard
![Ratacat showing the full 3-pane layout - blocks list, transaction hashes, and detailed transaction view while monitoring intents.near on NEAR mainnet](static/selection.png)

Main interface: blocks on the left, transaction hashes in the middle, full transaction details on the right. Filter bar shows active filtering.

### Fullscreen Details
![Ratacat in fullscreen mode showing detailed JSON transaction data for intents.near on NEAR mainnet](static/full-screen.png)

Press `Spacebar` to toggle fullscreen mode for maximum vertical space to inspect transaction payloads.

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

---

## Keyboard Shortcuts

### Navigation
- `Tab` / `Shift+Tab` - Switch panes
- `↑ / ↓` - Navigate lists or scroll details
- `← / →` - Jump to top / paginate down 12 items
- `PgUp / PgDn` - Page scroll
- `Home` - Return to auto-follow mode (blocks pane) / jump to top (other panes)
- `End` - Jump to bottom
- `Enter` - Select transaction

### View Controls
- `Spacebar` - Toggle fullscreen details
- `Ctrl+O` - Cycle FPS (20 → 30 → 60)
- `c` - Copy current details to clipboard
- `Ctrl+D` - Toggle debug panel
- `q` or `Ctrl+C` - Quit

### Filter & Search
- `/` or `f` - Enter filter mode
- `Ctrl+U` - Toggle owned-only filter
- `Ctrl+F` - Open history search
- `Esc` - Close details overlay (if open), clear filter, or exit mode
- `m` - Set mark at current location
- `Ctrl+P` - Pin/unpin mark
- `M` - Open marks overlay
- `'` - Jump to mark

---

## Configuration

All configuration via environment variables. See `.env.example` for full options.

### Essential Settings

**Defaults (no configuration required):**
- Network: `mainnet` (https://rpc.mainnet.fastnear.com/)
- Source: `rpc` (direct NEAR RPC polling)
- Filter: `intents.near` (to see all transactions, set `DEFAULT_FILTER=`)

**Common customizations:**

```bash
# Authentication (recommended to avoid rate limits)
FASTNEAR_AUTH_TOKEN=your_token_here

# Watch different accounts
WATCH_ACCOUNTS=alice.near,bob.near

# Disable default filtering (see all transactions)
DEFAULT_FILTER=

# Use testnet
NEAR_NODE_URL=https://rpc.testnet.fastnear.com/

# Enable unlimited history navigation
ARCHIVAL_RPC_URL=https://archival-rpc.mainnet.fastnear.com/
```

**Advanced settings** (rarely needed):
```bash
SOURCE=rpc                    # Data source: rpc (default) or ws
RENDER_FPS=30                 # Target FPS (1-120, default: 30)
KEEP_BLOCKS=100               # In-memory block limit (default: 100)
DEFAULT_FILTER=acct:alice.near method:swap  # Advanced filter syntax
```

### Filter Syntax

```
acct:alice.near              # Match signer OR receiver
signer:bob.near              # Match signer only
receiver:contract.near       # Match receiver only
action:FunctionCall          # Match action type
method:ft_transfer           # Match method name
raw:some_text                # Search in raw JSON
```

Combined example: `acct:alice.near action:Transfer` (Alice sending tokens)

Filters use AND logic (all conditions must match). Within each field type, OR logic applies.

---

## Architecture

### Quad-Mode Deployment

```
┌───────────────────────────────────────────────────┐
│              Ratacat Application                  │
├───────────────────────────────────────────────────┤
│                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────┐│
│  │   Terminal   │  │  Web Browser │  │Tauri App ││
│  │ (Crossterm)  │  │ (egui-web)   │  │(Desktop) ││
│  └──────┬───────┘  └──────┬───────┘  └────┬─────┘│
│         │                 │                │      │
│         └─────────────────┼────────────────┘      │
│                           ▼                       │
│              ┌────────────────────────────┐       │
│              │   Shared Core (Rust)       │       │
│              │ • App state & UI rendering │       │
│              │ • RPC client & polling     │       │
│              │ • Filter & search logic    │       │
│              └────────────────────────────┘       │
│                                                   │
│  Platform Abstraction:                            │
│  • Clipboard: copypasta / web-sys / tauri        │
│  • Storage: SQLite / in-memory                   │
│  • Runtime: tokio (full/wasm/tauri)              │
│                                                   │
└───────────────────────────────────────────────────┘
```

**Write once, run everywhere** - Same Rust code compiles to native terminal, WASM for browser, and Tauri for desktop.

### Data Flow

```
┌─────────────────────────────────────────────────┐
│           NEAR Blockchain Data                  │
│  ┌──────────────┐      ┌──────────────────┐     │
│  │  WebSocket   │  OR  │   RPC Polling    │     │
│  │ (Node side)  │      │  (Direct NEAR)   │     │
│  └──────┬───────┘      └────────┬─────────┘     │
│         └───────────┬───────────┘               │
│                     ▼                           │
│            ┌─────────────────┐                  │
│            │  Event Channel  │                  │
│            └────────┬────────┘                  │
│                     ▼                           │
│            ┌─────────────────┐                  │
│            │   App State     │                  │
│            └────────┬────────┘                  │
│                     ▼                           │
│   ┌─────────────────────────────────────────┐   │
│   │        3-Pane TUI Layout                │   │
│   │  ┌──────┐  ┌──────┐  ┌───────────┐      │   │
│   │  │Blocks│→ │Tx IDs│→ │ Details   │      │   │
│   │  │      │  │      │  │(JSON view)│      │   │
│   │  └──────┘  └──────┘  └───────────┘      │   │
│   └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### Key Components

- **`source_ws.rs`**: WebSocket client for Node breakout server
- **`source_rpc.rs`**: NEAR RPC poller with catch-up logic
- **`archival_fetch.rs`**: Background archival RPC fetcher
- **`app.rs`**: State management and event handling
- **`ui.rs`**: Ratatui rendering (70/30 layout split)
- **`filter.rs`**: Query parser and transaction matcher
- **`history.rs`**: SQLite persistence and search

---

## Building from Source

### Main Application

**Native Terminal:**
```bash
cargo build --release --features native
./target/release/nearx
```

**Web (egui + WebGL):**
```bash
# One-time setup
cargo install --locked trunk
rustup target add wasm32-unknown-unknown

# Build for deployment
trunk build --release
# Output: dist-egui/index-egui.html, dist-egui/*.wasm, dist-egui/*.js
```

**Tauri Desktop:**
```bash
cd tauri-workspace
cargo tauri build
```

### Build Verification

```bash
# Native terminal
cargo build --release --features native

# Web browser
trunk build --release

# Tauri desktop
cd tauri-workspace && cargo tauri build
```

---

## Release

One-button release automation that runs all checks, builds all targets, and creates tagged release:

```bash
# Cut version 0.9.0 (runs preflight, clippy, tests, builds Web + Tauri)
tools/release.sh 0.9.0
```

**Artifacts:**
- Webview WASM for Tauri: `dist-egui/nearx-web.js`, `dist-egui/nearx-web_bg.wasm`
- Tauri bundles: `tauri-workspace/src-tauri/target/release/bundle/`

**Notes:**
- Preflight enforces theme discipline + wasm-bindgen parity + loader sanity
- Deep links: `nearx://v1/...` registered via plugin; single-instance forward enabled
- Debugging UI: `?nxdebug=all`, overlay: **Ctrl+Shift+D**

---

## Web Build Technical Details

The web build uses **eframe** (egui's app framework) with **egui_ratatui** to render terminal UI in browser via WebGL.

**Key Configuration:**

`Trunk.toml` is pre-configured:
```toml
[build.rust]
no_default_features = true
features = ["egui-web"]
bin = "nearx-web"
```

`index-egui.html` specifies the binary and disables default features:
```html
<link data-trunk rel="rust" data-bin="nearx-web" data-cargo-no-default-features data-cargo-features="egui-web" />
```

**Why `--no-default-features`:**
- Disables native-only dependencies (crossterm, copypasta, rusqlite)
- Disables NEAR SDK crates with C dependencies (near-primitives, near-crypto)
- Disables full tokio runtime (incompatible with WASM)

**Web Authentication:**

Three methods (priority order):
1. **URL Parameter**: `http://127.0.0.1:8080?token=your_token`
2. **localStorage**: `localStorage.setItem('RPC_BEARER', 'your_token')`
3. **Compile-time**: `FASTNEAR_AUTH_TOKEN=xxx trunk build --release`

**Note:** WASM cannot access runtime environment variables - token must be set when **building** for compile-time method.

---

## Tauri Desktop App

Native desktop application with deep link support for `near://` protocol URLs.

### Development Modes

**For General Development (UI, features, logic):**
```bash
cd tauri-workspace
cargo tauri dev
```
- Fast hot-reload
- Debug logging enabled
- DevTools: `Cmd+Option+I` (macOS) or `F12` (Windows/Linux)
- **Note:** Deep links won't work in dev mode (see below)

**For Deep Link Testing:**

macOS caches URL scheme registrations, so `cargo tauri dev` often runs old code when opened via deep links. Use the helper script instead:

```bash
cd tauri-workspace

# Build debug bundle and register for deep links
./dev-deep-links.sh

# Or build, register, AND test with sample URL
./dev-deep-links.sh test

# Clean up old registrations only
./dev-deep-links.sh clean
```

**What the script does:**
1. Kills old app instances
2. Builds fresh debug bundle with `cargo tauri build --debug`
3. Clears macOS Launch Services cache
4. Registers the new bundle for `near://` URLs
5. Optionally tests with a sample deep link

**Manual Deep Link Testing:**
```bash
# After running dev-deep-links.sh, test with:
open 'near://tx/ABC123?network=mainnet'

# Monitor logs:
tail -f ~/Library/Logs/com.ratacat.fast/Ratacat.log
```

### Key Features
- Deep link handler for `near://tx/HASH?network=mainnet` URLs
- Single-instance enforcement (prevents duplicate launches)
- Native performance with desktop integration
- Comprehensive debug logging waterfall for deep link tracing

### Configuration
- **Bundle ID:** `com.ratacat.fast`
- **URL Scheme:** `near://`
- **Log Location:** `~/Library/Logs/com.ratacat.fast/` (macOS)
- **Dev Bundle:** `target/debug/bundle/macos/Ratacat.app`
- **Release Bundle:** `target/release/bundle/macos/Ratacat.app`

---

## Troubleshooting

### Connection Issues

**"Connection refused" with SOURCE=ws:**
- Ensure Node WebSocket server is running on port 63736
- Check `WS_URL` matches your configuration

**RPC timeouts:**
```bash
RPC_TIMEOUT_MS=15000 POLL_CHUNK_CONCURRENCY=2 cargo run --release
```

### Performance

**High CPU usage:**
```bash
RENDER_FPS=20 KEEP_BLOCKS=50 cargo run --release
```

### Web Build Errors

**`zstd-sys` or `secp256k1-sys` compilation failed:**
- Ensure `Trunk.toml` has `no_default_features = true`
- Verify `index-egui.html` has `data-cargo-no-default-features` attribute

**Connection refused to localhost:**
- Configure RPC endpoint via URL: `?rpc=https://rpc.mainnet.fastnear.com`

### macOS Deep Link Issues

**Problem: Deep links open old version of the app**

This is a common macOS issue where Launch Services caches URL scheme registrations and doesn't update when you rebuild.

**Symptoms:**
- `cargo tauri dev` runs, but deep links open a different (old) instance
- Deep links work but execute old code
- Multiple app icons in dock when opening deep links

**Solution 1: Use the helper script (recommended)**
```bash
cd tauri-workspace
./dev-deep-links.sh test
```

**Solution 2: Manual cleanup**
```bash
# Kill all instances
killall nearx-tauri
killall NEARx

# Find old app locations
mdfind "kMDItemCFBundleIdentifier == 'com.ratacat.fast'"

# Clear Launch Services cache
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -kill -r -domain local -domain system -domain user

# Remove old bundles from /Applications if present
rm -rf /Applications/Ratacat.app

# Build and register fresh debug bundle
cd tauri-workspace
cargo tauri build --debug
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f target/debug/bundle/macos/Ratacat.app

# Wait a few seconds for cache to rebuild
sleep 3

# Test
open 'near://tx/ABC123?network=mainnet'
```

**Why this happens:**
- macOS caches app→URL associations for performance
- `cargo tauri dev` creates temporary builds that aren't stable locations
- Launch Services prefers the first registered app for a URL scheme
- The cache doesn't auto-update when you rebuild

**Best practices:**
- Use `cargo tauri dev` for general development (no deep links needed)
- Use `./dev-deep-links.sh` when working on deep link features
- Run `./dev-deep-links.sh clean` if you see stale registrations
- Production bundles in `/Applications` are most stable for deep links

**Verify current registration:**
```bash
# Show all apps registered for com.ratacat.fast
mdfind "kMDItemCFBundleIdentifier == 'com.ratacat.fast'"

# Check which app will handle near:// URLs
# (Look for "near" in the output)
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -A 3 "near:"
```

---

## Tips

1. **Use WebSocket mode during development** - connects to your Node server for real-time updates
2. **Use RPC mode for production** - direct NEAR connection, more reliable
3. **Press `Spacebar` for fullscreen** - maximize space for complex transaction inspection
4. **Filter with `/`** - syntax like `acct:alice.near action:FunctionCall`
5. **Owned-only view with `Ctrl+U`** - see only your transactions (auto-detected from `~/.near-credentials`)
6. **Bookmark with `m`** - use `Ctrl+P` to pin important marks permanently
7. **Enable archival RPC** - explore unlimited blockchain history with `ARCHIVAL_RPC_URL`
8. **Adjust FPS with `Ctrl+O`** - lower FPS on CPU-constrained systems
9. **Copy with `c`** - paste transaction details anywhere

---

## Built with Official NEAR Infrastructure

Uses official NEAR Protocol crates:
- **`near-primitives`** (0.27.0) - Core blockchain data structures
- **`near-jsonrpc-client`** (0.15.0) - Official RPC client
- **`near-crypto`** (0.27.0) - Cryptographic primitives
- **`near-gas`** (0.2) - Gas formatting utilities
- **`near-token`** (0.2) - NEAR token formatting

This ensures compatibility with NEAR protocol changes and benefits from upstream improvements.

---

Built with [Ratatui](https://ratatui.rs), [Tokio](https://tokio.rs), and Rust. Designed for NEAR Protocol monitoring.

For detailed technical documentation, see `CLAUDE.md` and `COLLABORATION.md`.
