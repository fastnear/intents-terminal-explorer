# Ratacat - NEAR Blockchain Transaction Viewer

**Version 0.3.0** - High-performance terminal UI for monitoring NEAR Protocol blockchain transactions in real-time. Built with Ratatui and Rust.

## Architecture Overview

Ratacat follows a clean FPS-capped rendering architecture with async data sources and off-thread persistence:

```
┌─────────────────────────────────────────────────┐
│           NEAR Blockchain Data                  │
│  ┌──────────────┐      ┌──────────────────┐    │
│  │  WebSocket   │  OR  │   RPC Polling    │    │
│  │  (Node side) │      │  (Direct NEAR)   │    │
│  └──────┬───────┘      └────────┬─────────┘    │
│         │                       │               │
│         └───────────┬───────────┘               │
│                     ▼                            │
│            ┌─────────────────┐                  │
│            │  Event Channel  │                  │
│            └────────┬────────┘                  │
│                     ▼                            │
│            ┌─────────────────┐                  │
│            │   App State     │                  │
│            │  (filter/search)│                  │
│            └────────┬────────┘                  │
│                     ▼                            │
│   ┌─────────────────────────────────────────┐  │
│   │    3-Pane TUI + Filter + Search         │  │
│   │  ┌──────┐  ┌──────┐  ┌────────────┐   │  │
│   │  │Blocks│→ │Tx IDs│→ │  Details   │   │  │
│   │  └──────┘  └──────┘  │(PRETTY/RAW)│   │  │
│   │                       └────────────┘   │  │
│   └─────────────────────────────────────────┘  │
│                     ▲                            │
│                     │                            │
│            ┌─────────────────┐                  │
│            │ SQLite History  │                  │
│            │ (off-thread)    │                  │
│            └─────────────────┘                  │
└─────────────────────────────────────────────────┘
```

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
cargo run

# Or override with CLI arguments
cargo run -- --source rpc --render-fps 60
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
SOURCE=ws cargo run
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
SOURCE=ws RENDER_FPS=60 KEEP_BLOCKS=500 cargo run
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

## Building & Running

```bash
# Build release version
cargo build --release

# Run with default settings (WebSocket mode)
cargo run

# Run in RPC mode
cargo run -- --source rpc

# Run with custom settings
cargo run -- --source rpc --render-fps 60 --keep-blocks 200

# Or use environment variables
SOURCE=rpc RENDER_FPS=60 cargo run

# View all CLI options
cargo run -- --help

# Run release binary directly
./target/release/ratacat --source rpc --near-node-url https://rpc.mainnet.fastnear.com/
```

## Project Structure

```
ratacat/
├── Cargo.toml           # Dependencies
├── src/
│   ├── main.rs          # Entry point + event loop
│   ├── app.rs           # Application state + toast notifications
│   ├── ui.rs            # Ratatui rendering (70/30 layout split)
│   ├── config.rs        # CLI args + env config with validation
│   ├── types.rs         # Data models
│   ├── source_ws.rs     # WebSocket client
│   ├── source_rpc.rs    # NEAR RPC poller
│   ├── filter.rs        # Query parser + matcher
│   ├── history.rs       # SQLite persistence + search
│   ├── json_pretty.rs   # ANSI-colored JSON
│   ├── json_auto_parse.rs # Recursive nested JSON parser
│   ├── util_text.rs     # Soft-wrapping
│   ├── clipboard.rs     # Clipboard integration
│   ├── near_args.rs     # Base64 args decoder (3-tier fallback)
│   └── marks.rs         # Jump marks system
└── .env.example         # Configuration template
```

## Recent Improvements (v0.3.0)

### Block Selection Refactor (Height-Based Tracking)
- **Previous behavior**: Selection tracked by array index, causing UI to shift as new blocks arrived
- **New behavior**: Selection tracked by block height with auto-follow and manual modes
  - Auto-follow mode (`Home` key): Always shows newest block (index 0)
  - Manual mode (any navigation): Locks to specific block height, stable across new arrivals
  - Intelligent transaction selection: Resets on manual block change, preserves during auto-follow

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

```toml
[dependencies]
# TUI
ratatui = { version = "0.26", features = ["crossterm"] }
crossterm = "0.27"

# CLI & Configuration
clap = { version = "4.5", features = ["derive", "env"] }
dotenv = "0.15"

# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Blockchain data
tokio-tungstenite = "0.21"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
base64 = "0.22"

# Data & persistence
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.31", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }

# Utilities
anyhow = "1"
copypasta = "0.10"
log = "0.4"
env_logger = "0.10"
once_cell = "1"
notify = "6.1"  # Credentials file watcher
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
RENDER_FPS=20 KEEP_BLOCKS=50 cargo run
```

**RPC timeouts**:
```bash
RPC_TIMEOUT_MS=15000 POLL_CHUNK_CONCURRENCY=2 cargo run
```

**Search not finding results**:
- Ensure SQLite history has been populated (run for a few minutes first)
- Check query syntax matches filter grammar

---

Built with ❤️ using Ratatui, Tokio, and Rust. Designed for NEAR Protocol monitoring.
