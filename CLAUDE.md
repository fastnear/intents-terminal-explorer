# Ratacat - NEAR Blockchain Transaction Viewer

**Version 0.4.0** - High-performance **quad-mode** application for monitoring NEAR Protocol blockchain transactions. Runs in terminal (native), web browser (WASM), desktop app (Tauri), AND integrates with browsers via 1Password-style extension!

**🆕 October 2025 Update**: Production-ready browser integration with auto-installing Native Messaging host supporting Chrome, Edge, Chromium, and **Firefox**.

## Quad-Mode Architecture Overview

Ratacat v0.4.0 features a revolutionary **quad-deployment architecture** - write once, run everywhere:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Ratacat Quad-Mode Architecture                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────────────┐ │
│  │  Terminal  │  │  Browser   │  │   Tauri    │  │  Browser Ext +   │ │
│  │  (Native)  │  │   (WASM)   │  │  Desktop   │  │  Native Host     │ │
│  │            │  │            │  │            │  │                  │ │
│  │ • Crossterm│  │ • egui     │  │ • Deep     │  │ • MV3 Extension  │ │
│  │ • SQLite   │  │ • In-mem   │  │   links    │  │ • stdio bridge   │ │
│  │ • WS + RPC │  │ • RPC only │  │ • Single   │  │ • myapp://       │ │
│  └─────┬──────┘  └─────┬──────┘  │   instance │  │   deep links     │ │
│        │               │         └──────┬─────┘  └────────┬─────────┘ │
│        └───────────────┼────────────────┼──────────────────┘           │
│                        ▼                ▼                               │
│              ┌─────────────────────────────────────┐                    │
│              │      Shared Rust Core               │                    │
│              │  • App state (height-based blocks)  │                    │
│              │  • UI rendering (ratatui)           │                    │
│              │  • RPC client & polling             │                    │
│              │  • Filter & search (SQLite/memory)  │                    │
│              │  • Archival fetcher (native-only)   │                    │
│              └──────────┬──────────────────────────┘                    │
│                         ▼                                               │
│              ┌─────────────────────────────────────┐                    │
│              │    Platform Abstraction             │                    │
│              │  • Clipboard (unified 4-tier)       │                    │
│              │    - Tauri plugin / Extension relay │                    │
│              │    - Navigator API / execCommand    │                    │
│              │  • Storage (SQLite/in-memory)       │                    │
│              │  • Runtime (tokio full/wasm)        │                    │
│              └─────────────────────────────────────┘                    │
│                                                                         │
│              NEAR Blockchain + Browser Integration                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────────────┐    │
│  │WebSocket │  │   RPC    │  │ Archival │  │ Browser→Native→App │    │
│  │ (Native) │  │  (All)   │  │(Optional)│  │   Deep Link Flow   │    │
│  └──────────┘  └──────────┘  └──────────┘  └────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

### Deployment Modes

1. **Native Terminal**: Full-featured TUI with SQLite, WebSocket, file watching
2. **Web Browser (WASM)**: Same UI in browser, RPC-only, in-memory storage
3. **Tauri Desktop**: Native desktop app with deep link support (`myapp://` protocol)
4. **Browser Extension**: 1Password-style "Open in Ratacat" button on tx pages

## Key Design Principles

1. **FPS-Capped Rendering** - Coalesced draws (default 30 FPS) prevent UI thrashing
2. **Non-Blocking I/O** - All data fetching and persistence happens off the main render thread
3. **Catch-Up Limits** - RPC mode limits blocks per poll to prevent cascade failures
4. **Async Everything** - Tokio-based async runtime keeps UI responsive
5. **Soft-Wrapped Tokens** - ZWSP insertion for clean line breaking of long hashes

## Core Components

### Data Sources (`src/source_*.rs`)

**WebSocket Mode** (`source_ws.rs`):
- Connects to Node breakout server on port 63736
- Real-time block and transaction events
- Ideal for development alongside your Node server

**RPC Mode** (`source_rpc.rs`):
- Direct NEAR RPC polling with smart catch-up
- Non-overlapping polls with configurable limits
- Concurrent chunk fetching (default 4 parallel requests)
- Ideal for production monitoring

**Archival Fetch** (`archival_fetch.rs`):
- Background task for fetching historical blocks beyond the rolling buffer
- On-demand fetching via channel communication
- Reuses existing `fetch_block_with_txs()` RPC infrastructure
- Enables unlimited backward navigation through blockchain history
- Optional: only runs if `ARCHIVAL_RPC_URL` is configured

### Application State (`src/app.rs`)

```rust
pub struct App {
    // Core state - height-based block tracking for stable selection
    blocks: Vec<BlockRow>,
    sel_block_height: Option<u64>,  // None = auto-follow newest, Some(height) = locked
    sel_tx: usize,
    manual_block_nav: bool,         // Whether user navigated away from newest
    details: String,

    // Block caching (preserves ±12 blocks around selection after aging out)
    cached_blocks: HashMap<u64, BlockRow>,
    cached_block_order: Vec<u64>,   // LRU tracking

    // Archival fetch state (for fetching historical blocks beyond cache)
    loading_block: Option<u64>,                                   // Block height currently being fetched
    archival_fetch_tx: Option<UnboundedSender<u64>>,              // Channel to request archival fetches

    // Filter state
    filter_query: String,
    filter_compiled: CompiledFilter,

    // Search state
    search_query: String,
    search_results: Vec<HistoryHit>,
    search_selection: usize,

    input_mode: InputMode,  // Normal | Filter | Search | Marks | JumpPending

    // Jump marks for navigation
    marks_list: Vec<Mark>,
    marks_selection: usize,

    // Owned accounts filtering (from ~/.near-credentials)
    owned_accounts: HashSet<String>,
    owned_only_filter: bool,

    // Performance
    fps: u32,
    fps_choices: Vec<u32>,

    // Debug panel (Ctrl+D)
    debug_log: Vec<String>,
    debug_visible: bool,
}
```

### UI Rendering (`src/ui.rs`)

Two-row layout optimized for readability:
- **Header**: Tab-style pane selector with focus indicators
- **Filter Bar**: Dynamic height (collapses to 1 line when idle, expands to 3 when active)
- **Body**:
  - Top row (30% height): Blocks (left 50%) + Transaction hashes (right 50%)
  - Bottom row (70% height): Details pane (full width, **no left border for easy mouse-based copying**)
