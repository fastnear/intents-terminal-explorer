# NEARx E2E Tests

End-to-end tests for the NEARx Tauri desktop application using Selenium WebDriver and tauri-driver.

## Overview

These tests validate the complete desktop app integration including:

- **Deep link handling** - `nearx://` protocol routing
- **OAuth callback flow** - Token persistence and URL scrubbing
- **Clipboard integration** - Platform-specific copy/paste via Tauri plugin
- **Keyboard & mouse navigation** - Tab cycling, focus management
- **Rendering health** - Canvas sizing, viewport filling
- **Storage persistence** - localStorage operations

## Prerequisites

### System Dependencies

**macOS:**
```bash
# Note: macOS WKWebView doesn't support WebDriver
# Run these tests on Linux/Windows or test the web build with Playwright
```

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get update
sudo apt-get install -y webkit2gtk-driver xvfb
```

**Windows:**
```bash
# Install EdgeDriver matching your Edge version
# Or use msedgedriver-tool
```

### Rust Tools

```bash
# Install tauri-driver (WebDriver server for Tauri apps)
cargo install tauri-driver --locked
```

### Node.js

```bash
# Node 18+ required
node --version  # Should be >= 18.0.0

# Install test dependencies
cd e2e-tests
npm install
```

## Running Tests

### Local Development

```bash
# From e2e-tests directory
npm test

# With verbose output
npm run test:verbose

# Watch mode (reruns on changes)
npm run test:watch
```

### Manual Build + Test

```bash
# 1. Build the app with e2e features
cd ../tauri-workspace
cargo tauri build --debug --no-bundle --features e2e

# 2. Run tauri-driver in separate terminal
tauri-driver

# 3. Run tests in another terminal
cd ../e2e-tests
npm test
```

### CI Environment (Linux with xvfb)

```bash
# Build app
cd tauri-workspace
cargo tauri build --debug --no-bundle --features e2e

# Run tests with virtual display
cd ../e2e-tests
xvfb-run -a npm test
```

## Test Structure

```
e2e-tests/
├── package.json          # Dependencies (mocha, chai, selenium-webdriver)
├── test/
│   └── smoke.spec.mjs    # Main E2E test suite
└── README.md             # This file
```

### Test Suites

1. **Rendering & Layout** - Verifies canvas renders and fills viewport
2. **OAuth Router** - Tests callback handling, token persistence, URL scrubbing
3. **Deep Link Bridge** - Validates `nearx://` protocol handling via test IPC
4. **Clipboard Integration** - Tests copy/paste roundtrip with Tauri plugin
5. **Keyboard & Mouse Navigation** - Checks Tab navigation, focus management
6. **Storage & State** - Verifies localStorage operations
7. **Error Handling** - Ensures graceful degradation on invalid input

## Test API (NEARxTest Bridge)

The app exposes a `window.NEARxTest` API when built with `--features e2e`:

```javascript
// Route tracking
NEARxTest.getLastRoute()           // Returns last navigated route
NEARxTest.waitForRoute(route)      // Async wait for specific route

// Deep links
NEARxTest.getDeepLinkHistory()     // Returns array of received deep links
NEARxTest.clearDeepLinkHistory()   // Clear history

// Clipboard
NEARxTest.copyFocused()            // Trigger copy of focused pane

// OAuth
NEARxTest.getToken()               // Get token from localStorage
NEARxTest.setToken(token)          // Set token

// Keyboard simulation
NEARxTest.pressKey('Tab')          // Simulate key press

// Cursor state
NEARxTest.cursorIsPointer()        // Check if cursor is hover state
```

## Tauri Test Commands

The app exposes test-only IPC commands when built with `e2e` feature:

```javascript
// Emit deep link event (bypasses OS registration)
await window.__TAURI__.invoke('nearx_test_emit_deeplink', {
  url: 'nearx://v1/tx/HASH'
})

// Get last route (alternative to NEARxTest)
await window.__TAURI__.invoke('nearx_test_get_last_route')

// Clear storage (localStorage + sessionStorage)
await window.__TAURI__.invoke('nearx_test_clear_storage')
```

## Debugging

### View tauri-driver logs

```bash
# Run tauri-driver in foreground to see logs
tauri-driver
```

### Enable verbose test output

```bash
npm run test:verbose
```

### Check app logs

The Tauri app logs to console when built in debug mode. Logs include:

- `🧪 [E2E-TEST]` - Test command execution
- `🟢 [HANDLE-URLS]` - Deep link processing
- `🔴/🟠/🟡/🟢/🔵/🟣/🟤/⚪/⚫` - Deep link waterfall logging

### Interactive debugging

```bash
# Build app with e2e features
cargo tauri build --debug --no-bundle --features e2e

# Run app manually (not via tests)
./tauri-workspace/src-tauri/target/debug/nearx-tauri

# Open DevTools (auto-opens in debug builds)
# Cmd+Option+I (macOS) or F12 (Windows/Linux)

# Test commands in console
await window.__TAURI__.invoke('nearx_test_emit_deeplink', {
  url: 'nearx://v1/tx/TEST'
})

window.NEARxTest.getDeepLinkHistory()
```

## Platform Support

| Platform | WebDriver | Status | Notes |
|----------|-----------|---------|-------|
| **Linux** | WebKitWebDriver | ✅ Supported | Use webkit2gtk-driver + xvfb |
| **Windows** | EdgeDriver | ✅ Supported | Use msedgedriver matching Edge version |
| **macOS** | - | ❌ Not supported | WKWebView lacks WebDriver support |

For macOS development, use Playwright to test the web build (`trunk serve`).

## CI Integration

See `.github/workflows/e2e.yml` for GitHub Actions configuration.

Key points:
- Runs on `ubuntu-22.04` (Linux)
- Installs webkit2gtk-driver and xvfb
- Builds with `--features e2e`
- Runs tests under `xvfb-run -a`

## Troubleshooting

### Error: "tauri-driver not found"

```bash
cargo install tauri-driver --locked
```

### Error: "Application binary not found"

```bash
# Ensure you've built with correct features
cd tauri-workspace
cargo tauri build --debug --no-bundle --features e2e

# Check binary exists
ls -la src-tauri/target/debug/nearx-tauri
```

### Error: "Connection refused to localhost:4444"

```bash
# Ensure tauri-driver is running
tauri-driver &

# Give it time to start
sleep 2

# Run tests
npm test
```

### Tests hang or timeout

- Check that xvfb is running (Linux)
- Increase timeout in test file: `this.timeout(120000)` for 2 minutes
- Check app logs for crashes or errors

### Deep link events not received

- Verify `e2e` feature is enabled in build
- Check that test commands are registered (look for `🧪 [E2E-TEST]` in logs)
- Ensure `nearx-test-bridge.js` is loaded in the HTML

## Further Reading

- [Tauri WebDriver Testing Guide](https://v2.tauri.app/develop/tests/webdriver/introduction/)
- [Selenium WebDriver Docs](https://www.selenium.dev/documentation/webdriver/)
- [Mocha Test Framework](https://mochajs.org/)
- [Chai Assertion Library](https://www.chaijs.com/)
