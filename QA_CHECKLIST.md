# Ratacat QA Testing Checklist

## Build Verification ✅

> Pre-flight: ensure the pinned toolchain and wasm target are installed
> (`rustup toolchain install 1.89.0` and `rustup target add wasm32-unknown-unknown --toolchain 1.89.0`).

All three build targets verified:

- [x] **Native Terminal** - `cargo build --release` (0.59s)
- [x] **Tauri Desktop** - `cargo build --release --manifest-path tauri-workspace/src-tauri/Cargo.toml` (0.67s)
- [x] **egui-web WASM** - `TRUNK_BUILD_ARGS="--locked" trunk build --release` (2.50s)

> `TRUNK_BUILD_ARGS="--locked"` forwards the `--locked` flag to Cargo so Trunk honors `Cargo.lock`.

---

## egui-web Testing

### 1. Start Development Server

```bash
trunk serve
```

Open: http://localhost:8080

### 2. Visual Indicators Check

**Status Bar (top of page):**
- [ ] Status bar visible at top (dark gray background)
- [ ] Status indicator dot present (pulsing gray initially)
- [ ] Status text shows: "Initializing..."
- [ ] Helper text shows: "Press F12 for debug console"

**Loading Screen:**
- [ ] "Loading Ratacat (egui)" message appears centered
- [ ] Loading message has animated dots (...)
- [ ] Loading screen disappears after ~2 seconds
- [ ] Status bar updates to "WASM loaded, starting RPC..."

**After Connection:**
- [ ] Status indicator turns **green** (solid, no animation)
- [ ] Status text shows: "Connected | N blocks" (N increases over time)

### 3. Browser Console Logs (F12)

#### Startup Sequence:
```
✓ 🦀 Ratacat egui-web config: { rpc: "...", filter: "...", token: "..." }
✓ ╔═══════════════════════════════════════════════════════════╗
✓ ║       🦀 Ratacat egui-web v0.4.0 - Starting Up        ║
✓ ╚═══════════════════════════════════════════════════════════╝
✓ 📋 Startup Diagnostics:
✓   • WASM binary: ratacat-egui-web
✓   • UI framework: egui + egui_ratatui + soft_ratatui
✓   • Font backend: EmbeddedGraphics (8x13 monospace)
✓   • Async runtime: tokio (wasm-compatible subset)
✓   • Log level: Debug (all RPC activity visible)
✓ 🚀 Ratacat egui-web starting
✓ RPC: https://..., Filter: intents.near, Token: ... (from ...)
✓ 🚀 Spawning RPC polling task...
✓ 🎯 Creating App with filter: 'intents.near'
✓ ✅ App created successfully
✓ ✅ RPC task started, calling run_rpc()...
✓ 🚀 RPC polling loop started - endpoint: https://...
```

#### RPC Polling (repeating every 1 second):
```
✓ 📡 RPC loop tick - polling for latest block...
✓ ✅ Got latest block height: 123456789
✓ 🏁 Starting from block height: 123456789    (first time only)
✓ 😴 Sleeping for 1000ms...
✓ ⏰ Woke up from sleep!
```

#### When New Blocks Arrive:
```
✓ 📦 Fetching blocks 123456790 to 123456794 (5 blocks)
✓ 🔔 Sending NewBlock event - height: 123456790, txs: 12
✓ 📥 Received event in update(): NewBlock(BlockRow { height: 123456790, ... })
✓ ✅ Processed 1 events this frame
✓ 📊 App state: 1 blocks in buffer
```

### 4. UI Functionality

#### Canvas Rendering:
- [ ] Black canvas background visible (below status bar)
- [ ] Terminal UI renders using egui (WebGL canvas)
- [ ] Blocks pane shows blocks with transaction counts
- [ ] Transaction hashes pane shows tx IDs
- [ ] Details pane shows JSON data

#### Keyboard Controls:
- [ ] `Tab` - Switch between panes (Blocks → Txs → Details)
- [ ] `↑ / ↓` - Navigate lists
- [ ] `Enter` - Select transaction
- [ ] `/` or `f` - Enter filter mode
- [ ] `Esc` - Clear filter
- [ ] `c` - Copy to clipboard (check toast notification)