- **Debug Panel**: Toggleable with Ctrl+D (shows navigation events)
- **Footer**: Keyboard shortcuts + FPS indicator + owned filter status + pinned marks count + toast notifications
- **Search Overlay**: Centered modal for history search (Ctrl+F)
- **Marks Overlay**: Navigation bookmarks list (Shift+M)

**Layout Ratios**: Uses `Constraint::Ratio(3,10)` and `Ratio(7,10)` for precise 30/70 split, eliminating rounding gaps from percentage-based constraints. This gives the details pane more vertical space since it's the most information-dense area.

### Filter System (`src/filter.rs`)

Query grammar for real-time transaction filtering:
```
acct:alice.near       # Match signer OR receiver
signer:bob.near       # Match signer only
receiver:contract     # Match receiver only
action:FunctionCall   # Match action type
method:ft_transfer    # Match method name
raw:some_text         # Search in raw JSON
freetext              # Match anywhere
```

All filters use AND logic; within each field type, OR logic applies.

### History Search (`src/history.rs`)

Off-thread SQLite persistence with async search:
```rust
pub struct History {
    tx: UnboundedSender<HistoryMsg>,  // Async channel
}

// Query grammar (same as filter + height ranges)
signer:alice.near from:100000 to:200000 method:transfer
```

Features:
- Non-blocking writes via `spawn_blocking`
- WAL mode for concurrent reads
- Indexed on signer, receiver, height
- Query builder with LIKE-based search
- Prepared for FTS5 upgrade

Search results include:
- Transaction hash, height, timestamp
- Signer → receiver
- Method summary (extracted from actions_json)

## Configuration

Configuration is loaded with the following priority: **CLI args > Environment variables > Defaults**

### Quick Start

```bash
# Copy example configuration
cp .env.example .env

# Edit .env with your settings
vim .env

# Run with default settings
cargo run --bin ratacat --features native

# Or override with CLI arguments
cargo run --bin ratacat --features native -- --source rpc --render-fps 60
```

### Configuration Methods

**1. Environment Variables (recommended for persistent settings)**
```bash
# Set in .env file (see .env.example for all options)
SOURCE=rpc
NEAR_NODE_URL=https://rpc.mainnet.fastnear.com/
RENDER_FPS=30
```

**2. Command Line Arguments (recommended for temporary overrides)**
```bash
# View all available options
./ratacat --help

# Override specific settings
./ratacat --source rpc --render-fps 60 --keep-blocks 200

# Short form for source
./ratacat -s rpc
```

### Key Configuration Options

#### Data Source
- `SOURCE` / `--source, -s`: Data source (`ws` or `rpc`)
  - `ws`: WebSocket connection to Node server (real-time, low latency)
  - `rpc`: Direct NEAR RPC polling (more reliable, works without Node)
  - Default: `ws`

#### WebSocket Settings (when `SOURCE=ws`)
- `WS_URL` / `--ws-url`: WebSocket endpoint
  - Default: `ws://127.0.0.1:63736`
- `WS_FETCH_BLOCKS` / `--ws-fetch-blocks`: Fetch full block data
  - Default: `true`

#### RPC Settings (when `SOURCE=rpc`)
- `NEAR_NODE_URL` / `--near-node-url`: NEAR RPC endpoint
  - Default: `https://rpc.testnet.fastnear.com/`
  - Examples: mainnet, testnet, or custom endpoints
- `FASTNEAR_AUTH_TOKEN` / `--fastnear-auth-token`: FastNEAR API token
  - Recommended to avoid rate limits
  - Get free token at: https://fastnear.com
- `POLL_INTERVAL_MS` / `--poll-interval-ms`: Polling interval (100-10000ms)
  - Default: `1000` (1 second)
- `POLL_MAX_CATCHUP` / `--poll-max-catchup`: Max blocks per poll (1-100)
  - Default: `5`
- `POLL_CHUNK_CONCURRENCY` / `--poll-chunk-concurrency`: Concurrent fetches (1-16)
  - Default: `4`
- `RPC_TIMEOUT_MS` / `--rpc-timeout-ms`: Request timeout (1000-60000ms)
  - Default: `8000` (8 seconds)
- `RPC_RETRIES` / `--rpc-retries`: Retry attempts (0-10)
  - Default: `2`

#### Archival RPC (for historical block fetching)
- `ARCHIVAL_RPC_URL` / `--archival-rpc-url`: Archival RPC endpoint
  - Optional: enables unlimited backward navigation through blockchain history
  - Fetches historical blocks on-demand when navigating beyond cache
  - Requires `FASTNEAR_AUTH_TOKEN` for best performance
  - Examples:
    - FastNEAR Mainnet: `https://archival-rpc.mainnet.fastnear.com`
    - FastNEAR Testnet: `https://archival-rpc.testnet.fastnear.com`
  - Loading state shows "⏳ Loading block #..." during 1-2 second fetch
  - Fetched blocks are cached automatically for seamless navigation

#### UI Performance
- `RENDER_FPS` / `--render-fps`: Target FPS (1-120)
  - Default: `30`
  - Lower = less CPU, Higher = smoother updates
- `RENDER_FPS_CHOICES` / `--render-fps-choices`: Available FPS options (comma-separated)
  - Default: `20,30,60`
  - Cycle with Ctrl+O during runtime
- `KEEP_BLOCKS` / `--keep-blocks`: Blocks in memory (10-10000)
  - Default: `100`

#### Persistence
- `SQLITE_DB_PATH` / `--sqlite-db-path`: Database path
  - Default: `./ratacat_history.db`

#### Credentials (for owned account filtering)
- `NEAR_CREDENTIALS_DIR`: Credentials directory
  - Default: `$HOME/.near-credentials`
- `NEAR_NETWORK`: Network subdirectory
  - Default: `mainnet`
  - Options: `mainnet`, `testnet`, `betanet`

#### Default Filtering
- `WATCH_ACCOUNTS` / `--watch-accounts`: Comma-separated account list (simple filtering)
  - Default: `intents.near`
  - Example: `alice.near,bob.near,contract.near`
  - Automatically builds `acct:` filter for each account
  - Takes precedence over `DEFAULT_FILTER`
- `DEFAULT_FILTER` / `--default-filter`: Advanced filter syntax (power users)
  - Only used if `WATCH_ACCOUNTS` is not set
  - Default: `acct:intents.near`
  - Supports full filter grammar: `signer:`, `receiver:`, `action:`, `method:`, `raw:`

