#!/bin/bash
# Preflight checks for alpha release
# Validates all build targets compile successfully

set -e

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 NEARx Alpha Preflight Checks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

echo ""
echo "✓ Checking Native TUI build..."
cargo check --bin nearx --features native

echo ""
echo "✓ Checking Web (WASM) build..."
cargo check --target wasm32-unknown-unknown --no-default-features --features egui-web --bin nearx-web

echo ""
echo "✓ Checking Tauri Desktop build..."
(cd tauri-workspace/src-tauri && cargo check)

echo ""
echo "✓ Running cargo clippy (Native + Tauri)..."
cargo clippy --features native --bin nearx -- -D warnings
(cd tauri-workspace/src-tauri && cargo clippy -- -D warnings)

echo ""
echo "✓ Running cargo fmt check..."
cargo fmt --all -- --check

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ All preflight checks passed!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "Next steps:"
echo "  npm run e2e              # Run Playwright smoke tests"
echo "  trunk serve --open       # Test Web target"
echo "  cd tauri-workspace && cargo tauri dev  # Test Desktop target"
echo ""
