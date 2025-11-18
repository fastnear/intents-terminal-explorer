#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  NEARx Alpha Preflight"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo
echo "[alpha] Running fmt + basic checks..."
cd "$ROOT"
cargo fmt --all

# Clippy for all targets with all features
echo "[alpha] Running clippy (all targets, all features)..."
cargo clippy --all-targets --all-features -- -D warnings

echo
echo "[alpha] Building TUI (native)..."
cargo build --bin nearx --features native

echo
echo "[alpha] Building web (wasm)..."
"$ROOT/tools/build-web.sh"

echo
echo "[alpha] Building Tauri shell..."
cd "$ROOT/tauri-workspace"
cargo tauri build

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  ✅ Preflight OK!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "Quick test commands:"
echo "  TUI:   cargo run --bin nearx --features native"
echo "  Web:   ./tools/web-dev.sh   # http://localhost:4173"
echo "  Tauri: cd tauri-workspace && cargo tauri dev"
echo
