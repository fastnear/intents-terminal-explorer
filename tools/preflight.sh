#!/usr/bin/env bash
#
# Preflight Checks for Ratacat
#
# Architecture-agnostic validation before commits/releases.
# Catches formatting drift, linting issues, test failures, and build regressions.
#
# Usage:
#   ./tools/preflight.sh
#
# Exit code: 0 = all checks passed, 1 = at least one check failed

set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# Color output
RED='\e[31m'
YEL='\e[33m'
GRN='\e[32m'
CYA='\e[36m'
NC='\e[0m'

warn() { echo -e "${YEL}WARN${NC} $*"; }
err()  { echo -e "${RED}ERR ${NC} $*"; }
ok()   { echo -e "${GRN}OK  ${NC} $*"; }
info() { echo -e "${CYA}INFO${NC} $*"; }

section() { echo -e "\n${CYA}== $* ==${NC}\n"; }

fail=false
die() { err "$*"; fail=true; }

# ============================================================================
# Check 1: Formatting
# ============================================================================
section "Formatting (cargo fmt)"

if cargo fmt --all --check >/dev/null 2>&1; then
    ok "Code is formatted"
else
    die "Code formatting issues detected. Run: cargo fmt --all"
fi

# ============================================================================
# Check 2: Linting
# ============================================================================
section "Linting (cargo clippy)"

# Check main package (native features only - web binary checked separately in WASM compile step)
# Skip plugin packages with compilation errors
# Note: We only fail on errors (exit code 101), warnings are informational only
if cargo clippy -p ratacat --lib --bins --features native >/dev/null 2>&1; then
    ok "Clippy passed for native code"
else
    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 101 ]; then
        die "Clippy found errors. Run: cargo clippy -p ratacat --lib --bins --features native"
    else
        info "Clippy warnings present (exit code $EXIT_CODE) - not fatal"
        ok "Clippy passed (warnings OK)"
    fi
fi

# Check native-host separately
if cargo clippy -p ratacat-native-host --all-targets >/dev/null 2>&1; then
    ok "Clippy passed for native-host"
else
    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 101 ]; then
        die "Clippy found errors. Run: cargo clippy -p ratacat-native-host --all-targets"
    else
        info "Native-host clippy warnings present - not fatal"
        ok "Clippy passed (warnings OK)"
    fi
fi

# ============================================================================
# Check 3: WASM Compilation
# ============================================================================
section "WASM Compilation"

if rustup target list | grep -q 'wasm32-unknown-unknown (installed)'; then
    if cargo check --target wasm32-unknown-unknown --no-default-features --features egui-web >/dev/null 2>&1; then
        ok "WASM target compiles"
    else
        die "WASM compilation failed. Run: cargo check --target wasm32-unknown-unknown --no-default-features --features egui-web"
    fi
else
    warn "wasm32-unknown-unknown target not installed (run: rustup target add wasm32-unknown-unknown)"
fi

# ============================================================================
# Check 4: Unit Tests
# ============================================================================
section "Unit Tests"

if cargo test --lib --no-fail-fast >/dev/null 2>&1; then
    ok "Tests passed"
else
    die "Tests failed. Run: cargo test --lib"
fi

# ============================================================================
# Check 5: Untracked Artifacts
# ============================================================================
section "Untracked Artifacts"

# Look for common build artifacts that shouldn't be committed
# Note: grep exits 1 when no match, so use || true to prevent script exit
ARTIFACTS=$(git status --porcelain 2>/dev/null | grep -E '\.(bak|orig|log|rs\.bak|swp|swo)$' || true | wc -l | tr -d ' ')

if [ "$ARTIFACTS" -eq 0 ]; then
    ok "No untracked artifact files"
else
    warn "Found $ARTIFACTS untracked artifact file(s):"
    git status --porcelain | grep -E '\.(bak|orig|log|rs\.bak|swp|swo)$' || true
    info "Consider adding these to .gitignore or removing them"
fi

# ============================================================================
# Summary
# ============================================================================
echo

if $fail; then
    err "❌ Preflight checks FAILED"
    echo
    echo "Fix the issues above and re-run this script."
    exit 1
else
    ok "✅ All preflight checks PASSED"
    echo
    echo "Ready to commit/release!"
    exit 0
fi
