use serde_json::json;
use std::collections::{HashMap, HashSet};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(target_arch = "wasm32")]
use web_time::{Duration, Instant};

use crate::filter::{self, compile_filter, tx_matches_filter, CompiledFilter};
use crate::flags::UiFlags;
use crate::json_pretty::pretty;
use crate::theme::Theme;
use crate::types::{AppEvent, BlockRow, TxLite, WsPayload};

#[cfg(feature = "native")]
use crate::theme::rat;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Filter,
    Search,
    Marks,
    JumpPending,
}

/// Reason for block selection change - determines tx selection behavior
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BlockChangeReason {
    AutoFollow,   // New block arrived in auto-follow mode (keep tx index)
    ManualNav,    // User manually navigated to different block (reset tx)
    FilterChange, // Filter was applied/cleared (try to preserve tx)
}

pub struct App {
    quit: bool,
    pane: usize, // 0 blocks, 1 txs, 2 details
    blocks: Vec<BlockRow>,
    sel_block_height: Option<u64>, // None = auto-follow newest, Some(height) = locked to specific block
    sel_tx: usize,

    details: String,
    details_scroll: u16,

    fps: u32,
    fps_choices: Vec<u32>,

    keep_blocks: usize,
    follow_blocks_latest: bool, // True = auto-follow newest block, False = locked to selection

    // Filter state
    filter_query: String,
    filter_compiled: CompiledFilter,
    input_mode: InputMode,

    // Search state
    search_query: String,
    search_results: Vec<crate::history::HistoryHit>,
    search_selection: usize,

    // Marks state
    marks_list: Vec<crate::types::Mark>,
    marks_selection: usize,

    // Owned accounts state
    owned_accounts: HashSet<String>, // Lowercase account IDs from ~/.near-credentials
    owned_only_filter: bool,         // Ctrl+U toggle for owned-only view
    owned_counts: HashMap<u64, usize>, // Cached owned tx count per block height

    // Manually-selected blocks cache (preserves blocks after they age out of rolling buffer)
    cached_blocks: HashMap<u64, BlockRow>, // height -> block
    cached_block_order: Vec<u64>,          // LRU tracking for cache eviction

    // Archival fetch state (for fetching historical blocks beyond cache)
    loading_block: Option<u64>, // Block height currently being fetched from archival
    archival_fetch_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>, // Channel to request archival fetches

    // Debug log (for development)
    debug_log: Vec<String>, // Rolling buffer of debug messages
    debug_visible: bool,    // Toggle debug panel visibility (Ctrl+D)

    // Toast notification state
    toast_message: Option<(String, Instant)>, // (message, timestamp)

    // UI layout state
    details_fullscreen: bool,     // Spacebar toggle for 100% details view
    details_viewport_height: u16, // Actual visible height of details pane (set by UI layer)

    // Theme (single source of truth for all UI targets)
    theme: Theme,

    // Cached ratatui styles (invalidated when theme changes)
    #[cfg(feature = "native")]
    rat_styles_cache: Option<rat::Styles>,

    // UI feature flags (for Web/Tauri enhanced behaviors)
    ui_flags: UiFlags,
}

impl App {
    pub fn new(
        fps: u32,
        fps_choices: Vec<u32>,
        keep_blocks: usize,
        default_filter: String,
        archival_fetch_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>,
    ) -> Self {
        let filter_compiled = if default_filter.is_empty() {
            CompiledFilter::default()
        } else {
            compile_filter(&default_filter)
        };

        Self {
            quit: false,
            pane: 0,
            blocks: Vec::with_capacity(keep_blocks),
            sel_block_height: None,
            sel_tx: 0, // Start in auto-follow mode
            details: "(No blocks yet)".into(),
            details_scroll: 0,
            fps,
            fps_choices,
            keep_blocks,
            follow_blocks_latest: true, // Start in auto-follow mode
            filter_query: default_filter,
            filter_compiled,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            search_results: Vec::new(),
            search_selection: 0,
            marks_list: Vec::new(),
            marks_selection: 0,
            owned_accounts: HashSet::new(),
            owned_only_filter: false,
            owned_counts: HashMap::new(),
            cached_blocks: HashMap::new(),
            cached_block_order: Vec::new(),
            loading_block: None,
            archival_fetch_tx,
            debug_log: Vec::new(),
            debug_visible: false, // Hidden by default
            toast_message: None,
            details_fullscreen: false,   // Normal view by default
            details_viewport_height: 20, // Default estimate, will be updated by UI
            theme: Theme::default(),     // Single source of truth for UI colors
            #[cfg(feature = "native")]
            rat_styles_cache: None, // Computed on first use
            ui_flags: UiFlags::default(), // Safe defaults for Web/Tauri
        }
    }

    // ----- getters -----
    pub fn fps(&self) -> u32 {
        self.fps
    }
    pub fn quit_flag(&self) -> bool {
        self.quit
    }
    pub fn pane(&self) -> usize {
        self.pane
    }
    pub fn sel_tx(&self) -> usize {
        self.sel_tx
    }
    pub fn is_viewing_cached_block(&self) -> bool {
        if let Some(height) = self.sel_block_height {
            self.find_block_index(Some(height)).is_none()
                && self.cached_blocks.contains_key(&height)
        } else {
            false
        }
    }

    /// Find the array index for a given block height
    /// Returns None if height is None (auto-follow mode) or block not found
    fn find_block_index(&self, height: Option<u64>) -> Option<usize> {
        if self.blocks.is_empty() {
            return None;
        }

        match height {
            None => None, // Auto-follow mode: return None to trigger auto-follow logic in current_block()
            Some(h) => self.blocks.iter().position(|b| b.height == h),
        }
    }

    /// Get the currently selected block (fallback to cache if aged out of main buffer)
    /// Filter-aware in auto-follow mode: returns first block with matching transactions
    pub fn current_block(&self) -> Option<&BlockRow> {
        if let Some(idx) = self.find_block_index(self.sel_block_height) {
            // Manual mode: return block at specific height
            self.blocks.get(idx)
        } else if let Some(height) = self.sel_block_height {
            // Block not in main buffer, check cache
            self.cached_blocks.get(&height)
        } else {
            // Auto-follow mode: respect filter
            if filter::is_empty(&self.filter_compiled) && !self.owned_only_filter {
                // No filter: return newest block
                self.blocks.first()
            } else {
                // Filter active: return first block with matching transactions
                self.blocks.iter().find(|b| self.count_matching_txs(b) > 0)
            }
        }
    }

