#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "" ]]; then
  echo "usage: tools/release.sh <version>   # e.g. 0.9.0"
  exit 1
fi
VER="$1"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

echo "==> Formatting & linting"
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings

echo "==> Tests"
cargo test --workspace

echo "==> Preflight"
tools/preflight.sh

echo "==> Web build (dev server uses trunk; no artifacts needed here)"
echo "     (Optional) trunk build --release  # if you publish static web bundle"

echo "==> Tauri WASM (no-modules) build for webview"
cargo build \
  --target wasm32-unknown-unknown \
  --bin nearx-web \
  --no-default-features \
  --features egui-web \
  --release

WASM="target/wasm32-unknown-unknown/release/nearx-web.wasm"
if [[ ! -f "$WASM" ]]; then
  echo "error: expected $WASM"
  exit 2
fi

LOCK_VER="$(awk '/name = "wasm-bindgen"/{f=1} f && /version =/{gsub(/"/,""); print $3; exit}' Cargo.lock)"
CLI_VER="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}')"
if [[ "$LOCK_VER" != "$CLI_VER" ]]; then
  echo "warn: wasm-bindgen CLI ($CLI_VER) != Cargo.lock ($LOCK_VER)"
fi

mkdir -p dist-egui
wasm-bindgen \
  --target no-modules \
  --out-dir dist-egui \
  --out-name nearx-web \
  "$WASM"

ls -l dist-egui/nearx-web.js dist-egui/nearx-web_bg.wasm >/dev/null

echo "==> Tauri desktop build"
pushd tauri-workspace/src-tauri >/dev/null
cargo tauri build
popd >/dev/null

echo "==> Version/tag"
git add -A
git commit -m "NEARx v$VER: Web/Tauri parity, deep links, debug harness, polish"
git tag -s "v$VER" -m "NEARx v$VER"
git push origin HEAD --tags

echo "==> Done"
echo "Artifacts:"
echo "  - Tauri bundles under: tauri-workspace/src-tauri/target/release/bundle/"
echo "  - Webview WASM bundle for Tauri: dist-egui/"
echo
echo "Release notes template:"
cat <<EOF
## NEARx v$VER

### Highlights
- Web/Tauri visual parity with TUI (flat theme, crisp DPR, zero chrome)
- Deep links (nearx://) cold/warm + single-instance forwarding
- Standardized keymap (Tab/Shift+Tab, Space, c, wheel, dbl-click)
- Debug overlay + filterable logs (?nxdebug=…)

### Fixes
- ES module vs no-modules loader pitfalls documented + preflight guards
- Blue-clear slab fixed (eframe extreme_bg_color = theme.bg)
- Wheel normalized (3 lines per notch)

### Build
- Web dev: trunk serve --open
- Tauri:   cargo tauri dev / build
EOF
