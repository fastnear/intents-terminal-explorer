# Chapter 3: Configuration

Configuration is loaded with the following priority: **CLI args > Environment variables > Defaults**

## Configuration Methods

### 1. Environment Variables (recommended for persistent settings)
```bash
# Set in .env file (see .env.example for all options)
SOURCE=rpc
NEAR_NODE_URL=https://rpc.mainnet.fastnear.com/
RENDER_FPS=30
```

### 2. Command Line Arguments (recommended for temporary overrides)
```bash
# View all available options
./nearx --help

# Override specific settings
./nearx --source rpc --render-fps 60 --keep-blocks 200

# Short form for source
./nearx -s rpc
```

## Key Configuration Options

### Data Source
- `SOURCE` / `--source, -s`: Data source (`ws` or `rpc`)
  - `ws`: WebSocket connection to Node server (real-time, low latency)
  - `rpc`: Direct NEAR RPC polling (more reliable, works without Node)
  - Default: `ws`

### WebSocket Settings (when `SOURCE=ws`)
- `WS_URL` / `--ws-url`: WebSocket endpoint
  - Default: `ws://127.0.0.1:63736`
- `WS_FETCH_BLOCKS` / `--ws-fetch-blocks`: Fetch full block data
  - Default: `true`

### RPC Settings (when `SOURCE=rpc`)
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

### Archival RPC (for historical block fetching)
- `ARCHIVAL_RPC_URL` / `--archival-rpc-url`: Archival RPC endpoint
  - Optional: enables unlimited backward navigation through blockchain history
  - Fetches historical blocks on-demand when navigating beyond cache
  - Requires `FASTNEAR_AUTH_TOKEN` for best performance
  - Examples:
    - FastNEAR Mainnet: `https://archival-rpc.mainnet.fastnear.com`
    - FastNEAR Testnet: `https://archival-rpc.testnet.fastnear.com`
  - Loading state shows "⏳ Loading block #..." during 1-2 second fetch
  - Fetched blocks are cached automatically for seamless navigation

### UI Performance
- `RENDER_FPS` / `--render-fps`: Target FPS (1-120)
  - Default: `30`
  - Lower = less CPU, Higher = smoother updates
- `RENDER_FPS_CHOICES` / `--render-fps-choices`: Available FPS options (comma-separated)
  - Default: `20,30,60`
  - Cycle with Ctrl+O during runtime
- `KEEP_BLOCKS` / `--keep-blocks`: Blocks in memory (10-10000)
  - Default: `100`

### Persistence
- `SQLITE_DB_PATH` / `--sqlite-db-path`: Database path
  - Default: `./nearx_history.db`

### Credentials (for owned account filtering)
- `NEAR_CREDENTIALS_DIR`: Credentials directory
  - Default: `$HOME/.near-credentials`
- `NEAR_NETWORK`: Network subdirectory
  - Default: `mainnet`
  - Options: `mainnet`, `testnet`, `betanet`

### Default Filtering
- `WATCH_ACCOUNTS` / `--watch-accounts`: Comma-separated account list (simple filtering)
  - Default: `intents.near`
  - Example: `alice.near,bob.near,contract.near`
  - Automatically builds `acct:` filter for each account
  - Takes precedence over `DEFAULT_FILTER`
- `DEFAULT_FILTER` / `--default-filter`: Advanced filter syntax (power users)
  - Only used if `WATCH_ACCOUNTS` is not set
  - Default: `acct:intents.near`
  - Supports full filter grammar: `signer:`, `receiver:`, `action:`, `method:`, `raw:`

## Configuration Validation

All configuration values are validated on startup with helpful error messages:

```bash
# Invalid FPS range
$ ./nearx --render-fps 200
Error: RENDER_FPS must be in range [1, 120], got 200

# Invalid URL scheme
$ ./nearx --near-node-url example.com
Error: NEAR_NODE_URL must start with ws://, wss://, http://, or https://

# Invalid poll interval
$ POLL_INTERVAL_MS=50000 ./nearx
Error: POLL_INTERVAL_MS must be in range [100, 10000], got 50000
```

## Common Configuration Examples

### Development with local Node server
```bash
SOURCE=ws cargo run --bin nearx --features native
```

### Production mainnet monitoring
```bash
./nearx \
  --source rpc \
  --near-node-url https://rpc.mainnet.fastnear.com/ \
  --fastnear-auth-token your_token_here \
  --keep-blocks 200
```

### Low-resource environment (e.g., Raspberry Pi)
```bash
./nearx \
  --source rpc \
  --render-fps 10 \
  --keep-blocks 50 \
  --poll-interval-ms 2000 \
  --poll-chunk-concurrency 2
```

### High-performance local monitoring
```bash
SOURCE=ws RENDER_FPS=60 KEEP_BLOCKS=500 cargo run --bin nearx --features native
```

### Web/Tauri Token Configuration

For web and Tauri builds, the token handling uses a **priority fallback chain**:

1. **OAuth token** (highest priority): User's authentication token from localStorage
2. **Compile-time token** (fallback): `FASTNEAR_API_TOKEN_WEB` or `FASTNEAR_API_TOKEN` environment variable baked into WASM at build time

Example:
```bash
# Set token before building
export FASTNEAR_API_TOKEN_WEB="your-token-here"

# Build web bundle
./tools/build-web.sh

# Or for Tauri
cd tauri-workspace
cargo tauri build
```

## Configuration File Template

For complete documentation of all options, see `.env.example` in the project root. This file contains:
- All available environment variables
- Detailed descriptions for each option
- Example values and valid ranges
- Platform-specific notes

## Next Steps

- For user interface guide, see [Chapter 2: User Guide](02-user-guide.md)
- For architecture details, see [Chapter 4: Architecture](04-architecture.md)
- For building instructions, see [Chapter 5: Building](05-building.md)