    /// Count how many transactions in a block match the current filter
    fn count_matching_txs(&self, block: &BlockRow) -> usize {
        if filter::is_empty(&self.filter_compiled) && !self.owned_only_filter {
            return block.transactions.len(); // No filter = all match
        }

        block
            .transactions
            .iter()
            .filter(|tx| {
                // Apply owned-only filter if active
                if self.owned_only_filter && !self.is_owned_tx(tx) {
                    return false;
                }
                // Apply text filter
                let v = json!({
                    "hash": &tx.hash,
                    "signer_id": tx.signer_id.as_deref().unwrap_or(""),
                    "receiver_id": tx.receiver_id.as_deref().unwrap_or("")
                });
                tx_matches_filter(&v, &self.filter_compiled)
            })
            .count()
    }

    /// Returns blocks that have at least one matching transaction
    /// Returns (filtered_blocks, selected_index, total_count)
    pub fn filtered_blocks(&self) -> (Vec<&BlockRow>, Option<usize>, usize) {
        let total = self.blocks.len();

        if filter::is_empty(&self.filter_compiled) && !self.owned_only_filter {
            // No filter active - return all blocks
            let idx = self
                .find_block_index(self.sel_block_height)
                .or(if !self.blocks.is_empty() {
                    Some(0)
                } else {
                    None
                });
            let refs: Vec<&BlockRow> = self.blocks.iter().collect();
            return (refs, idx, total);
        }

        // Filter blocks with matching transactions
        let filtered: Vec<&BlockRow> = self
            .blocks
            .iter()
            .filter(|block| self.count_matching_txs(block) > 0)
            .collect();

        // Find selected block index in filtered list
        let sel_idx = if let Some(height) = self.sel_block_height {
            filtered
                .iter()
                .position(|b| b.height == height)
                .or(if !filtered.is_empty() { Some(0) } else { None })
        } else if !filtered.is_empty() {
            Some(0) // Auto-follow: select first filtered block (newest with matches)
        } else {
            None // No matching blocks
        };

        (filtered, sel_idx, total)
    }

    /// Get the list of blocks to navigate through (respects current filter)
    /// Returns Vec of heights in display order (newest first)
    fn get_navigation_list(&self) -> Vec<u64> {
        if filter::is_empty(&self.filter_compiled) && !self.owned_only_filter {
            // No filter - navigate through all blocks
            self.blocks.iter().map(|b| b.height).collect()
        } else {
            // Filter active - navigate only through blocks with matching transactions
            self.blocks
                .iter()
                .filter(|block| self.count_matching_txs(block) > 0)
                .map(|b| b.height)
                .collect()
        }
    }

    /// Check if a specific block height is available (in buffer or cache)
    pub fn is_block_height_available(&self, height: u64) -> bool {
        self.is_block_available(height)
    }

    /// Get the currently selected block's height (for UI purposes)
    pub fn selected_block_height(&self) -> Option<u64> {
        self.sel_block_height.or_else(|| {
            // Auto-follow mode: return newest block height if available
            self.blocks.first().map(|b| b.height)
        })
    }

    pub fn txs(&self) -> (Vec<TxLite>, usize, usize) {
        if let Some(b) = self.current_block() {
            let total = b.transactions.len();
            let filtered: Vec<TxLite> = b
                .transactions
                .iter()
                .filter(|tx| {
                    // Apply owned-only filter first (if active)
                    if self.owned_only_filter && !self.is_owned_tx(tx) {
                        return false;
                    }
                    // Then apply text filter - pass complete tx data for filtering
                    // Note: actions omitted here since ActionSummary doesn't derive Serialize
                    // TODO: Add Serialize to ActionSummary for full filtering support
                    let v = json!({
                        "hash": &tx.hash,
                        "signer_id": tx.signer_id.as_deref().unwrap_or(""),
                        "receiver_id": tx.receiver_id.as_deref().unwrap_or("")
                    });
                    tx_matches_filter(&v, &self.filter_compiled)
                })
                .cloned()
                .collect();
            (filtered, self.sel_tx, total)
        } else {
            (vec![], 0, 0)
        }
    }