#### Filter Functionality:
- [ ] Default filter applied: `acct:intents.near`
- [ ] Blocks panel shows only blocks with matching transactions
- [ ] Filtered count displayed: "Blocks (12 / 100)"
- [ ] Transactions panel shows only matching txs: "Txs (3 / 15)"
- [ ] Filter query visible in filter bar

### 5. Configuration Testing

#### URL Parameters:
```bash
# Test custom RPC endpoint
http://localhost:8080?rpc=https://rpc.testnet.fastnear.com/

# Test auth token
http://localhost:8080?token=YOUR_FASTNEAR_TOKEN

# Test custom filter
http://localhost:8080?filter=alice.near

# Test combined
http://localhost:8080?rpc=https://rpc.mainnet.fastnear.com/&token=YOUR_TOKEN&filter=alice.near
```

**Check:**
- [ ] Console shows correct RPC URL
- [ ] Console shows token (first 8 chars + "...")
- [ ] Console shows filter value
- [ ] Status bar reflects configuration

#### localStorage:
```javascript
// In browser console:
localStorage.setItem('RPC_BEARER', 'your_token_here');
localStorage.setItem('RPC_URL', 'https://rpc.mainnet.fastnear.com/');
// Reload page
```

**Check:**
- [ ] Token persists across page reloads
- [ ] RPC URL persists across page reloads
- [ ] Console shows: "Token: ... (from localStorage)"

### 6. Error Scenarios

#### CORS Errors:
```
❌ Access to fetch at '...' has been blocked by CORS policy
```
**Solution:** Add auth token via `?token=` parameter

#### RPC Timeouts:
```
❌ RPC error: timeout
```
**Check:**
- [ ] Error logged to console
- [ ] Status indicator turns **red** (blinking)
- [ ] Status text shows error message

#### No Network:
- [ ] Polling continues attempting
- [ ] Errors logged but app doesn't crash
- [ ] Recovery when network restored

### 7. Performance Testing

**Metrics to Check:**
- [ ] Initial WASM load < 5 seconds
- [ ] UI renders at 30-60 FPS (smooth scrolling)
- [ ] No console errors or warnings (except deprecations)
- [ ] Memory usage stable (check browser Task Manager)
- [ ] CPU usage < 10% idle, < 30% when scrolling

**Long-Running Test:**
- [ ] Leave running for 5+ minutes
- [ ] Block count increases steadily
- [ ] No memory leaks (check DevTools Memory tab)
- [ ] UI remains responsive

### 8. Browser Compatibility

Test in multiple browsers:
- [ ] **Chrome/Chromium** - Latest version
- [ ] **Firefox** - Latest version
- [ ] **Safari** - Latest version (macOS only)
- [ ] **Edge** - Latest version

---

## Native Terminal Testing

```bash
cargo run --release
```

**Quick Checks:**
- [ ] Builds without warnings
- [ ] Connects to RPC endpoint
- [ ] Shows blocks in terminal UI
- [ ] Keyboard controls work (Tab, arrows, Enter, /, q)
- [ ] Filter works (type `/` and enter query)
- [ ] Clipboard copy works (`c` key)
- [ ] History search works (Ctrl+F)
- [ ] Quit cleanly (q or Ctrl+C)

---

## Tauri Desktop Testing

```bash
cd tauri-workspace
cargo tauri dev
```

**Quick Checks:**
- [ ] App launches in native window
- [ ] Terminal UI renders correctly
- [ ] Deep link support works (`myapp://` protocol)
- [ ] All keyboard shortcuts functional
- [ ] Native window controls work (minimize, maximize, close)
- [ ] App quits cleanly

---

## Known Issues / Expected Behavior

### Normal Behavior:
- **"💤 No new blocks"** - Normal when blockchain is idle or caught up
- **Blocks not immediately visible** - May take 1-2 seconds for first block
- **Filter shows 0 results** - Normal if no matching transactions in current blocks

### Web-Specific Limitations:
- **No SQLite** - History search not available (shows empty results)
- **No file watching** - Owned accounts filter disabled
- **In-memory only** - Marks/history cleared on page reload
- **RPC only** - WebSocket mode not available

---

## Sign-Off

**Tested by:** ___________________
**Date:** ___________________
**Browser:** ___________________
**OS:** ___________________

**Issues Found:**
1.
2.
3.

**Overall Status:** ☐ Pass  ☐ Fail  ☐ Pass with Issues
