# Ratacat - NEAR Blockchain Transaction Viewer

**Version 0.4.0** - High-performance **quad-mode** application for monitoring NEAR Protocol blockchain transactions. Runs as native terminal app, web browser app, Tauri desktop app, or browser extension integration. Built in Rust with [Ratatui](https://ratatui.rs).

## Features

### 3-Pane Dashboard
![Ratacat showing the full 3-pane layout - blocks list, transaction hashes, and detailed transaction view while monitoring intents.near on NEAR mainnet](static/selection.png)

The main interface shows blocks on the left (with transaction counts and filtering), transaction hashes in the middle, and full transaction details on the right. The filter bar at the top shows `acct:intents.near` actively filtering the transaction stream.

### Fullscreen Details View
![Ratacat in fullscreen mode showing detailed JSON transaction data for intents.near on NEAR mainnet](static/full-screen.png)

Press `Spacebar` to toggle fullscreen mode for the details pane, giving maximum vertical space to inspect complex transaction payloads. The filter remains visible at the top for context.

### Core Capabilities
- **3-Pane Dashboard**: Blocks → Transaction Hashes → Transaction Details
- **Dual Data Sources**:
  - WebSocket: Real-time updates from your Node breakout server
  - RPC Polling: Direct NEAR RPC with smart catch-up limits
- **Fullscreen Details**: Toggle fullscreen view with `Spacebar` for maximum transaction inspection space
- **Smooth Scrolling**: Navigate large transaction details with arrow keys, PgUp/PgDn, Home/End
- **FPS Control**: Runtime FPS adjustment with Ctrl+O (default 30 FPS)
- **Clipboard Integration**: Copy transaction details with `c`

### New in v0.3.0
- **Function Call Args Decoding**: Three-tier decoding strategy (JSON → Text → Binary) with auto-parsing of nested JSON strings
- **Smart Block Filtering**: Blocks panel automatically shows only blocks with matching transactions when filter is active
  - Shows filtered count: "Blocks (12 / 100)" - 12 have matches out of 100 total
  - Transactions panel shows: "Txs (0 / 5)" when filter hides some transactions
  - Navigation (Up/Down arrows) follows filtered list for stable, predictable selection
  - Clear visual feedback prevents confusion about missing transactions
- **Auto-Lock to Matching Blocks**: First block with matching transactions automatically locks for stable viewing
  - Block stays highlighted and stable - won't jump to newer arrivals while you're viewing
  - Navigate with Up/Down arrows - immediate response, no lag
  - Press `Home` in blocks pane to return to auto-follow mode (tracks newest matching block)
- **Archival RPC Support**: Navigate unlimited blocks backward through blockchain history
  - Configure `ARCHIVAL_RPC_URL` to enable on-demand historical block fetching
  - Loading state shows "⏳ Loading block #..." during 1-2 second fetch
  - Fetched blocks cached automatically for seamless navigation
  - Works with FastNEAR archival endpoints
- **Context-Aware Block Caching**: Navigate ±12 blocks around selection even after aging out of 100-block buffer
  - Gray visual indicator for unavailable blocks
  - "Blocks (cached)" title when viewing aged-out blocks
  - Left arrow (←) returns to recent blocks
- **Filter Bar**: Filter transactions by account, action type, method name, or free text
- **SQLite History**: Non-blocking persistence of blocks and transactions to SQLite (off main thread)
- **Owned Accounts Awareness**: Automatically detect your NEAR accounts from credentials files
  - Star badges on blocks showing owned transaction count
  - Bold yellow highlighting of owned transactions
  - `Ctrl+U` to filter for owned-only transactions
  - Zero overhead when no credentials present
- **Jump Marks**: Bookmark important blocks/transactions with `m`, pin with `Ctrl+P`, jump with `'`
- **History Search**: Full-text search across all persisted transactions with `Ctrl+F`
- **70/30 Layout Split**: Details pane gets 70% of vertical space (up from 50%) for better readability
- **Smart Scroll Clamping**: Scrolling stops at actual content end instead of continuing indefinitely
- **Toast Notifications**: 2-second visual feedback when copying content ("Copied block info", "Copied tx hash", "Copied details")

### Performance
- **Coalesced Rendering**: FPS-capped to prevent UI thrashing
- **Non-blocking I/O**: Async data fetching keeps UI responsive
- **Catch-up Limits**: Prevents cascade failures during network delays
- **Soft-wrapped Tokens**: Long base58/base64 strings wrapped cleanly

## Installation & Quick Start

Ratacat runs in **four modes**: native terminal (recommended), Tauri desktop app, web browser (experimental), and browser extension.

### Native Terminal Mode (Recommended)

```bash
# Build and run
cargo build --release
./target/release/ratacat

# Or run directly with cargo
cargo run --release
```

**WebSocket Mode** (for development):
```bash
# Terminal 1: Start your Node server with WebSocket breakout
cd ../node
npm run dev

# Terminal 2: Run Ratacat
SOURCE=ws cargo run
```

**RPC Mode** (for production):
```bash
# Testnet
SOURCE=rpc NEAR_NODE_URL=https://rpc.testnet.fastnear.com/ cargo run

# Mainnet
SOURCE=rpc NEAR_NODE_URL=https://rpc.mainnet.near.org/ cargo run
```

### Tauri Desktop App Mode

Native desktop application with deep link support for `near://` protocol URLs.

**Quick Start**:
```bash
cd tauri-workspace
cargo tauri dev
```

**Build for Distribution**:
```bash
cd tauri-workspace
cargo tauri build

# Manual workaround for bundler bug:
mkdir -p target/release/bundle/macos/Ratacat.app/Contents/MacOS
cp target/release/explorer-tauri target/release/bundle/macos/Ratacat.app/Contents/MacOS/
```

**Key Features**:
- **Deep Link Handler**: Opens `near://tx/HASH?network=mainnet` URLs from browser
- **Single Instance**: Prevents multiple app windows
- **Native Performance**: Full desktop integration
- **DevTools**: Press `Cmd+Option+I` (macOS) or `F12` (Windows/Linux)
- **Debug Logging**: Comprehensive waterfall logs at `~/Library/Logs/com.ratacat.fast/Ratacat.log` (macOS)

**Testing Deep Links**:
```bash
# Open app with deep link
open 'near://tx/ABC123?network=mainnet'

# View logs
tail -f ~/Library/Logs/com.ratacat.fast/Ratacat.log
```

**Configuration**:
- Bundle ID: `com.ratacat.fast`
- URL Scheme: `near://`
- Log Location: `~/Library/Logs/com.ratacat.fast/` (macOS)

**Known Issues**:
- Tauri bundler bug requires manual binary copy (see build steps above)
- DevTools button requires `devtools` Cargo feature (already enabled)
- macOS only for now (Windows/Linux testing pending)

For detailed technical documentation, see `CLAUDE.md` § Tauri Desktop App Mode.

### Web Browser Mode (Experimental) 🚀

Run Ratacat in your browser with the same terminal UI experience!

```bash
# Install web build tools (one-time setup)
cargo install --locked trunk
rustup target add wasm32-unknown-unknown

# Build and serve locally
trunk serve --release --no-default-features --features web
# Opens at http://127.0.0.1:8080

# Build for deployment
trunk build --release --no-default-features --features web
# Output in dist/ directory - deploy to any static host!
```

**Important Build Flags:**
- `--no-default-features` is **required** - prevents pulling in native-only NEAR SDK crates with C dependencies
- `--features web` enables web-specific dependencies (Ratzilla, wasm-bindgen, etc.)
- Without these flags, the build will fail with zstd/secp256k1 compilation errors

**Web Features:**
- ✅ Same 3-pane TUI interface in browser
- ✅ All keyboard shortcuts work identically
- ✅ RPC polling with real-time updates
- ✅ Filtering, search, and FPS control
- ✅ Web-native clipboard support
- ✅ Deploy to GitHub Pages, Vercel, Netlify, etc.
- ⚠️ No SQLite history (in-memory only)
- ⚠️ No WebSocket mode (RPC only)
- ⚠️ No jump marks persistence (in-memory only)

**Try it now:** Visit `http://localhost:8080?rpc=https://rpc.mainnet.fastnear.com&filter=intents.near` after running `trunk serve`

**Technical Notes:**
- Web build isolates NEAR SDK crates (near-primitives, near-crypto, etc.) which have C dependencies incompatible with WASM
- Uses platform abstraction layer for clipboard (web-sys), storage (in-memory), and runtime (wasm-bindgen)
- Ratatui 0.29+ required for Ratzilla compatibility

## Keyboard Shortcuts

### Navigation
- `Tab` / `Shift+Tab` - Switch between panes
- `↑ / ↓` - Navigate lists or scroll details (immediate response, no lag)
- `← / →` - Left: jump to top of current list; Right: paginate down 12 items
- `PgUp / PgDn` - Page up/down in details pane
- `Home` - In blocks pane: return to auto-follow mode (track newest matching block); Other panes: jump to top
- `End` - Jump to bottom in details pane
- `Enter` - Select transaction and view details

### View Controls
- `Spacebar` - Toggle fullscreen details view (maximizes transaction inspection area)
- `Ctrl+O` - Cycle FPS (20 → 30 → 60)
- `c` - Copy current details to clipboard (shows toast notification)
- `Ctrl+D` - Toggle debug panel visibility
- `q` or `Ctrl+C` - Quit

### Filter Controls
- `/` or `f` - Enter filter mode
- Type to filter transactions
- `Enter` - Apply filter
- `Esc` - Clear filter and exit filter mode
- `Ctrl+U` - Toggle owned-only filter (show only your transactions)

### Jump Marks & Search
- `m` - Set mark at current location (auto-labeled)
- `Ctrl+P` - Pin/unpin mark at current location
- `M` (Shift+M) - Open marks overlay
- `'` (apostrophe) - Jump to mark (type label)
- `[` / `]` - Jump to previous/next mark
- `d` (in marks overlay) - Delete selected mark
- `Ctrl+F` - Open history search overlay

## Configuration

All configuration is via environment variables. See `.env.example` for full options.

### Data Source
```bash
SOURCE=ws                                    # Use WebSocket (default)
SOURCE=rpc                                   # Use NEAR RPC polling
WS_URL=ws://127.0.0.1:63736                 # WebSocket endpoint
WS_FETCH_BLOCKS=true                        # Hybrid mode: fetch full block data via RPC (default: true)
```

**WebSocket Modes:**
- `WS_FETCH_BLOCKS=true` (default): **Hybrid mode** - WS notifications trigger RPC fetches for complete block data with transactions
- `WS_FETCH_BLOCKS=false`: **Legacy mode** - WS only updates details pane, blocks show "0 txs"

Hybrid mode gives the best of both worlds: real-time push notifications + complete transaction data.

**Network Auto-Detection:**
If `NEAR_NODE_URL` is not explicitly set, Ratacat automatically detects the network (mainnet vs testnet) from block heights:
- Block heights > 100M → mainnet (uses `https://rpc.mainnet.near.org`)
- Block heights < 100M → testnet (uses `https://rpc.testnet.fastnear.com`)

This prevents mainnet/testnet mismatches when using WebSocket mode.

### RPC Configuration
```bash
NEAR_NODE_URL=https://rpc.testnet.fastnear.com/
FASTNEAR_AUTH_TOKEN=xxx                     # Bearer token for authenticated fastnear API access (recommended)
ARCHIVAL_RPC_URL=https://archival-rpc.mainnet.fastnear.com  # Archival endpoint for historical blocks (optional)
POLL_INTERVAL_MS=1000                        # Poll frequency (default 1s)
POLL_MAX_CATCHUP=5                          # Max blocks per poll (prevents cascade)
POLL_CHUNK_CONCURRENCY=4                    # Parallel chunk fetches
RPC_TIMEOUT_MS=8000                         # Request timeout
RPC_RETRIES=2                               # Retry attempts
```

**FastNEAR Authentication**: To avoid rate limiting (429 errors), get an API token from [fastnear.com](https://fastnear.com) and set `FASTNEAR_AUTH_TOKEN`. Authenticated requests have significantly higher rate limits.

**Archival RPC**: Set `ARCHIVAL_RPC_URL` to enable unlimited backward navigation through blockchain history. When you navigate beyond the rolling 100-block buffer and ±12 block cache, Ratacat automatically fetches historical blocks from the archival endpoint. Requires `FASTNEAR_AUTH_TOKEN` for best performance.

### UI Configuration
```bash
RENDER_FPS=30                               # Target FPS (1-120)
RENDER_FPS_CHOICES=20,30,60                 # Ctrl+O cycle options
KEEP_BLOCKS=100                             # In-memory block limit
```

### History Configuration
```bash
SQLITE_DB_PATH=./ratacat_history.db         # SQLite database path for persistence
```

### Owned Accounts Configuration
```bash
NEAR_CREDENTIALS_DIR=~/.near-credentials    # Path to NEAR CLI credentials directory (default: ~/.near-credentials)
NEAR_NETWORK=mainnet                        # Network to watch: mainnet or testnet (default: mainnet)
```

Ratacat automatically watches your NEAR CLI credentials directory for account files. When detected, it:
- Shows star badges on blocks with your transactions (e.g., "5 txs (3 owned)")
- Highlights your transactions in **bold yellow**
- Enables `Ctrl+U` to filter for owned-only view
- Updates in real-time when you add/remove credentials

**No setup required** - works automatically if you have NEAR CLI credentials at `~/.near-credentials/<network>/*.json`

### Account Filtering Configuration
```bash
WATCH_ACCOUNTS=intents.near,alice.near      # Comma-separated list of accounts to watch (simple filtering)
DEFAULT_FILTER=acct:alice.near method:swap  # Advanced filter syntax (only used if WATCH_ACCOUNTS not set)
```

**Simple filtering with WATCH_ACCOUNTS**: Just list account names separated by commas. Ratacat will automatically filter to show only transactions to/from these accounts. Takes precedence over `DEFAULT_FILTER`.

**Advanced filtering with DEFAULT_FILTER**: Use full filter syntax for complex queries (e.g., `signer:alice.near receiver:bob.near action:FunctionCall`). See Filter Syntax section below for details.

## Architecture

### Data Flow
```
┌─────────────────────────────────────────────────┐
│           NEAR Blockchain Data                  │
│  ┌──────────────┐      ┌──────────────────┐     │
│  │  WebSocket   │  OR  │   RPC Polling    │     │
│  │ (Node side)  │      │  (Direct NEAR)   │     │
│  └──────┬───────┘      └────────┬─────────┘     │
│         │                       │               │
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
- **`archival_fetch.rs`**: Background archival RPC fetcher for historical blocks
- **`app.rs`**: State management and event handling
- **`ui.rs`**: Ratatui rendering with 3-pane layout
- **`config.rs`**: Environment-based configuration
- **`types.rs`**: Blockchain data models
- **`filter.rs`**: Query parser and transaction matcher
- **`history.rs`**: SQLite persistence and search
- **`marks.rs`**: Jump marks system

## Design Principles

1. **FPS-Capped Rendering**: Coalesced draws prevent UI thrashing
2. **Non-Overlapping Polls**: RPC mode uses catch-up limits to prevent cascades
3. **Soft-Wrapped Tokens**: ZWSP insertion for clean line breaking
4. **Human-Readable JSON**: Formatted transaction details with ANSI colors for quick scanning
5. **Async Everything**: Tokio-based async I/O keeps UI responsive

## Tips

1. **Use WebSocket mode during development** - connects to your existing Node server
2. **Use RPC mode for production monitoring** - direct NEAR connection, no middleman
3. **Press `Spacebar` for fullscreen** - maximize vertical space for inspecting complex transactions
4. **Adjust FPS with Ctrl+O** - lower FPS if CPU-constrained
5. **Copy with `c`** - paste transaction details anywhere (shows toast confirmation)
6. **Stable block selection** - First matching block automatically locks for stable viewing
   - Block stays highlighted and won't jump to newer arrivals
   - Tab to details pane, scroll around - block remains stable
   - Press `Home` in blocks pane to return to auto-follow mode
7. **Filter transactions** - Press `/` and use syntax like `acct:alice.near action:FunctionCall method:transfer`
   - Blocks panel automatically shows only blocks with matching transactions
   - Shows count like "Blocks (5 / 100)" - 5 blocks match your filter
8. **Track your accounts** - Owned transactions show in **bold yellow** with star badges on blocks
9. **Quick owned-only view** - Press `Ctrl+U` to see only your transactions
10. **Bookmark important moments** - Use `m` to set marks, `Ctrl+P` to pin them permanently
11. **Search history** - Press `Ctrl+F` to search all persisted transactions by account, method, or hash
12. **Navigate through history** - Enable `ARCHIVAL_RPC_URL` to explore unlimited blocks backward
    - Navigate beyond cache and Ratacat fetches historical blocks automatically
    - Loading state shows progress during 1-2 second fetch
13. **Details pane gets 70% height** - Optimized layout gives more space to transaction details

## Filter Syntax

The filter bar supports powerful query syntax:

### Field Filters
```
acct:alice.near              # Match signer OR receiver
signer:bob.near              # Match signer only
receiver:contract.near       # Match receiver only
action:FunctionCall          # Match action type
method:ft_transfer           # Match FunctionCall method name
raw:some_text                # Search in raw JSON
```

### Free Text
```
alice                        # Match anywhere in signer/receiver/hash/methods
```

### Combined Filters
```
acct:alice.near action:Transfer                    # Alice sending tokens
signer:bob.near method:ft_transfer                 # Bob calling ft_transfer
action:FunctionCall method:transfer alice          # Function calls with "transfer" and "alice"
```

Filters use AND logic (all conditions must match). Within each field type, OR logic applies (any value matches).

## Known Limitations

### Plugin System (Disabled)
The plugin system is temporarily disabled due to lifetime compilation issues. Once fixed, it will enable:
- Validator monitoring
- Transaction pattern analysis
- Custom alerts and filters
- External tool integrations

### Copy Functionality
Current implementation copies pane content as-is. Future enhancement planned for csli-dashboard parity:
- **Pane 0 (Blocks)**: Export all transactions in block with network/height/hash metadata
- **Pane 1 (Tx Hashes)**: Dual format with raw chain data + human-readable decoded version
- **Pane 2 (Details)**: Current implementation (human-readable only)
- **Display vs Copy**: Show truncated data in UI, copy full data (complete hashes, full base64)

## Built on Official NEAR Infrastructure

Ratacat uses official NEAR Protocol crates from the nearcore repository, ensuring compatibility and future-proofing:

- **`near-primitives`** (0.27.0) - Core blockchain data structures (Block, Transaction, Receipt, etc.)
- **`near-jsonrpc-client`** (0.15.0) - Official RPC client with built-in retry logic and proper error handling
- **`near-jsonrpc-primitives`** (0.27.0) - RPC request/response types that match the NEAR RPC specification
- **`near-crypto`** (0.27.0) - Cryptographic primitives for signature verification and key handling
- **`near-account-id`** (1.0.0) - Validated account ID types with proper parsing rules
- **`near-gas`** (0.2) - Gas amount formatting and display utilities
- **`near-token`** (0.2) - NEAR token amount formatting with proper decimal handling

By leveraging these official crates, Ratacat:
- **Stays synchronized** with NEAR protocol changes and upgrades
- **Avoids reimplementation** of complex blockchain logic
- **Maintains compatibility** with NEAR RPC endpoints across networks (mainnet, testnet)
- **Benefits from upstream improvements** in performance, security, and correctness

This approach ensures that transaction parsing, block structure handling, and RPC communication remain accurate as the NEAR Protocol evolves.

## Quad-Mode Architecture

Ratacat v0.4.0 features a **quad-mode architecture** - write once, run everywhere:

```
┌─────────────────────────────────────────────────────────────┐
│                  Ratacat Application                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Terminal   │  │  Web Browser │  │  Tauri App   │     │
│  │ (Crossterm)  │  │  (Ratzilla)  │  │(Deep Links)  │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                 │                 │              │
│         └─────────────────┼─────────────────┘              │
│                           ▼                                │
│              ┌────────────────────────────┐                │
│              │   Shared Core (Rust)       │                │
│              │ • App state & UI rendering │                │
│              │ • RPC client & polling     │                │
│              │ • Filter & search logic    │                │
│              │ • Types & JSON formatting  │                │
│              └────────────────────────────┘                │
│                                                             │
│  Platform Abstraction:                                      │
│  • Clipboard: copypasta / web-sys / tauri                  │
│  • Storage: SQLite / in-memory                             │
│  • Runtime: tokio (full/wasm/tauri)                        │
│  • Deep Links: native messaging / tauri plugin             │
│                                                             │
│  ┌──────────────────────────────────────────────────┐      │
│  │  Browser Extension (Native Messaging)            │      │
│  │  • Chrome/Firefox/Edge integration               │      │
│  │  • "Open in Ratacat" button on tx pages         │      │
│  │  • Sends near:// deep links to Tauri app        │      │
│  └──────────────────────────────────────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Key Benefits:**
- ✨ **Write once, deploy everywhere** - Terminal, browser, AND desktop from same code
- 🎨 **True terminal UI everywhere** - Not a simulation, actual ratatui rendering
- ⚡ **Zero JavaScript core** - Pure Rust compiled to native/WASM
- 🚀 **Fast & lightweight** - 30+ FPS, native performance
- 🔧 **Platform-specific optimizations** - SQLite on native, web APIs in browser, native messaging for extensions
- 🔗 **Deep link integration** - Browser → Desktop app flow via `near://` protocol

## Building from Source

**Native:**
```bash
git clone <repo>
cd ratacat
cargo build --release
./target/release/ratacat
```

**Web:**
```bash
# One-time setup
cargo install --locked trunk
rustup target add wasm32-unknown-unknown

# Build for deployment
trunk build --release --no-default-features --features web

# Or serve locally for development
trunk serve --release --no-default-features --features web
```

**Understanding the Build Flags:**

The web build requires careful feature flag management to avoid pulling in NEAR SDK crates with C dependencies (zstd-sys, secp256k1-sys) that cannot compile to WebAssembly:

- `--no-default-features`: Disables the default `native` feature, which includes:
  - crossterm, copypasta, rusqlite, notify (native-only UI/IO)
  - near-primitives, near-crypto, near-jsonrpc-client (C dependencies)
  - Full tokio runtime (incompatible with WASM)

- `--features web`: Enables web-specific dependencies:
  - ratzilla (renders ratatui TUI in browser via egui)
  - wasm-bindgen, wasm-bindgen-futures (Rust↔JavaScript bridge)
  - web-sys (Web APIs for clipboard, storage)
  - getrandom with "js" feature (WASM-compatible RNG)
  - console_error_panic_hook, wasm-logger (debugging)

**Trunk Configuration:**

The `Trunk.toml` file is pre-configured with these flags:
```toml
[build.rust]
default-features = false
features = ["web"]
bin = "ratacat-web"
```

The `index.html` specifies which binary to build:
```html
<link data-trunk rel="rust" data-bin="ratacat-web" />
```

## Troubleshooting

### "Connection refused" with SOURCE=ws
- Ensure your Node WebSocket server is running on port 63736
- Check WS_URL matches your Node configuration

### High CPU usage
- Lower FPS: `RENDER_FPS=20 cargo run`
- Reduce block history: `KEEP_BLOCKS=50 cargo run`

### RPC timeouts
- Increase timeout: `RPC_TIMEOUT_MS=15000 cargo run`
- Reduce concurrency: `POLL_CHUNK_CONCURRENCY=2 cargo run`

### Web build errors

**Error: `zstd-sys` or `secp256k1-sys` compilation failed**
- Ensure you're using `--no-default-features --features web` flags
- Check `Trunk.toml` has `default-features = false`

**Runtime error: "time not implemented on this platform"**
- Known issue: Some time-related code not yet fully WASM-compatible
- Workaround: Use direct RPC endpoints instead of proxy
- Status: Active development, fix planned for v0.4.0

**Connection refused to localhost:3030**
- Web version expects RPC proxy or direct RPC endpoint
- Configure via URL parameters: `?rpc=https://rpc.mainnet.fastnear.com`
- Or set default in `load_web_config()` function

## Version History

- **v0.3.0** (Current)
  - Function call args decoding with three-tier fallback (JSON → Text → Binary)
  - Auto-parsing of nested JSON-serialized strings
  - Smart block filtering: blocks panel shows only blocks with matching transactions when filter active
  - Auto-lock to matching blocks: first matching block locks automatically for stable viewing
  - Immediate navigation response: Up/Down arrows navigate instantly without lag
  - Archival RPC support for unlimited backward navigation through blockchain history
  - Context-aware block caching: ±12 blocks preserved around selection after aging out
  - Filter bar with powerful query syntax
  - SQLite history persistence and search
  - Jump marks system for bookmarking transactions
  - Owned accounts awareness with credential file watching
  - 70/30 layout split for better details visibility
  - Smart scroll clamping and toast notifications
  - Dynamic UI chrome (collapsible filter bar and debug panel)
  - Filtered count display: "Blocks (12 / 100)" and "Txs (0 / 5)" formats
- **v0.2.0** - Blockchain viewer with WebSocket + RPC sources, view modes, scrolling
- **v0.1.0** - Initial todo list prototype (pre-pivot)

---

Built using Ratatui, Tokio, and Rust. Designed for NEAR Protocol monitoring.
