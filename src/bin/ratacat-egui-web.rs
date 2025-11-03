//! Ratacat Web Binary - egui + egui_ratatui
//!
//! Production-grade web harness using egui for WebGL canvas rendering
//! and egui_ratatui to bridge ratatui widgets into egui.
//!
//! This uses immediate-mode Canvas rendering (WebGL) instead of DOM manipulation
//! for superior performance and reliability.

#![cfg_attr(target_arch = "wasm32", no_main)]

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use std::{cell::RefCell, rc::Rc};
        use egui_ratatui::RataguiBackend;
        use soft_ratatui::{EmbeddedGraphics, SoftBackend};
        use ratatui::Terminal;
        use wasm_bindgen::prelude::*;
        use eframe::wasm_bindgen;

        use ratacat::{
            App, InputMode,
            config::{Config, Source},
            types::AppEvent,
            source_rpc,
            theme::Theme,
            platform,
        };

        // ---------------------------
        // Egui Application
        // ---------------------------

        struct RatacatApp {
            terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
            app: Rc<RefCell<App>>,
            event_rx: Rc<RefCell<tokio::sync::mpsc::UnboundedReceiver<AppEvent>>>,
        }

        impl RatacatApp {
            fn new(config: Config, event_rx: tokio::sync::mpsc::UnboundedReceiver<AppEvent>) -> Self {
                log::info!("🎯 Creating App with filter: '{}'", config.default_filter);
                let app = App::new(
                    config.render_fps,
                    config.render_fps_choices.clone(),
                    config.keep_blocks,
                    config.default_filter.clone(),
                    None, // No archival fetch for web
                    config.theme.colors(),
                );
                log::info!("✅ App created successfully");

                // Create egui_ratatui backend with soft renderer
                // Use pixel-perfect bitmap font atlases (8x13) for crisp rendering
                // No TTF rasterization = no antialiasing blur
                use soft_ratatui::embedded_graphics_unicodefonts::{
                    mono_8x13_atlas,
                    mono_8x13_bold_atlas,
                    mono_8x13_italic_atlas,
                };

                let font_regular = mono_8x13_atlas();
                let font_bold = Some(mono_8x13_bold_atlas());
                let font_italic = Some(mono_8x13_italic_atlas());

                let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
                    85,            // width in columns (8x13 bitmap font)
                    30,            // height in rows (maintains aspect ratio)
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
                                (egui::Key::Tab, false) if modifiers.shift => app.prev_pane(),
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
                                    let payload = ratacat::copy_api::get_copy_content(&app);
                                    let _ok = ratacat::platform::copy_to_clipboard(&payload);
                                    let msg = match app.pane() {
                                        0 => "Copied block JSON",
                                        1 => "Copied transaction JSON",
                                        2 => "Copied details JSON",
                                        _ => "Copied",
                                    };
                                    app.show_toast(msg.to_string());
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
                // ═══════════════════════════════════════════════════════════════
                // TIME-BUDGETED EVENT DRAIN: Process events with 3ms budget
                // ═══════════════════════════════════════════════════════════════
                // This prevents UI stutter during initial catch-up by limiting
                // how long we spend processing events per frame. If we hit the
                // budget, we request another repaint to continue processing.
                if let Ok(mut rx) = self.event_rx.try_borrow_mut() {
                    let start = platform::Instant::now();
                    let budget_ms = 3; // 3ms budget per frame
                    let mut count = 0;
                    let mut hit_budget = false;

                    loop {
                        match rx.try_recv() {
                            Ok(event) => {
                                log::debug!("📥 Received event in update(): {:?}", event);
                                self.app.borrow_mut().on_event(event);
                                count += 1;

                                // Check if we've exceeded our time budget
                                if start.elapsed().as_millis() >= budget_ms {
                                    hit_budget = true;
                                    log::debug!("⏱️  Hit {}ms budget after {} events - will continue next frame", budget_ms, count);
                                    break;
                                }
                            }
                            Err(_) => break, // Channel empty or closed
                        }
                    }

                    if count > 0 {
                        log::info!("✅ Processed {} events this frame in {:.1}ms{}",
                            count,
                            start.elapsed().as_secs_f64() * 1000.0,
                            if hit_budget { " (budget hit - more pending)" } else { "" }
                        );

                        // Log current app state
                        let app = self.app.borrow();
                        let (_, _, total_blocks) = app.filtered_blocks();
                        log::info!("📊 App state: {} blocks in buffer", total_blocks);

                        // Update HTML status bar
                        if let Some(window) = web_sys::window() {
                            let _ = js_sys::Reflect::get(&window, &"updateStatus".into())
                                .and_then(|f| {
                                    let func = js_sys::Function::from(f);
                                    func.call2(&window, &format!("Connected | {} blocks", total_blocks).into(), &"connected".into())
                                });
                        }
                    }
                } else {
                    log::error!("❌ Failed to borrow event_rx!");
                }

                // Handle keyboard input
                self.handle_input(ctx);

                // Render the terminal using egui_ratatui
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                    .show(ctx, |ui| {
                        // Draw ratatui UI
                        let app_ref = self.app.clone();
                        self.terminal.draw(|f| {
                            let mut app = app_ref.borrow_mut();
                            ratacat::ui::draw(f, &mut app, &[]); // Empty marks list for web
                        }).ok();

                        // Render the terminal as an egui widget
                        ui.add(self.terminal.backend_mut());
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

            // Auth token priority: ?token param > localStorage > compile-time env > none
            let (auth_token, token_source) = if let Some(token) = search_params.as_ref().and_then(|p| p.get("token")) {
                (Some(token), "URL param")
            } else if let Some(token) = local_storage.as_ref().and_then(|ls| ls.get_item("RPC_BEARER").ok().flatten()) {
                (Some(token), "localStorage")
            } else if let Some(token) = option_env!("FASTNEAR_AUTH_TOKEN") {
                (Some(token.to_string()), "compile-time env")
            } else {
                (None, "none")
            };

            let filter = search_params
                .as_ref()
                .and_then(|p| p.get("filter"))
                .or_else(|| local_storage.as_ref().and_then(|ls| ls.get_item("DEFAULT_FILTER").ok().flatten()))
                .unwrap_or_else(|| "".to_string()); // Empty = show all blocks

            // Theme priority: ?theme param > localStorage > default (Nord)
            let theme = search_params
                .as_ref()
                .and_then(|p| p.get("theme"))
                .or_else(|| local_storage.as_ref().and_then(|ls| ls.get_item("THEME").ok().flatten()))
                .and_then(|s| Theme::from_str(&s).ok())
                .unwrap_or_default();

            log::info!("🚀 Ratacat egui-web starting");
            log::info!("RPC: {}", rpc_url);
            log::info!("Filter: {}", if filter.is_empty() { "(empty - showing all blocks)".to_string() } else { format!("'{}'", filter) });
            log::info!("Theme: {:?}", theme);
            log::info!("Token: {} (from {})",
                auth_token.as_ref().map(|t| format!("{}...", &t.chars().take(8).collect::<String>())).unwrap_or_else(|| "❌ NONE".to_string()),
                token_source
            );

            // Update HTML status bar
            if let Some(window) = web_sys::window() {
                let _ = js_sys::Reflect::get(&window, &"updateStatus".into())
                    .and_then(|f| {
                        let func = js_sys::Function::from(f);
                        func.call2(&window, &format!("RPC: {} | Filter: {}", rpc_url, filter).into(), &"loading".into())
                    });
            }

            Config {
                source: Source::Rpc,
                ws_url: String::new(),
                ws_fetch_blocks: false,
                near_node_url: rpc_url,
                near_node_url_explicit: true,
                fastnear_auth_token: auth_token,
                poll_interval_ms: 400,  // 400ms polling for NEAR's 600ms block time
                poll_max_catchup: 5,
                poll_chunk_concurrency: 4,
                rpc_timeout_ms: 8000,
                rpc_retries: 2,
                archival_rpc_url: None,
                render_fps: 30,
                render_fps_choices: vec![20, 30, 60],
                keep_blocks: 100,
                default_filter: filter,
                theme,
            }
        }

        // ---------------------------
        // Main Entry Point
        // ---------------------------

        /// WASM entry point
        #[wasm_bindgen(start)]
        pub async fn main() {
            // Setup panic hook and logging
            platform::install_panic_hook();
            platform::init_logging(log::Level::Debug);

            log::info!("╔═══════════════════════════════════════════════════════════╗");
            log::info!("║       🦀 Ratacat egui-web v0.4.0 - Starting Up        ║");
            log::info!("╚═══════════════════════════════════════════════════════════╝");

            // Load configuration
            let config = Rc::new(load_web_config());

            log::info!("📋 Startup Diagnostics:");
            log::info!("  • WASM binary: ratacat-egui-web");
            log::info!("  • UI framework: egui + egui_ratatui + soft_ratatui");
            log::info!("  • Font backend: EmbeddedGraphics (8x13 monospace)");
            log::info!("  • Async runtime: tokio (wasm-compatible subset)");
            log::info!("  • Log level: Debug (all RPC activity visible)");

            // Setup event channel for blockchain data
            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();

            // ═══════════════════════════════════════════════════════════════
            // 🚀 START RPC POLLING IMMEDIATELY - BEFORE UI INITIALIZATION
            // ═══════════════════════════════════════════════════════════════
            // This is CRITICAL for performance: spawn the RPC task NOW so it
            // starts running during eframe's .await (which yields control).
            // Result: RPC fills cache while UI loads = instant blocks!
            let config_for_rpc = config.clone();
            log::info!("╔═══════════════════════════════════════════════════════════╗");
            log::info!("║  🚀 SPAWNING RPC TASK **BEFORE** UI INIT (MAX SPEED)   ║");
            log::info!("╚═══════════════════════════════════════════════════════════╝");
            log::info!("  • RPC URL: {}", config_for_rpc.near_node_url);
            log::info!("  • Poll Interval: {}ms", config_for_rpc.poll_interval_ms);
            log::info!("  • Has Auth Token: {}", config_for_rpc.fastnear_auth_token.is_some());
            log::info!("  • Filter: '{}'", config_for_rpc.default_filter);
            log::info!("  • Strategy: RPC runs DURING UI load (parallel!)");

            wasm_bindgen_futures::spawn_local(async move {
                log::info!("╔═══════════════════════════════════════════════════════════╗");
                log::info!("║  ✅ RPC TASK SPAWNED - WILL RUN DURING UI INIT         ║");
                log::info!("╚═══════════════════════════════════════════════════════════╝");

                log::info!("📞 Calling source_rpc::run_rpc() NOW...");
                match source_rpc::run_rpc(&config_for_rpc, event_tx).await {
                    Ok(_) => {
                        log::warn!("⚠️  RPC loop exited normally (unexpected!)");
                    }
                    Err(e) => {
                        log::error!("╔═══════════════════════════════════════════════════════════╗");
                        log::error!("║  ❌ RPC POLLING FAILED                                   ║");
                        log::error!("╚═══════════════════════════════════════════════════════════╝");
                        log::error!("  Error: {}", e);
                        log::error!("  Error (debug): {:?}", e);
                        web_sys::console::error_1(&format!("RPC polling failed: {}", e).into());
                    }
                }
                log::error!("🛑 RPC task exited - this should never happen!");
            });

            // Create egui app (UI will receive blocks from RPC via event channel)
            let app = RatacatApp::new((*config).clone(), event_rx);

            // Get canvas element from DOM
            let window = web_sys::window().expect("no window");
            let document = window.document().expect("no document");
            let canvas = document
                .get_element_by_id("canvas")
                .expect("no canvas element")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("canvas is not HtmlCanvasElement");

            // Start eframe web runner with scaled-up rendering for better readability
            let web_options = eframe::WebOptions::default();

            eframe::WebRunner::new()
                .start(
                    canvas,
                    web_options,
                    Box::new(|_cc| {
                        // Let egui auto-detect DPI using window.devicePixelRatio for crisp fonts
                        // Non-integral scaling (like 1.5) causes blurry fonts with bilinear sampling
                        // Auto-detection provides native resolution rendering
                        Ok(Box::new(app))
                    }),
                )
                .await
                .expect("failed to start eframe");

            log::info!("✅ Ratacat egui-web running!");
            log::info!("   (RPC task already running in parallel - check logs above)");
        }
    }
}
