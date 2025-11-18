# NEARx - NEAR Blockchain Transaction Viewer

**Version 0.4.5+** - High-performance **quad-mode** application for monitoring NEAR Protocol blockchain transactions. Runs in terminal (native), web browser (WASM), desktop app (Tauri), AND integrates with browsers via 1Password-style extension!

**🆕 Latest Updates**:
- JSON streaming with 5000-line truncation for performance
- Pure DOM frontend (no egui/canvas)
- OAuth authentication (Google + Magic links)
- WCAG AA compliant unified theme system
- Two-list architecture with automatic backfill

## Documentation Structure

This documentation has been organized into focused chapters for easier navigation and maintenance:

### Core Documentation

- **[Chapter 1: Getting Started](md-claude-chapters/01-getting-started.md)** - Installation, quick start, first run
- **[Chapter 2: User Guide](md-claude-chapters/02-user-guide.md)** - Keyboard shortcuts, filters, navigation
- **[Chapter 3: Configuration](md-claude-chapters/03-configuration.md)** - Environment variables, CLI args, examples
- **[Chapter 4: Architecture](md-claude-chapters/04-architecture.md)** - Design principles, core components
- **[Chapter 5: Building](md-claude-chapters/05-building.md)** - Native, Web, and build processes
- **[Chapter 6: Tauri Desktop](md-claude-chapters/06-tauri-desktop.md)** - Desktop app, deep links, platform integration
- **[Chapter 7: Testing & Security](md-claude-chapters/07-testing-security.md)** - E2E tests, OAuth, CSP headers
- **[Chapter 8: Reference](md-claude-chapters/08-reference.md)** - Dependencies, troubleshooting, performance

### Additional Resources

- **[CHANGELOG.md](CHANGELOG.md)** - Version history and recent improvements
- **[.env.example](.env.example)** - Complete configuration template
- **[Makefile](Makefile)** - Build automation commands

## Quick Links

- [Keyboard Shortcuts](md-claude-chapters/02-user-guide.md#keyboard-controls)
- [Configuration Options](md-claude-chapters/03-configuration.md#key-configuration-options)
- [Building for Web](md-claude-chapters/05-building.md#web-browser-mode-dom-frontend)
- [Tauri Deep Links](md-claude-chapters/06-tauri-desktop.md#deep-link-architecture)
- [Troubleshooting](md-claude-chapters/08-reference.md#troubleshooting)

## At a Glance

### Quad-Mode Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    NEARx Quad-Mode Architecture                         │
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
│        └───────────────┼────────────────────────────────────┘           │
│                        ▼                ▼                               │
│              ┌─────────────────────────────────────┐                    │
│              │      Shared Rust Core               │                    │
│              │  • App state (height-based blocks)  │                    │
│              │  • UI rendering (ratatui)           │                    │
│              │  • RPC client & polling             │                    │
│              │  • Filter & search (SQLite/memory)  │                    │
│              └─────────────────────────────────────┘                    │
└─────────────────────────────────────────────────────────────────────────┘
```

### Key Features

- **Real-time monitoring** of NEAR blockchain transactions
- **Advanced filtering** with query grammar
- **Height-based navigation** with stable block selection
- **Archival RPC support** for unlimited history
- **Cross-platform** with unified codebase
- **Accessible** with WCAG AA compliance
- **Secure** with CSP headers and OAuth

### Quick Start

```bash
# Native Terminal
cargo run --bin nearx --features native

# Web Browser (http://localhost:8000)
make dev

# Tauri Desktop
cd tauri-workspace
cargo tauri dev
```

For detailed instructions, see [Chapter 1: Getting Started](md-claude-chapters/01-getting-started.md).

## Known Issues Being Investigated

1. **Performance regression** in Web/Tauri targets
2. **Tab key not working** - likely due to egui remnants in DOM build
3. **Incorrect target paths** in CI and release scripts

These issues are tracked and will be addressed in upcoming releases.

## Contributing

This is an open source project. Contributions, issues, and feature requests are welcome!

Built with ❤️ using Ratatui, Tokio, and Rust. Designed for NEAR Protocol monitoring.