### Configuration Validation

All configuration values are validated on startup with helpful error messages:

```bash
# Invalid FPS range
$ ./ratacat --render-fps 200
Error: RENDER_FPS must be in range [1, 120], got 200

# Invalid URL scheme
$ ./ratacat --near-node-url example.com
Error: NEAR_NODE_URL must start with ws://, wss://, http://, or https://

# Invalid poll interval
$ POLL_INTERVAL_MS=50000 ./ratacat
Error: POLL_INTERVAL_MS must be in range [100, 10000], got 50000
```

### Common Configuration Examples

**Development with local Node server:**
```bash
SOURCE=ws cargo run --bin ratacat --features native
```

**Production mainnet monitoring:**
```bash
./ratacat \
  --source rpc \
  --near-node-url https://rpc.mainnet.fastnear.com/ \
  --fastnear-auth-token your_token_here \
  --keep-blocks 200
```

**Low-resource environment (e.g., Raspberry Pi):**
```bash
./ratacat \
  --source rpc \
  --render-fps 10 \
  --keep-blocks 50 \
  --poll-interval-ms 2000 \
  --poll-chunk-concurrency 2
```

**High-performance local monitoring:**
```bash
SOURCE=ws RENDER_FPS=60 KEEP_BLOCKS=500 cargo run --bin ratacat --features native
```

For complete documentation of all options, see `.env.example`.

## Keyboard Controls

### Navigation
- `Tab` / `Shift+Tab` - Switch panes (circular: Blocks → Txs → Details → Blocks)
- `↑ / ↓` - Navigate lists or scroll details (pane-specific)
- `← / →` - Jump to top / Paginate down 12 items
- `PgUp / PgDn` - Page scroll (20 lines)
- `Home` - In blocks pane: return to auto-follow mode; Other panes: jump to top
- `End` - Jump to bottom
- `Enter` - Select transaction

### Filtering & Search
- `/` or `f` - Enter filter mode (real-time filtering)
- `Ctrl+F` - Open history search (SQLite-backed)
- `Esc` - Clear filter/search or exit mode
- `Ctrl+U` - Toggle owned-only filter (shows only txs from your accounts)

### Bookmarks (Jump Marks)
- `m` - Set mark at current position (auto-labeled)
- `Ctrl+P` - Pin/unpin current position (persistent across sessions)
- `Shift+M` - Open marks overlay (list all marks)
- `'` (apostrophe) - Quick jump (type label character)
- `[` / `]` - Jump to previous/next mark
- `d` - Delete mark (when in marks overlay)

### Performance & Debug
- `Ctrl+O` - Cycle FPS (toggles through configured choices, e.g., 20 → 30 → 60)
- `Ctrl+D` - Toggle debug panel (shows selection events)
- `c` - Copy details to clipboard (shows toast notification with pane-specific message)
- `q` or `Ctrl+C` - Quit

### Text Selection & Copying

**Terminal Version (Native)**:
Ratacat enables mouse capture for pane navigation. To select text from the terminal:

- **macOS iTerm2**: Hold `Option/Alt` while clicking and dragging
- **macOS Terminal.app**: Hold `Fn` while selecting
- **Linux**: Hold `Shift` while clicking and dragging (GNOME Terminal, Alacritty, xterm, etc.)
- **Windows**: Hold `Shift` while selecting (Windows Terminal, ConEmu)

**Tips**:
- Double-click with modifier key to select entire words (useful for transaction hashes, account names)
- Triple-click with modifier to select entire lines
- The details pane has no left border specifically to make mouse selection easier

**Web Version**:
Text selection works natively in the browser. Simply click and drag to select - no modifier keys needed.

**Copy Shortcuts**:
Press `c` to copy pane-specific content to clipboard:
- **Blocks pane**: All transactions in selected block (structured format with metadata)
- **Transactions pane**: Human-readable view + raw JSON payload
- **Details pane**: Full JSON content (what you see in the pane)

## Building & Running

### Native Terminal Mode

**Font Rendering Note**: The native terminal version uses your terminal emulator's font settings. Ratacat does not control font rendering - this is managed by your terminal emulator (iTerm2, Alacritty, Terminal.app, etc.).

**Recommended Monospace Fonts**:
- **JetBrains Mono** - Excellent Unicode coverage, designed for code
- **Cascadia Code** - Microsoft's modern terminal font with ligatures
- **Fira Code** - Popular with programmers, good ligature support
- **SF Mono** (macOS) - Apple's system monospace font
- **Menlo** (macOS) - Classic Mac terminal font

**Terminal Emulator Configuration Examples**:

<details>
<summary>Alacritty (YAML config)</summary>

```yaml
# ~/.config/alacritty/alacritty.yml
font:
  normal:
    family: "JetBrains Mono"
    style: Regular
  bold:
    family: "JetBrains Mono"
    style: Bold
  italic:
    family: "JetBrains Mono"
    style: Italic
  size: 14.0

  # Optional: adjust spacing for better readability
  offset:
    x: 0
    y: 0
  glyph_offset:
    x: 0
    y: 0
```
</details>

<details>
<summary>iTerm2 (macOS GUI settings)</summary>

1. Open **iTerm2 → Preferences → Profiles → Text**
2. Click **Change Font** button
3. Select font family (e.g., "JetBrains Mono") and size (14pt recommended)
4. Enable **Anti-aliased** and **Use thin strokes** for crisp rendering
</details>

```bash
# Build release version (requires native feature)
cargo build --bin ratacat --features native --release

# Run with default settings (WebSocket mode)
cargo run --bin ratacat --features native

# Run in RPC mode
cargo run --bin ratacat --features native -- --source rpc

# Run with custom settings
cargo run --bin ratacat --features native -- --source rpc --render-fps 60 --keep-blocks 200

# Or use environment variables
SOURCE=rpc RENDER_FPS=60 cargo run --bin ratacat --features native

# View all CLI options
cargo run --bin ratacat --features native -- --help

# Run release binary directly
./target/release/ratacat --source rpc --near-node-url https://rpc.mainnet.fastnear.com/
```

### Web Browser Mode

