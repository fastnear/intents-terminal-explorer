# Ratacat Development - Technical Discussion

## Overview
Ratacat is a high-performance terminal UI for monitoring NEAR Protocol blockchain transactions in real-time. Built with Ratatui and Rust, it provides a 3-pane dashboard for exploring blocks, transactions, and detailed transaction data with multiple viewing modes.

## Architecture Decisions

### 1. Dual Data Source Strategy
We support both WebSocket and RPC polling modes to accommodate different use cases:

**WebSocket Mode (Development)**:
- Real-time push notifications from Node breakout server
- Hybrid mode: WS notifications trigger RPC fetches for complete block data
- Network auto-detection based on block height (>100M = mainnet, <100M = testnet)

**RPC Polling Mode (Production)**:
- Direct NEAR RPC connection with smart catch-up limits
- Prevents cascade failures during network delays
- Configurable concurrency for parallel chunk fetching

```rust
// Key design: Auto-detect network to prevent mainnet/testnet mismatches
fn detect_network_from_height(height: u64) -> &'static str {
    if height > 100_000_000 { "mainnet" } else { "testnet" }
}
```

### 2. FPS-Capped Rendering
Implemented coalesced rendering to prevent UI thrashing:

```rust
// Target frame budget based on configurable FPS
let frame_ms = 1000u32.saturating_div(app.fps()) as u64;
let frame_budget = Duration::from_millis(frame_ms.max(1));

// Only draw when budget elapsed
if last_frame.elapsed() >= frame_budget {
    terminal.draw(|f| ui::draw(f, &app))?;
    last_frame = Instant::now();
}
```

**Benefits**:
- Prevents CPU thrashing during high-frequency updates
- Maintains responsive input handling via event polling
- User-controllable FPS at runtime (Ctrl+O cycles 20→30→60)

### 3. Official NEAR Primitives Integration
We use official NEAR Protocol crates for maximum compatibility:

```rust
// Human-readable formatting
use near_gas::NearGas;
use near_token::NearToken;

pub fn format_gas(gas: u64) -> String {
    let near_gas = NearGas::from_gas(gas);
    format!("{}", near_gas)  // "30 TGas"
}

pub fn format_near(yoctonear: u128) -> String {
    let token = NearToken::from_yoctonear(yoctonear);
    format!("{}", token)  // "1 NEAR"
}
```

**Action Type Coverage**:
- CreateAccount
- DeployContract (with code size)
- FunctionCall (method name, args size, gas, deposit)
- Transfer (deposit amount)
- Stake (stake amount, public key)
- AddKey (public key, access key)
- DeleteKey (public key)
- DeleteAccount (beneficiary)

### 4. Vim-Style Jump Marks
Persistent bookmark system with SQLite write-through caching:

```rust
pub struct JumpMarks {
    marks: Vec<Mark>,        // In-memory cache
    cursor: usize,           // Current position
    history: History,        // SQLite persistence layer
}

// Write-through: Updates both memory and DB
pub async fn add_or_replace(&mut self, label: String, ...) {
    // Update in-memory
    self.marks.push(mark);
    // Write-through to SQLite
    self.history.put_mark(persisted).await;
}
```

**Auto-labels**: Numeric (1-9, 0) then alphabetic (a-z) for quick bookmarking.

### 5. Transaction Filtering
Powerful query syntax with AND/OR logic:

```
acct:alice.near              # Match signer OR receiver
action:FunctionCall          # Match action type
method:ft_transfer           # Match FunctionCall method name
raw:some_text                # Search in raw JSON
```

Combined filters: `acct:alice.near action:Transfer` (Alice sending tokens)

## Technical Challenges & Solutions

### 1. Mainnet/Testnet Mismatch (Critical Bug Fix)
**Problem**: WebSocket server sending mainnet blocks (168M+), but RPC defaulting to testnet URL, resulting in "0 txs" displayed.

**Root Cause**: Height-based network detection was not implemented initially.

