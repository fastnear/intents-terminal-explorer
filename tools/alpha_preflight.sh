#!/usr/bin/env bash
set -euo pipefail

say() { printf "%b\n" "$*"; }
ok()  { say "✅ $*"; }
err() { say "❌ $*"; exit 1; }
warn(){ say "⚠️  $*"; }

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)

cd "$root"

say "━━ format + clippy ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo fmt --all
cargo clippy --features native --bin nearx -q
cargo clippy --target wasm32-unknown-unknown --no-default-features --features egui-web --bin nearx-web -q
( cd tauri-workspace && cargo clippy -q )
ok "clippy clean"

say "━━ build all targets ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
cargo build --features native --bin nearx
trunk build --release
( cd tauri-workspace && cargo tauri build )
ok "builds OK"

say "━━ invariants (grep) ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
g() { if ! git --no-pager grep -n -- "$1" ${2:-}; then warn "missing: $1"; return 1; fi; }

# Theme tokens export
g "pub mod tokens;" "src/theme.rs" || err "theme::tokens not exported"

# Web: DPR snap, Tab policy, clipboard guard, pretty JSON truncation, filter rows
g "pixels_per_point"      "src/bin/nearx-web.rs" || err "DPR snap missing"
g "Key::Tab"              "src/bin/nearx-web.rs" || err "Tab handling missing"
g "has_focus(self.filter_id)" "src/bin/nearx-web.rs" || err "'c' guard missing"
g "MAX_CHARS"             "src/bin/nearx-web.rs" || err "details soft truncation missing"
g "desired_rows("         "src/bin/nearx-web.rs" || err "filter rows not tokenized"

# Tauri plumbing: opener + deep-link emit + open_external
g "tauri_plugin_opener"   "tauri-workspace/src-tauri/src/main.rs" || err "opener plugin missing"
g "nearx://open"          "tauri-workspace/src-tauri/src/main.rs" || err "deep-link emit missing"
g "open_external"         "tauri-workspace/src-tauri/src/main.rs" || err "open_external command missing"

# Touch ID bridge command present (optional, but we suggested it)
if git --no-pager grep -n "touch_id(" tauri-workspace/src-tauri/src/main.rs >/dev/null; then
  ok "Touch ID command present"
else
  warn "Touch ID command not found (ok for alpha if deferred)"
fi

# Trunk public_url for Tauri
if grep -nE '^\s*public_url\s*=\s*"\./"' Trunk.toml >/dev/null; then
  ok "Trunk.toml public_url = ./"
else
  err "Trunk.toml public_url should be \"./\" for Tauri"
fi

# Tauri CSP includes ipc:
if git --no-pager grep -n "connect-src 'self' ipc:" tauri-workspace/src-tauri/tauri.conf.json >/dev/null; then
  ok "CSP connect-src includes ipc:"
else
  err "tauri.conf.json connect-src must include ipc:"
fi

# TUI parity: thick border + always-colored details + clipboard respects filter
g "BorderType::Thick"     "src/ui.rs" || err "Focused thick border missing in TUI"
g "colorize_json"         "src/ui.rs" || err "Details not forced to colored in TUI"
g "InputMode::Filter"     "src/bin/nearx.rs" || warn "Filter mode guard not found"

ok "invariants OK"

say "━━ quick tips ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
say "• Web:  trunk serve --open   → no WASM traps; click block/tx; Tab cycles panes;"
say "• Tauri: cargo tauri dev     → OAuth opens system browser; deep links emit;"
say "• TUI:  cargo run --bin nearx → thick focus border; details always colored;"
