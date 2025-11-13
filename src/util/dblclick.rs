//! Simple double-click detector for terminal UI mouse events
//!
//! Tracks consecutive clicks at the same cell position within a time threshold.
//! Useful for implementing double-click gestures in terminal UIs.

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

/// Simple double-click detector: two clicks at the same cell within threshold.
///
/// # Example
/// ```
/// use nearx::util::dblclick::DblClick;
/// use std::time::Duration;
///
/// let mut dbl = DblClick::new(Duration::from_millis(280));
///
/// // First click at (10, 5) - returns false
/// assert_eq!(dbl.register(10, 5), false);
///
/// // Second click at same position within threshold - returns true
/// assert_eq!(dbl.register(10, 5), true);
///
/// // Third click - resets, returns false
/// assert_eq!(dbl.register(10, 5), false);
/// ```
#[derive(Debug, Clone)]
pub struct DblClick {
    last: Option<(u16, u16, Instant)>,
    threshold: Duration,
}

impl Default for DblClick {
    fn default() -> Self {
        Self::new(Duration::from_millis(280))
    }
}

impl DblClick {
    /// Create a new double-click detector with specified time threshold
    ///
    /// # Arguments
    /// * `threshold` - Maximum time between clicks to count as double-click
    ///
    /// # Recommended Values
    /// - 280ms: Standard for most UIs (default)
    /// - 400ms: More forgiving for slow clickers
    /// - 200ms: Strict, for fast clickers only
    pub fn new(threshold: Duration) -> Self {
        Self {
            last: None,
            threshold,
        }
    }

    /// Register a click and check if it's a double-click
    ///
    /// Returns `true` if this click completes a double-click gesture
    /// (same position as previous click, within time threshold).
    ///
    /// # Arguments
    /// * `x` - Column position (terminal cell X coordinate)
    /// * `y` - Row position (terminal cell Y coordinate)
    ///
    /// # Returns
    /// - `true` if double-click detected (resets state)
    /// - `false` if first click or different position or timeout expired
    pub fn register(&mut self, x: u16, y: u16) -> bool {
        let now = Instant::now();

        if let Some((lx, ly, when)) = self.last {
            // Check if same position and within time threshold
            if lx == x && ly == y && now.saturating_duration_since(when) <= self.threshold {
                // Double-click detected! Reset state and return true
                self.last = None;
                return true;
            }
        }

        // First click or different position - store for next comparison
        self.last = Some((x, y, now));
        false
    }

    /// Reset the detector state (useful when focus changes)
    pub fn reset(&mut self) {
        self.last = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_double_click_same_position() {
        let mut dbl = DblClick::new(Duration::from_millis(280));

        // First click should return false
        assert!(!dbl.register(10, 5));

        // Immediate second click at same position should return true
        assert!(dbl.register(10, 5));

        // Third click resets - should return false
        assert!(!dbl.register(10, 5));
    }

    #[test]
    fn test_different_position_resets() {
        let mut dbl = DblClick::new(Duration::from_millis(280));

        // First click
        assert!(!dbl.register(10, 5));

        // Second click at different position - should return false
        assert!(!dbl.register(15, 8));

        // Third click at new position - should return true
        assert!(dbl.register(15, 8));
    }

    #[test]
    fn test_manual_reset() {
        let mut dbl = DblClick::new(Duration::from_millis(280));

        // First click
        assert!(!dbl.register(10, 5));

        // Reset state
        dbl.reset();

        // Next click at same position is treated as first click
        assert!(!dbl.register(10, 5));
    }
}
