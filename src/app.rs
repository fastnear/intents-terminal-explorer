use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use crate::types::{AppEvent, BlockRow, TxLite, WsPayload, ActionSummary};
use crate::util_text::{format_gas, format_near};
use crate::json_pretty::pretty;
use crate::filter::{self, compile_filter, tx_matches_filter, CompiledFilter};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum InputMode { Normal, Filter, Search, Marks, JumpPending }

/// Reason for block selection change - determines tx selection behavior
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BlockChangeReason {
    AutoFollow,      // New block arrived in auto-follow mode (keep tx index)
    ManualNav,       // User manually navigated to different block (reset tx)
    FilterChange,    // Filter was applied/cleared (try to preserve tx)
}

pub struct App {
    quit: bool,
    pane: usize,                   // 0 blocks, 1 txs, 2 details
    blocks: Vec<BlockRow>,
    sel_block_height: Option<u64>, // None = auto-follow newest, Some(height) = locked to specific block
    sel_tx: usize,

    details: String,
    details_scroll: u16,

    fps: u32,
    fps_choices: Vec<u32>,

    keep_blocks: usize,
    manual_block_nav: bool,        // True if user manually navigated (stops auto-selection)

    // Filter state
    filter_query: String,
    filter_compiled: CompiledFilter,
    input_mode: InputMode,

    // Search state
    search_query: String,
    search_results: Vec<crate::history::HistoryHit>,
    search_selection: usize,

    // Marks state
    marks_list: Vec<crate::marks::Mark>,
    marks_selection: usize,

    // Owned accounts state
    owned_accounts: HashSet<String>,     // Lowercase account IDs from ~/.near-credentials
    owned_only_filter: bool,             // Ctrl+U toggle for owned-only view
    owned_counts: HashMap<u64, usize>,   // Cached owned tx count per block height

    // Manually-selected blocks cache (preserves blocks after they age out of rolling buffer)
    cached_blocks: HashMap<u64, BlockRow>,  // height -> block
    cached_block_order: Vec<u64>,           // LRU tracking for cache eviction

    // Archival fetch state (for fetching historical blocks beyond cache)
    loading_block: Option<u64>,                                   // Block height currently being fetched from archival
    archival_fetch_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>,  // Channel to request archival fetches

    // Debug log (for development)
    debug_log: Vec<String>,              // Rolling buffer of debug messages
    debug_visible: bool,                 // Toggle debug panel visibility (Ctrl+D)

    // Toast notification state
    toast_message: Option<(String, Instant)>,  // (message, timestamp)

    // UI layout state
    details_fullscreen: bool,                  // Spacebar toggle for 100% details view
    details_viewport_height: u16,              // Actual visible height of details pane (set by UI layer)
}

/// Recursively format an action for PRETTY view display
fn format_action(action: &ActionSummary) -> serde_json::Value {
    match action {
        ActionSummary::CreateAccount => json!({"type": "CreateAccount"}),
        ActionSummary::DeployContract { code_len } => {
            json!({"type": "DeployContract", "code_size": format!("{} bytes", code_len)})
        }
        ActionSummary::FunctionCall { method_name, args_decoded, gas, deposit, .. } => {
            use crate::near_args::DecodedArgs;

            let args_display = match args_decoded {
                DecodedArgs::Json(v) => {
                    // Auto-parse nested JSON-serialized strings for better readability
                    crate::json_auto_parse::auto_parse_nested_json(v.clone(), 5, 0)
                }
                DecodedArgs::Text(t) => json!(t),
                DecodedArgs::Bytes { preview, .. } => json!(format!("[binary: {}]", preview)),
                DecodedArgs::Empty => json!({}),
                DecodedArgs::Error(e) => json!(format!("<decode error: {}>", e)),
            };

            json!({
                "type": "FunctionCall",
                "method": method_name,
                "args": args_display,
                "gas": format_gas(*gas),
                "deposit": format_near(*deposit),
            })
        }
        ActionSummary::Transfer { deposit } => {
            json!({"type": "Transfer", "amount": format_near(*deposit)})
        }
        ActionSummary::Stake { stake, public_key } => {
            json!({"type": "Stake", "amount": format_near(*stake), "public_key": public_key})
        }
        ActionSummary::AddKey { public_key, access_key } => {
            // Parse access_key if it's stringified JSON (same pattern as FunctionCall args)
            let parsed_access_key = if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(access_key) {
                crate::json_auto_parse::auto_parse_nested_json(json_val, 5, 0)
            } else {
                json!(access_key)  // Fallback to string if not valid JSON
            };
            json!({"type": "AddKey", "public_key": public_key, "access_key": parsed_access_key})
        }
        ActionSummary::DeleteKey { public_key } => {
            json!({"type": "DeleteKey", "public_key": public_key})
        }
        ActionSummary::DeleteAccount { beneficiary_id } => {
            json!({"type": "DeleteAccount", "beneficiary": beneficiary_id})
        }
        ActionSummary::Delegate { sender_id, receiver_id, actions } => {
            // Recursively format nested actions
            let nested_formatted: Vec<serde_json::Value> = actions.iter()
                .map(|nested_action| format_action(nested_action))
                .collect();
            json!({
                "type": "Delegate",
                "sender": sender_id,
                "receiver": receiver_id,
                "actions": nested_formatted
            })
        }
    }
}