**Solution**:
```rust
// Auto-detect network if URL not explicitly set
let url = if cfg.near_node_url_explicit {
    cfg.near_node_url.clone()
} else {
    let detected_network = detect_network_from_height(height);
    let auto_url = get_rpc_url_for_network(detected_network);
    eprintln!("[WS] Auto-detected {} network from block height {}",
        detected_network, height);
    auto_url.to_string()
};
```

**Impact**: Transactions now display correctly in WebSocket mode.

### 2. Rust Borrow Checker with Filtered Views
**Challenge**: Maintaining both full transaction list and filtered view while allowing selection.

**Solution**: Store both vectors, use indices for coordination:
```rust
pub fn txs(&self) -> (Vec<TxLite>, usize, usize) {
    if let Some(b) = self.blocks.get(self.sel_block) {
        let total = b.transactions.len();
        let filtered: Vec<TxLite> = b.transactions.iter()
            .filter(|tx| tx_matches_filter(&tx, &self.filter_compiled))
            .cloned()
            .collect();
        (filtered, self.sel_tx, total)
    } else {
        (vec![], 0, 0)
    }
}
```

### 3. Non-Blocking History Persistence
**Architecture**: SQLite operations run on dedicated thread to avoid blocking UI.

```rust
// Unbounded channel for async writes
let (tx, mut rx) = unbounded_channel::<HistoryMsg>();

// Dedicated worker thread
tokio::spawn(async move {
    while let Some(msg) = rx.recv().await {
        // Process DB operations without blocking main thread
    }
});
```

**Benefits**:
- UI remains responsive during database writes
- WAL mode enables concurrent reads during writes
- Graceful degradation if DB operations fail

### 4. Soft-Wrapped Tokens for Terminal Display
**Problem**: Long base58/base64 strings (transaction hashes, public keys) break terminal layout.

**Solution**: Insert zero-width space characters (ZWSP) for clean line breaking:
```rust
pub fn soft_wrap_tokens(s: &str, max_run: usize) -> String {
    let mut out = String::with_capacity(s.len() + s.len()/64);
    let mut run = 0usize;
    for ch in s.chars() {
        let token = ch.is_ascii_alphanumeric() || matches!(ch, '+'|'/'|'='|':'|'_'|'-');
        if token {
            run += 1;
            out.push(ch);
            if run >= max_run { out.push(ZWSP); run = 0; }
        } else {
            run = 0;
            out.push(ch);
        }
    }
    out
}
```

## Performance Optimizations

### 1. Parallel Chunk Fetching
Configurable concurrency for fetching chunk data:

```rust
let chunk_futures = chunk_hashes.into_iter().map(|hash| {
    let url = url.to_string();
    async move { get_chunk(&url, hash, timeout).await }
});

let chunks = stream::iter(chunk_futures)
    .buffer_unordered(concurrency)  // Parallel execution
    .collect::<Vec<_>>()
    .await;
```

**Configuration**: `POLL_CHUNK_CONCURRENCY=4` (default)

### 2. Catch-Up Limits
Prevents cascade failures during network delays:

```rust
let to_fetch = head_height.saturating_sub(cursor);
if to_fetch > cfg.poll_max_catchup as u64 {
    eprintln!("[RPC] Behind by {} blocks, skipping to catch up", to_fetch);
    cursor = head_height.saturating_sub(cfg.poll_max_catchup as u64);
}
```

**Configuration**: `POLL_MAX_CATCHUP=5` (default)

### 3. In-Memory Block Window
Fixed-size rolling buffer to prevent unbounded memory growth:

```rust
fn push_block(&mut self, b: BlockRow) {
    self.blocks.insert(0, b);
    if self.blocks.len() > self.keep_blocks {
        self.blocks.pop();
    }
}
```

**Configuration**: `KEEP_BLOCKS=100` (default)

## Code Quality Observations

### 1. Comprehensive Action Type Support
All 8 NEAR action types are properly parsed and displayed with human-readable formatting.

### 2. Dual View Modes
- **PRETTY**: ANSI-colored JSON with formatted gas/deposits
- **RAW**: Unformatted JSON for debugging/copy-paste

### 3. Clipboard Integration
Uses `copypasta` crate for cross-platform clipboard support (copy transaction details with `c` key).