**Technology Stack**: Uses **eframe** (egui's app framework) with **egui_ratatui** to render terminal UI in browser via WebGL.

**Prerequisites:**
```bash
# Install Trunk (WASM build tool)
cargo install --locked trunk

# Add WASM target
rustup target add wasm32-unknown-unknown
```

**Build Commands:**
```bash
# Development server (auto-reload on changes) - uses egui
trunk serve
# Opens at http://127.0.0.1:8080

# Production build
trunk build --release
# Output: dist/index.html, dist/*.wasm, dist/*.js
```

**Critical Build Details:**
- `--no-default-features` - **REQUIRED** - Configured in `Trunk.toml` to prevent NEAR SDK crates (C dependencies)
- `--features egui-web` - Enables eframe, egui_ratatui, soft_ratatui, wasm-bindgen, web-sys
- Binary: `ratacat-egui-web` (specified in `Trunk.toml`)
- HTML: `index-egui.html` (specifies `data-bin="ratacat-egui-web"`)

**Common Build Errors & Solutions:**

1. **Error: `zstd-sys` compilation failed**
   - **Cause:** Default features enabled (pulls in NEAR SDK)
   - **Fix:** Add `--no-default-features` flag

2. **Error: `mio` not supported on wasm32**
   - **Cause:** Tokio's `net` feature enabled
   - **Fix:** Already handled by target-specific tokio config

3. **Error: Entry symbol `main` declared multiple times**
   - **Cause:** WASM binaries need `#![no_main]` attribute
   - **Fix:** Already in `src/bin/ratacat-egui-web.rs`:
     ```rust
     #![cfg_attr(target_arch = "wasm32", no_main)]
     ```

4. **Error: Multiple target artifacts found**
   - **Cause:** Trunk doesn't know which binary to build
   - **Fix:** Already in `index-egui.html`:
     ```html
     <link data-trunk rel="rust" data-bin="ratacat-egui-web" />
     ```

**Verifying the Build:**
```bash
# Check that no NEAR crates are in WASM dependency tree
cargo tree --target wasm32-unknown-unknown --no-default-features --features egui-web | grep near-

# Should return empty (no near-* crates)
```

### Ratatui Version Requirements

**Web builds require ratatui 0.29+** for egui_ratatui compatibility:
- egui_ratatui 2.0 depends on ratatui ^0.29
- Native builds work with any version, but 0.29 used for consistency

**Breaking Changes in 0.29 Upgrade:**
- `Frame::size()` → deprecated, use `Frame::area()`
- `Frame::set_cursor()` → deprecated, use `Frame::set_cursor_position()`

These deprecation warnings are safe to ignore (fixes planned for future release).

### Tauri Desktop App Mode

**Overview**: Native desktop application with deep link support for handling `near://` URLs. Built with Tauri v2, combining Rust backend with web frontend.

**Key Features**:
- Deep link handler for `near://` protocol (e.g., `near://tx/ABC123?network=mainnet`)
- Single-instance enforcement (prevents duplicate app launches)
- Comprehensive debug logging waterfall
- DevTools integration (keyboard shortcuts + UI controls)
- Native host sidecar support for browser extension integration

#### Deep Link Architecture

Ratacat implements an **8-point color-coded debug logging waterfall** to trace deep link URLs through the system:

```
🔴 SINGLE-INSTANCE → 🟠 GET-CURRENT → 🟡 ON-OPEN-URL → 🟢 HANDLE-URLS
    → 🔵 NORMALIZE → 🟣 PARSE-EVENT → 🟤 EMIT-OR-QUEUE → ⚪ FRONTEND-INIT → ⚫ ROUTE-EVENT
```

**Flow Explanation**:

1. **🔴 SINGLE-INSTANCE**: Tauri plugin intercepts new launches, captures `argv` on Windows/Linux
2. **🟠 GET-CURRENT**: Retrieves initial deep links from Tauri on first run (macOS primary method)
3. **🟡 ON-OPEN-URL**: macOS system callback when URL opens while app already running
4. **🟢 HANDLE-URLS**: Central processing function, receives raw URL strings
5. **🔵 NORMALIZE**: Cleans URLs (trim, lowercase scheme, strip trailing slashes)
6. **🟣 PARSE-EVENT**: Extracts host/path/query into `DeepLinkEvent` struct
7. **🟤 EMIT-OR-QUEUE**: Emits to frontend if ready, queues if still initializing (prevents race conditions)
8. **⚪ FRONTEND-INIT**: Frontend calls `get_queued_urls()` after DOM ready
9. **⚫ ROUTE-EVENT**: JavaScript routes event to appropriate UI handler

**Example Output**:
```
🟢 [HANDLE-URLS] Processing 1 URL(s)
🟢 [HANDLE-URLS] URL[0]: "near://tx/ABC123?network=mainnet"
🔵 [NORMALIZE] Input raw: "near://tx/ABC123?network=mainnet"
🔵 [NORMALIZE] After scheme normalization: "near://tx/ABC123?network=mainnet"
🟣 [PARSE-EVENT] ✅ Created DeepLinkEvent:
🟣 [PARSE-EVENT]    host: "tx"
🟣 [PARSE-EVENT]    path: ["ABC123"]
🟣 [PARSE-EVENT]    query: {"network": "mainnet"}
🟤 [EMIT-OR-QUEUE] Frontend ready - emitting to window
⚫ [ROUTE-EVENT] Received event: {"host":"tx","path":["ABC123"],"query":{"network":"mainnet"}}
```

#### Configuration

**Bundle Identifier**: `com.ratacat.fast`
- **Note**: Bundle identifiers ending in `.app` are reserved by Apple
- Configured in `tauri-workspace/src-tauri/tauri.conf.json`

**Deep Link Scheme**: `near://`
- Registered via `CFBundleURLTypes` in `Info.plist` (auto-generated by Tauri)
- Configured in `tauri.conf.json`:
  ```json
  "plugins": {
    "deep-link": {
      "desktop": {
        "schemes": ["near"]
      }
    }
  }
  ```

**Logging**:
- **Development**: `tauri-plugin-log` forwards Rust logs to browser DevTools console
- **Production**: Logs written to `~/Library/Logs/com.ratacat.fast/Ratacat.log` (macOS)
- Both: `env_logger` outputs to stdout/stderr

**Clipboard**:
- **Plugin**: `tauri-plugin-clipboard-manager` (v2.3+) - Official Tauri clipboard plugin
- **JavaScript Bridge**: `web/platform.js` calls `__TAURI__.invoke("copy_text", { text })` as first fallback
- **Command**: `copy_text` command in `lib.rs` using `ClipboardExt` trait
- **Graceful Degradation**: Falls back to Navigator API → execCommand if plugin unavailable

#### Build Process

**Standard Build**:
```bash
cd tauri-workspace
cargo tauri build
```

**Known Issue**: Tauri bundler bug tries to copy `.rs` source files instead of binaries.

**Manual Workaround**:
```bash
# 1. Build the release binary
cargo build --release --manifest-path src-tauri/Cargo.toml

# 2. Create bundle structure
mkdir -p target/release/bundle/macos/Ratacat.app/Contents/MacOS

# 3. Copy binary manually
cp target/release/explorer-tauri target/release/bundle/macos/Ratacat.app/Contents/MacOS/

# 4. Continue with bundle finalization
cargo tauri build  # Will use existing binary
```

**Important**: Ensure no extra binaries in `src/bin/` directory that aren't listed in `Cargo.toml`. Move unused binaries to `.bak` extension if needed.

#### Development Mode

```bash
cd tauri-workspace
cargo tauri dev
```

**DevTools Access** (4 methods):
1. **Keyboard**: `Cmd+Option+I` (macOS) or `F12` (Windows/Linux)
2. **UI Button**: Click "Toggle DevTools" button in app (requires `devtools` feature)
3. **Rust Commands**: `open_devtools()` / `close_devtools()` (requires `devtools` feature)
4. **Auto-open**: Automatically opens in debug builds

**Note**: The `devtools` Cargo feature is enabled in `tauri-workspace/src-tauri/Cargo.toml` for early development. The comprehensive debug logging waterfall provides detailed visibility into deep link processing without needing browser DevTools.

#### Testing Deep Links

**Test from command line**:
```bash
# Open the app with a deep link
open 'near://tx/ABC123?network=mainnet'

# Or with multiple paths
open 'near://account/alice.near/history?from=100'
```

**Verify in logs**:
```bash
# Watch live logs (development)
# Check browser DevTools console

# View production logs (macOS)
tail -f ~/Library/Logs/com.ratacat.fast/Ratacat.log
```

**Expected Behavior**:
1. App launches if not running (single-instance prevents duplicates)
2. Deep link received via `get_current()` (first launch) or `on_open_url()` (already running)
3. Full debug waterfall appears in logs
4. Frontend receives parsed event with host, path, query
5. UI updates to show transaction/account details

#### Registering Deep Links with macOS

**Fresh Registration** (after moving app to /Applications):
```bash
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/Ratacat.app
```

**Verify Registration**:
```bash
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -A 3 "near:"
```

**Reset Deep Link Association** (if pointing to old app):
1. Kill all instances: `killall explorer-tauri`
2. Remove old app: `rm -rf /Applications/Ratacat.app`
3. Copy fresh build: `cp -r target/release/bundle/macos/Ratacat.app /Applications/`
4. Re-register: Run `lsregister -f` command above

#### File Structure

```
tauri-workspace/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs           # Core logic with 8-point debug waterfall
│   │   └── main.rs          # Entry point (minimal, calls lib.rs)
│   ├── Cargo.toml           # Dependencies + binary config
│   ├── tauri.conf.json      # Tauri configuration
│   └── build.rs             # Tauri build script
├── assets/
│   └── index.html           # Frontend with deep link handler
└── target/release/bundle/
    └── macos/
        └── Ratacat.app/
            └── Contents/
                ├── Info.plist       # Auto-generated, includes CFBundleURLTypes
                └── MacOS/
                    └── explorer-tauri  # Binary executable
```

#### Deep Link Event Structure

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeepLinkEvent {
    pub host: String,                    // e.g., "tx", "account"
    pub path: Vec<String>,               // e.g., ["ABC123"], ["alice.near", "history"]
    pub query: HashMap<String, String>,  // e.g., {"network": "mainnet"}
}
```

**Example Parsing**:
- Input: `near://tx/ABC123?network=mainnet`
- Output:
  ```json
  {
    "host": "tx",
    "path": ["ABC123"],
    "query": {"network": "mainnet"}
  }
  ```

#### Known Issues & Workarounds

**Issue 1**: Tauri bundler tries to copy `.rs` files instead of binaries
- **Error**: `Failed to copy binary from "target/release/ratacat-egui-tauri.rs"`
- **Workaround**: Manual binary copy (see Build Process above)
- **Prevention**: Keep `src/bin/` clean, move unused binaries to `.bak`

**Issue 2**: Bundle identifier restrictions
- **Error**: `Bundle identifier cannot end with .app (reserved by Apple)`
- **Solution**: Use `com.ratacat.fast` instead of `com.ratacat.app`

**Issue 3**: Old app captures deep links
- **Symptom**: Deep links open wrong app version
- **Solution**: Kill all instances, remove old app, re-register fresh build

**Issue 4**: No logs appearing in DevTools
- **Cause**: Logger not initialized or plugin missing
- **Solution**: Ensure both `env_logger` and `tauri-plugin-log` in dependencies

#### Integration with Browser Extension

The Tauri app includes a sidecar spawn utility for the native messaging host:

```rust
// Automatically spawns native-host binary when needed
// Located at: Contents/Resources/ratacat-native-host
// Configured in tauri.conf.json bundle.resources
```

This enables the browser extension to send `near://` deep links to the desktop app via native messaging, creating a seamless "Open in Ratacat" experience from transaction pages.

#### Production Deployment

**macOS Considerations**:
1. **Code Signing**: Required for distribution outside App Store
2. **Notarization**: Required for Gatekeeper approval
3. **Universal Binary**: Build for both Intel and Apple Silicon
4. **Auto-updater**: Tauri supports built-in update mechanism

**Future Enhancements**:
- Windows/Linux deep link testing
- Auto-updater integration
- Code signing automation
- DMG installer with drag-to-Applications

## Project Structure

```
ratacat/
├── Cargo.toml           # Dependencies with feature flags (native/web)
├── index-egui.html      # Web app entry point (egui + ratatui)
├── Trunk.toml           # Web build configuration
├── web/
│   └── platform.js      # Unified clipboard bridge (Tauri/Extension/Navigator/execCommand)
├── src/
│   ├── lib.rs           # Library exports (shared core)
│   ├── bin/
│   │   ├── ratacat.rs   # Native terminal binary
│   │   ├── ratacat-egui-web.rs # Web browser binary (WASM + egui)
│   │   └── ratacat-proxy.rs    # RPC proxy server (development)
│   ├── platform/        # Platform abstraction layer
│   │   ├── mod.rs       # Platform dispatch
│   │   ├── native.rs    # Native implementations (copypasta)
│   │   └── web.rs       # Web implementations (WASM-bindgen bridge)
│   ├── app.rs           # Application state (shared)
│   ├── ui.rs            # Ratatui rendering (70/30 layout split, shared)
│   ├── config.rs        # CLI args + env config with validation
│   ├── types.rs         # Data models (shared)
│   ├── theme.rs         # Color themes (Nord/DosBlue/AmberCrt/GreenPhosphor)
│   ├── json_syntax.rs   # JSON syntax highlighting (WCAG AAA colors)
│   ├── source_ws.rs     # WebSocket client (native-only)
│   ├── source_rpc.rs    # NEAR RPC poller (shared)
│   ├── archival_fetch.rs # Background archival RPC fetcher (shared)
│   ├── filter.rs        # Query parser + matcher (shared)
│   ├── history.rs       # SQLite persistence + search (native) / stub (web)
│   ├── json_pretty.rs   # ANSI-colored JSON (shared)
│   ├── json_auto_parse.rs # Recursive nested JSON parser (shared)
│   ├── util_text.rs     # Soft-wrapping (shared)
│   ├── rpc_utils.rs     # RPC client utilities (shared)
│   ├── near_args.rs     # Base64 args decoder (shared)
│   ├── marks.rs         # Jump marks system (native-only)
│   └── credentials.rs   # Credentials watcher (native-only)
├── tauri-workspace/
│   └── src-tauri/
│       ├── Cargo.toml   # Tauri dependencies + clipboard plugin
│       ├── src/
│       │   ├── lib.rs   # Core logic + clipboard command
│       │   ├── main.rs  # Entry point
│       │   └── bin/
│       │       └── ratacat-egui-tauri.rs # Tauri + egui integration
│       └── tauri.conf.json # Tauri configuration
├── vendor/
│   ├── egui_ratatui/    # egui + ratatui bridge (local patches)
│   └── soft_ratatui/    # Software rendering backend (local patches)
└── .env.example         # Configuration template
```

**Key Architectural Decisions:**
- **Library-first design**: Core logic in `lib.rs`, platform-specific in `bin/`
- **Feature flags**: `native` vs `web` enable/disable platform-specific code
- **Conditional compilation**: `#[cfg(feature = "native")]` for native-only modules
- **Platform abstraction**: `platform/` module provides unified interface for clipboard, storage, etc.
- **Shared UI**: Same `ui.rs` and `app.rs` code renders in both terminal and browser
- **egui_ratatui bridge**: Web uses `egui_ratatui` to render ratatui widgets in egui

## Recent Improvements (v0.3.0)

### Unified Clipboard System (v0.4.1)
- **Problem**: Inline clipboard code duplicated across web and Tauri binaries, no fallback chain for reliability
- **Solution**: Platform abstraction with JavaScript bridge and 4-tier fallback chain
- **Key Features**:
  - `src/platform/web.rs`: WASM-bindgen bridge to JavaScript
  - `web/platform.js`: Unified clipboard facade with fallback chain:
    1. Tauri clipboard plugin (via invoke command)
    2. Browser extension relay (via `chrome.runtime.sendMessage`)
    3. Navigator Clipboard API (modern browsers, HTTPS/localhost only)
    4. Legacy execCommand fallback (older browsers/WebViews)
  - `tauri-plugin-clipboard-manager`: Official Tauri v2 clipboard plugin
  - Removed code duplication from `ratacat-egui-web.rs` and `ratacat-egui-tauri.rs`
  - All binaries now use `ratacat::platform::copy_to_clipboard()` abstraction
- **Benefits**:
  - Maximum compatibility across all environments (web, Tauri, extension, legacy)
  - Single source of truth eliminates maintenance burden
  - Production-ready with graceful degradation
  - Browser extension integration ready

### Block Selection Refactor (Height-Based Tracking)
- **Previous behavior**: Selection tracked by array index, causing UI to shift as new blocks arrived
- **New behavior**: Selection tracked by block height with auto-follow and manual modes
  - Auto-follow mode (`Home` key): Always shows newest block (index 0)
  - Manual mode (any navigation): Locks to specific block height, stable across new arrivals
  - Intelligent transaction selection: Resets on manual block change, preserves during auto-follow

### Smart Block Filtering
- **Problem**: When filtering by account (e.g., `WATCH_ACCOUNTS=intents.near`), blocks panel showed ALL blocks including those with no matching transactions, causing confusion
- **Solution**: Blocks panel automatically shows only blocks with matching transactions when filter is active
- **Key Features**:
  - Filtered count display: "Blocks (12 / 100)" shows 12 blocks have matches out of 100 total
  - Transactions panel shows: "Txs (0 / 5)" when filter hides some transactions
  - **Navigation follows filtered list**: Up/Down arrows navigate only through blocks with matching transactions
  - Clear visual feedback prevents confusion about missing blocks
  - No filter active → show all blocks (default behavior)
- **Implementation**:
  - `count_matching_txs()`: Counts transactions matching filter in a block
  - `filtered_blocks()`: Returns only blocks with ≥1 matching transaction
  - `get_navigation_list()`: Returns appropriate block list based on filter state (critical for stable navigation)
- **Critical bug fix**: Navigation used to navigate through full block list while UI showed filtered list, causing unpredictable selection jumps. Now navigation list matches display list.

### Archival RPC Support
- **Problem**: Users could only navigate through 100 recent blocks + ±12 cached blocks, couldn't explore deep blockchain history
- **Solution**: On-demand fetching of historical blocks from archival RPC endpoint
- **Key Features**:
  - Unlimited backward navigation through entire blockchain history
  - Loading state: "⏳ Loading block #... from archival..." during 1-2 second fetch
  - Automatic caching of fetched blocks for seamless re-navigation
  - Optional: only enabled if `ARCHIVAL_RPC_URL` is configured
  - Works with FastNEAR archival endpoints (requires `FASTNEAR_AUTH_TOKEN` for best performance)
- **Implementation**:
  - `archival_fetch.rs`: Background async task listening on channel for block height requests
  - Reuses existing `fetch_block_with_txs()` RPC infrastructure
  - Channel-based communication: `UnboundedSender<u64>` for requests
  - App tracks loading state in `loading_block: Option<u64>`
  - Navigation triggers archival fetch when navigating to unavailable blocks
  - Fetched blocks sent via existing `AppEvent::NewBlock` channel

### Function Call Arguments Decoding
- **Three-tier decoding strategy**: JSON → Printable Text → Binary Hex Dump
- **Auto-parsing nested JSON**: Detects and parses JSON-serialized strings within args (common NEAR pattern like `"msg": "{\"action\":\"swap\"}"`)
- **Before**: Only showed byte count `"args_size": "89 bytes"`
- **After**: Full decoded args with recursive JSON parsing
- **Implementation**: `src/near_args.rs` with `DecodedArgs` enum + `src/json_auto_parse.rs` for nested strings
- **Also applied to**: AddKey `access_key` field (parses stringified access key objects)

### Delegate Action Support
- **Full recursive parsing**: Delegate actions now show all nested actions (Transfer, FunctionCall, etc.)
- **Before**: `"actions": "3 delegated action(s)"` (just a count)
- **After**: Full array with formatted nested actions showing method names, amounts, gas, etc.
- **Example**:
  ```json
  {
    "type": "Delegate",
    "sender": "relay.tg",
    "receiver": "user.tg",
    "actions": [
      {"type": "FunctionCall", "method": "ft_transfer", "gas": "30 TGas", "deposit": "1 yN"},
      {"type": "Transfer", "amount": "1 NEAR"}
    ]
  }
  ```

### UI Optimizations
- **70/30 layout split**: Details pane gets 70% of vertical space (was 50%), matching csli-dashboard
- **No left border on details pane**: Makes it easy to click-and-drag with mouse to copy JSON
- **Dynamic chrome**: Filter bar and debug panel collapse when not in use, maximizing vertical space
- **Ratio-based layouts**: Eliminates rounding gaps from percentage-based constraints
- **Smart scroll clamping**: Details pane scrolling stops at actual content end (not u16::MAX)
- **Toast notifications**: 2-second visual feedback when copying content (green bold text in footer)
  - Pane-specific messages: "Copied block info", "Copied tx hash", "Copied details"
  - Error handling: "Copy failed" on clipboard errors
- **Dynamic title hints**: Details pane shows "Press 'c' to copy" when focused

### Configuration System
- **CLI argument support**: Override any setting via command-line flags (e.g., `--render-fps 60`)
- **Priority chain**: CLI args > Environment variables > Defaults
- **Validation with helpful errors**: All values range-checked on startup with clear error messages
- **`.env.example`**: Comprehensive 147-line configuration template with examples

### Jump Marks System
- **Persistent bookmarks**: Mark interesting transactions/blocks for quick navigation
- **Pinning support**: Pin important marks to keep them across sessions
- **Quick jump**: Press `'` then a label character to instantly jump
- **SQLite persistence**: Marks saved to database and restored on restart

### Owned Accounts Filtering
- **Auto-discovery**: Watches `~/.near-credentials` directory for your accounts
- **Ctrl+U toggle**: Instantly filter to show only transactions involving your accounts
- **Real-time updates**: File watcher automatically detects new credential files

### Context-Aware Block Caching
- **Problem**: With a rolling 100-block buffer, selected blocks age out after ~100 new blocks arrive, causing selection to jump
- **Solution**: Cache ±12 blocks around selection for persistent navigation context
- **LRU eviction**: Caches up to 50 blocks total with least-recently-used eviction
- **Visual indicators**:
  - Gray out blocks not available for navigation
  - Show "Blocks (cached) · ← Recent" title when viewing cached block
  - Left arrow (←) returns to recent blocks in auto-follow mode
- **Implementation**:
  - `cached_blocks: HashMap<u64, BlockRow>` for O(1) lookup by height
  - `cached_block_order: Vec<u64>` for LRU tracking
  - Fallback logic in `current_block()` checks cache if block not in main buffer

### Filter UX Improvements
- **Filtered count display**: Transactions panel shows "Txs (0 / 5)" when filter hides transactions
  - Makes it clear when blocks have transactions that don't match the filter
  - Prevents confusion when blocks panel shows "5 txs" but transactions panel appears empty
- **Default filtering**: Auto-filter to `intents.near` on startup via `WATCH_ACCOUNTS` env var
- **Simple account watching**: Comma-separated account list without needing to learn filter syntax

## Future Enhancements

- **FTS5 Support**: Full-text search upgrade when SQLite has FTS5
- **Plugin System**: Currently disabled due to lifetime issues
- **Nested Delegate Actions**: Support for deeply nested DelegateAction chains (NEP-366 currently prevents this)
- **Copy Structure Parity**: Implement csli-dashboard's pane-specific copy formats:
  - Pane 0 (Blocks): Export all transactions in block with metadata
  - Pane 1 (Tx Hashes): Dual format with raw chain data + human-readable
  - Pane 2 (Details): Current implementation (human-readable only)
  - Display vs Copy: Show truncated data in UI, copy full data (complete hashes, full base64)

## Dependencies

### Quad-Mode Dependency Strategy

Ratacat uses **feature flags** and **optional dependencies** with strict `dep:` mappings to prevent cross-contamination:

```toml
[features]
default = []  # No defaults - explicit feature selection required
native = [
    # Native UI/IO (ALL optional with dep: mapping)
    "dep:crossterm", "dep:copypasta", "dep:rusqlite", "dep:notify",
    # WebSocket support
    "dep:tokio-tungstenite", "dep:tungstenite", "dep:futures-util",
    # NEAR SDK crates (have C dependencies)
    "dep:near-primitives", "dep:near-crypto", "dep:near-jsonrpc-client",
    "dep:near-jsonrpc-primitives", "dep:near-account-id", "dep:near-gas", "dep:near-token",
    # Tokio features
    "tokio/rt-multi-thread", "tokio/macros", "tokio/time", "tokio/signal",
    "tokio/fs", "tokio/io-util",
]

egui-web = [
    "dep:egui",
    "dep:eframe",
    "dep:egui_ratatui",
    "dep:soft_ratatui",
    "dep:wasm-bindgen",
    "dep:wasm-bindgen-futures",
    "dep:js-sys",
    "dep:web-sys",
    "dep:getrandom",
]

[dependencies]
# Core dependencies (both platforms)
anyhow = "1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4.5", features = ["derive", "env"] }
async-trait = "0.1"
log = "0.4"
base64 = "0.22"
once_cell = "1"
cfg-if = "1"

# TUI (version 0.29+ for egui_ratatui compatibility)
ratatui = { version = "0.29", default-features = false }

# Chrono with WASM support
chrono = { version = "0.4", features = ["serde", "wasmbind"] }

# HTTP client (rustls works on both platforms)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }

# NEAR Protocol crates (optional - C dependencies prevent WASM)
near-primitives = { version = "0.27.0", optional = true }
near-crypto = { version = "0.27.0", optional = true }
near-jsonrpc-client = { version = "0.15.0", features = ["any"], optional = true }
near-jsonrpc-primitives = { version = "0.27.0", optional = true }
near-account-id = { version = "1.0.0", optional = true }
near-gas = { version = "0.2", features = ["serde", "borsh"], optional = true }
near-token = { version = "0.2", features = ["serde", "borsh"], optional = true }

# Tokio (base features for both, extended via features)
tokio = { version = "1", default-features = false }

# Target-specific tokio (WASM-compatible subset)
[target.'cfg(target_arch = "wasm32")'.dependencies]
tokio = { version = "1", default-features = false, features = ["sync", "macros", "time"] }

# Native-only dependencies
crossterm = { version = "0.27", optional = true }
copypasta = { version = "0.10", optional = true }
rusqlite = { version = "0.31", features = ["bundled"], optional = true }
notify = { version = "6.1", optional = true }
tokio-tungstenite = { version = "0.21", optional = true }
tungstenite = { version = "0.21", optional = true }
futures-util = { version = "0.3", optional = true }

# Web-only dependencies
egui = { version = "0.32", optional = true }
eframe = { version = "0.32", optional = true, default-features = false, features = ["glow", "default_fonts"] }
egui_ratatui = { version = "2.0", optional = true }
soft_ratatui = { version = "0.1", optional = true }
wasm-bindgen = { version = "0.2", optional = true }
wasm-bindgen-futures = { version = "0.4", optional = true }
web-sys = { version = "0.3", optional = true, features = ["Window", "Navigator", "Clipboard", "Storage", "console"] }
getrandom = { version = "0.2", optional = true, features = ["js"] }
```

### WASM Compatibility Challenges & Solutions

**Challenge 1: NEAR SDK C Dependencies**

The official NEAR SDK crates (near-primitives, near-crypto, etc.) depend on native C libraries:
- `zstd-sys` - Compression library (C code)
- `secp256k1-sys` - Cryptographic primitives (C code)
- `ed25519-dalek` with native features

**Solution:**
- Made all NEAR crates **optional** dependencies
- Use conditional compilation `#[cfg(feature = "native")]` throughout codebase
- Created web-compatible stub implementations for formatters:
  ```rust
  // src/util_text.rs
  #[cfg(feature = "near-gas")]
  use near_gas::NearGas;

  pub fn format_gas(gas: u64) -> String {
      #[cfg(feature = "near-gas")]
      {
          format!("{}", NearGas::from_gas(gas))
      }
      #[cfg(not(feature = "near-gas"))]
      {
          // Fallback formatter for web
          const TERA: u64 = 1_000_000_000_000;
          if gas >= TERA {
              format!("{} TGas", gas / TERA)
          } else {
              format!("{} Gas", gas)
          }
      }
  }
  ```

**Challenge 2: Tokio Runtime**

Tokio's default features include `net` which uses `mio` (not WASM-compatible).

**Solution:**
- Target-specific tokio configuration:
  ```toml
  [target.'cfg(target_arch = "wasm32")'.dependencies]
  tokio = { version = "1", default-features = false, features = ["sync", "macros", "time"] }
  ```
- WASM builds get minimal tokio with only async primitives

**Challenge 3: Platform-Specific Features**

Features like clipboard, SQLite, file watching are native-only.

**Solution:**
- Platform abstraction layer (`src/platform/`)
- Separate implementations:
  - `platform/native.rs` - Uses copypasta for clipboard, rusqlite for storage
  - `platform/web.rs` - WASM-bindgen bridge to JavaScript clipboard facade (`web/platform.js`)
  - `web/platform.js` - 4-tier fallback: Tauri plugin → Extension relay → Navigator API → execCommand
- Conditional module selection in lib.rs:
  ```rust
  #[cfg(feature = "native")]
  pub mod history;  // Full SQLite implementation

  #[cfg(not(feature = "native"))]
  pub mod history;  // Stub with empty methods
  ```

## Performance Characteristics

- **Memory**: ~10MB baseline + (100 blocks × avg tx size)
- **CPU**: <5% on modern hardware at 30 FPS
- **Disk I/O**: WAL mode enables concurrent reads during writes
- **Network**: Configurable polling interval + catch-up limits

## Troubleshooting

**Connection refused with SOURCE=ws**:
- Ensure Node WebSocket server is running on port 63736
- Check WS_URL matches your Node configuration

**High CPU usage**:
```bash
RENDER_FPS=20 KEEP_BLOCKS=50 cargo run --bin ratacat --features native
```

**RPC timeouts**:
```bash
RPC_TIMEOUT_MS=15000 POLL_CHUNK_CONCURRENCY=2 cargo run --bin ratacat --features native
```

**Search not finding results**:
- Ensure SQLite history has been populated (run for a few minutes first)
- Check query syntax matches filter grammar

**Web build errors**:

1. **Build fails with zstd-sys/secp256k1-sys errors**:
   - **Cause**: Default features include native NEAR SDK crates
   - **Fix**: Use `--no-default-features --features web` flags

2. **Runtime panic: "time not implemented on this platform"**:
   - **Cause**: Some `std::time` usage not WASM-compatible
   - **Status**: Known issue, active development
   - **Workaround**: Affects specific time-based features
   - **Fix planned**: v0.4.0 will use wasm-compatible time crates

3. **Connection refused errors in browser console**:
   - **Cause**: Web app trying to connect to localhost proxy
   - **Fix**: Configure RPC endpoint via URL parameters:
     ```
     http://localhost:8080?rpc=https://rpc.mainnet.fastnear.com
     ```

## Known Limitations (Web Mode)

- ⚠️ **Time-based features**: Some chrono usage not fully WASM-compatible
- ⚠️ **No SQLite**: History and marks are in-memory only
- ⚠️ **RPC only**: WebSocket mode not available
- ⚠️ **No file access**: Credential watching disabled
- ✅ **Core functionality**: Block viewing, filtering, and navigation work perfectly

---