impl App {
    pub fn new(
        fps:u32,
        fps_choices:Vec<u32>,
        keep_blocks:usize,
        default_filter:String,
        archival_fetch_tx: Option<tokio::sync::mpsc::UnboundedSender<u64>>,
    ) -> Self {
        let filter_compiled = if default_filter.is_empty() {
            CompiledFilter::default()
        } else {
            compile_filter(&default_filter)
        };

        Self {
            quit:false, pane:0,
            blocks:Vec::with_capacity(keep_blocks),
            sel_block_height:None, sel_tx:0,  // Start in auto-follow mode
            details:"(No blocks yet)".into(),
            details_scroll:0,
            fps, fps_choices, keep_blocks,
            manual_block_nav: false,
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
            debug_visible: false,  // Hidden by default
            toast_message: None,
            details_fullscreen: false,  // Normal view by default
            details_viewport_height: 20,  // Default estimate, will be updated by UI
        }
    }

    // ----- getters -----
    pub fn fps(&self)->u32{ self.fps }
    pub fn quit_flag(&self)->bool{ self.quit }
    pub fn pane(&self)->usize{ self.pane }
    pub fn is_viewing_cached_block(&self)->bool {
        if let Some(height) = self.sel_block_height {
            self.find_block_index(Some(height)).is_none() && self.cached_blocks.contains_key(&height)
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
            None => None,  // Auto-follow mode: return None to trigger auto-follow logic in current_block()
            Some(h) => self.blocks.iter().position(|b| b.height == h),
        }
    }

    /// Get the currently selected block (fallback to cache if aged out of main buffer)
    /// Filter-aware in auto-follow mode: returns first block with matching transactions
    fn current_block(&self) -> Option<&BlockRow> {
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

        block.transactions.iter()
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
            let idx = self.find_block_index(self.sel_block_height)
                .or_else(|| if !self.blocks.is_empty() { Some(0) } else { None });
            let refs: Vec<&BlockRow> = self.blocks.iter().collect();
            return (refs, idx, total);
        }

        // Filter blocks with matching transactions
        let filtered: Vec<&BlockRow> = self.blocks.iter()
            .filter(|block| self.count_matching_txs(block) > 0)
            .collect();