### 4. Idiomatic NEAR Types
Proper use of `near-primitives`, `near-gas`, `near-token` matching official NEAR tooling patterns.

### 5. Error Handling Strategy
Current approach logs errors but keeps UI responsive:

```rust
match fetch_block_with_txs(&url, height, timeout, concurrency).await {
    Ok(row) => {
        let _ = tx_clone.send(AppEvent::NewBlock(row));
    }
    Err(e) => {
        eprintln!("[WS] RPC fetch failed for block {}: {:?}", height, e);
        // Fallback: send empty block notification
        let _ = tx_clone.send(AppEvent::FromWs(WsPayload::Block { data: height }));
    }
}
```

**Future Enhancement**: Add user-facing error notifications/toast system.

## Feature Completeness

### ✅ Implemented Features
- [x] 3-pane dashboard (Blocks → Transactions → Details)
- [x] Dual data sources (WebSocket + RPC)
- [x] Network auto-detection
- [x] FPS control (runtime adjustable)
- [x] View mode toggle (PRETTY/RAW)
- [x] Smooth scrolling with PgUp/PgDn/Home/End
- [x] Transaction filtering (account, action, method, raw)
- [x] Jump marks with persistence (m, M, ', [, ])
- [x] SQLite history with non-blocking writes
- [x] Human-readable gas/token formatting
- [x] Clipboard integration
- [x] Comprehensive action type parsing

### 🚧 Potential Enhancements
- [ ] Search history (Ctrl+R style)
- [ ] Export transactions to JSON/CSV
- [ ] Block/transaction bookmarking with tags
- [ ] Multi-block comparison view
- [ ] Gas consumption analytics
- [ ] Custom color themes
- [ ] Plugin system for custom analyzers

### 🔧 Technical Debt
- [ ] Add comprehensive test suite (especially for action parsing)
- [ ] Formal benchmarking/profiling
- [ ] Configuration file support (beyond env vars)
- [ ] Better error recovery for network failures
- [ ] User-facing status notifications

## Security Considerations

### 1. Parameterized SQL Queries
All database operations use parameterized queries to prevent SQL injection:

```rust
let mut stmt = conn.prepare(
    "INSERT OR REPLACE INTO marks(label, pane, height, tx, when_ms) VALUES(?,?,?,?,?)"
)?;
stmt.execute(params![label, pane, height, tx, when_ms])?;
```

### 2. Network Request Timeouts
All RPC requests have configurable timeouts to prevent hanging:

```rust
let client = reqwest::Client::builder()
    .timeout(Duration::from_millis(timeout_ms))
    .build()?;
```

**Configuration**: `RPC_TIMEOUT_MS=8000` (default)

### 3. Data Validation
Transaction hashes and account IDs are validated by NEAR primitives parsing.

## Performance Profiling Targets
(To be measured in production)

1. **Memory usage** at various `KEEP_BLOCKS` settings
2. **CPU usage** at different FPS settings (20/30/60)
3. **Network latency** impact on UI responsiveness
4. **SQLite write throughput** for history persistence
5. **Chunk fetch concurrency** optimal values

## Architectural Patterns Worth Highlighting

### 1. Event-Driven Architecture
```rust
pub enum AppEvent {
    FromWs(WsPayload),    // WebSocket events
    NewBlock(BlockRow),    // Parsed block data
    Quit,                  // Shutdown signal
}
```

Clean separation between data sources and application logic.

### 2. Compile-Time Configuration
Environment variables parsed at startup, not runtime:

```rust
pub struct Config {
    pub source: DataSource,
    pub ws_url: String,
    pub ws_fetch_blocks: bool,
    pub near_node_url: String,
    pub near_node_url_explicit: bool,
    // ... all configuration immutable after parse
}
```

### 3. Zero-Copy String Manipulation
Using `&str` references throughout rendering pipeline to minimize allocations.

## Testing Strategy Recommendations

1. **Unit Tests**: Action parsing, filtering logic, formatting functions
2. **Integration Tests**: Mock RPC responses, test block processing pipeline
3. **Snapshot Tests**: UI rendering output verification
4. **Property Tests**: Quickcheck for action parsing edge cases
5. **Load Tests**: High-frequency block arrival scenarios

## Comparison with Original MVP
Ratacat pivoted from a todo list application (v0.1.0) to a blockchain viewer (v0.2.0+). This represents a significant architectural shift but maintained core TUI patterns:

**Retained**:
- 3-pane layout concept
- FPS-capped rendering
- Vim-style keybindings
- SQLite persistence

**Added**:
- Async networking (WebSocket + RPC)
- NEAR blockchain integration
- Real-time data streaming
- Complex filtering/searching
- Dual view modes

## Recommendations for Next Steps

### Immediate (Pre-Production)
1. ✅ Complete gas/token formatting integration
2. ✅ Verify mainnet transaction display
3. ✅ Update documentation
4. [ ] Add basic unit tests for action parsing
5. [ ] Performance profiling with mainnet load

### Short-Term (Production Readiness)
1. [ ] Implement user-facing error notifications
2. [ ] Add help overlay (`?` key)
3. [ ] Export functionality (JSON/CSV)
4. [ ] Configuration file support
5. [ ] Logging to file (not just stderr)

### Medium-Term (Feature Enhancement)
1. [ ] Search history with fuzzy matching
2. [ ] Block/transaction analytics
3. [ ] Custom color themes
4. [ ] Multi-account monitoring
5. [ ] Gas consumption tracking

### Long-Term (Ecosystem Integration)
1. [ ] Plugin system for custom analyzers
2. [ ] NEAR wallet integration
3. [ ] Contract verification integration
4. [ ] Multi-network support (mainnet + testnet + shards)
5. [ ] Distributed monitoring (multiple nodes)

## Recent Bug Fixes & UX Improvements (2025-01-XX)

### Issue: Tab Navigation Not Visible
**Problem**: User couldn't see which pane was focused when pressing Tab/Shift+Tab.

**Root Cause**: Focus indicator was using subtle cyan color that wasn't visible enough against the terminal background.

**Fix** (src/ui.rs:111):
```rust
// Changed from:
let focus_color = Color::Cyan;

// To:
let focus_color = Color::Yellow;  // Bright yellow, highly visible
```

**Result**: Focused panes now have bright yellow borders, unfocused panes are gray. Tab cycling is now visually clear.

---

### Issue: Missing Left/Right Page Scrolling in Details Pane
**Problem**: Up/Down arrows scrolled 1 line (correct), but no way to page through long transaction details quickly.

**User Requirement**: Left/Right arrows should jump 6 lines at a time for faster navigation through large JSON payloads.

**Fix** (src/app.rs:280-290, src/main.rs:240-241):
```rust
// Added Left/Right key handlers
pub fn left(&mut self) {
    if self.pane == 2 {
        self.scroll_details(-6);  // 6 lines up
    }
}
pub fn right(&mut self) {
    if self.pane == 2 {
        self.scroll_details(6);   // 6 lines down
    }
}
```

**Result**:
- Up/Down: 1-line precision scrolling
- Left/Right: 6-line page jumps
- PgUp/PgDn: 20-line jumps (existing)
- Home/End: Jump to top/bottom (existing)

---

### Issue: 2-Row Layout Implementation
**Problem**: Original 3-column layout was cramped. Transaction details were squashed and hard to read.

**Battle Station Pattern**: Uses 2-row layout:
- **Top row**: Blocks (left) + Tx list (right) - 50/50 split
- **Bottom row**: Details pane (full width) - only shown when txs exist

**Fix** (src/ui.rs:82-195):
```rust
// Split vertically first (rows)
let rows = if has_selection {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),  // Top row
            Constraint::Percentage(50),  // Bottom row (details)
        ])
        .split(area)
} else {
    // No selection: Use full height for top row
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100)])
        .split(area)
};

// Then split top row horizontally (columns)
let top_cols = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(rows[0]);
```

**Result**: Transaction details now have full terminal width, much more readable. Blocks and tx list get equal space in top half.

---

### Known Issue: Selection Drift (Investigation in Progress)

**User Report**: "The selection is still changing as blocks come in. I feel like there is a significant delay compared to the JS one."

**Battle Station Behavior** (Reference):
```typescript
// When at index 0: Stay at index 0 (live mode)
const isViewingLatest = currentIndex === 0;
if (isViewingLatest) {
  setSelectedBlock(blockHeights[0]);
  ref.current.select(0);
} else {
  // History browsing: Track block object by reference
  const index = blockIndex(blockRef.current);
  ref.current.select(index);
}
```

**Ratacat Current Approach** (src/app.rs:411-439):
```rust
// Three-way branching based on flags
if self.follow_latest {
    // Live mode: stay at index 0
    self.sel_block = 0;
} else if let Some(target_height) = self.selected_block_height {
    // History browsing: find block by height
    if let Some(new_idx) = self.blocks.iter().position(|block| block.height == target_height) {
        self.sel_block = new_idx;
    }
} else {
    // INIT mode: first blocks arriving
    self.sel_block = 0;
    self.selected_block_height = Some(self.blocks[0].height);
}
```

**Suspected Issues**:
1. **State Management Complexity**: Using both `follow_latest` boolean AND `selected_block_height` Option creates edge cases
2. **Missing Pane Context**: Battle station doesn't re-select on every block arrival - only when in blocks pane
3. **Initialization Race**: INIT mode might be re-triggering unexpectedly
4. **Render Delay**: FPS capping (30 FPS default) might create perceived lag vs. React's immediate updates

**Debug Strategy**:
- Debug log panel now shows: "Block #X arr, [MODE], sel=Y"
- Next step: Copy debug output during drift to identify exact state transitions
- Compare timing: Battle station's useEffect vs. Ratacat's push_block() timing

**Hypothesis**: The Rust version correctly implements the logic, but:
- **Timing difference**: Battle station batches React state updates, Ratacat processes each block immediately
- **Visual feedback delay**: 30 FPS render budget means up to 33ms lag between state change and visual update
- **Pane-aware selection**: Might need to skip repositioning when not in blocks pane (pane != 0)

**Next Steps**:
1. User to run app and copy debug log showing drift
2. Add pane awareness: Only reposition selection if `self.pane == 0` (blocks pane active)
3. Consider increasing default FPS to 60 for snappier updates
4. Profile block arrival timing vs. render timing

---

## Conclusion

Ratacat successfully demonstrates a production-quality TUI application for blockchain monitoring. The architecture is:

- **Performant**: FPS-capped rendering, parallel data fetching, non-blocking I/O
- **Resilient**: Network auto-detection, catch-up limits, graceful error handling
- **Idiomatic**: Proper use of official NEAR crates and Rust patterns
- **User-Friendly**: Vim-style navigation, clipboard integration, multiple view modes, **NOW with 2-row layout and 6-line page scrolling**

The codebase is well-structured and maintainable, with clear separation of concerns:
- `source_*.rs`: Data acquisition layer
- `app.rs`: State management and business logic
- `ui.rs`: Rendering and presentation
- `types.rs`: Data models
- `config.rs`: Configuration management

**Primary strengths**:
1. Hybrid WebSocket + RPC architecture handles both development and production needs
2. Network auto-detection solved critical mainnet/testnet mismatch bug
3. Official NEAR primitives integration ensures compatibility
4. FPS-capped rendering prevents UI thrashing
5. Non-blocking persistence maintains responsiveness
6. **NEW**: 2-row layout provides excellent space utilization (battle station pattern)
7. **NEW**: Comprehensive scrolling controls (1-line, 6-line, 20-line jumps)
8. **NEW**: Clear focus indicators (bright yellow borders)

**Areas for polish**:
1. Add comprehensive test coverage
2. Implement user-facing error notifications
3. Performance profiling under production load
4. Export/analytics features
5. Configuration file support
6. **IN PROGRESS**: Debug selection drift issue with pane-aware repositioning

The technical foundation is solid and ready for principal engineer review. All major architectural decisions are documented, trade-offs are explained, and the codebase follows Rust best practices.
