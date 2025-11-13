# NEARx Deep Link Routing

**Version**: 1.0
**Status**: Implemented across all targets (TUI, Web, Tauri)

## Overview

NEARx supports versioned deep link routing with the `nearx://` URL scheme. Deep links allow direct navigation to specific blockchain entities (transactions, blocks, accounts) from external sources like browsers, scripts, or other applications.

**Key Features:**
- Versioned routes (`nearx://v1/*`) for forward compatibility
- Cross-platform support (native terminal, web browser, Tauri desktop)
- Maps to existing explorer UI (no new screens required)
- Integration with filter DSL for complex queries

**Important Note on URL Scheme:**
- The official URL scheme is `nearx://` (registered in Tauri and documented throughout)
- For backward compatibility, legacy `near://` URLs are automatically converted to `nearx://`
- The normalization happens transparently in the Tauri backend (`lib.rs::normalize()`)
- This ensures old links and integrations continue to work without changes

## Route Format Specification

### V1 Routes

All V1 routes follow the pattern: `nearx://v1/<type>/<identifier>[?query]`

#### Transaction Route

```
nearx://v1/tx/<tx_hash>
```

**Behavior:**
- Focus transactions pane (pane 1)
- Set filter to transaction hash
- Display transaction details when found

**Example:**
```
nearx://v1/tx/ABC123XYZ789
```

#### Block Route

```
nearx://v1/block/<height>
```

**Behavior:**
- Focus blocks pane (pane 0)
- Set filter to `height:<height>`
- Display block and all transactions

**Example:**
```
nearx://v1/block/150000000
```

#### Account Route

```
nearx://v1/account/<account_id>
```

**Behavior:**
- Focus transactions pane (pane 1)
- Set filter to `acct:<account_id>`
- Shows all transactions involving the account (signer OR receiver)

**Example:**
```
nearx://v1/account/alice.near
```

#### Home Route

```
nearx://v1/home
```

**Behavior:**
- Clear all filters
- Return to auto-follow mode (newest blocks)
- Reset to default view

### Alternative Formats

NEARx's router supports multiple URL formats for flexibility:

| Format | Example | Use Case |
|--------|---------|----------|
| Full URL | `nearx://v1/tx/ABC123` | OS-level deep links (Tauri) |
| Hash-based | `#/v1/tx/ABC123` | Web browser routing |
| Encoded hash | `#/deeplink/nearx%3A%2F%2Fv1%2Ftx%2FABC123` | Tauri→Web bridge |
| Path-only | `/v1/tx/ABC123` | Web app internal routing |

All formats are parsed by the same `nearx::router::parse()` function.

## Usage by Platform

### 1. Native Terminal (TUI)

**Binary**: `nearx`

**CLI Arguments:**

```bash
# Transaction lookup
./nearx nearx://v1/tx/ABC123

# Block navigation
./nearx nearx://v1/block/150000000

# Account filtering
./nearx nearx://v1/account/alice.near

# Multiple arguments (first valid route wins)
./nearx --source rpc nearx://v1/tx/ABC123
```

**Implementation:**
- Routes parsed from `std::env::args()` after app initialization
- Applied via `app.apply_route()` before main event loop
- Only first valid route is processed

**Code Location:** `src/bin/nearx.rs` (lines 83-97)

### 2. Web Browser (WASM)

**Binary**: `nearx-web`

**URL Hash Routing:**

```bash
# Direct hash navigation
https://nearx.example.com/#/v1/tx/ABC123

# Home page with hash change
https://nearx.example.com/
# Then: window.location.hash = '#/v1/block/150000000'
```

**JavaScript Integration:**

```javascript
// Navigate to transaction
window.location.hash = '#/v1/tx/ABC123';

// Navigate to account
window.location.hash = '#/v1/account/alice.near';

// Return home
window.location.hash = '#/v1/home';
```

**Implementation:**
- WASM binary monitors `window.location.hash` changes in update loop
- Hash changes trigger route parsing and application
- Previous hash tracked to avoid duplicate processing
- Supports both direct hash URLs and runtime hash changes

**Code Location:** `src/bin/nearx-web.rs` (lines 240-257)

### 3. Tauri Desktop App

**Binary**: `nearx-tauri` (in NEARx.app bundle)

**Operating System Integration:**

