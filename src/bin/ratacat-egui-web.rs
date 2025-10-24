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
        use soft_ratatui::SoftBackend;
        use ratatui::Terminal;
        use wasm_bindgen::prelude::*;
        use eframe::wasm_bindgen;

        use ratacat::{
            App, InputMode,
            config::{Config, Source},
            types::AppEvent,
            source_rpc,
        };

        // ---------------------------
        // Egui Application
        // ---------------------------

        struct RatacatApp {
            backend: RataguiBackend<SoftBackend>,
            app: Rc<RefCell<App>>,
            event_rx: Rc<RefCell<tokio::sync::mpsc::UnboundedReceiver<AppEvent>>>,
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

                // Create egui_ratatui backend with soft renderer
                let soft_backend = SoftBackend::new(80, 24); // Initial size
                let backend = RataguiBackend::new("ratacat", soft_backend);

                Self {
                    backend,
                    app: Rc::new(RefCell::new(app)),
                    event_rx: Rc::new(RefCell::new(event_rx)),
                }
            }

            fn handle_input(&mut self, ctx: &egui::Context) {
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

                            // Normal mode
                            match (key, modifiers.ctrl || modifiers.command) {
                                (egui::Key::Q, false) | (egui::Key::C, true) => {
                                    log::info!("Quit requested (close tab)");
                                    app.show_toast("Press Ctrl+W to close tab".to_string());
                                }
                                (egui::Key::Tab, false) => app.next_pane(),
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
                                (egui::Key::D, true) => app.toggle_debug_panel(),
                                (egui::Key::F, true) => app.start_search(),
                                (egui::Key::U, true) => app.toggle_owned_filter(),
                                (egui::Key::C, false) => {
                                    let content = app.get_copy_content();
                                    // Copy to clipboard using web-sys
                                    if let Some(window) = web_sys::window() {
                                        let clipboard = window.navigator().clipboard();
                                        let _ = clipboard.write_text(&content);
                                        let msg = match app.pane() {
                                            0 => "Copied block info",
                                            1 => "Copied tx hash",
                                            2 => "Copied details",
                                            _ => "Copied",
                                        };
                                        app.show_toast(msg.to_string());
                                    } else {
                                        app.show_toast("Copy failed".to_string());
                                    }
                                }
                                (egui::Key::Slash, false) => app.start_filter(),
                                (egui::Key::F, false) if !modifiers.ctrl => app.start_filter(),
                                (egui::Key::Escape, false) => app.clear_filter(),
                                _ => {}
                            }
                        }
                    }
                });
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
        }

        impl eframe::App for RatacatApp {
            fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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

                // Handle keyboard input
                self.handle_input(ctx);

                // Render the terminal using egui_ratatui
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                    .show(ctx, |ui| {
                        // Create terminal with our backend
                        let mut terminal = Terminal::new(&mut self.backend).expect("terminal");

                        // Draw ratatui UI
                        let app_ref = self.app.clone();
                        terminal.draw(|f| {
                            let mut app = app_ref.borrow_mut();
                            ratacat::ui::draw(f, &mut app, &[]); // Empty marks list for web
                        }).ok();

                        // Render the terminal in egui
                        self.backend.show(ui);
                    });

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

        /// WASM entry point
        #[wasm_bindgen(start)]
        pub async fn main() {
            // Setup panic hook and logging
            console_error_panic_hook::set_once();
            wasm_logger::init(wasm_logger::Config::new(log::Level::Info));

            log::info!("🦀 Ratacat egui-web initializing...");

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

            // Get canvas element from DOM
            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");
            let canvas = document
                .get_element_by_id("canvas")
                .expect("no canvas element")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("canvas is not HtmlCanvasElement");

            // Start eframe web runner
            let web_options = eframe::WebOptions::default();

            eframe::WebRunner::new()
                .start(
                    canvas,
                    web_options,
                    Box::new(|_cc| Ok(Box::new(app))),
                )
                .await
                .expect("failed to start eframe");

            log::info!("✅ Ratacat egui-web running!");
        }
    }
}