        // Find selected block index in filtered list
        let sel_idx = if let Some(height) = self.sel_block_height {
            filtered.iter().position(|b| b.height == height)
                .or_else(|| if !filtered.is_empty() { Some(0) } else { None })
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
            self.blocks.iter()
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

    pub fn txs(&self)->(Vec<TxLite>, usize, usize) {
        if let Some(b) = self.current_block() {
            let total = b.transactions.len();
            let filtered: Vec<TxLite> = b.transactions.iter()
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

    pub fn details(&self)->&str {
        &self.details
    }
    pub fn details_scroll(&self)->u16 { self.details_scroll }
    pub fn input_mode(&self)->InputMode { self.input_mode }
    pub fn filter_query(&self)->&str { &self.filter_query }
    pub fn owned_only_filter(&self)->bool { self.owned_only_filter }
    #[allow(dead_code)]
    pub fn owned_accounts(&self)->&HashSet<String> { &self.owned_accounts }
    pub fn debug_log(&self)->&[String] { &self.debug_log }
    pub fn debug_visible(&self)->bool { self.debug_visible }
    pub fn details_fullscreen(&self)->bool { self.details_fullscreen }
    pub fn loading_block(&self)->Option<u64> { self.loading_block }

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
    pub fn cycle_fps(&mut self){
        if self.fps_choices.is_empty() { return; }
        let mut idx = self.fps_choices.iter().position(|&v| v==self.fps).unwrap_or(0);
        idx = (idx+1)%self.fps_choices.len();
        self.fps = self.fps_choices[idx];
    }

    pub fn log_debug(&mut self, msg: String) {
        const MAX_LOG_ENTRIES: usize = 50;

        // Write to file for debugging
        use std::fs::OpenOptions;
        use std::io::Write;
        let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open("ratacat_debug.log")
        {
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
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
        const MAX_TOTAL_CACHED: usize = 50;  // Safety limit

        // Find the center block's index
        let center_idx = match self.find_block_index(Some(center_height)) {
            Some(idx) => idx,
            None => return,  // Center block not in buffer, can't cache context
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
                if !self.cached_blocks.contains_key(&height) {
                    self.cached_blocks.insert(height, block.clone());
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
            self.log_debug(format!("Cached block #{} with ±{} context ({} new, {} total)",
                center_height, CONTEXT_RANGE, cached_count, self.cached_blocks.len()));
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
        self.log_debug(format!("Debug panel: {}", if self.debug_visible { "visible" } else { "hidden" }));
    }

    /// Toggle details fullscreen mode (Spacebar when details pane focused)
    pub fn toggle_details_fullscreen(&mut self) {
        if self.pane == 2 {  // Only toggle when details pane is focused
            self.details_fullscreen = !self.details_fullscreen;
            let mode = if self.details_fullscreen { "fullscreen" } else { "normal" };
            self.log_debug(format!("Details view: {}", mode));
        }
    }

    /// Return to auto-follow mode (track newest block)
    pub fn return_to_auto_follow(&mut self) {
        let old_height = self.sel_block_height;
        self.manual_block_nav = false;
        self.sel_block_height = None;  // None = auto-follow newest
        self.sel_tx = 0;  // Reset to first tx
        if !self.blocks.is_empty() {
            self.validate_and_refresh_tx(BlockChangeReason::AutoFollow);
            self.log_debug(format!("[USER_ACTION] Return to AUTO-FOLLOW mode (Home key pressed), was locked to height={:?}", old_height));
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
        self.validate_and_refresh_tx(BlockChangeReason::FilterChange);  // Try to preserve tx
    }

    pub fn apply_filter(&mut self) {
        self.filter_compiled = compile_filter(&self.filter_query);
        self.input_mode = InputMode::Normal;
        self.validate_and_refresh_tx(BlockChangeReason::FilterChange);  // Try to preserve tx
    }

    pub fn filter_add_char(&mut self, ch: char) {
        self.filter_query.push(ch);
    }

    pub fn filter_backspace(&mut self) {
        self.filter_query.pop();
    }

    // ----- copy functionality -----
    pub fn get_copy_content(&self) -> String {
        match self.pane {
            0 => self.copy_block_info(),
            1 => self.copy_tx_hash(),
            2 => self.details().to_string(),
            _ => String::new(),
        }
    }

    fn copy_block_info(&self) -> String {
        if let Some(block) = self.current_block() {
            format!("#{} | {}", block.height, block.hash)
        } else {
            String::new()
        }
    }

    fn copy_tx_hash(&self) -> String {
        if self.current_block().is_some() {
            let (filtered_txs, _, _) = self.txs();
            if let Some(tx) = filtered_txs.get(self.sel_tx) {
                // If we have signer/receiver, format as "signer → receiver | hash"
                if let (Some(signer), Some(receiver)) = (&tx.signer_id, &tx.receiver_id) {
                    format!("{} → {} | {}", signer, receiver, tx.hash)
                } else {
                    tx.hash.clone()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    // ----- selection / scrolling -----
    /// Move focus to next pane (circular: 0→1→2→0)
    pub fn next_pane(&mut self){
        self.pane = (self.pane + 1) % 3;
        self.log_debug(format!("Tab -> pane={}", self.pane));
    }

    /// Move focus to previous pane (circular: 2→1→0→2)
    pub fn prev_pane(&mut self){
        // Backward navigation: subtract 1 with wrap-around
        // (pane - 1 + 3) % 3 ensures we don't underflow (e.g., 0-1 = -1)
        self.pane = (self.pane + 3 - 1) % 3;
        self.log_debug(format!("BackTab -> pane={}", self.pane));
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
    pub fn up(&mut self){
        match self.pane {
            0 => {  // Blocks pane: navigate to previous block (newer)
                self.log_debug(format!("[USER_NAV_UP] manual_nav={}, sel_height={:?}", self.manual_block_nav, self.sel_block_height));

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
                        self.manual_block_nav = true;
                        self.cache_block_with_context(h);
                        self.log_debug(format!("[USER_NAV_UP] edge case lock to #{}", h));
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
                            self.manual_block_nav = true;
                            self.details_scroll = 0;
                            self.cache_block_with_context(new_height);
                            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                            self.log_debug(format!("Blocks UP -> #{}", new_height));
                        } else {
                            // Block not available - try archival fetch
                            self.log_debug(format!("Blocks UP -> #{} not available", new_height));
                            self.request_archival_block(new_height);
                        }
                    }
                } else {
                    // Current selection not in navigation list (filtered out), jump to newest
                    let new_height = nav_list[0];
                    self.sel_block_height = Some(new_height);
                    self.manual_block_nav = true;
                    self.details_scroll = 0;
                    self.cache_block_with_context(new_height);
                    self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                    self.log_debug(format!("Blocks UP -> not in list, jump to newest #{}", new_height));
                }
            }
            1 => {  // Tx pane: navigate to previous transaction
                if self.sel_tx > 0 {
                    self.sel_tx -= 1;
                    self.details_scroll = 0;
                    self.select_tx();
                    self.log_debug(format!("Tx UP, sel={}", self.sel_tx));
                }
            }
            2 => {  // Details pane: scroll up
                self.scroll_details(-1);
            }
            _ => {}
        }
    }
    /// Handle Down arrow key - behavior depends on which pane is focused
    pub fn down(&mut self){
        match self.pane {
            0 => {  // Blocks pane: navigate to next block (older)
                self.log_debug(format!("[USER_NAV_DOWN] manual_nav={}, sel_height={:?}", self.manual_block_nav, self.sel_block_height));

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
                            self.manual_block_nav = true;
                            self.details_scroll = 0;
                            self.cache_block_with_context(next_h);
                            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                            self.log_debug(format!("[USER_NAV_DOWN] first press, move to #{}", next_h));
                        } else {
                            // Only one block, just lock to it
                            self.sel_block_height = Some(h);
                            self.manual_block_nav = true;
                            self.cache_block_with_context(h);
                            self.log_debug(format!("Blocks DOWN -> only one block, lock to #{}", h));
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
                            self.manual_block_nav = true;
                            self.details_scroll = 0;
                            self.cache_block_with_context(new_height);
                            self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                            self.log_debug(format!("Blocks DOWN -> #{}", new_height));
                        } else {
                            // Block not available - try archival fetch
                            self.log_debug(format!("Blocks DOWN -> #{} not available", new_height));
                            self.request_archival_block(new_height);
                        }
                    }
                } else {
                    // Current selection not in navigation list (filtered out), jump to newest
                    let new_height = nav_list[0];
                    self.sel_block_height = Some(new_height);
                    self.manual_block_nav = true;
                    self.details_scroll = 0;
                    self.cache_block_with_context(new_height);
                    self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                    self.log_debug(format!("Blocks DOWN -> not in list, jump to newest #{}", new_height));
                }
            }
            1 => {  // Tx pane: navigate to next transaction
                let (txs, _, _) = self.txs();
                if self.sel_tx + 1 < txs.len() {
                    self.sel_tx += 1;
                    self.details_scroll = 0;
                    self.select_tx();
                    self.log_debug(format!("Tx DOWN, sel={}", self.sel_tx));
                }
            }
            2 => {  // Details pane: scroll down
                self.scroll_details(1);
            }
            _ => {}
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
                        self.sel_block_height = Some(newest_height);  // Lock to newest
                        self.manual_block_nav = true;  // Enter manual mode
                        self.details_scroll = 0;
                        self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                        self.log_debug(format!("Left -> jump to newest #{}", newest_height));
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
                    self.sel_block_height = Some(new_height);  // Lock to specific height
                    self.manual_block_nav = true;  // Enter manual mode
                    self.details_scroll = 0;
                    self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
                    self.log_debug(format!("Right -> paginate to block #{}", new_height));
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

    pub fn page_up(&mut self, page:u16){ if self.pane==2 { self.scroll_details(-(page as i32)); } }
    pub fn page_down(&mut self, page:u16){ if self.pane==2 { self.scroll_details(page as i32); } }
    pub fn home(&mut self){ if self.pane==2 { self.details_scroll = 0; } }
    pub fn end(&mut self){
        if self.pane==2 {
            // Jump to bottom (clamped to actual content height)
            let content_lines = self.details.lines().count() as u16;
            self.details_scroll = content_lines.saturating_sub(15);  // ~typical viewport size
        }
    }

    fn scroll_details(&mut self, delta:i32){
        let cur = self.details_scroll as i32;

        // Calculate actual content height for max scroll
        // Use actual viewport height (set by UI layer) for accurate clamping
        let content_lines = self.details.lines().count() as u16;
        let max_scroll = content_lines.saturating_sub(self.details_viewport_height);

        let next = (cur + delta).max(0).min(max_scroll as i32);
        self.details_scroll = next as u16;
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
                self.log_debug(format!("Requesting archival fetch for block #{}", height));
                if let Err(e) = tx.send(height) {
                    self.log_debug(format!("Failed to send archival fetch request: {}", e));
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
                    let formatted_actions: Vec<serde_json::Value> = actions.iter().map(|action| {
                        format_action(action)
                    }).collect();
                    pretty_json["actions"] = json!(formatted_actions);
                }

                self.details = pretty(&pretty_json, 2);
            }
        }
    }

    // ----- events -----
    pub fn on_event(&mut self, ev:AppEvent) {
        match ev {
            AppEvent::Quit => self.quit = true,
            AppEvent::FromWs(WsPayload::Block{data}) => {
                self.push_block(BlockRow{
                    height: data,
                    hash: "".into(),
                    timestamp: 0,
                    tx_count: 0,
                    when: "".into(),
                    transactions: vec![]
                });
            }
            AppEvent::FromWs(WsPayload::Tx{identifier:_, data}) => {
                if let Some(t) = data {
                    // For WS summary, show pretty-formatted JSON
                    let raw = serde_json::to_value(&t).unwrap_or(serde_json::json!({}));
                    self.details = pretty(&raw, 2);
                    self.details_scroll = 0;
                }
            }
            AppEvent::NewBlock(b) => {
                // Check if this block was being fetched from archival
                if self.loading_block == Some(b.height) {
                    self.loading_block = None;  // Clear loading state
                }
                self.push_block(b);
            }
        }
    }

    fn push_block(&mut self, b:BlockRow){
        // Compute owned count for this block before inserting
        let owned_count = self.count_owned_txs(&b.transactions);
        let height = b.height;

        // Log state BEFORE push
        self.log_debug(format!("[PUSH_START] Block #{}, manual_nav={}, sel_height={:?}, blocks_count={}",
            height, self.manual_block_nav, self.sel_block_height, self.blocks.len()));

        self.blocks.insert(0, b);
        if self.blocks.len()>self.keep_blocks {
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
        if !self.manual_block_nav {
            // Auto-follow mode: always track newest block (index 0)
            if self.blocks.len() == 1 {
                // First block: select it and reset tx
                self.sel_block_height = None;  // None = auto-follow newest
                self.sel_tx = 0;
                self.select_tx();

                // Auto-lock if this block has matching transactions (provides stability)
                if self.count_matching_txs(&self.blocks[0]) > 0 {
                    self.sel_block_height = Some(height);
                    self.manual_block_nav = true;
                    self.log_debug(format!("[AUTO_LOCK] Block #{} arr, FIRST MATCH, auto-locked -> manual_nav=true, sel_height={:?}", height, self.sel_block_height));
                } else {
                    self.log_debug(format!("[AUTO_FOLLOW] Block #{} arr, FIRST block, auto-follow ON (no matches) -> manual_nav=false, sel_height=None", height));
                }
            } else {
                // Subsequent blocks: check if we should auto-lock to first matching block
                let current_matches = self.current_block()
                    .map(|b| self.count_matching_txs(b))
                    .unwrap_or(0);
                let new_matches = self.count_matching_txs(&self.blocks[0]);

                self.log_debug(format!("[AUTO_FOLLOW_CHECK] Block #{}, cur_matches={}, new_matches={}, sel_height={:?}",
                    height, current_matches, new_matches, self.sel_block_height));

                if current_matches == 0 && new_matches > 0 {
                    // Switching from no matches to has matches - auto-lock for stability
                    self.sel_block_height = Some(height);
                    self.manual_block_nav = true;
                    self.sel_tx = 0;
                    self.select_tx();
                    self.log_debug(format!("[AUTO_LOCK] Block #{} arr, FIRST MATCH, auto-locked -> manual_nav=true, sel_height={:?}", height, self.sel_block_height));
                } else {
                    // Continue auto-follow
                    let old_sel_height = self.sel_block_height;
                    self.sel_block_height = None;
                    self.validate_and_refresh_tx(BlockChangeReason::AutoFollow);
                    self.log_debug(format!("[AUTO_FOLLOW] Block #{} arr, continuing auto-follow, cur_matches={}, new_matches={}, sel_height: {:?} -> None",
                        height, current_matches, new_matches, old_sel_height));
                }
            }
        } else {
            // Manual mode: maintain locked block height (block may be in cache if aged out)
            if let Some(locked_height) = self.sel_block_height {
                if self.find_block_index(Some(locked_height)).is_some() {
                    // Block still in main buffer
                    self.log_debug(format!("Block #{} arr, MANUAL mode locked to #{}", height, locked_height));
                } else if self.cached_blocks.contains_key(&locked_height) {
                    // Block aged out but available in cache
                    self.log_debug(format!("[MANUAL_CACHED] Block #{} arr, MANUAL mode viewing cached block #{}", height, locked_height));
                } else {
                    // Block not in buffer or cache - shouldn't happen, but handle gracefully
                    self.log_debug(format!("[FALLBACK] Block #{} arr, WARNING: locked block #{} not found, FORCING auto-follow -> manual_nav=false, sel_height=None", height, locked_height));
                    self.manual_block_nav = false;
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
    pub fn open_marks(&mut self, marks_list: Vec<crate::marks::Mark>) {
        self.marks_list = marks_list;
        self.marks_selection = 0;
        self.input_mode = InputMode::Marks;
    }

    pub fn close_marks(&mut self) {
        self.input_mode = InputMode::Normal;
        self.marks_list.clear();
        self.marks_selection = 0;
    }

    #[allow(dead_code)]
    pub fn marks_list(&self) -> &[crate::marks::Mark] {
        &self.marks_list
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

    pub fn get_selected_mark(&self) -> Option<&crate::marks::Mark> {
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

    pub fn jump_to_mark(&mut self, mark: &crate::marks::Mark) {
        // Navigate to the mark's location
        if let Some(height) = mark.height {
            if self.blocks.iter().any(|b| b.height == height) {
                self.sel_block_height = Some(height);  // Lock to specific block height
                self.manual_block_nav = true;  // Jumping to mark locks the selection
                self.validate_and_refresh_tx(BlockChangeReason::ManualNav);
            }
        }
        self.pane = mark.pane as usize;
        if self.pane > 2 {
            self.pane = 0;
        }
    }
}
