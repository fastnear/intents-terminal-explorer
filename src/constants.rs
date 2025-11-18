//! Application constants
//!
//! Centralized constants for UI dimensions, timing, caching, and other magic numbers
//! used throughout the application.

/// UI layout and display constants
pub mod ui {
    /// Minimum terminal width in columns for usable display
    pub const MIN_WIDTH: u16 = 60;

    /// Minimum terminal height in rows for usable display
    pub const MIN_HEIGHT: u16 = 15;

    /// Width threshold for switching to narrow layout (columns)
    ///
    /// Terminals narrower than this will use vertical stacking instead
    /// of side-by-side panes.
    pub const NARROW_THRESHOLD: u16 = 80;
}

/// Application state and behavior constants
pub mod app {
    /// Duration to show toast notifications (seconds)
    pub const TOAST_DURATION_SECS: u64 = 2;

    /// Maximum number of debug log lines to retain in memory
    pub const MAX_DEBUG_LOG_LINES: usize = 50;

    /// Number of blocks to preserve around selection when caching
    ///
    /// When a selected block ages out, we cache this many blocks before
    /// and after it to maintain navigation context.
    pub const CACHE_CONTEXT_BLOCKS: usize = 50;

    /// Maximum total cached blocks (safety limit = 6× context window)
    ///
    /// This prevents unbounded cache growth while allowing ample room for
    /// navigation context around multiple pinned blocks.
    pub const MAX_TOTAL_CACHED: usize = CACHE_CONTEXT_BLOCKS * 6;

    /// Window size for archival backfill around selected block
    ///
    /// When entering fullscreen mode or navigating to a block, the app
    /// proactively requests this many blocks ahead and behind the selection
    /// from the archival RPC endpoint. This enables smooth navigation through
    /// historical blocks without per-block fetch latency.
    pub const ARCHIVAL_CONTEXT_BLOCKS: u64 = 50;

    /// Backward window size for chain-walking (ancestors of anchor block)
    ///
    /// Same as ARCHIVAL_CONTEXT_BLOCKS, but used for the backwards-fill
    /// slot mechanism that gradually fetches ancestors.
    pub const BACK_WINDOW: usize = ARCHIVAL_CONTEXT_BLOCKS as usize;

    /// Forward window size for live update pause threshold
    ///
    /// When the live stream gets this many blocks ahead of the user's
    /// selected anchor, live updates are paused to prevent jarring jumps.
    /// Press ← in Blocks pane to resume.
    pub const FRONT_WINDOW: u64 = ARCHIVAL_CONTEXT_BLOCKS;
}

/// User-facing message strings
pub mod messages {
    /// Toast message when copying block data (pane 0)
    pub const COPY_BLOCK: &str = "Copied block JSON";

    /// Toast message when copying transaction data (pane 1)
    pub const COPY_TX: &str = "Copied transaction JSON";

    /// Toast message when copying details pane content (pane 2)
    pub const COPY_DETAILS: &str = "Copied details JSON";

    /// Generic copy success message (fallback)
    pub const COPY_GENERIC: &str = "Copied";

    /// Toast message when clipboard operation fails
    pub const COPY_FAILED: &str = "Copy failed";
}