```bash
# macOS
open 'nearx://v1/tx/ABC123'

# Linux (if .desktop file configured)
xdg-open 'nearx://v1/tx/ABC123'

# Windows (if registry configured)
start nearx://v1/tx/ABC123
```

**Tauri Event Flow:**

```
1. OS triggers deep link → Tauri plugin captures URL
2. handle_urls() emits "nearx://open" event with raw URL
3. web/deep_link.js listens for event
4. JavaScript updates window.location.hash = '#/deeplink/<encoded>'
5. WASM detects hash change and parses route
6. app.apply_route() updates UI state
```

**Implementation:**
- Tauri backend emits `"nearx://open"` events (via `app.emit()`)
- JavaScript bridge (`web/deep_link.js`) listens and updates hash
- Web frontend parses encoded deep link from hash
- Uses encodeURIComponent/decodeURIComponent for URL safety

**Code Locations:**
- Event emitter: `tauri-workspace/src-tauri/src/lib.rs` (lines 252-257)
- JavaScript bridge: `web/deep_link.js`
- WASM parser: `src/bin/nearx-web.rs` + `src/router.rs`

## Testing Deep Links

### Testing Native Terminal

```bash
# Build release binary
cargo build --bin nearx --features native --release

# Test transaction route
./target/release/nearx nearx://v1/tx/ABC123

# Test block route
./target/release/nearx nearx://v1/block/150000000

# Test account route
./target/release/nearx nearx://v1/account/alice.near

# Test with RPC source
SOURCE=rpc ./target/release/nearx nearx://v1/tx/ABC123
```

**Expected Behavior:**
- App launches with route applied
- Appropriate pane is focused (0=blocks, 1=txs, 2=details)
- Filter is set according to route
- Debug log shows: `"Applied deep link route from CLI: nearx://v1/tx/ABC123"`

### Testing Web Browser

```bash
# Start development server
trunk serve

# Open in browser with routes:
# http://127.0.0.1:8080/#/v1/tx/ABC123
# http://127.0.0.1:8080/#/v1/block/150000000
# http://127.0.0.1:8080/#/v1/account/alice.near
```

**Browser Console Testing:**

```javascript
// Open DevTools console (F12)

// Test transaction route
window.location.hash = '#/v1/tx/ABC123';

// Test block route
window.location.hash = '#/v1/block/150000000';

// Test account route
window.location.hash = '#/v1/account/alice.near';

// Test home route
window.location.hash = '#/v1/home';
```

**Expected Behavior:**
- UI updates immediately when hash changes
- Appropriate pane is focused
- Filter applied correctly
- Browser console shows: `"Applied deep link route from hash: #/v1/tx/ABC123"`

### Testing Tauri Desktop

```bash
# Build Tauri app
cd tauri-workspace
cargo tauri dev

# In separate terminal, test deep links:
open 'nearx://v1/tx/ABC123'
open 'nearx://v1/block/150000000'
open 'nearx://v1/account/alice.near'
```

**Debug Logging:**

Check for the following log sequence (in DevTools console or logs):

```
🟢 [HANDLE-URLS] Processing 1 URL(s)
🟢 [HANDLE-URLS] URL[0]: "nearx://v1/tx/ABC123"
🟢 [HANDLE-URLS] Emitted 'nearx://open' event with URL: nearx://v1/tx/ABC123
[deep_link] Received deep link: nearx://v1/tx/ABC123
[deep_link] Updated location hash: #/deeplink/nearx%3A%2F%2Fv1%2Ftx%2FABC123
Applied deep link route from hash: #/deeplink/nearx%3A%2F%2Fv1%2Ftx%2FABC123
```

**Expected Behavior:**
- App launches if not already running (single-instance enforcement)
- Deep link flows through Tauri → JS bridge → WASM
- UI updates to show transaction/block/account
- Full debug waterfall visible in logs

## Implementation Details

### Router Module (`src/router.rs`)

**Core Types:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteV1 {
    Tx { hash: String },
    Block { height: u64 },
    Account { id: String },
    Home,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    V1(RouteV1),
}
```

**Parser Function:**

```rust
pub fn parse(raw: &str) -> Option<Route> {
    // Handles multiple input formats
    // Returns None for unsupported versions or invalid URLs
}
```

**Test Coverage:**

```bash
# Run router tests
cargo test router::tests
```

Tests cover:
- Full URLs (`nearx://v1/...`)
- Hash routes (`#/v1/...`)
- Encoded deep links (`#/deeplink/<encoded>`)
- Path-only routes (`/v1/...`)
- Invalid routes (returns None)
- Future versions (returns None)