    pub fn details(&self) -> &str {
        &self.details
    }
    pub fn details_scroll(&self) -> u16 {
        self.details_scroll
    }
    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }
    pub fn filter_query(&self) -> &str {
        &self.filter_query
    }
    pub fn owned_only_filter(&self) -> bool {
        self.owned_only_filter
    }
    #[allow(dead_code)]
    pub fn owned_accounts(&self) -> &HashSet<String> {
        &self.owned_accounts
    }
    pub fn debug_log(&self) -> &[String] {
        &self.debug_log
    }
    pub fn debug_visible(&self) -> bool {
        self.debug_visible
    }
    pub fn details_fullscreen(&self) -> bool {
        self.details_fullscreen
    }
    pub fn loading_block(&self) -> Option<u64> {
        self.loading_block
    }

    /// Get the active theme (single source of truth for UI colors)
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Set the active theme (for runtime theme switching)
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            #[cfg(feature = "native")]
            {
                self.rat_styles_cache = None; // Invalidate cached styles
            }
        }
    }

    /// Get cached ratatui styles for current theme (computed on first use, invalidated on theme change)
    #[cfg(feature = "native")]
    pub fn rat_styles(&mut self) -> rat::Styles {
        if let Some(ref styles) = self.rat_styles_cache {
            return *styles;
        }
        let styles = rat::styles(&self.theme);
        self.rat_styles_cache = Some(styles);
        styles
    }

    /// Get UI feature flags (controls Web/Tauri enhanced behaviors)
    pub fn ui_flags(&self) -> UiFlags {
        self.ui_flags
    }

    /// Set UI feature flags (for runtime toggling or testing)
    pub fn set_ui_flags(&mut self, flags: UiFlags) {
        self.ui_flags = flags;
    }

    /// Set the actual viewport height of details pane (called from UI layer)
    pub fn set_details_viewport_height(&mut self, height: u16) {
        self.details_viewport_height = height;
    }

    /// Show a toast notification for 2 seconds
    pub fn show_toast(&mut self, msg: String) {
        self.toast_message = Some((msg, Instant::now()));
    }

    /// Get current toast message if still active (visible for 2 seconds)
    pub fn toast_message(&self) -> Option<&str> {
        const TOAST_DURATION: Duration = Duration::from_secs(2);
        self.toast_message.as_ref().and_then(|(msg, time)| {
            if time.elapsed() < TOAST_DURATION {
                Some(msg.as_str())
            } else {
                None
            }
        })
    }

    // ----- knobs -----
    pub fn cycle_fps(&mut self) {
        if self.fps_choices.is_empty() {
            return;
        }
        let mut idx = self
            .fps_choices
            .iter()
            .position(|&v| v == self.fps)
            .unwrap_or(0);
        idx = (idx + 1) % self.fps_choices.len();
        self.fps = self.fps_choices[idx];
    }

    pub fn log_debug(&mut self, msg: String) {
        const MAX_LOG_ENTRIES: usize = 50;

        // Write to file for debugging (native only - WASM doesn't have filesystem)
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs::OpenOptions;
            use std::io::Write;
            let timestamp = chrono::Utc::now().format("%H:%M:%S%.3f");
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open("ratacat_debug.log")
            {
                let _ = writeln!(file, "[{timestamp}] {msg}");
            }
        }

        // Also keep in memory for debug panel
        self.debug_log.push(msg);
        if self.debug_log.len() > MAX_LOG_ENTRIES {
            self.debug_log.remove(0);
        }
    }

    // ----- block cache methods -----
    /// Cache selected block and ±12 blocks around it for context navigation
    fn cache_block_with_context(&mut self, center_height: u64) {
        const CONTEXT_RANGE: i64 = 12;
        const MAX_TOTAL_CACHED: usize = 50; // Safety limit

        // Find the center block's index
        let center_idx = match self.find_block_index(Some(center_height)) {
            Some(idx) => idx,
            None => return, // Center block not in buffer, can't cache context
        };

        // Cache blocks in range [center - 12, center + 12]
        let start_idx = center_idx.saturating_sub(CONTEXT_RANGE as usize);
        let end_idx = (center_idx + CONTEXT_RANGE as usize + 1).min(self.blocks.len());

        let mut cached_count = 0;
        for idx in start_idx..end_idx {
            if let Some(block) = self.blocks.get(idx) {
                let height = block.height;

                // Update LRU: remove if exists, add to end
                self.cached_block_order.retain(|&h| h != height);
                self.cached_block_order.push(height);

                // Add to cache
                if let std::collections::hash_map::Entry::Vacant(e) =
                    self.cached_blocks.entry(height)
                {
                    e.insert(block.clone());
                    cached_count += 1;
                }
            }
        }

        // Evict oldest if over limit
        while self.cached_block_order.len() > MAX_TOTAL_CACHED {
            if let Some(old_height) = self.cached_block_order.first().copied() {
                self.cached_block_order.remove(0);
                self.cached_blocks.remove(&old_height);
            }
        }

        if cached_count > 0 {
            self.log_debug(format!(
                "Cached block #{} with ±{} context ({} new, {} total)",
                center_height,
                CONTEXT_RANGE,
                cached_count,
                self.cached_blocks.len()
            ));
        }
    }

    /// Check if a block is available for viewing (in main buffer or cache)
    fn is_block_available(&self, height: u64) -> bool {
        self.find_block_index(Some(height)).is_some() || self.cached_blocks.contains_key(&height)
    }

    // ----- owned accounts methods -----
    pub fn set_owned_accounts(&mut self, accounts: HashSet<String>) {
        self.owned_accounts = accounts;
        // Recompute owned counts for all blocks
        self.recompute_owned_counts();
    }

    pub fn is_owned_tx(&self, tx: &TxLite) -> bool {
        if self.owned_accounts.is_empty() {
            return false;
        }
        // Check if signer or receiver matches any owned account (case-insensitive)
        if let Some(ref signer) = tx.signer_id {
            if self.owned_accounts.contains(&signer.to_lowercase()) {
                return true;
            }
        }
        if let Some(ref receiver) = tx.receiver_id {
            if self.owned_accounts.contains(&receiver.to_lowercase()) {
                return true;
            }
        }
        false
    }

    pub fn count_owned_txs(&self, txs: &[TxLite]) -> usize {
        if self.owned_accounts.is_empty() {
            return 0;
        }
        txs.iter().filter(|tx| self.is_owned_tx(tx)).count()
    }

    pub fn toggle_owned_filter(&mut self) {
        self.owned_only_filter = !self.owned_only_filter;
        self.sel_tx = 0; // Reset selection when filter changes
    }

    /// Toggle debug panel visibility (Ctrl+D)
    pub fn toggle_debug_panel(&mut self) {
        self.debug_visible = !self.debug_visible;
        self.log_debug(format!(
            "Debug panel: {}",
            if self.debug_visible {
                "visible"
            } else {
                "hidden"
            }
        ));
    }

    /// Toggle details fullscreen mode (Spacebar when details pane focused)
    pub fn toggle_details_fullscreen(&mut self) {
        if self.pane == 2 {
            // Only toggle when details pane is focused
            self.details_fullscreen = !self.details_fullscreen;
            let mode = if self.details_fullscreen {
                "fullscreen"
            } else {
                "normal"
            };
            self.log_debug(format!("Details view: {mode}"));
        }
    }

    /// Return to auto-follow mode (track newest block)
    pub fn return_to_auto_follow(&mut self) {
        let old_height = self.sel_block_height;
        self.follow_blocks_latest = true; // Re-enable auto-follow mode
        self.sel_block_height = None; // None = auto-follow newest
        self.sel_tx = 0; // Reset to first tx
        if !self.blocks.is_empty() {
            self.validate_and_refresh_tx(BlockChangeReason::AutoFollow);
            self.log_debug(format!("[USER_ACTION] Return to AUTO-FOLLOW mode (Home key pressed), was locked to height={old_height:?}"));
        }
    }

    pub fn owned_count(&self, height: u64) -> usize {
        self.owned_counts.get(&height).copied().unwrap_or(0)
    }

    fn recompute_owned_counts(&mut self) {
        self.owned_counts.clear();
        for block in &self.blocks {
            let count = self.count_owned_txs(&block.transactions);
            if count > 0 {
                self.owned_counts.insert(block.height, count);
            }
        }
    }

    // ----- filter methods -----
    pub fn start_filter(&mut self) {
        self.input_mode = InputMode::Filter;
    }

    pub fn clear_filter(&mut self) {
        self.filter_query.clear();
        self.filter_compiled = CompiledFilter::default();
        self.input_mode = InputMode::Normal;
        self.validate_and_refresh_tx(BlockChangeReason::FilterChange); // Try to preserve tx
    }

    pub fn apply_filter(&mut self) {
        self.filter_compiled = compile_filter(&self.filter_query);
        self.input_mode = InputMode::Normal;
        self.validate_and_refresh_tx(BlockChangeReason::FilterChange); // Try to preserve tx
    }

    pub fn filter_add_char(&mut self, ch: char) {
        self.filter_query.push(ch);
    }

    pub fn filter_backspace(&mut self) {
        self.filter_query.pop();
    }

    // ----- copy functionality -----
    /// Get copy content for the currently focused pane.
    ///
    /// Delegates to `copy_api` for consistent behavior across all targets.
    pub fn get_copy_content(&self) -> String {
        crate::copy_api::current_text(self).unwrap_or_default()
    }

    // ----- selection / scrolling -----
    /// Move focus to next pane (circular: 0→1→2→0)
    pub fn next_pane(&mut self) {
        self.pane = (self.pane + 1) % 3;
        self.log_debug(format!("Tab -> pane={}", self.pane));
    }

    /// Move focus to previous pane (circular: 2→1→0→2)
    pub fn prev_pane(&mut self) {
        // Backward navigation: subtract 1 with wrap-around
        // (pane - 1 + 3) % 3 ensures we don't underflow (e.g., 0-1 = -1)
        self.pane = (self.pane + 3 - 1) % 3;
        self.log_debug(format!("BackTab -> pane={}", self.pane));
    }

    /// Set pane directly (used by deep link router)
    pub fn set_pane_direct(&mut self, pane: usize) {
        if pane < 3 {
            self.pane = pane;
            self.log_debug(format!("DeepLink -> pane={}", self.pane));
        }
    }

    /// Apply a deep link route to the current app state
    ///
    /// This method maps deep link routes to the existing explorer UI by:
    /// - Setting the appropriate pane focus
    /// - Applying filters to match the route target
    ///
    /// Example routes:
    /// - `Tx{hash}` → Focus transactions pane, filter to hash
    /// - `Block{height}` → Focus blocks pane, filter to height
    /// - `Account{id}` → Focus transactions pane, filter to account
    pub fn apply_route(&mut self, route: &crate::router::Route) {
        use crate::router::{Route, RouteV1};

        match route {
            Route::V1(RouteV1::Tx { hash }) => {
                // Focus transactions pane and filter to the specific hash
                self.set_pane_direct(1);
                self.filter_query = hash.clone();
                self.apply_filter();
                self.log_debug(format!("Route: tx/{hash}"));
            }
            Route::V1(RouteV1::Block { height }) => {
                // Focus blocks pane and filter to the specific height
                self.set_pane_direct(0);
                self.filter_query = format!("height:{height}");
                self.apply_filter();
                self.log_debug(format!("Route: block/{height}"));
            }
            Route::V1(RouteV1::Account { id }) => {
                // Focus transactions pane and filter to the account
                self.set_pane_direct(1);
                self.filter_query = format!("acct:{id}");
                self.apply_filter();
                self.log_debug(format!("Route: account/{id}"));
            }
            Route::V1(RouteV1::Home) => {
                // Clear filter and return to auto-follow mode
                self.clear_filter();
                self.return_to_auto_follow();
                self.log_debug("Route: home".to_string());
            }
        }
    }

    /// Refresh tx details if current selection is still valid, otherwise reset
    fn validate_and_refresh_tx(&mut self, reason: BlockChangeReason) {
        let (txs, _, _) = self.txs();

        match reason {
            BlockChangeReason::AutoFollow => {
                // Auto-follow: keep tx index if valid, clamp if out of bounds
                if self.sel_tx >= txs.len() {
                    self.sel_tx = txs.len().saturating_sub(1);
                }
                if !txs.is_empty() {
                    self.select_tx();
                }
            }
            BlockChangeReason::ManualNav => {
                // Manual navigation: reset to first tx in new block
                self.sel_tx = 0;
                if !txs.is_empty() {
                    self.select_tx();
                }
            }
            BlockChangeReason::FilterChange => {
                // Filter change: preserve tx if valid, otherwise reset
                if self.sel_tx >= txs.len() {
                    self.sel_tx = 0;
                }
                if !txs.is_empty() {
                    self.select_tx();
                }
            }
        }
    }

    /// Handle Up arrow key - behavior depends on which pane is focused
    pub fn up(&mut self) {
        match self.pane {
            0 => {
                // Blocks pane: navigate to previous block (newer)
                self.log_debug(format!(
                    "[USER_NAV_UP] follow_latest={}, sel_height={:?}",
                    self.follow_blocks_latest, self.sel_block_height
                ));

                // Get the navigation list (respects filter)
                let nav_list = self.get_navigation_list();

                if nav_list.is_empty() {
                    return; // No blocks to navigate
                }

                // Ensure we have a locked selection (handle edge case of None)
                let current_height = match self.sel_block_height {
                    Some(h) => h,
                    None => {
                        // Edge case: not locked yet, lock to current and don't navigate
                        // This shouldn't happen with auto-lock, but handle gracefully
                        let h = nav_list[0];
                        self.sel_block_height = Some(h);
                        self.follow_blocks_latest = false; // User navigation disables auto-follow
                        self.cache_block_with_context(h);
                        self.log_debug(format!("[USER_NAV_UP] edge case lock to #{h}"));
                        return;
                    }
                };

                // Navigate to next newer block in navigation list
                if let Some(current_idx) = nav_list.iter().position(|&h| h == current_height) {
                    if current_idx > 0 {
                        let new_height = nav_list[current_idx - 1];
                        // Only navigate if target block is available
                        if self.is_block_available(new_height) {
                            self.sel_block_height = Some(new_height);
                            self.follow_blocks_latest = false; // User navigation disables auto-follow
                            self.details_scroll = 0;
                            self.cache_block_with_context(new_height);
                            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                            self.log_debug(format!("Blocks UP -> #{new_height}"));
                        } else {
                            // Block not available - try archival fetch
                            self.log_debug(format!("Blocks UP -> #{new_height} not available"));
                            self.request_archival_block(new_height);
                        }
                    }
                } else {
                    // Current selection not in navigation list (filtered out), jump to newest
                    let new_height = nav_list[0];
                    self.sel_block_height = Some(new_height);
                    self.follow_blocks_latest = false; // User navigation disables auto-follow
                    self.details_scroll = 0;
                    self.cache_block_with_context(new_height);
                    self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                    self.log_debug(format!(
                        "Blocks UP -> not in list, jump to newest #{new_height}"
                    ));
                }
            }
            1 => {
                // Tx pane: navigate to previous transaction
                if self.sel_tx > 0 {
                    self.sel_tx -= 1;
                    self.details_scroll = 0;
                    self.select_tx();
                    self.log_debug(format!("Tx UP, sel={}", self.sel_tx));
                }
            }
            2 => {
                // Details pane: scroll up
                self.scroll_details(-1);
            }
            _ => {}
        }
    }
    /// Handle Down arrow key - behavior depends on which pane is focused
    pub fn down(&mut self) {
        match self.pane {
            0 => {
                // Blocks pane: navigate to next block (older)
                self.log_debug(format!(
                    "[USER_NAV_DOWN] follow_latest={}, sel_height={:?}",
                    self.follow_blocks_latest, self.sel_block_height
                ));

                // Get the navigation list (respects filter)
                let nav_list = self.get_navigation_list();

                if nav_list.is_empty() {
                    return; // No blocks to navigate
                }

                // Ensure we have a locked selection (handle edge case of None)
                let current_height = match self.sel_block_height {
                    Some(h) => h,
                    None => {
                        // Edge case: not locked yet, lock to current and navigate down
                        // With auto-lock, this means we want to move to NEXT block
                        let h = nav_list[0];
                        if nav_list.len() > 1 {
                            // Move to next older block
                            let next_h = nav_list[1];
                            self.sel_block_height = Some(next_h);
                            self.follow_blocks_latest = false; // User navigation disables auto-follow
                            self.details_scroll = 0;
                            self.cache_block_with_context(next_h);
                            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                            self.log_debug(format!(
                                "[USER_NAV_DOWN] first press, move to #{next_h}"
                            ));
                        } else {
                            // Only one block, just lock to it
                            self.sel_block_height = Some(h);
                            self.follow_blocks_latest = false; // User navigation disables auto-follow
                            self.cache_block_with_context(h);
                            self.log_debug(format!("Blocks DOWN -> only one block, lock to #{h}"));
                        }
                        return;
                    }
                };

                // Navigate to next older block in navigation list
                if let Some(current_idx) = nav_list.iter().position(|&h| h == current_height) {
                    if current_idx + 1 < nav_list.len() {
                        let new_height = nav_list[current_idx + 1];
                        // Only navigate if target block is available
                        if self.is_block_available(new_height) {
                            self.sel_block_height = Some(new_height);
                            self.follow_blocks_latest = false; // User navigation disables auto-follow
                            self.details_scroll = 0;
                            self.cache_block_with_context(new_height);
                            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                            self.log_debug(format!("Blocks DOWN -> #{new_height}"));
                        } else {
                            // Block not available - try archival fetch
                            self.log_debug(format!("Blocks DOWN -> #{new_height} not available"));
                            self.request_archival_block(new_height);
                        }
                    }
                } else {
                    // Current selection not in navigation list (filtered out), jump to newest
                    let new_height = nav_list[0];
                    self.sel_block_height = Some(new_height);
                    self.follow_blocks_latest = false; // User navigation disables auto-follow
                    self.details_scroll = 0;
                    self.cache_block_with_context(new_height);
                    self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                    self.log_debug(format!(
                        "Blocks DOWN -> not in list, jump to newest #{new_height}"
                    ));
                }
            }
            1 => {
                // Tx pane: navigate to next transaction
                let (txs, _, _) = self.txs();
                if self.sel_tx + 1 < txs.len() {
                    self.sel_tx += 1;
                    self.details_scroll = 0;
                    self.select_tx();
                    self.log_debug(format!("Tx DOWN, sel={}", self.sel_tx));
                }
            }
            2 => {
                // Details pane: scroll down
                self.scroll_details(1);
            }
            _ => {}
        }
    }

    /// Select a block by row index (for mouse mapping)
    pub fn select_block_row(&mut self, idx: usize) {
        let nav_list = self.get_navigation_list();
        if let Some(&height) = nav_list.get(idx) {
            self.sel_block_height = Some(height);
            self.follow_blocks_latest = false; // User interaction disables auto-follow
            self.details_scroll = 0;
            self.cache_block_with_context(height);
            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
            self.log_debug(format!("Mouse select block #{height} (idx {idx})"));
        }
    }

    /// Select a transaction by row index (for mouse mapping)
    pub fn select_tx_row(&mut self, idx: usize) {
        let (txs, _, _) = self.txs();
        if idx < txs.len() {
            self.sel_tx = idx;
            self.details_scroll = 0;
            self.select_tx();
            self.log_debug(format!("Mouse select tx (idx {idx})"));
        }
    }

    /// Left arrow: Jump to top of current list
    pub fn left(&mut self) {
        match self.pane {
            0 => {
                // Jump to newest block (index 0)
                if !self.blocks.is_empty() {
                    let newest_height = self.blocks[0].height;
                    let current_idx = self.find_block_index(self.sel_block_height).unwrap_or(0);

                    if current_idx != 0 {
                        self.sel_block_height = Some(newest_height); // Lock to newest
                        self.follow_blocks_latest = false; // User navigation disables auto-follow
                        self.details_scroll = 0;
                        self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                        self.log_debug(format!("Left -> jump to newest #{newest_height}"));
                    }
                }
            }
            1 => {
                // Jump to first tx
                if self.sel_tx != 0 {
                    self.sel_tx = 0;
                    self.details_scroll = 0;
                    self.select_tx();
                    self.log_debug("Left -> jump to first tx".into());
                }
            }
            2 => {
                // Scroll to top of details
                if self.details_scroll != 0 {
                    self.details_scroll = 0;
                    self.log_debug("Left -> scroll to top".into());
                }
            }
            _ => {}
        }
    }

    /// Right arrow: Paginate down 12 items
    pub fn right(&mut self) {
        match self.pane {
            0 => {
                // Paginate down 12 blocks (toward older)
                let current_idx = self.find_block_index(self.sel_block_height).unwrap_or(0);
                let new_idx = (current_idx + 12).min(self.blocks.len().saturating_sub(1));

                if new_idx != current_idx && !self.blocks.is_empty() {
                    let new_height = self.blocks[new_idx].height;
                    self.sel_block_height = Some(new_height); // Lock to specific height
                    self.follow_blocks_latest = false; // User navigation disables auto-follow
                    self.details_scroll = 0;
                    self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                    self.log_debug(format!("Right -> paginate to block #{new_height}"));
                }
            }
            1 => {
                // Paginate down 12 txs
                let (txs, _, _) = self.txs();
                let new_sel = (self.sel_tx + 12).min(txs.len().saturating_sub(1));
                if new_sel != self.sel_tx && !txs.is_empty() {
                    self.sel_tx = new_sel;
                    self.details_scroll = 0;
                    self.select_tx();
                    self.log_debug(format!("Right -> paginate to tx {}", self.sel_tx));
                }
            }
            2 => {
                // Scroll down 12 lines
                self.scroll_details(12);
                self.log_debug("Right -> scroll down 12 lines".into());
            }
            _ => {}
        }
    }

    pub fn page_up(&mut self, page: u16) {
        if self.pane == 2 {
            self.scroll_details(-(page as i32));
        }
    }
    pub fn page_down(&mut self, page: u16) {
        if self.pane == 2 {
            self.scroll_details(page as i32);
        }
    }
    pub fn home(&mut self) {
        if self.pane == 2 {
            self.details_scroll = 0;
        }
    }
    pub fn end(&mut self) {
        if self.pane == 2 {
            // Jump to bottom (clamped to actual content height)
            let content_lines = self.details.lines().count() as u16;
            self.details_scroll = content_lines.saturating_sub(15); // ~typical viewport size
        }
    }

    fn scroll_details(&mut self, delta: i32) {
        let cur = self.details_scroll as i32;

        // Calculate actual content height for max scroll
        // Use actual viewport height (set by UI layer) for accurate clamping
        let content_lines = self.details.lines().count() as u16;
        let max_scroll = content_lines.saturating_sub(self.details_viewport_height);

        let next = (cur + delta).max(0).min(max_scroll as i32);
        self.details_scroll = next as u16;
    }

    /// Generic line scrolling based on current focused pane (for wheel events).
    /// Positive delta = down/next, negative delta = up/prev.
    pub fn scroll_lines(&mut self, delta: i32) {
        if delta == 0 {
            return;
        }

        // Call up()/down() repeatedly to leverage existing navigation logic
        let steps = delta.abs();
        for _ in 0..steps {
            if delta > 0 {
                self.down();
            } else {
                self.up();
            }
        }
    }

    /// Request archival fetch for a block that's not in buffer or cache
    fn request_archival_block(&mut self, height: u64) {
        // Only request if we have archival fetch channel
        // Clone the sender to avoid borrow conflicts
        let tx = self.archival_fetch_tx.clone();
        if let Some(tx) = tx {
            // Only request if not already loading this block
            if self.loading_block != Some(height) {
                self.loading_block = Some(height);
                self.log_debug(format!("Requesting archival fetch for block #{height}"));
                if let Err(e) = tx.send(height) {
                    self.log_debug(format!("Failed to send archival fetch request: {e}"));
                    self.loading_block = None;
                }
            }
        }
    }

    pub fn select_tx(&mut self) {
        if let Some(b) = self.current_block() {
            let (filtered_txs, _, _) = self.txs();
            if let Some(tx) = filtered_txs.get(self.sel_tx) {
                // Build PRETTY view with formatted actions
                let mut pretty_json = json!({
                    "hash": tx.hash,
                    "block": b.height,
                });

                // Add signer/receiver if available
                if let Some(ref signer) = tx.signer_id {
                    pretty_json["signer"] = json!(signer);
                }
                if let Some(ref receiver) = tx.receiver_id {
                    pretty_json["receiver"] = json!(receiver);
                }
                if let Some(nonce) = tx.nonce {
                    pretty_json["nonce"] = json!(nonce);
                }

                // Format actions with human-readable gas/deposits
                if let Some(ref actions) = tx.actions {
                    let formatted_actions: Vec<serde_json::Value> = actions
                        .iter()
                        .map(crate::copy_payload::format_action)
                        .collect();
                    pretty_json["actions"] = json!(formatted_actions);
                }

                self.details = pretty(&pretty_json, 2);
            }
        }
    }

    // ----- events -----
    pub fn on_event(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::Quit => self.quit = true,
            AppEvent::FromWs(WsPayload::Block { data }) => {
                self.push_block(BlockRow {
                    height: data,
                    hash: "".into(),
                    timestamp: 0,
                    tx_count: 0,
                    when: "".into(),
                    transactions: vec![],
                });
            }
            AppEvent::FromWs(WsPayload::Tx {
                identifier: _,
                data,
            }) => {
                if let Some(t) = data {
                    // For WS summary, show pretty-formatted JSON
                    let raw = serde_json::to_value(&t).unwrap_or(serde_json::json!({}));
                    self.details = pretty(&raw, 2);
                    self.details_scroll = 0;
                }
            }
            AppEvent::NewBlock(b) => {
                log::info!(
                    "📥 App received NewBlock event - height: {}, txs: {}",
                    b.height,
                    b.tx_count
                );
                // Check if this block was being fetched from archival
                if self.loading_block == Some(b.height) {
                    self.loading_block = None; // Clear loading state
                }
                self.push_block(b);
            }
        }
    }

    fn push_block(&mut self, b: BlockRow) {
        // Compute owned count for this block before inserting
        let owned_count = self.count_owned_txs(&b.transactions);
        let height = b.height;

        // Log state BEFORE push
        self.log_debug(format!(
            "[PUSH_START] Block #{}, follow_latest={}, sel_height={:?}, blocks_count={}",
            height,
            self.follow_blocks_latest,
            self.sel_block_height,
            self.blocks.len()
        ));

        self.blocks.insert(0, b);
        if self.blocks.len() > self.keep_blocks {
            // Remove oldest block and its count
            if let Some(old_block) = self.blocks.pop() {
                self.owned_counts.remove(&old_block.height);
            }
        }

        // Store owned count if non-zero
        if owned_count > 0 {
            self.owned_counts.insert(height, owned_count);
        }

        // Height-based selection behavior
        if self.follow_blocks_latest {
            // Auto-follow mode: always jump to newest matching block
            if self.blocks.len() == 1 {
                // First block: select it and reset tx
                self.sel_block_height = None; // None = auto-follow newest
                self.sel_tx = 0;
                self.select_tx();
                self.log_debug(format!(
                    "[AUTO_FOLLOW] Block #{height} arr, FIRST block, staying in follow-latest mode"
                ));
            } else {
                // Continue auto-follow (no auto-lock!)
                let old_sel_height = self.sel_block_height;
                self.sel_block_height = None; // Always jump to newest
                self.validate_and_refresh_tx(BlockChangeReason::AutoFollow);
                self.log_debug(format!("[AUTO_FOLLOW] Block #{height} arr, jumping to newest, sel_height: {old_sel_height:?} -> None"));
            }
        } else {
            // Manual mode: maintain locked block height (block may be in cache if aged out)
            if let Some(locked_height) = self.sel_block_height {
                if self.find_block_index(Some(locked_height)).is_some() {
                    // Block still in main buffer
                    self.log_debug(format!(
                        "Block #{height} arr, MANUAL mode locked to #{locked_height}"
                    ));
                } else if self.cached_blocks.contains_key(&locked_height) {
                    // Block aged out but available in cache
                    self.log_debug(format!("[MANUAL_CACHED] Block #{height} arr, MANUAL mode viewing cached block #{locked_height}"));
                } else {
                    // Block not in buffer or cache - shouldn't happen, but handle gracefully
                    self.log_debug(format!("[FALLBACK] Block #{height} arr, WARNING: locked block #{locked_height} not found, FORCING auto-follow"));
                    self.follow_blocks_latest = true; // Return to auto-follow mode
                    self.sel_block_height = None;
                    self.sel_tx = 0;
                    self.validate_and_refresh_tx(BlockChangeReason::AutoFollow);
                }
            }
        }
    }

    // ----- Search methods -----
    pub fn start_search(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_query.clear();
        self.search_results.clear();
        self.search_selection = 0;
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn search_results(&self) -> &[crate::history::HistoryHit] {
        &self.search_results
    }

    pub fn search_selection(&self) -> usize {
        self.search_selection
    }

    pub fn search_add_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn search_backspace(&mut self) {
        self.search_query.pop();
    }

    pub fn set_search_results(&mut self, results: Vec<crate::history::HistoryHit>) {
        self.search_results = results;
        self.search_selection = 0;
    }

    pub fn search_up(&mut self) {
        if self.search_selection > 0 {
            self.search_selection -= 1;
        }
    }

    pub fn search_down(&mut self) {
        if self.search_selection + 1 < self.search_results.len() {
            self.search_selection += 1;
        }
    }

    pub fn close_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_query.clear();
        self.search_results.clear();
        self.search_selection = 0;
    }

    pub fn get_selected_search_result(&self) -> Option<&crate::history::HistoryHit> {
        self.search_results.get(self.search_selection)
    }

    pub fn display_tx_from_json(&mut self, raw_json: &str) {
        // Parse and display transaction from raw JSON
        if let Ok(tx) = serde_json::from_str::<serde_json::Value>(raw_json) {
            self.details = pretty(&tx, 2);
            self.details_scroll = 0;
        }
    }

    // ----- Marks methods -----
    pub fn open_marks(&mut self, marks_list: Vec<crate::types::Mark>) {
        self.marks_list = marks_list;
        self.marks_selection = 0;
        self.input_mode = InputMode::Marks;
    }

    pub fn marks_list(&self) -> &[crate::types::Mark] {
        &self.marks_list
    }

    pub fn close_marks(&mut self) {
        self.input_mode = InputMode::Normal;
        self.marks_list.clear();
        self.marks_selection = 0;
    }

    pub fn marks_selection(&self) -> usize {
        self.marks_selection
    }

    pub fn marks_up(&mut self) {
        if self.marks_selection > 0 {
            self.marks_selection -= 1;
        }
    }

    pub fn marks_down(&mut self) {
        if self.marks_selection + 1 < self.marks_list.len() {
            self.marks_selection += 1;
        }
    }

    pub fn get_selected_mark(&self) -> Option<&crate::types::Mark> {
        self.marks_list.get(self.marks_selection)
    }

    pub fn start_jump_pending(&mut self) {
        self.input_mode = InputMode::JumpPending;
    }

    pub fn current_context(&self) -> (u8, Option<u64>, Option<String>) {
        let pane = self.pane as u8;
        let height = self.current_block().map(|b| b.height);
        let tx_hash = if let Some(b) = self.current_block() {
            b.transactions.get(self.sel_tx).map(|t| t.hash.clone())
        } else {
            None
        };
        (pane, height, tx_hash)
    }

    pub fn jump_to_mark(&mut self, mark: &crate::types::Mark) {
        // Navigate to the mark's location
        if let Some(height) = mark.height {
            if self.blocks.iter().any(|b| b.height == height) {
                self.sel_block_height = Some(height); // Lock to specific block height
                self.follow_blocks_latest = false; // Jumping to mark disables auto-follow
                self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
            }
        }
        self.pane = mark.pane as usize;
        if self.pane > 2 {
            self.pane = 0;
        }
    }

    // ----- Web/egui helper methods -----

    /// Get count of blocks (for display)
    pub fn blocks_len(&self) -> usize {
        self.blocks.len()
    }

    /// Get count of transactions in current block
    pub fn txs_len(&self) -> usize {
        self.current_block().map_or(0, |b| b.transactions.len())
    }

    /// Get current block selection index (0-based)
    pub fn sel_block(&self) -> usize {
        if let Some(height) = self.sel_block_height {
            self.find_block_index(Some(height)).unwrap_or(0)
        } else {
            0 // Auto-follow mode: newest block is at index 0
        }
    }

    /// Select block by index with clamping
    pub fn select_block_clamped(&mut self, idx: usize) {
        if idx < self.blocks.len() {
            self.select_block_row(idx);
        }
    }

    /// Select transaction by index with clamping
    pub fn select_tx_clamped(&mut self, idx: usize) {
        if let Some(b) = self.current_block() {
            if idx < b.transactions.len() {
                self.select_tx_row(idx);
            }
        }
    }

    /// Set filter query and recompile
    pub fn set_filter_query(&mut self, query: String) {
        self.filter_query = query;
        self.filter_compiled = compile_filter(&self.filter_query);
        self.validate_and_refresh_tx(BlockChangeReason::FilterChange);
    }

    /// Get details as pretty-printed string
    pub fn details_pretty_string(&self) -> String {
        self.details.clone()
    }

    /// Get details as raw JSON string
    pub fn details_raw_string(&self) -> String {
        // For now, return the same as pretty (already contains JSON)
        // TODO: Could add a separate raw JSON field if needed
        self.details.clone()
    }

    /// Get JSON for currently focused pane (for copy operation)
    pub fn focused_json_string(&self) -> Option<String> {
        Some(self.get_copy_content())
    }

    /// Get display-ready list of blocks (filtered if filter active)
    pub fn blocks_for_display(&self) -> Vec<&BlockRow> {
        let (blocks, _sel, _total) = self.filtered_blocks();
        blocks
    }

    /// Get display-ready list of transactions from current block (filtered)
    pub fn txs_for_display(&self) -> Vec<&TxLite> {
        if let Some(b) = self.current_block() {
            b.transactions
                .iter()
                .filter(|tx| {
                    // Apply owned-only filter first (if active)
                    if self.owned_only_filter && !self.is_owned_tx(tx) {
                        return false;
                    }
                    // Then apply text filter
                    if filter::is_empty(&self.filter_compiled) {
                        return true;
                    }
                    // Convert TxLite to JSON for filtering (same pattern as txs() method)
                    let v = json!({
                        "hash": tx.hash,
                        "signer_id": tx.signer_id.as_deref().unwrap_or(""),
                        "receiver_id": tx.receiver_id.as_deref().unwrap_or("")
                    });
                    tx_matches_filter(&v, &self.filter_compiled)
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get lightweight block data for display (owned copy)
    /// Returns None if index out of bounds
    pub fn block_lite(&self, idx: usize) -> Option<BlockLite> {
        self.blocks_for_display().get(idx).map(|b| BlockLite {
            height: b.height,
            tx_count: b.tx_count,
            when: b.when.clone(),
            time_utc: format_timestamp_utc(b.timestamp),
        })
    }

    /// Get lightweight transaction data for display (owned copy)
    /// Returns None if index out of bounds
    pub fn tx_lite(&self, idx: usize) -> Option<TxLite> {
        self.txs_for_display().get(idx).map(|tx| (*tx).clone())
    }

    /// Get count of filtered blocks
    pub fn filtered_blocks_len(&self) -> usize {
        self.blocks_for_display().len()
    }

    /// Get count of filtered transactions in current block
    pub fn filtered_txs_len(&self) -> usize {
        self.txs_for_display().len()
    }

    /// Handle mouse click at terminal coordinates
    pub fn handle_mouse_click(&mut self, col: u16, row: u16) {
        // Determine which pane was clicked based on layout
        // The layout is approximately:
        // - Header: 2 rows
        // - Filter bar: 1-3 rows (let's assume 2 average)
        // - Top 30%: Blocks (left 50%) + Txs (right 50%)
        // - Bottom 70%: Details

        // Skip header and filter (approximately 4 rows)
        if row < 4 {
            return;
        }

        // Get terminal size from last draw (this is approximate)
        // In real implementation, we'd pass the actual layout areas
        let term_height: u16 = 40; // Default assumption, will be resized dynamically
        let body_start: u16 = 4;
        let body_height = term_height.saturating_sub(body_start + 1); // -1 for footer
        let top_height = (body_height * 3) / 10; // 30%
        let top_end = body_start + top_height;

        if row < top_end {
            // We're in the top row
            if col < 60 {
                // Left half - Blocks pane
                self.pane = 0;
                // Calculate which block was clicked
                let block_idx = (row - body_start) as usize;
                if block_idx < self.blocks_len() {
                    self.select_block_clamped(block_idx);
                }
            } else {
                // Right half - Transactions pane
                self.pane = 1;
                // Calculate which tx was clicked
                let tx_idx = (row - body_start) as usize;
                if tx_idx < self.txs_len() {
                    self.select_tx_clamped(tx_idx);
                }
            }
        } else {
            // Bottom section - Details pane
            self.pane = 2;
        }
    }

    /// Check if coordinates are within details pane
    pub fn is_details_pane_at(&self, _col: u16, row: u16) -> bool {
        // Similar logic to handle_mouse_click
        // This is approximate - in real implementation we'd use actual layout
        let term_height: u16 = 40;
        let body_start: u16 = 4;
        let body_height = term_height.saturating_sub(body_start + 1);
        let top_height = (body_height * 3) / 10;
        let top_end = body_start + top_height;

        row >= top_end
    }

    /// Handle scroll wheel events
    pub fn handle_scroll(&mut self, col: u16, row: u16, lines: i32) {
        // First determine which pane we're scrolling in
        self.handle_mouse_click(col, row);

        // Then apply the scroll
        match self.pane {
            0 => {
                // Blocks pane
                let current = self.sel_block() as i32;
                let new_idx = (current + lines).max(0) as usize;
                self.select_block_clamped(new_idx);
            }
            1 => {
                // Transactions pane
                let current = self.sel_tx() as i32;
                let new_idx = (current + lines).max(0) as usize;
                self.select_tx_clamped(new_idx);
            }
            2 => {
                // Details pane
                // Scrolling is handled separately via viewport
                if lines > 0 {
                    self.details_scroll = self
                        .details_scroll
                        .saturating_add(lines.unsigned_abs() as u16);
                } else {
                    self.details_scroll = self
                        .details_scroll
                        .saturating_sub(lines.unsigned_abs() as u16);
                }
            }
            _ => {}
        }
    }
}

/// Lightweight block data for display
#[derive(Debug, Clone)]
pub struct BlockLite {
    pub height: u64,
    pub tx_count: usize,
    pub when: String,     // Relative time ("2s ago")
    pub time_utc: String, // UTC timestamp
}

/// Format timestamp as UTC string
fn format_timestamp_utc(timestamp: u64) -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use chrono::{TimeZone, Utc};
        let secs = (timestamp / 1_000_000_000) as i64;
        let nsecs = (timestamp % 1_000_000_000) as u32;
        match Utc.timestamp_opt(secs, nsecs) {
            chrono::LocalResult::Single(dt) => dt.format("%H:%M:%S").to_string(),
            _ => format!("{}s", secs),
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Simplified timestamp for WASM (just show seconds)
        let secs = (timestamp / 1_000_000_000) as i64;
        format!("{}s", secs)
    }
}

impl App {
    // UI Snapshot and Action handling are now centralized in ui_snapshot module
    // Use ui_snapshot::UiSnapshot::from_app(app) instead of app.ui_snapshot()
    // Use ui_snapshot::apply_ui_action(app, action) instead of app.handle_ui_action(action)
}
