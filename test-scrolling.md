# NEARx Fullscreen JSON Scrolling Test Plan

## Test Scenarios

### 1. Dark Rectangle Fix
- Launch NEARx: `cargo run --bin nearx --features native`
- Navigate to a transaction with JSON details
- Press Space to enter fullscreen
- **Expected**: No dark rectangle at the bottom of the screen
- **Verify**: Content fills the entire pane without visual artifacts

### 2. Batch Scrolling with Arrow Keys
- In fullscreen JSON view:
  - Press Down arrow → scrolls 1 line down
  - Press Up arrow → scrolls 1 line up
  - Press Right arrow → scrolls 10 lines down (batch)
  - Press Left arrow → scrolls 10 lines up (batch)
  - Press PageDown → scrolls full page down
  - Press PageUp → scrolls full page up
  - Press Home → goes to top
  - Press End → goes to bottom

### 3. Tab Toggle Between Modes
- In fullscreen JSON view:
  - Press Tab → switches to Navigate mode
  - Arrow keys should now navigate blocks/transactions (not scroll JSON)
  - Press Tab again → switches back to Scroll mode
  - Arrow keys should scroll JSON again

### 4. Truncation Message
- Find a large block with lots of data
- Enter fullscreen view
- Scroll to the bottom
- **Expected**: See "(truncated, 'c' to copy raw JSON)" message
- Press 'c' to copy → should copy the full JSON

### 5. Exit Fullscreen
- Press Space or Escape → exits fullscreen
- UI returns to normal split view

## Quick Commands Summary

In fullscreen Scroll mode:
- ↑/↓ or j/k: Scroll 1 line
- ←/→ or h/l: Scroll 10 lines (batch)
- PageUp/PageDown: Scroll full page
- Home/End: Go to start/end
- Tab: Switch to Navigate mode
- Space/Esc: Exit fullscreen
- c: Copy full JSON

In fullscreen Navigate mode:
- ↑/↓: Navigate blocks/transactions
- Tab: Switch back to Scroll mode
- Space/Esc: Exit fullscreen