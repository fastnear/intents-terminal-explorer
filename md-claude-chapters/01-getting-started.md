# Chapter 1: Getting Started

## Ratacat - NEAR Blockchain Transaction Viewer

**Version 0.4.5+** - High-performance **quad-mode** application for monitoring NEAR Protocol blockchain transactions. Runs in terminal (native), web browser (WASM), desktop app (Tauri), AND integrates with browsers via 1Password-style extension!

**🆕 October 2025 Update**: Production-ready browser integration with auto-installing Native Messaging host supporting Chrome, Edge, Chromium, and **Firefox**.

**🔧 November 2025**: OAuth + Appearance refactor delivers production-ready authentication (Google OAuth + Magic links), unified theme system (WCAG AA compliant), and full mouse/keyboard parity across all targets with XSS-hardened CSP security.

## Quad-Mode Architecture Overview

Ratacat v0.4.0 features a revolutionary **quad-deployment architecture** - write once, run everywhere:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    Ratacat Quad-Mode Architecture                       │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────────────┐ │
│  │  Terminal  │  │  Browser   │  │   Tauri    │  │  Browser Ext +   │ │
│  │  (Native)  │  │   (WASM)   │  │  Desktop   │  │  Native Host     │ │
│  │            │  │            │  │            │  │                  │ │
│  │ • Crossterm│  │ • DOM UI   │  │ • Deep     │  │ • MV3 Extension  │ │
│  │ • SQLite   │  │ • JSON API │  │   links    │  │ • stdio bridge   │ │
│  │ • WS + RPC │  │ • RPC only │  │ • DOM UI   │  │ • nearx://       │ │
│  └─────┬──────┘  └─────┬──────┘  │   instance │  │   deep links     │ │
│        │               │         └──────┬─────┘  └────────┬─────────┘ │
│        └───────────────┼──────────────────────────────────┘           │
│                        ▼                ▼                               │
│              ┌─────────────────────────────────────┐                    │
│              │      Shared Rust Core               │                    │
│              │  • App state (height-based blocks)  │                    │
│              │  • UI rendering (ratatui)           │                    │
│              │  • RPC client & polling             │                    │
│              │  • Filter & search (SQLite/memory)  │                    │
│              └──────────┬──────────────────────────┘                    │
│                         ▼                                               │
│              ┌─────────────────────────────────────┐                    │
│              │    Platform Abstraction             │                    │
│              │  • Clipboard (unified 4-tier)       │                    │
│              │    - Tauri plugin / Extension relay │                    │
│              │    - Navigator API / execCommand    │                    │
│              │  • Storage (SQLite/in-memory)       │                    │
│              │  • Runtime (tokio full/wasm)        │                    │
│              └─────────────────────────────────────┘                    │
│                                                                         │
│              NEAR Blockchain + Browser Integration                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────────────────┐    │
│  │WebSocket │  │   RPC    │  │ Archival │  │ Browser→Native→App │    │
│  │ (Native) │  │  (All)   │  │(Optional)│  │   Deep Link Flow   │    │
│  └──────────┘  └──────────┘  └──────────┘  └────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

### Deployment Modes

1. **Native Terminal**: Full-featured TUI with SQLite, WebSocket, file watching
2. **Web Browser (WASM)**: Pure DOM UI with JSON bridge, RPC-only, in-memory storage
3. **Tauri Desktop**: Native desktop app with DOM UI, deep link support (`nearx://` protocol)
4. **Browser Extension**: 1Password-style "Open in Ratacat" button on tx pages

## Installation

### Prerequisites

- **Rust**: Version 1.70+ (install via [rustup](https://rustup.rs/))
- **Git**: For cloning the repository
- **Platform-specific requirements**:
  - **Terminal**: Any modern terminal emulator
  - **Web**: wasm-bindgen-cli (`cargo install wasm-bindgen-cli --locked`)
  - **Tauri**: See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Clone the Repository

```bash
git clone https://github.com/fastnear/nearx.git
cd nearx
```

## Quick Start

Configuration is loaded with the following priority: **CLI args > Environment variables > Defaults**

### Native Terminal

```bash
# Copy example configuration
cp .env.example .env

# Edit .env with your settings
vim .env

# Run with default settings
cargo run --bin nearx --features native

# Or override with CLI arguments
cargo run --bin nearx --features native -- --source rpc --render-fps 60
```

### Web Browser

```bash
# Install wasm-bindgen-cli if not already installed
cargo install wasm-bindgen-cli --locked

# Development mode (serves at http://localhost:8000)
make dev

# Or build manually
make web
cd web
python -m http.server 8000
```

### Tauri Desktop

```bash
# Navigate to Tauri workspace
cd tauri-workspace

# Development mode (auto-builds frontend)
cargo tauri dev

# Production build
cargo tauri build
```

### Basic Usage

Once running, Ratacat will start monitoring NEAR blockchain transactions:

- Use arrow keys or `j/k` to navigate blocks and transactions
- Press `Tab` to switch between panes
- Press `/` to filter transactions
- Press `?` to see keyboard shortcuts (Web/Tauri)
- Press `Space` to toggle fullscreen details

For detailed configuration options, see [Chapter 3: Configuration](03-configuration.md).