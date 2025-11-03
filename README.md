# Ratacat - NEAR Blockchain Transaction Viewer

**Version 0.4.0** · Monitor NEAR Protocol blocks and transactions from a unified Rust codebase that targets native terminals, the web, a Tauri desktop shell, and a browser extension.

## Quick Start (Copy/Paste Friendly)

### 0. Prerequisites (run once per machine)
```bash
rustup toolchain install 1.89.0
rustup target add wasm32-unknown-unknown --toolchain 1.89.0

# Warm the cache so offline builds succeed
cargo fetch --locked \
  --target x86_64-unknown-linux-gnu \
  --target wasm32-unknown-unknown
cargo fetch --locked --manifest-path tauri-workspace/src-tauri/Cargo.toml
cargo fetch --locked --manifest-path native-host/Cargo.toml

# Helper CLIs used below
cargo install --locked trunk
cargo install --locked tauri-cli
```

### 1. Native Terminal UI (recommended)
```bash
cargo build --locked --bin ratacat --features native --release
SOURCE=rpc \
  NEAR_NODE_URL=https://rpc.mainnet.fastnear.com/ \
  ./target/release/ratacat
```

### 2. Web App (WASM via Trunk)
```bash
# Serve locally
TRUNK_BUILD_ARGS="--locked" trunk serve

# Produce a release bundle in dist/
TRUNK_BUILD_ARGS="--locked" trunk build --release
```
> If you see `can't find crate for 'std'`, rerun `rustup target add wasm32-unknown-unknown --toolchain 1.89.0`.

### 3. Tauri Desktop App
```bash
# Native messaging helper (bundled into the app)
cargo build --locked --release --manifest-path native-host/Cargo.toml

# Desktop shell
cd tauri-workspace
cargo tauri dev              # Live reload
cargo tauri build --bundles app   # Production bundle
```

### 4. Browser Extension + Native Host Bridge
```bash
# Build the native host binary (needed for deep links)
cargo build --locked --release --manifest-path native-host/Cargo.toml

# Package extensions (run from repo root)
cd extension
zip -r ../ratacat-chrome-ext.zip manifest.chrome.json background.js content.js
zip -r ../ratacat-firefox-ext.zip manifest.firefox.json background.js content.js

# Install the unpacked extension in Chrome/Firefox and point
# native-host/com.ratacat.native.json to the built binary.
```

## Features at a Glance
- 3-pane Ratatui interface with keyboard-driven navigation.
- Live RPC polling with optional WebSocket breakout support.
- WASM build runs the same TUI in-browser via egui + WebGL.
- Tauri desktop shell with deep-link handling and native messaging.
- Browser extension launches deep links directly into the desktop app.

## Docs & Support
- **One-page checklist**: [QUICK_START.md](./QUICK_START.md)
- **QA checklist**: [QA_CHECKLIST.md](./QA_CHECKLIST.md)
- **Architecture & implementation notes**: [CLAUDE.md](./CLAUDE.md)

Issues or questions? File them on the repository tracker so we can help quickly.
