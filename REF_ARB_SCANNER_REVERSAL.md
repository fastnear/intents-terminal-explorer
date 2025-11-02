# Reversal Guide: Bringing ref-arb-scanner Back into Main Codebase

This document explains how to reverse the workspace separation and bring the ref-arb-scanner code back into the main ratacat codebase.

## Quick Reversal (5 minutes)

```bash
# 1. Copy files back to main src/
cp ref-arb-scanner/src/arb_config.rs src/
cp ref-arb-scanner/src/arb_engine.rs src/
cp ref-arb-scanner/src/ref_finance_client.rs src/
cp ref-arb-scanner/src/price_discovery.rs src/
cp ref-arb-scanner/src/risk_manager.rs src/
cp ref-arb-scanner/src/execution_engine.rs src/
cp ref-arb-scanner/src/slippage.rs src/
cp ref-arb-scanner/src/main.rs src/bin/ref-arb-scanner.rs

# 2. Update main.rs imports (change ref_arb_scanner:: back to ratacat::)
sed -i '' 's/use ref_arb_scanner::/use ratacat::/g' src/bin/ref-arb-scanner.rs

# 3. Restore modules in src/lib.rs
# Replace the NOTE comment (lines 62-64) with:
#[cfg(feature = "native")]
pub mod arb_engine;

#[cfg(feature = "native")]
pub mod ref_finance_client;

#[cfg(feature = "native")]
pub mod price_discovery;

#[cfg(feature = "native")]
pub mod arb_config;

#[cfg(feature = "native")]
pub mod slippage;

#[cfg(feature = "native")]
pub mod risk_manager;

#[cfg(feature = "native")]
pub mod execution_engine;

# 4. Restore binary in Cargo.toml
# Replace the NOTE comment (lines 26-28) with:
[[bin]]
name = "ref-arb-scanner"
path = "src/bin/ref-arb-scanner.rs"
required-features = ["native"]

# 5. Remove workspace member
# Edit Cargo.toml line 36, remove "ref-arb-scanner" from members array

# 6. Delete workspace directory
rm -rf ref-arb-scanner/

# 7. Verify build
cargo build --bin ref-arb-scanner --features native --release
```

## Manual Reversal Steps

If you prefer manual editing:

### Step 1: Restore Source Files

Move files from `ref-arb-scanner/src/` back to main `src/`:
- `arb_config.rs` → `src/arb_config.rs`
- `arb_engine.rs` → `src/arb_engine.rs`
- `ref_finance_client.rs` → `src/ref_finance_client.rs`
- `price_discovery.rs` → `src/price_discovery.rs`
- `risk_manager.rs` → `src/risk_manager.rs`
- `execution_engine.rs` → `src/execution_engine.rs`
- `slippage.rs` → `src/slippage.rs`
- `main.rs` → `src/bin/ref-arb-scanner.rs`

### Step 2: Fix Import Paths

In `src/bin/ref-arb-scanner.rs`, change:
```rust
use ref_arb_scanner::arb_config::ArbConfig;
use ref_arb_scanner::arb_engine::LightningArbEngine;
// ... etc
```

To:
```rust
use ratacat::arb_config::ArbConfig;
use ratacat::arb_engine::LightningArbEngine;
// ... etc
```

### Step 3: Restore lib.rs Module Declarations

In `src/lib.rs`, replace lines 62-64 (the NOTE comment) with:

```rust
// Arbitrage engine (native-only)
#[cfg(feature = "native")]
pub mod arb_engine;

#[cfg(feature = "native")]
pub mod ref_finance_client;

#[cfg(feature = "native")]
pub mod price_discovery;

#[cfg(feature = "native")]
pub mod arb_config;

#[cfg(feature = "native")]
pub mod slippage;

#[cfg(feature = "native")]
pub mod risk_manager;

#[cfg(feature = "native")]
pub mod execution_engine;
```

### Step 4: Restore Binary Declaration

In `Cargo.toml`, replace lines 26-28 (the NOTE comment) with:

```toml
[[bin]]
name = "ref-arb-scanner"
path = "src/bin/ref-arb-scanner.rs"
required-features = ["native"]
```

### Step 5: Remove Workspace Member

In `Cargo.toml` line 36, remove `"ref-arb-scanner"` from the members array:

```toml
[workspace]
resolver = "2"
members = [
  "tauri-workspace/src-tauri",
  "native-host"
  # Removed: "ref-arb-scanner"
]
```

### Step 6: Clean Up

Delete the workspace directory:
```bash
rm -rf ref-arb-scanner/
```

### Step 7: Verify

```bash
cargo build --bin ref-arb-scanner --features native --release
cargo run --bin ref-arb-scanner --features native -- --help
```

## Why This Is Easy to Reverse

1. **No external dependencies**: The arbitrage scanner only depends on standard crates, nothing custom
2. **Clean module boundaries**: All `crate::` imports stay valid in both locations
3. **Single binary**: Only one entry point to update
4. **Feature-gated**: Already wrapped in `#[cfg(feature = "native")]`, so easy to re-add

## Verification Checklist

After reversal:
- [ ] `cargo build --bin ref-arb-scanner --features native --release` succeeds
- [ ] `cargo build --bin ratacat --features native --release` still works
- [ ] No warnings from cargo about missing modules
- [ ] `cargo run -p ref-arb-scanner` fails (workspace member gone)
- [ ] `cargo run --bin ref-arb-scanner --features native` works

## Original State

Before separation:
- Location: `src/arb_*.rs`, `src/ref_finance_client.rs`, etc.
- Binary: `src/bin/ref-arb-scanner.rs`
- Total files: 8 (7 modules + 1 binary)
- Total lines: ~2,313

After separation:
- Location: `ref-arb-scanner/src/`
- Workspace member: Independent crate
- Same 8 files, same line count
- Buildable with: `cargo build -p ref-arb-scanner`

Both states are functionally identical - just different organizational structures!
