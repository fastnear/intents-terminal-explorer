#!/usr/bin/env bash
# Preflight checks for NEARx before opening a PR
# Ensures cross-target consistency, deep link integration, and code quality

set -euo pipefail

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

section() {
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BLUE}$1${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

ok() {
  echo -e "${GREEN}✓${NC} $1"
}

warn() {
  echo -e "${YELLOW}⚠${NC} $1"
}

die() {
  echo -e "${RED}✗${NC} $1"
  exit 1
}

# Move to repo root
cd "$(dirname "$0")/.."

section "1. Build checks"

# Native binary
if ! cargo build --bin nearx --features native --quiet 2>&1; then
  die "Native binary (nearx) failed to compile"
else
  ok "Native binary compiles"
fi

# Web binary
if ! cargo build --bin nearx-web --no-default-features --features egui-web --target wasm32-unknown-unknown --quiet 2>&1; then
  die "Web binary (nearx-web) failed to compile"
else
  ok "Web binary compiles"
fi

# Tauri binary
if ! cargo build --manifest-path tauri-workspace/src-tauri/Cargo.toml --quiet 2>&1; then
  die "Tauri binary failed to compile"
else
  ok "Tauri binary compiles"
fi

section "2. Router tests"

if ! cargo test --lib router::tests --quiet 2>&1; then
  die "Router tests failed"
else
  ok "Router tests pass"
fi

section "3. Deep link integration"

# TUI: router parse called
if ! grep -q "router::parse" src/bin/nearx.rs 2>/dev/null; then
  warn "TUI: router::parse not referenced (CLI deep links may not work)"
else
  ok "TUI: router parse present"
fi

# Web: router parse called
if ! grep -q "router::parse" src/bin/nearx-web.rs 2>/dev/null; then
  warn "Web: router::parse not referenced (hash deep links may not work)"
else
  ok "Web: router parse present"
fi

# Tauri: deep-link plugin declared
if ! grep -q "tauri-plugin-deep-link" tauri-workspace/src-tauri/Cargo.toml 2>/dev/null; then
  warn "Tauri: deep-link plugin not declared"
else
  ok "Tauri: deep-link plugin declared"
fi

# Web: deep_link.js listener
if ! grep -q "nearx://open" web/deep_link.js 2>/dev/null; then
  warn "Web: deep_link.js not listening for nearx://open"
else
  ok "Web: deep_link.js listener present"
fi

section "4. Binary lib imports"

# Binaries must import the lib crate as `nearx::`, not `crate::`
bad_bin_imports=$(git grep -n "use crate::" -- 'src/bin' 2>/dev/null || true)
if [ -n "$bad_bin_imports" ]; then
  die "Binaries should import from lib using 'nearx::', not 'use crate::':
$bad_bin_imports"
else
  ok "Binary imports look correct (nearx::)"
fi

section "5. TODO/FIXME/HACK/debug leftovers"

# Scan for development notes and debug macros
notes=$(git grep -nE '\b(TODO|FIXME|HACK|XXX)\b|todo!\(|unimplemented!\(|dbg!\(' -- 'src' 'tauri-workspace/src-tauri' ':!**/tests/**' 2>/dev/null || true)
if [ -n "$notes" ]; then
  warn "Found notes or stubs to resolve (or whitelist):
$notes"
else
  ok "No obvious leftover TODO/FIXME/HACK or debug macros"
fi

section "6. unwrap/expect scan (informational)"

# Scan for unwrap/expect in library sources (some may be legitimate)
unwraps=$(git grep -nE '\.unwrap\(|\.expect\(' -- 'src' ':!**/tests/**' 2>/dev/null || true)
if [ -n "$unwraps" ]; then
  warn "Found unwrap/expect in library sources (review for fallbacks):
$(echo "$unwraps" | head -20)"
  unwrap_count=$(echo "$unwraps" | wc -l | tr -d ' ')
  if [ "$unwrap_count" -gt 20 ]; then
    echo "  ... and $(($unwrap_count - 20)) more"
  fi
else
  ok "No unwrap/expect found in library sources"
fi

section "7. stdio debug prints"

# Scan for println!/eprintln! (should use log/tracing instead)
prints=$(git grep -nE '\b(eprintln!|println!)\b' -- 'src' 'tauri-workspace/src-tauri' ':!**/tests/**' 2>/dev/null || true)
if [ -n "$prints" ]; then
  warn "Found println!/eprintln! (prefer log:: or remove):
$prints"
else
  ok "No stray println!/eprintln! found"
fi

section "8. Scheme consistency"

# Check that tauri.conf.json uses "nearx" scheme
if ! grep -q '"nearx"' tauri-workspace/src-tauri/tauri.conf.json 2>/dev/null; then
  die "Tauri config should register 'nearx' scheme"
else
  ok "Tauri registers 'nearx' scheme"
fi

# Check that router.rs expects "nearx://" URLs
if ! grep -q 'nearx://' src/router.rs 2>/dev/null; then
  warn "router.rs should parse 'nearx://' URLs"
else
  ok "router.rs parses 'nearx://' URLs"
fi

section "9. Tauri Deep Link System"

# Check for deep-link plugin
if ! grep -q "tauri-plugin-deep-link" tauri-workspace/src-tauri/Cargo.toml 2>/dev/null; then
  warn "Tauri: deep-link plugin not declared"
else
  ok "Tauri: deep-link plugin declared"
fi

# Check for single-instance plugin
if ! grep -q "tauri-plugin-single-instance" tauri-workspace/src-tauri/Cargo.toml 2>/dev/null; then
  warn "Tauri: single-instance plugin not declared (nearx:// from a second process may be dropped)"
else
  ok "Tauri: single-instance plugin declared"
fi

# Check for deep link listener in web bridge
if ! grep -q "nearx://open" web/deep_link.js 2>/dev/null; then
  warn "Web: deep_link.js not listening for nearx://open"
else
  ok "Web: deep_link.js listener present"
fi

# Check for Tauri dist artifacts
if [[ -f tauri-workspace/src-tauri/tauri.conf.json ]]; then
  distdir=$(jq -r '.build.distDir // empty' tauri-workspace/src-tauri/tauri.conf.json 2>/dev/null || echo "")
  if [[ -n "$distdir" ]] && [[ -d "$distdir" ]]; then
    missing=()
    [[ -f "$distdir/nearx-web.js" ]] || missing+=("$distdir/nearx-web.js")
    [[ -f "$distdir/nearx-web_bg.wasm" ]] || missing+=("$distdir/nearx-web_bg.wasm")
    if (( ${#missing[@]} )); then
      warn "Dist artifacts missing for Tauri (did you run wasm-bindgen --target no-modules?):"
      for m in "${missing[@]}"; do
        echo "  - $m"
      done
    else
      ok "Tauri dist artifacts present ($distdir)"
    fi
  fi
fi

section "10. Theme synchronization"

# Check that CSS theme vars exist
if ! [ -f web/theme.css ]; then
  warn "web/theme.css not found (theme may not sync to browser)"
else
  ok "web/theme.css exists"
fi

# Check that Rust theme module exports themes
if ! grep -q "pub struct Theme\|pub enum Theme" src/theme.rs 2>/dev/null; then
  warn "src/theme.rs should export Theme type"
else
  ok "src/theme.rs exports Theme type"
fi

section "11. Theme single-source enforcement"

# No direct ratatui Color usage outside theme/
stray_rat=$(git grep -nE '\bratatui::style::Color::' -- 'src' ':!src/theme.rs' 2>/dev/null || true)
if [ -n "$stray_rat" ]; then
  warn "Direct ratatui::style::Color usage outside theme.rs (route through theme/):"
  echo "$stray_rat"
else
  ok "No stray ratatui Color usage outside theme.rs"
fi

# No direct egui Color32 usage outside theme/
stray_eg=$(git grep -nE '\begui::Color32::' -- 'src' ':!src/theme.rs' 2>/dev/null || true)
if [ -n "$stray_eg" ]; then
  warn "Direct egui::Color32 usage outside theme.rs (route through theme/):"
  echo "$stray_eg"
else
  ok "No stray egui Color32 usage outside theme.rs"
fi

section "12. Web/Tauri bins must apply theme"

# Check that Web binary applies theme
if ! grep -q "theme::eg::apply\|egui_theme.apply" src/bin/nearx-web.rs 2>/dev/null; then
  warn "Web binary missing theme::eg::apply(ctx, &theme) or egui_theme.apply()"
else
  ok "Web binary applies egui theme"
fi

# Check that web/theme.css exists (static CSS vars sufficient for now)
if ! grep -q "^:root {" web/theme.css 2>/dev/null; then
  warn "web/theme.css missing :root CSS variables"
else
  ok "Web CSS variables present in web/theme.css"
fi

# Check Tauri binary (if it exists separately)
if [ -f tauri-workspace/src-tauri/src/bin/nearx-tauri.rs ]; then
  if ! grep -q "theme::eg::apply\|egui_theme.apply" tauri-workspace/src-tauri/src/bin/nearx-tauri.rs 2>/dev/null; then
    warn "Tauri binary missing theme::eg::apply(ctx, &theme)"
  else
    ok "Tauri binary applies egui theme"
  fi
fi

section "13. Keymap documentation"

if [[ -f docs/KEYMAP.md ]]; then
  ok "docs/KEYMAP.md present (standardized shortcuts)"
else
  warn "Missing docs/KEYMAP.md (standardized shortcuts for contributors)"
fi

section "14. wasm-bindgen version parity"

# wasm-bindgen CLI version must match Cargo.lock (prevents white-screen link errors)
lock_ver="$(awk '/name = "wasm-bindgen"/{f=1} f && /version =/{gsub(/"/,""); print $3; exit}' Cargo.lock 2>/dev/null || true)"
cli_ver="$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)"
if [[ -z "$lock_ver" ]]; then
  warn "Cannot detect wasm-bindgen version from Cargo.lock"
elif [[ -z "$cli_ver" ]]; then
  warn "wasm-bindgen CLI not found in PATH (cargo install wasm-bindgen-cli)"
elif [[ "$lock_ver" != "$cli_ver" ]]; then
  warn "wasm-bindgen CLI ($cli_ver) != Cargo.lock ($lock_ver) — build may fail with ES6/module or LinkError"
else
  ok "wasm-bindgen CLI matches Cargo.lock ($cli_ver)"
fi

section "15. HTML loader sanity"

# HTML loader sanity (ESM for trunk; no-modules for Tauri)
if grep -q 'nearx-web.js' index-egui.html 2>/dev/null; then
  if grep -q '<script src="nearx-web.js">' index-egui.html; then
    ok "index-egui.html uses <script src> (no-modules friendly)"
  elif grep -q 'type="module"' index-egui.html; then
    ok "index-egui.html uses ES modules (trunk)"
  else
    warn "index-egui.html script type not explicit; confirm ESM vs no-modules per target"
  fi
else
  warn "nearx-web.js not referenced in index-egui.html (ensure correct loader)"
fi

section "✅ Preflight Complete"

echo ""
echo -e "${GREEN}All critical checks passed!${NC}"
echo ""
echo "Next steps:"
echo "  1. Review any warnings above"
echo "  2. Test deep links manually:"
echo "     - macOS: open 'nearx://v1/tx/ABC123'"
echo "     - Linux: xdg-open 'nearx://v1/tx/ABC123'"
echo "     - Windows: start nearx://v1/tx/ABC123"
echo "  3. Test all three modes:"
echo "     - cargo run --bin nearx"
echo "     - trunk serve --open"
echo "     - cd tauri-workspace && cargo tauri dev"
echo ""
