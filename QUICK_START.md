# Ratacat Quick Start

One-page reference for building and testing all components.

## ✅ Prerequisites

```bash
rustup toolchain install 1.89.0
rustup target add wasm32-unknown-unknown --toolchain 1.89.0

cargo install --locked trunk       # required for `trunk build` / `trunk serve`
cargo install --locked tauri-cli   # provides the `cargo tauri` subcommand
```

## 🚀 Build Everything

```bash
# 1. Native messaging host
cd native-host
cargo build --release

# 2. Tauri desktop app
cd ../tauri-workspace
cargo tauri build --bundles app

# 3. Web app (WASM)
cd ..
# Pass --locked through to cargo so Trunk honors Cargo.lock
TRUNK_BUILD_ARGS="--locked" trunk build --release

# 4. Extension packages
cd extension
zip -r ../ratacat-chrome-ext.zip manifest.chrome.json background.js content.js
zip -r ../ratacat-firefox-ext.zip manifest.firefox.json background.js content.js
```

## 🧪 Test

```bash
# Run unit tests
cd tauri-workspace/src-tauri
cargo test --lib deeplink

# Test native host standalone
cd ../../native-host
echo '{"type":"hello","requested_version":1}' | cargo run
# Should output: {"type":"hello","version":1}

# Test deep link (macOS)
open "near://tx/abc123"
```

## 📦 Install

```bash
# Install Tauri app (macOS)
cp -r tauri-workspace/target/release/bundle/macos/Ratacat.app /Applications/

# Load Chrome extension (unpacked)
# 1. Navigate to: chrome://extensions/
# 2. Enable "Developer mode"
# 3. Click "Load unpacked"
# 4. Select: extension/ directory
# 5. Copy the extension ID

# Update extension ID in lib.rs
# Edit: tauri-workspace/src-tauri/src/lib.rs line 280
# Replace: "REPLACE_WITH_DEV_EXTENSION_ID" with copied ID
# Rebuild: cd tauri-workspace && cargo tauri build
# Reinstall: cp -r target/release/bundle/macos/Ratacat.app /Applications/
```

## 🔍 Verify

```bash
# Check native messaging manifest installed
ls -la ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/
# Should see: com.ratacat.native.json

cat ~/Library/Application\ Support/Google/Chrome/NativeMessagingHosts/com.ratacat.native.json
# Verify: "path" points to Ratacat.app/Contents/Resources/.../ratacat-native-host

# Check URL scheme registered
/usr/libexec/PlistBuddy -c "Print CFBundleURLTypes" \
  /Applications/Ratacat.app/Contents/Info.plist
# Should see: URLSchemes = (near)
```

## 🧩 Test End-to-End

```bash
# 1. Install everything (see above)

# 2. Open Chrome and navigate to a NEAR tx page
#    Example: https://nearblocks.io/txns/abc123

# 3. Click the purple "Open in Ratacat" button (bottom-right)

# 4. Verify in browser console (F12):
#    Connected to native host v1
#    Operation succeeded: open_deep_link

# 5. Verify Tauri app launches and displays the transaction
```

## 🐛 Debug

```bash
# Enable debug logging (native host)
RUST_LOG=debug cargo run

# Check browser console
# F12 → Console tab
# Look for: "Connected to native host v1"

# Check native messaging logs (macOS)
tail -f ~/Library/Logs/Chrome/NativeMessaging/stderr.log

# Verify deep link works directly
open "near://tx/test123"
# App should launch immediately
```

## 📚 Full Documentation

- **Setup Guide**: [EXTENSION_SETUP.md](./EXTENSION_SETUP.md)
- **Implementation Summary**: [PE_PASS_SUMMARY.md](./PE_PASS_SUMMARY.md)
- **Architecture**: [CLAUDE.md](./CLAUDE.md)

## 🆘 Troubleshooting

| Issue | Solution |
|-------|----------|
| "Native host has exited" | Rebuild native host + Tauri app |
| Extension ID mismatch | Update lib.rs:280 + rebuild Tauri |
| Deep link doesn't work | Check URL scheme: `/usr/libexec/PlistBuddy ...` |
| Button doesn't appear | Check content.js loaded: Browser DevTools → Sources |
| No "Connected to host" | Check manifest path: `cat ~/Library/.../com.ratacat.native.json` |

---

**Last Updated**: October 23, 2025