### App Integration (`src/app.rs`)

**Navigation API:**

```rust
impl App {
    /// Set pane directly (0=blocks, 1=txs, 2=details)
    pub fn set_pane_direct(&mut self, pane: usize);

    /// Apply a deep link route to the current app state
    pub fn apply_route(&mut self, route: &crate::router::Route);
}
```

**Route Application Logic:**

| Route | Pane | Filter Applied | Additional Actions |
|-------|------|----------------|-------------------|
| Tx | 1 (txs) | `<hash>` | Focus transactions |
| Block | 0 (blocks) | `height:<height>` | Focus blocks |
| Account | 1 (txs) | `acct:<account_id>` | Focus transactions |
| Home | - | Clear filter | Return to auto-follow |

### Tauri JavaScript Bridge (`web/deep_link.js`)

**Event Listener:**

```javascript
window.__TAURI__.event.listen('nearx://open', function(event) {
    const url = event.payload;
    const encoded = encodeURIComponent(url);
    window.location.hash = '#/deeplink/' + encoded;
});
```

**Safety:**
- No-op if Tauri APIs not available (safe for plain web builds)
- Error handling for malformed events
- Comprehensive logging for debugging

## Future Integration: Browser Extension

### Native Messaging Host Template

For "Open in NEARx" browser extension integration, implement a native messaging host:

**Host Manifest (JSON):**

```json
{
  "name": "com.nearx.native_host",
  "description": "NEARx Native Messaging Host",
  "path": "/usr/local/bin/nearx-native-host",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://YOUR_EXTENSION_ID/"
  ]
}
```

**Host Binary (Rust):**

```rust
// Pseudo-code for native-host binary
use std::io::{stdin, stdout, Read, Write};

fn main() {
    loop {
        // Read message length (4 bytes, native endian)
        let mut len_bytes = [0u8; 4];
        stdin().read_exact(&mut len_bytes)?;
        let len = u32::from_ne_bytes(len_bytes) as usize;

        // Read JSON message
        let mut msg_bytes = vec![0u8; len];
        stdin().read_exact(&mut msg_bytes)?;
        let msg: serde_json::Value = serde_json::from_slice(&msg_bytes)?;

        // Extract deep link URL from extension
        if let Some(url) = msg.get("url").and_then(|v| v.as_str()) {
            // Open NEARx with deep link
            #[cfg(target_os = "macos")]
            std::process::Command::new("open")
                .arg(url)
                .spawn()?;

            // Send success response
            let response = serde_json::json!({"status": "ok"});
            send_message(&response)?;
        }
    }
}

fn send_message(msg: &serde_json::Value) -> std::io::Result<()> {
    let json = serde_json::to_vec(msg)?;
    let len = (json.len() as u32).to_ne_bytes();
    stdout().write_all(&len)?;
    stdout().write_all(&json)?;
    stdout().flush()?;
    Ok(())
}
```

**Extension Message (JavaScript):**

```javascript
// Browser extension sends:
chrome.runtime.sendNativeMessage(
    'com.nearx.native_host',
    { url: 'nearx://v1/tx/ABC123' },
    function(response) {
        console.log('NEARx opened:', response);
    }
);
```

**Installation:**

```bash
# macOS
cp nearx-native-host /usr/local/bin/
cp com.nearx.native_host.json ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/

# Linux
cp nearx-native-host /usr/local/bin/
cp com.nearx.native_host.json ~/.config/google-chrome/NativeMessagingHosts/

# Windows
copy nearx-native-host.exe C:\Program Files\NEARx\
reg add "HKCU\Software\Google\Chrome\NativeMessagingHosts\com.nearx.native_host" /ve /d "C:\path\to\manifest.json"
```

## Versioning and Forward Compatibility

### Current Version: V1

The V1 route format is stable and supports:
- Transaction lookup by hash
- Block navigation by height
- Account filtering by ID
- Home/reset navigation

### Future Versions

To add new route types in V2:

