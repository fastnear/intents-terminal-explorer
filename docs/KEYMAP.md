# NEARx – Standard Keymap (TUI / Web / Tauri)

> Uniform interactions. Keyboard everywhere. Mouse/trackpad on Web & Tauri.

## Global
- **Tab / Shift+Tab** – Cycle focus across Blocks → Transactions → Details.
- **Ctrl/Cmd+F** – Focus the filter input (Web/Tauri).
  - TUI: uses inline filter prompt (no mouse focus).
- **c** – Copy pane‑aware JSON to clipboard (Blocks/Tx/Details).
  - (Not intercepted when Ctrl/Cmd is held; OS copy works as usual.)
- **Space** – Toggle full‑screen Details when Details is focused.
- **Esc** – If Details is full‑screen → exit overlay; else clear filter (Web/Tauri).

## Navigation
- **Up / Down** – Move selection in the focused list (Blocks or Tx).
  - In Details pane, scroll one line.
- **PgUp / PgDn** – Page selection (or scroll Details).
- **Home / End** – Jump to top/bottom (or start/end in Details).

## Mouse/Trackpad (Web & Tauri - default ON, TUI - Ctrl+M to enable)
- **Click (Blocks)** → Focus Blocks and select clicked row.
- **Click (Transactions)** → Focus Transactions and select clicked row.
- **Click (Details)** → Focus Details.
- **Double‑click (Details)** → Toggle full‑screen Details overlay (Web/Tauri only).
- **Scroll wheel** → Scroll content in current pane.

## TUI-Specific
- **Ctrl+M** – Toggle mouse support on/off (TUI only).
  - Default: OFF (respects terminal text selection conventions)
  - When enabled: Click to select, scroll wheel to navigate
  - Toast notification confirms state change

## Notes
- Focused pane shows accented border + bg from the shared Theme.
- Filter uses monospace, unobtrusive frame, AA contrast, and persists per target.
- Web/Tauri render at Hi‑DPI using pixels_per_point snap; ratatui texture is crisp.
- TUI respects terminal conventions: mouse disabled by default, Shift/Alt+drag for text selection.
