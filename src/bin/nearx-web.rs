//! Ratacat Web Binary - egui + egui_ratatui
//!
//! Production-grade web harness using egui for WebGL canvas rendering
//! and egui_ratatui to bridge ratatui widgets into egui.
//!
//! This solves all the DOM timing issues we had with Ratzilla by using
//! immediate-mode Canvas rendering instead of DOM manipulation.

#![cfg_attr(target_arch = "wasm32", no_main)]

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use std::{cell::RefCell, rc::Rc};
        use egui_ratatui::RataguiBackend;
        use embedded_graphics_unicodefonts::{
            mono_9x18_atlas, mono_9x18_bold_atlas,
        };
        use soft_ratatui::{EmbeddedGraphics, SoftBackend};
        use ratatui::Terminal;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsValue;
        use eframe::wasm_bindgen;

        use nearx::{
            App, InputMode,
            config::{Config, Source},
            types::{AppEvent, Mark},
            source_rpc,
            theme::Theme,
            ui,
            debug::{self, cat},
        };

        // ---------------------------
        // Egui Application
        // ---------------------------

        struct RatacatApp {
            terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
            app: Rc<RefCell<App>>,
            event_rx: Rc<RefCell<tokio::sync::mpsc::UnboundedReceiver<AppEvent>>>,
            last_egui_theme: Option<Theme>,
            last_hash: Option<String>, // Track URL hash for deep link routing
            last_dpr: Option<f32>,     // Track devicePixelRatio for Hi-DPI snapping
            // In-memory marks storage (web doesn't persist across sessions)
            marks: Rc<RefCell<Vec<Mark>>>,
            marks_cursor: Rc<RefCell<usize>>,
            // Debug state
            debug_inited: bool,
            debug_overlay: bool,
        }

        impl RatacatApp {
            fn new(config: Config, event_rx: tokio::sync::mpsc::UnboundedReceiver<AppEvent>) -> Self {
                let app = App::new(
                    config.render_fps,
                    config.render_fps_choices.clone(),
                    config.keep_blocks,
                    config.default_filter.clone(),
                    None, // No archival fetch for web
                );

                // Create egui_ratatui backend with soft renderer and bitmap fonts
                // Using 9x18 fonts for better readability (matches Tauri)
                let font_regular = mono_9x18_atlas();
                let font_bold = Some(mono_9x18_bold_atlas());
                // Note: 9x18 lacks italic variant, using regular as fallback
                let font_italic = Some(mono_9x18_atlas());

                let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
                    100,  // width in columns (web-optimized with 9x18 fonts)
                    35,   // height in rows (web-optimized with 9x18 fonts)
                    font_regular,
                    font_bold,
                    font_italic,
                );
                let backend = RataguiBackend::new("ratacat", soft_backend);
                let terminal = Terminal::new(backend).expect("Failed to create terminal");

                Self {
                    terminal,
                    app: Rc::new(RefCell::new(app)),
                    event_rx: Rc::new(RefCell::new(event_rx)),
                    last_egui_theme: None,
                    last_hash: None,
                    last_dpr: None,
                    marks: Rc::new(RefCell::new(Vec::new())),
                    marks_cursor: Rc::new(RefCell::new(0)),
                    debug_inited: false,
                    debug_overlay: false,
                }
            }

            fn handle_input(&mut self, ctx: &egui::Context) -> (bool, bool) {
                // Track whether Tab or Shift+Tab was pressed (for consumption outside this closure)
                let mut tab_pressed = false;
                let mut shift_pressed = false;

                // Process keyboard input from egui
                ctx.input(|i| {
                    for event in &i.events {
                        if let egui::Event::Key { key, pressed, modifiers, .. } = event {
                            if !pressed {
                                continue;
                            }

                            let mut app = self.app.borrow_mut();

                            // Handle filter mode
                            if app.input_mode() == InputMode::Filter {
                                match key {
                                    egui::Key::Backspace => app.filter_backspace(),
                                    egui::Key::Enter => app.apply_filter(),
                                    egui::Key::Escape => app.clear_filter(),
                                    _ => {
                                        // Handle text input
                                        if let Some(c) = self.key_to_char(key, modifiers) {
                                            app.filter_add_char(c);
                                        }
                                    }
                                }
                                continue;
                            }

                            // Handle search mode
                            if app.input_mode() == InputMode::Search {
                                match key {
                                    egui::Key::Backspace => app.search_backspace(),
                                    egui::Key::Enter => app.close_search(),
                                    egui::Key::ArrowUp => app.search_up(),
                                    egui::Key::ArrowDown => app.search_down(),
                                    egui::Key::Escape => app.close_search(),
                                    _ => {
                                        if let Some(c) = self.key_to_char(key, modifiers) {
                                            app.search_add_char(c);
                                        }
                                    }
                                }
                                continue;
                            }

                            // Handle marks overlay mode
                            if app.input_mode() == InputMode::Marks {
                                match key {
                                    egui::Key::ArrowUp => app.marks_up(),
                                    egui::Key::ArrowDown => app.marks_down(),
                                    egui::Key::Escape => app.close_marks(),
                                    egui::Key::Enter => {
                                        // Jump to selected mark
                                        drop(app);
                                        // TODO: Implement mark jump navigation
                                        let mut app = self.app.borrow_mut();
                                        app.close_marks();
                                        app.show_toast("Mark jump (TODO)".to_string());
                                    }
                                    egui::Key::D if !modifiers.ctrl => {
                                        // Delete selected mark
                                        drop(app);
                                        let app_ref = self.app.borrow();
                                        if let Some(mark) = app_ref.marks_list().get(app_ref.marks_selection()) {
                                            let label = mark.label.clone();
                                            drop(app_ref);
                                            self.marks_remove_by_label(&label);
                                            let mut app = self.app.borrow_mut();
                                            let marks_list = self.marks_list();
                                            app.open_marks(marks_list);
                                            app.show_toast(format!("Deleted mark '{}'", label));
                                        }
                                    }
                                    _ => {}
                                }
                                continue;
                            }

                            // Normal mode - Handle Tab/Shift+Tab navigation
                            if matches!(key, egui::Key::Tab) {
                                // Navigate panes
                                if modifiers.shift {
                                    app.prev_pane();
                                    debug::log(cat::INPUT, "Tab ← prev pane");
                                    shift_pressed = true;
                                } else {
                                    app.next_pane();
                                    debug::log(cat::INPUT, "Tab → next pane");
                                }
                                tab_pressed = true;
                                continue;
                            }

                            // Skip normal mode shortcuts if egui wants keyboard input (e.g., text field focused)
                            // Note: Filter and Search modes already handled above, Tab handled above
                            if ctx.wants_keyboard_input() {
                                continue;
                            }

                            // Normal mode
                            match (key, modifiers.ctrl || modifiers.command) {
                                (egui::Key::Q, false) | (egui::Key::C, true) => {
                                    log::info!("Quit requested (close tab)");
                                    app.show_toast("Press Ctrl+W to close tab".to_string());
                                }
                                (egui::Key::ArrowUp, false) => app.up(),
                                (egui::Key::ArrowDown, false) => app.down(),
                                (egui::Key::ArrowLeft, false) => app.left(),
                                (egui::Key::ArrowRight, false) => app.right(),
                                (egui::Key::PageUp, false) => app.page_up(20),
                                (egui::Key::PageDown, false) => app.page_down(20),
                                (egui::Key::Home, false) => {
                                    if app.pane() == 0 {
                                        app.return_to_auto_follow();
                                    } else {
                                        app.home();
                                    }
                                }
                                (egui::Key::End, false) => app.end(),
                                (egui::Key::Enter, false) => app.select_tx(),
                                (egui::Key::O, true) => app.cycle_fps(),
                                (egui::Key::D, true) if modifiers.shift => {
                                    // Ctrl+Shift+D - Toggle debug overlay
                                    drop(app);
                                    self.debug_overlay = !self.debug_overlay;
                                    debug::set_overlay(self.debug_overlay);
                                    debug::log(cat::UI, format!("debug overlay {}", if self.debug_overlay { "ON" } else { "OFF" }));
                                }
                                (egui::Key::D, true) => app.toggle_debug_panel(),
                                (egui::Key::F, true) => app.start_search(),
                                (egui::Key::U, true) => app.toggle_owned_filter(),
                                (egui::Key::Space, false) if app.pane() == 2 => app.toggle_details_fullscreen(),
                                (egui::Key::C, false) => {
                                    // Copy using unified copy_api (pane-aware)
                                    // Note: copy_api delegates to platform::copy_to_clipboard which uses the unified bridge
                                    drop(app); // Release borrow before copy_current
                                    let pane = self.app.borrow().pane();
                                    if nearx::copy_api::copy_current(&self.app.borrow()) {
                                        let mut app = self.app.borrow_mut();
                                        let msg = match app.pane() {
                                            0 => "Copied block info",
                                            1 => "Copied tx hash",
                                            2 => "Copied details",
                                            _ => "Copied",
                                        };
                                        debug::log(cat::COPY, format!("copy_current ok=true pane={}", pane));
                                        app.show_toast(msg.to_string());
                                    } else {
                                        debug::log(cat::COPY, format!("copy_current ok=false pane={}", pane));
                                        self.app.borrow_mut().show_toast("Copy failed".to_string());
                                    }
                                }
                                (egui::Key::Slash, false) => app.start_filter(),
                                (egui::Key::F, false) if !modifiers.ctrl => app.start_filter(),
                                (egui::Key::Escape, false) => {
                                    // Esc: close details overlay if open; else clear filter (Web/Tauri UX)
                                    if app.details_fullscreen() {
                                        debug::log(cat::INPUT, "Esc -> close details overlay");
                                        app.toggle_details_fullscreen();
                                    } else if !app.filter_query().is_empty() {
                                        debug::log(cat::INPUT, "Esc -> clear filter");
                                        app.clear_filter();
                                    }
                                }
                                (egui::Key::Backtick, false) => {
                                    // Backtick (`) - Quick toggle for debug overlay
                                    drop(app);
                                    self.debug_overlay = !self.debug_overlay;
                                    debug::set_overlay(self.debug_overlay);
                                    debug::log(cat::UI, format!("debug overlay {} (backtick)", if self.debug_overlay { "ON" } else { "OFF" }));
                                }

                                // ===== Marks System =====
                                (egui::Key::M, false) if modifiers.shift => {
                                    // Shift+M - Open marks overlay
                                    let marks_list = self.marks_list();
                                    app.open_marks(marks_list);
                                }
                                (egui::Key::M, false) if !modifiers.shift && !modifiers.ctrl => {
                                    // m - Set mark at current position
                                    drop(app); // Release borrow
                                    let mut app = self.app.borrow_mut();
                                    let label = self.marks_next_auto_label();
                                    let pane = app.pane();
                                    let height = app.current_block().map(|b| b.height);
                                    let tx_hash = if pane == 1 {
                                        let (txs, _, _) = app.txs();
                                        let sel = app.sel_tx();
                                        txs.get(sel).map(|tx| tx.hash.clone())
                                    } else {
                                        None
                                    };
                                    self.marks_add_or_replace(label.clone(), pane as u8, height, tx_hash);
                                    app.show_toast(format!("Mark '{}' set", label));
                                }
                                (egui::Key::P, true) => {
                                    // Ctrl+P - Pin/unpin current mark
                                    drop(app); // Release borrow
                                    let mut app = self.app.borrow_mut();
                                    let pane = app.pane();
                                    let height = app.current_block().map(|b| b.height);
                                    let tx_hash = if pane == 1 {
                                        let (txs, _, _) = app.txs();
                                        let sel = app.sel_tx();
                                        txs.get(sel).map(|tx| tx.hash.clone())
                                    } else {
                                        None
                                    };
                                    if let Some(label) = self.marks_find_by_context(pane as u8, height, tx_hash.as_deref()) {
                                        self.marks_toggle_pin(&label);
                                        app.show_toast(format!("Toggled pin for mark '{}'", label));
                                    } else {
                                        // No mark at current position, create and pin
                                        let label = self.marks_next_auto_label();
                                        self.marks_add_or_replace(label.clone(), pane as u8, height, tx_hash);
                                        self.marks_toggle_pin(&label);
                                        app.show_toast(format!("Mark '{}' set and pinned", label));
                                    }
                                }
                                (egui::Key::OpenBracket, false) => {
                                    // [ - Previous mark
                                    drop(app);
                                    if let Some(mark) = self.marks_prev() {
                                        let mut app = self.app.borrow_mut();
                                        // TODO: Navigate to mark position
                                        app.show_toast(format!("Mark '{}' (prev)", mark.label));
                                    }
                                }
                                (egui::Key::CloseBracket, false) => {
                                    // ] - Next mark
                                    drop(app);
                                    if let Some(mark) = self.marks_next() {
                                        let mut app = self.app.borrow_mut();
                                        // TODO: Navigate to mark position
                                        app.show_toast(format!("Mark '{}' (next)", mark.label));
                                    }
                                }

                                _ => {}
                            }
                        }
                    }
                });

                // Return Tab press state for consumption outside the closure
                (tab_pressed, shift_pressed)
            }

            fn key_to_char(&self, key: &egui::Key, modifiers: &egui::Modifiers) -> Option<char> {
                // Convert egui key to character
                match key {
                    egui::Key::Space => Some(' '),
                    egui::Key::A => Some(if modifiers.shift { 'A' } else { 'a' }),
                    egui::Key::B => Some(if modifiers.shift { 'B' } else { 'b' }),
                    egui::Key::C => Some(if modifiers.shift { 'C' } else { 'c' }),
                    egui::Key::D => Some(if modifiers.shift { 'D' } else { 'd' }),
                    egui::Key::E => Some(if modifiers.shift { 'E' } else { 'e' }),
                    egui::Key::F => Some(if modifiers.shift { 'F' } else { 'f' }),
                    egui::Key::G => Some(if modifiers.shift { 'G' } else { 'g' }),
                    egui::Key::H => Some(if modifiers.shift { 'H' } else { 'h' }),
                    egui::Key::I => Some(if modifiers.shift { 'I' } else { 'i' }),
                    egui::Key::J => Some(if modifiers.shift { 'J' } else { 'j' }),
                    egui::Key::K => Some(if modifiers.shift { 'K' } else { 'k' }),
                    egui::Key::L => Some(if modifiers.shift { 'L' } else { 'l' }),
                    egui::Key::M => Some(if modifiers.shift { 'M' } else { 'm' }),
                    egui::Key::N => Some(if modifiers.shift { 'N' } else { 'n' }),
                    egui::Key::O => Some(if modifiers.shift { 'O' } else { 'o' }),
                    egui::Key::P => Some(if modifiers.shift { 'P' } else { 'p' }),
                    egui::Key::Q => Some(if modifiers.shift { 'Q' } else { 'q' }),
                    egui::Key::R => Some(if modifiers.shift { 'R' } else { 'r' }),
                    egui::Key::S => Some(if modifiers.shift { 'S' } else { 's' }),
                    egui::Key::T => Some(if modifiers.shift { 'T' } else { 't' }),
                    egui::Key::U => Some(if modifiers.shift { 'U' } else { 'u' }),
                    egui::Key::V => Some(if modifiers.shift { 'V' } else { 'v' }),
                    egui::Key::W => Some(if modifiers.shift { 'W' } else { 'w' }),
                    egui::Key::X => Some(if modifiers.shift { 'X' } else { 'x' }),
                    egui::Key::Y => Some(if modifiers.shift { 'Y' } else { 'y' }),
                    egui::Key::Z => Some(if modifiers.shift { 'Z' } else { 'z' }),
                    egui::Key::Num0 => Some(if modifiers.shift { ')' } else { '0' }),
                    egui::Key::Num1 => Some(if modifiers.shift { '!' } else { '1' }),
                    egui::Key::Num2 => Some(if modifiers.shift { '@' } else { '2' }),
                    egui::Key::Num3 => Some(if modifiers.shift { '#' } else { '3' }),
                    egui::Key::Num4 => Some(if modifiers.shift { '$' } else { '4' }),
                    egui::Key::Num5 => Some(if modifiers.shift { '%' } else { '5' }),
                    egui::Key::Num6 => Some(if modifiers.shift { '^' } else { '6' }),
                    egui::Key::Num7 => Some(if modifiers.shift { '&' } else { '7' }),
                    egui::Key::Num8 => Some(if modifiers.shift { '*' } else { '8' }),
                    egui::Key::Num9 => Some(if modifiers.shift { '(' } else { '9' }),
                    egui::Key::Period => Some(if modifiers.shift { '>' } else { '.' }),
                    egui::Key::Comma => Some(if modifiers.shift { '<' } else { ',' }),
                    egui::Key::Minus => Some(if modifiers.shift { '_' } else { '-' }),
                    egui::Key::Equals => Some(if modifiers.shift { '+' } else { '=' }),
                    egui::Key::Colon => Some(':'),
                    _ => None,
                }
            }

            // ===== Marks Management (In-Memory, No Persistence) =====

            fn marks_list(&self) -> Vec<Mark> {
                let mut sorted = self.marks.borrow().clone();
                sorted.sort_by(|a, b| b.when_ms.cmp(&a.when_ms)); // Newest first
                sorted
            }

            #[allow(dead_code)]
            fn marks_get_by_label(&self, label: &str) -> Option<Mark> {
                self.marks.borrow().iter().find(|m| m.label == label).cloned()
            }

            fn marks_next_auto_label(&self) -> String {
                const LABELS: &[&str] = &[
                    "1", "2", "3", "4", "5", "6", "7", "8", "9",
                    "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
                    "n", "o", "p", "q", "r", "s", "t", "u", "v", "w", "x", "y", "z",
                ];
                for &label in LABELS {
                    if !self.marks.borrow().iter().any(|m| m.label == label) {
                        return label.to_string();
                    }
                }
                // If all labels taken, reuse oldest
                self.marks.borrow()
                    .iter()
                    .min_by_key(|m| m.when_ms)
                    .map(|m| m.label.clone())
                    .unwrap_or_else(|| "a".to_string())
            }

            fn marks_add_or_replace(&self, label: String, pane: u8, height: Option<u64>, tx_hash: Option<String>) {
                let now = chrono::Utc::now().timestamp_millis();
                let mut marks = self.marks.borrow_mut();

                // Preserve pinned status if updating existing mark
                let pinned = marks.iter()
                    .find(|m| m.label == label)
                    .map(|m| m.pinned)
                    .unwrap_or(false);

                let mark = Mark {
                    label: label.clone(),
                    pane,
                    height,
                    tx_hash,
                    when_ms: now,
                    pinned,
                };

                // Update or add
                if let Some(pos) = marks.iter().position(|m| m.label == label) {
                    marks[pos] = mark;
                } else {
                    marks.push(mark);
                }

                *self.marks_cursor.borrow_mut() = 0;
            }

            fn marks_remove_by_label(&self, label: &str) {
                self.marks.borrow_mut().retain(|m| m.label != label);
                let mut cursor = self.marks_cursor.borrow_mut();
                let len = self.marks.borrow().len();
                if *cursor >= len && *cursor > 0 {
                    *cursor = len - 1;
                }
            }

            fn marks_next(&self) -> Option<Mark> {
                let list = self.marks_list();
                if list.is_empty() {
                    return None;
                }
                let mut cursor = self.marks_cursor.borrow_mut();
                *cursor = (*cursor + 1) % list.len();
                Some(list[*cursor].clone())
            }

            fn marks_prev(&self) -> Option<Mark> {
                let list = self.marks_list();
                if list.is_empty() {
                    return None;
                }
                let mut cursor = self.marks_cursor.borrow_mut();
                *cursor = if *cursor == 0 {
                    list.len() - 1
                } else {
                    *cursor - 1
                };
                Some(list[*cursor].clone())
            }

            fn marks_find_by_context(&self, pane: u8, height: Option<u64>, tx_hash: Option<&str>) -> Option<String> {
                self.marks.borrow().iter().find(|m| {
                    // Match by tx_hash if present (most specific)
                    if let Some(hash) = tx_hash {
                        return m.tx_hash.as_deref() == Some(hash);
                    }
                    // Otherwise match by height + pane if height present
                    if let Some(h) = height {
                        return m.height == Some(h) && m.pane == pane && m.tx_hash.is_none();
                    }
                    // Otherwise match by pane only
                    m.pane == pane && m.height.is_none() && m.tx_hash.is_none()
                }).map(|m| m.label.clone())
            }

            fn marks_toggle_pin(&self, label: &str) {
                if let Some(mark) = self.marks.borrow_mut().iter_mut().find(|m| m.label == label) {
                    mark.pinned = !mark.pinned;
                }
            }
        }

        impl eframe::App for RatacatApp {
            fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
                // Initialize debug system once from URL params and localStorage
                if !self.debug_inited {
                    debug::init_from_url_and_storage_once();
                    self.debug_overlay = debug::overlay();
                    self.debug_inited = true;
                }

                // Apply or re-apply theme when app.theme changes
                let cur_theme = *self.app.borrow().theme();
                if self.last_egui_theme != Some(cur_theme) {
                    nearx::theme::eg::apply(ctx, &cur_theme);
                    self.last_egui_theme = Some(cur_theme);
                    debug::log(cat::THEME, "egui visuals + CSS vars applied");
                }

                // --- Hi-DPI snap (Web only): keep pixels-per-point aligned with devicePixelRatio
                // This avoids fractional resampling that makes the ratatui texture look soft.
                // Gated by ui_flags.dpr_snap for easy disable if needed.
                #[cfg(target_arch = "wasm32")]
                {
                    let flags = self.app.borrow().ui_flags();
                    if flags.dpr_snap {
                        if let Some(win) = web_sys::window() {
                            let dpr = win.device_pixel_ratio() as f32;
                            // Snap to nearest 0.5 step (1.0, 1.5, 2.0, …) to reduce blur on common displays
                            let snapped = (dpr * 2.0).round() / 2.0;
                            if self.last_dpr.map_or(true, |prev| (prev - snapped).abs() > f32::EPSILON) {
                                ctx.set_pixels_per_point(snapped.max(1.0));
                                self.last_dpr = Some(snapped);
                                debug::log(cat::DPR, format!("devicePixelRatio={} -> pixels_per_point={}", dpr, snapped));
                                // egui will relayout with the new scale; ratatui texture stays crisp
                            }
                        }
                    }
                }

                // Check for deep link route in URL hash
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(window) = web_sys::window() {
                        let location = window.location();
                        if let Ok(hash) = location.hash() {
                            // Only process if hash changed
                            if self.last_hash.as_ref() != Some(&hash) {
                                self.last_hash = Some(hash.clone());

                                debug::log(cat::ROUTER, format!("hash changed: {}", hash));

                                // Parse route from hash - handle both direct and Tauri-encoded formats
                                let route_opt = if let Some(rest) = hash.strip_prefix("#/deeplink/") {
                                    // Tauri format: #/deeplink/nearx%3A%2F%2F...
                                    // Decode and parse
                                    if let Ok(decoded) = js_sys::decode_uri_component(rest) {
                                        let decoded_str = decoded.as_string().unwrap_or_default();
                                        debug::log(cat::ROUTER, format!("decoded deep link: {}", decoded_str));
                                        nearx::router::parse(&decoded_str)
                                    } else {
                                        None
                                    }
                                } else if let Some(rest) = hash.strip_prefix('#') {
                                    // Direct format: #/v1/tx/ABC123
                                    nearx::router::parse(rest)
                                } else {
                                    None
                                };

                                if let Some(route) = route_opt {
                                    self.app.borrow_mut().apply_route(&route);
                                    debug::log(cat::ROUTER, "route applied");
                                }
                            }
                        }
                    }
                }

                // Process blockchain events
                if let Ok(mut rx) = self.event_rx.try_borrow_mut() {
                    let mut count = 0;
                    while let Ok(event) = rx.try_recv() {
                        self.app.borrow_mut().on_event(event);
                        count += 1;
                    }
                    if count > 0 {
                        log::debug!("Processed {} events this frame", count);
                    }
                }

                // Handle keyboard input and get Tab press state
                let (tab_pressed, _shift) = self.handle_input(ctx);

                // Consume Tab keys outside the input closure (if flag enabled)
                // This prevents egui from stealing focus after we've handled pane navigation
                if tab_pressed && self.app.borrow().ui_flags().consume_tab {
                    ctx.input_mut(|i| {
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
                        i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
                    });
                }

                // Render the terminal using egui_ratatui
                // Central panel = the app. No inset, fill with theme background.
                let panel_fill = ctx.style().visuals.panel_fill;
                egui::CentralPanel::default()
                    .frame(
                        egui::Frame::NONE
                            .fill(panel_fill)
                            .inner_margin(egui::Margin::same(0))
                            .outer_margin(egui::Margin::same(0))
                    )
                    .show(ctx, |ui| {
                        // Draw ratatui UI
                        let app_ref = self.app.clone();
                        let marks_list = self.marks_list();
                        let _ = self.terminal.draw(|f| {
                            let mut app = app_ref.borrow_mut();
                            ui::draw(f, &mut app, &marks_list[..]); // Pass actual marks as slice
                        });

                        // Render the terminal widget - FORCE it to fill all available space
                        let resp = ui.add_sized(ui.available_size(), self.terminal.backend_mut());

                        // Request focus when clicked (efficient - only when needed)
                        if resp.clicked() {
                            resp.request_focus();
                        }

                        // Mouse mapping (click to focus/select; double-click expands) - gated by flags
                        let flags = self.app.borrow().ui_flags();
                        if flags.mouse_map && resp.hovered() {
                            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                                // Convert pixels to terminal cells
                                let rect = resp.rect;
                                let size_cells = self.terminal.size().unwrap_or(ratatui::prelude::Size {
                                    width: 80, height: 24
                                });

                                let cell_w = rect.width() / size_cells.width as f32;
                                let cell_h = rect.height() / size_cells.height as f32;
                                let col = ((pos.x - rect.min.x) / cell_w).floor() as i32;
                                let row = ((pos.y - rect.min.y) / cell_h).floor() as i32;

                                // Split like the TUI layout: top half → [Blocks|Txs], bottom → Details
                                // This assumes the standard 70/30 layout with dynamic chrome
                                let mid_row = (size_cells.height as i32) / 2;
                                let mid_col = (size_cells.width as i32) / 2;

                                // Handle any click event
                                if ui.input(|i| i.pointer.any_click()) {
                                    debug::log(cat::MOUSE, format!("click px=({:.1},{:.1}) cell=({},{}), grid={}x{}",
                                        pos.x, pos.y, col, row, size_cells.width, size_cells.height));

                                    if row >= mid_row {
                                        // Details pane (bottom half)
                                        self.app.borrow_mut().set_pane_direct(2);
                                        debug::log(cat::MOUSE, "focus=Details");

                                        // Double-click toggles fullscreen (gated by flag)
                                        if flags.dblclick_details
                                            && ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                                            self.app.borrow_mut().toggle_details_fullscreen();
                                            debug::log(cat::MOUSE, "double-click details → toggle fullscreen");
                                        }
                                    } else {
                                        // Top half: split columns
                                        if col < mid_col {
                                            // Blocks pane (left)
                                            self.app.borrow_mut().set_pane_direct(0);

                                            // Conservative row mapping: skip ~2 rows for title/border
                                            // Adjust this offset if your header height differs
                                            let idx = (row - 2).max(0) as usize;
                                            self.app.borrow_mut().select_block_row(idx);
                                            debug::log(cat::MOUSE, format!("focus=Blocks select_row={}", idx));
                                        } else {
                                            // Transactions pane (right)
                                            self.app.borrow_mut().set_pane_direct(1);

                                            let idx = (row - 2).max(0) as usize;
                                            self.app.borrow_mut().select_tx_row(idx);
                                            debug::log(cat::MOUSE, format!("focus=Tx select_row={}", idx));
                                        }
                                    }
                                }
                            }

                            // Wheel scrolling: map pixels -> lines, apply to focused pane
                            // Normalize: many browsers report ≈120 px per "notch". Use 120 divisor, clamp to [-10,10].
                            let sd = ui.input(|i| i.raw_scroll_delta);
                            if sd.y.abs() > 0.0 {
                                let lines = ((sd.y / -120.0) * 3.0).round().clamp(-10.0, 10.0) as i32; // 3 lines per notch, bounded
                                if lines != 0 {
                                    debug::log(cat::MOUSE, format!("wheel dy={:.1} -> lines={}", sd.y, lines));
                                    self.app.borrow_mut().scroll_lines(lines);
                                }
                            }
                        }
                    });

                // Debug overlay window (Ctrl+Shift+D to toggle)
                if self.debug_overlay && debug::is(cat::UI) {
                    let mut open = true;
                    egui::Window::new("NEARx Debug")
                        .open(&mut open)
                        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -8.0))
                        .resizable(true)
                        .show(ctx, |ui| {
                            let cells = self.terminal.size().ok();
                            let avail = ui.available_size_before_wrap();
                            let dpr = self.last_dpr.unwrap_or(1.0);
                            let a = self.app.borrow();

                            ui.monospace(format!("pane={}  block_height={:?}  sel_tx={}",
                                a.pane(), a.selected_block_height(), a.sel_tx()));
                            ui.monospace(format!("filter len={}",
                                a.filter_query().len()));
                            ui.monospace(format!("dpr={:.2}  avail={:.0}x{:.0}  cells={:?}",
                                dpr, avail.x, avail.y, cells));
                            ui.separator();

                            // Category toggles
                            let mut m = debug::mask();
                            for (bit, name) in &[
                                (cat::UI, "ui"), (cat::INPUT, "input"), (cat::MOUSE, "mouse"),
                                (cat::COPY, "copy"), (cat::ROUTER, "router"), (cat::RENDER, "render"),
                                (cat::THEME, "theme"), (cat::DPR, "dpr")
                            ] {
                                let on = (m & *bit) != 0;
                                let mut toggled = on;
                                ui.checkbox(&mut toggled, *name);
                                if toggled != on {
                                    if toggled {
                                        m |= *bit;
                                    } else {
                                        m &= !*bit;
                                    }
                                }
                            }
                            debug::set(m);

                            ui.horizontal(|ui| {
                                if ui.button("All").clicked() {
                                    debug::set(cat::ALL);
                                }
                                if ui.button("None").clicked() {
                                    debug::set(0);
                                }
                            });
                        });

                    if !open {
                        self.debug_overlay = false;
                        debug::set_overlay(false);
                    }
                }

                // Request continuous repaint (60 FPS)
                ctx.request_repaint();
            }
        }

        /// Load configuration from localStorage + URL parameters
        fn load_web_config() -> Config {
            let window = web_sys::window().expect("no window");

            // Parse URL parameters
            let url = window.location().href().unwrap_or_default();
            let url_obj = web_sys::Url::new(&url).ok();
            let search_params = url_obj.as_ref().map(|u| u.search_params());

            // Get localStorage
            let local_storage = window.local_storage().ok().flatten();

            // RPC URL priority: ?rpc param > localStorage > default
            let rpc_url = search_params
                .as_ref()
                .and_then(|p| p.get("rpc"))
                .or_else(|| local_storage.as_ref().and_then(|ls| ls.get_item("RPC_URL").ok().flatten()))
                .unwrap_or_else(|| "https://rpc.mainnet.fastnear.com/".to_string());

            // Auth token priority: ?token param > localStorage > none
            let (auth_token, token_source) = if let Some(token) = search_params.as_ref().and_then(|p| p.get("token")) {
                (Some(token), "URL param")
            } else if let Some(token) = local_storage.as_ref().and_then(|ls| ls.get_item("RPC_BEARER").ok().flatten()) {
                (Some(token), "localStorage")
            } else {
                (None, "none")
            };

            let filter = search_params
                .as_ref()
                .and_then(|p| p.get("filter"))
                .unwrap_or_else(|| "intents.near".to_string());

            log::info!("🚀 Ratacat egui-web starting");
            log::info!("RPC: {}, Filter: {}, Token: {} (from {})",
                rpc_url, filter,
                auth_token.as_ref().map(|t| format!("{}...", &t.chars().take(8).collect::<String>())).unwrap_or_else(|| "none".to_string()),
                token_source
            );

            Config {
                source: Source::Rpc,
                ws_url: String::new(),
                ws_fetch_blocks: false,
                near_node_url: rpc_url,
                near_node_url_explicit: true,
                fastnear_auth_token: auth_token,
                poll_interval_ms: 1000,
                poll_max_catchup: 5,
                poll_chunk_concurrency: 4,
                rpc_timeout_ms: 8000,
                rpc_retries: 2,
                archival_rpc_url: None,
                render_fps: 30,
                render_fps_choices: vec![20, 30, 60],
                keep_blocks: 100,
                default_filter: filter,
            }
        }

        // ---------------------------
        // Main Entry Point
        // ---------------------------

        /// Initialize and start app automatically (runs when WASM loads)
        #[wasm_bindgen(start)]
        pub fn setup() {
            console_error_panic_hook::set_once();
            wasm_logger::init(wasm_logger::Config::new(log::Level::Info));
            log::info!("🦀 Ratacat WASM module loaded");

            // Spawn async initialization - will wait for DOM ready internally
            wasm_bindgen_futures::spawn_local(async {
                if let Err(e) = start_app_when_ready().await {
                    log::error!("Failed to start app: {:?}", e);
                }
            });
        }

        /// Wait for DOM ready, then start the app
        async fn start_app_when_ready() -> Result<(), JsValue> {
            use gloo_timers::future::sleep;
            use std::time::Duration;

            // Wait for DOM to be ready (poll for canvas element)
            let window = web_sys::window().ok_or("no window")?;
            let document = window.document().ok_or("no document")?;

            log::info!("⏳ Waiting for DOM ready...");
            for i in 0..50 {  // 5 seconds max
                if let Some(canvas_elem) = document.get_element_by_id("canvas") {
                    if let Ok(_) = canvas_elem.dyn_into::<web_sys::HtmlCanvasElement>() {
                        log::info!("✅ DOM ready, starting app...");
                        return start_app().await;
                    }
                }
                sleep(Duration::from_millis(100)).await;
                if i == 49 {
                    return Err(JsValue::from_str("Timeout waiting for canvas element"));
                }
            }
            unreachable!()
        }

        /// Main entry point - initializes app after DOM is ready
        async fn start_app() -> Result<(), JsValue> {
            log::info!("🚀 Starting Ratacat egui-web...");

            // Load configuration
            let config = Rc::new(load_web_config());

            // Setup event channel for blockchain data
            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

            // Start RPC polling task
            let config_clone = config.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(e) = source_rpc::run_rpc(&config_clone, event_tx).await {
                    log::error!("RPC polling failed: {}", e);
                }
            });

            // Create egui app
            let app = RatacatApp::new((*config).clone(), event_rx);

            // Get canvas element from DOM (verified ready by start_app_when_ready)
            let window = web_sys::window().ok_or("no window")?;
            let document = window.document().ok_or("no document")?;
            let canvas = document
                .get_element_by_id("canvas")
                .ok_or("no canvas element")?
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .map_err(|_| "canvas is not HtmlCanvasElement")?;

            log::info!("📦 Canvas element found, starting eframe...");

            // Start eframe web runner
            let web_options = eframe::WebOptions::default();

            eframe::WebRunner::new()
                .start(
                    canvas,
                    web_options,
                    Box::new(|_cc| Ok(Box::new(app))),
                )
                .await
                .map_err(|e| JsValue::from_str(&format!("eframe start failed: {:?}", e)))?;

            log::info!("✅ Ratacat egui-web running!");

            // Hide loading screen
            if let Ok(js_code) = js_sys::eval("window.hideLoading && window.hideLoading()") {
                log::debug!("hideLoading result: {:?}", js_code);
            }

            Ok(())
        }
    }
}