1. **Add new enum variant:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteV2 {
    Tx { hash: String },
    Block { height: u64 },
    Account { id: String },
    Home,
    // New in V2:
    Receipt { id: String },
    Validator { id: String },
}
```

2. **Update Route enum:**

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Route {
    V1(RouteV1),
    V2(RouteV2), // New version
}
```

3. **Update parser:**

```rust
pub fn parse(raw: &str) -> Option<Route> {
    // ... existing normalization ...

    if path.starts_with("/v1/") {
        parse_v1(&path[4..])
    } else if path.starts_with("/v2/") {
        parse_v2(&path[4..]) // New parser
    } else {
        None
    }
}
```

4. **Update app integration:**

```rust
impl App {
    pub fn apply_route(&mut self, route: &crate::router::Route) {
        match route {
            Route::V1(v1) => { /* existing */ }
            Route::V2(v2) => { /* new handlers */ }
        }
    }
}
```

### Deprecation Policy

- Old versions remain supported indefinitely
- Parser returns `None` for unrecognized versions
- Clients gracefully handle `None` (treat as no-op)
- Breaking changes require new version (V3, V4, etc.)

## Troubleshooting

### Route Not Applied

**Symptom:** Deep link opens app but doesn't navigate

**Debugging:**
1. Check logs for "Applied deep link route" message
2. Verify route format matches specification
3. Test with `nearx::router::parse()` in unit test
4. Check that filter DSL is valid

**Common Issues:**
- Typo in route format (e.g., `/v1/transaction/` instead of `/v1/tx/`)
- Invalid characters in hash/height/account_id
- Missing URL encoding for special characters

### Tauri Bridge Not Working

**Symptom:** macOS `open nearx://...` doesn't update UI

**Debugging:**
1. Check DevTools console for `[deep_link]` logs
2. Verify Tauri plugin emits `"nearx://open"` event
3. Confirm `web/deep_link.js` is loaded (check Network tab)
4. Test JavaScript bridge manually:
   ```javascript
   window.__TAURI__.event.emit('nearx://open', 'nearx://v1/tx/ABC123');
   ```

**Common Issues:**
- `deep_link.js` not included in HTML (missing `<script defer>`)
- Tauri event name mismatch (must be exactly `"nearx://open"`)
- URL encoding issues (bridge handles this automatically)

### Web Hash Routing Not Detecting

**Symptom:** Changing `window.location.hash` doesn't trigger route

**Debugging:**
1. Check browser console for "Applied deep link route from hash" message
2. Verify `last_hash` field is tracking correctly
3. Test with direct URL: `http://localhost:8080/#/v1/tx/ABC123`

**Common Issues:**
- Hash format invalid (must start with `#/v1/` or `#/deeplink/`)
- WASM update loop not running (eframe issue)
- Hash change event not firing (use direct assignment, not `pushState`)

### OS Not Recognizing nearx:// Scheme

**Symptom:** macOS says "No application set to open URL"

**Solution:**
```bash
# Re-register Tauri app
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/Ratacat.app

# Verify registration
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -A 3 "nearx:"
```

**Linux Solution:**
```bash
# Create .desktop file
cat > ~/.local/share/applications/nearx.desktop <<EOF
[Desktop Entry]
Name=NEARx
Exec=/usr/local/bin/nearx %u
Type=Application
MimeType=x-scheme-handler/nearx;
EOF

# Register scheme handler
xdg-mime default nearx.desktop x-scheme-handler/nearx
```

**Windows Solution:**
```batch
REM Register in registry (run as Administrator)
reg add "HKCU\Software\Classes\nearx" /ve /d "URL:NEARx Protocol" /f
reg add "HKCU\Software\Classes\nearx" /v "URL Protocol" /d "" /f
reg add "HKCU\Software\Classes\nearx\shell\open\command" /ve /d "\"C:\Program Files\NEARx\nearx.exe\" \"%1\"" /f
```

## Summary

NEARx's deep link routing system provides:

✅ **Cross-platform support** - TUI, Web, Tauri
✅ **Versioned routes** - Forward-compatible design
✅ **Multiple input formats** - URLs, hashes, paths
✅ **Existing UI integration** - No new screens required
✅ **Comprehensive testing** - Unit tests + manual testing
✅ **Future-ready** - Extension integration template

For questions or contributions, see main README.md and CONTRIBUTING.md.
