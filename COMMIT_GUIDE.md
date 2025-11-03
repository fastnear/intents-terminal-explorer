# Commit & PR Guide - Build Verification Checklist

**Last updated**: March 2026  
**Scope**: Ratacat workspace (root crate + plugins + Tauri + native host)

---

## Executive Summary

- ✅ All build targets compile cleanly on the pinned Rust **1.89.0** toolchain (`rust-toolchain.toml`).
- ✅ README build commands (`--locked`, feature-gated) are the single source of truth.
- ✅ Lints (`cargo clippy -D warnings`) stay green once crates.io is reachable.
- ✅ Documentation (README/CLAUDE) reflects the current platform runtime shims.

Use this page as a pre-flight checklist before opening a PR.

---

## Build & Lint Checklist

All commands assume you are in the repository root. They intentionally mirror README sections so reviewers can reproduce results verbatim.

> Ensure the README prerequisites are satisfied (`rustup toolchain install 1.89.0`, `rustup target add wasm32-unknown-unknown --toolchain 1.89.0`, and the `cargo fetch` cache warm-up). Without the wasm target the web build and Trunk pipeline will fail with `can't find crate for 'std'`, and without the fetch step offline builds will error out when registry access is unavailable.

```bash
# Native terminal app
cargo build --locked --bin ratacat --features native --release

# Optional proxy helper
cargo build --locked --bin ratacat-proxy --features proxy --release

# Web/WASM build (matches Trunk.toml)
cargo build --locked --bin ratacat-egui-web \
  --target wasm32-unknown-unknown \
  --no-default-features --features egui-web --release

# Workspace members
cargo build --locked --release --manifest-path tauri-workspace/src-tauri/Cargo.toml
cargo build --locked --release --manifest-path native-host/Cargo.toml

# Lints (run once registry access is available)
cargo clippy --locked --bin ratacat --features native -- -D warnings
cargo clippy --locked --bin ratacat-egui-web --target wasm32-unknown-unknown \
  --no-default-features --features egui-web -- -D warnings

# Format only the files you touched
cargo fmt
```

> ℹ️ Run the README `cargo fetch` commands once while online so all workspace crates (native, wasm, Tauri, native host) are cached locally. After that, you can flip on `CARGO_NET_OFFLINE=true` to confirm no new dependencies were introduced.

---

## Staging & Commit Tips

1. **Audit `Cargo.lock`** whenever you touch `Cargo.toml`. `--locked` builds will fail if the lockfile is stale.
2. **Restrict commits to related changes** (e.g., runtime shim tweak + doc update). Avoid mixing dependency upgrades with feature work unless tightly coupled.
3. **Stage selectively**:
   ```bash
   git add README.md Cargo.toml Cargo.lock src/<touched files>
   git status
   git diff --staged
   ```
4. **Suggested commit message template**:
   ```text
   feat|fix|chore(scope): concise summary
   
   - bullet list of noteworthy changes
   - note any follow-up TODOs if applicable
   ```
5. **Before pushing**:
   - Re-run the build checklist.
   - Ensure docs that reference your change (README, CLAUDE, QUICK_START) are updated.
   - Capture any known offline limitations in the PR body.

---

## Pull Request Expectations

- **Summary**: Reference the README section(s) touched and highlight user-facing impact.
- **Testing**: Paste the exact commands from the checklist (including failures if due to sandbox/offline limits).
- **Screenshots**: Required for UI-affecting changes in web/Tauri builds (use the provided browser container tooling).
- **Follow-ups**: If registry access was blocked, mention that CI should rerun the lint/build steps once connectivity is restored.

With this checklist, reviewers can verify builds without guesswork, and the README stays as the authoritative source for supported targets.
