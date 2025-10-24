//! Ratacat Web Binary - Browser-based TUI using Ratzilla
//!
//! Production-grade web harness with RAF-driven event loop, proper keyboard
//! handling, focus management, and smooth rendering.
//!
//! Uses Ratzilla (https://github.com/orhun/ratzilla) for rendering ratatui
//! widgets directly in the browser via DOM backend.

#![cfg_attr(target_arch = "wasm32", no_main)]

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use std::{cell::RefCell, rc::Rc};
        use ratzilla::DomBackend;
        use ratatui::Terminal;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::{closure::Closure, JsCast};
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{window, Document, HtmlElement, KeyboardEvent, WheelEvent, ResizeObserver};
        use gloo_render::request_animation_frame;
        use gloo_timers::callback::Timeout;

        use ratacat::{
            App, InputMode,
            config::{Config, Source},
            types::AppEvent,
            source_rpc,
        };

        // ---------------------------
        // Global State (thread-local with OnceCell)
        // ---------------------------

        use once_cell::unsync::OnceCell;

        thread_local! {
            static APP: OnceCell<Rc<RefCell<App>>> = const { OnceCell::new() };
            static TERM: OnceCell<Rc<RefCell<Terminal<DomBackend>>>> = const { OnceCell::new() };
        }

        // Initialize globals with actual values (called once from main)
        fn init_globals(app: App, term: Terminal<DomBackend>) {
            APP.with(|cell| {
                cell.set(Rc::new(RefCell::new(app)))
                    .unwrap_or_else(|_| panic!("APP already initialized"));
            });
            TERM.with(|cell| {
                cell.set(Rc::new(RefCell::new(term)))
                    .unwrap_or_else(|_| panic!("TERM already initialized"));
            });
        }

        // ---------------------------
        // Key Mapping (Crossterm-style)
        // ---------------------------

        #[derive(Clone, Copy, Debug)]
        enum Key {
            Char(char),
            Enter,
            Backspace,
            Esc,
            Tab,
            Left,
            Right,
            Up,
            Down,
            Home,
            End,
            PageUp,
            PageDown,
            Unknown,
        }

        /// Translate web KeyboardEvent to our Key enum
        fn translate_key(ev: &KeyboardEvent) -> Option<Key> {
            let k = ev.key();
            let out = match k.as_str() {
                "Enter" => Key::Enter,
                "Backspace" => Key::Backspace,
                "Escape" => Key::Esc,
                "Tab" => Key::Tab,
                "ArrowLeft" => Key::Left,
                "ArrowRight" => Key::Right,
                "ArrowUp" => Key::Up,
                "ArrowDown" => Key::Down,
                "Home" => Key::Home,
                "End" => Key::End,
                "PageUp" => Key::PageUp,
                "PageDown" => Key::PageDown,
                _ => {
                    // Single character keys
                    let s = k.as_str();
                    if s.len() == 1 {
                        if let Some(c) = s.chars().next() {
                            // Allow space and non-control chars
                            if c == ' ' || !c.is_control() {
                                return Some(Key::Char(c));
                            }
                        }
                    }
                    Key::Unknown
                }
            };
            Some(out)
        }

        /// Check if this is a Ctrl+key combo
        fn is_ctrl(ev: &KeyboardEvent) -> bool {
            ev.ctrl_key() || ev.meta_key()
        }

        // ---------------------------
        // Event Handlers
        // ---------------------------

        /// Handle keyboard input (same logic as native crossterm version)
        fn handle_key(key: Key, ctrl: bool) {
            APP.with(|cell| {
                let app_rc = cell.get().expect("APP not initialized");
                let mut app = app_rc.borrow_mut();

                // Filter input mode
                if app.input_mode() == InputMode::Filter {
                    match key {
                        Key::Backspace => app.filter_backspace(),
                        Key::Enter => app.apply_filter(),
                        Key::Esc => app.clear_filter(),
                        Key::Char(c) => app.filter_add_char(c),
                        _ => {}
                    }
                    return;
                }

                // Search input mode
                if app.input_mode() == InputMode::Search {
                    match key {
                        Key::Backspace => app.search_backspace(),
                        Key::Enter => app.close_search(),
                        Key::Up => app.search_up(),
                        Key::Down => app.search_down(),
                        Key::Esc => app.close_search(),
                        Key::Char(c) => app.search_add_char(c),
                        _ => {}
                    }
                    return;
                }

                // Normal mode
                match (key, ctrl) {
                    // Quit (Ctrl+C or 'q')
                    (Key::Char('c'), true) | (Key::Char('q'), false) => {
                        log::info!("Quit requested (noop in web - close tab)");
                        app.show_toast("Press Ctrl+W to close tab".to_string());
                    }

                    // Pane navigation
                    (Key::Tab, false) => app.next_pane(),

                    // List navigation
                    (Key::Up, false) => app.up(),
                    (Key::Down, false) => app.down(),
                    (Key::Left, false) => app.left(),
                    (Key::Right, false) => app.right(),
                    (Key::PageUp, false) => app.page_up(20),
                    (Key::PageDown, false) => app.page_down(20),
                    (Key::Home, false) => {
                        if app.pane() == 0 {
                            app.return_to_auto_follow();
                        } else {
                            app.home();
                        }
                    }
                    (Key::End, false) => app.end(),
                    (Key::Enter, false) => app.select_tx(),

                    // Commands with Ctrl
                    (Key::Char('o'), true) => app.cycle_fps(),
                    (Key::Char('d'), true) => app.toggle_debug_panel(),
                    (Key::Char('f'), true) => app.start_search(),
                    (Key::Char('u'), true) => app.toggle_owned_filter(),

                    // Copy to clipboard
                    (Key::Char('c'), false) => {
                        let content = app.get_copy_content();
                        if ratacat::platform::copy_to_clipboard(&content) {
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

                    // Filter mode
                    (Key::Char('/'), false) | (Key::Char('f'), false) => {
                        app.start_filter();
                    }

                    // Clear/Escape
                    (Key::Esc, false) => app.clear_filter(),

                    _ => {}
                }
            });
        }

        /// Handle mouse wheel scrolling (convert pixels to terminal lines)
        fn handle_wheel(dy: f64) {
            // Translate pixel delta to "terminal lines"
            // Reasonable mapping: 100px ≈ 3 lines
            let lines = ((dy / 100.0).round() as i32).clamp(-5, 5);
            if lines != 0 {
                APP.with(|cell| {
                    let app_rc = cell.get().expect("APP not initialized");
                    let mut app = app_rc.borrow_mut();
                    // Simulate up/down key presses
                    for _ in 0..lines.abs() {
                        if lines < 0 {
                            app.up();
                        } else {
                            app.down();
                        }
                    }
                });
            }
        }

        /// Draw the UI once (terminal auto-computes size from DOM)
        /// Protected against re-entrant calls with atomic guard
        fn draw_once() {
            use std::sync::atomic::{AtomicBool, Ordering};
            static DRAWING: AtomicBool = AtomicBool::new(false);

            // Guard: skip if already drawing (prevents BorrowMutError)
            if DRAWING.swap(true, Ordering::Acquire) {
                log::trace!("⏭️  Skipping draw - already in progress");
                return;
            }

            TERM.with(|term_cell| {
                APP.with(|app_cell| {
                    let term_rc = term_cell.get().expect("TERM not initialized");
                    let app_rc = app_cell.get().expect("APP not initialized");
                    let _ = term_rc.borrow_mut().draw(|f| {
                        let mut app = app_rc.borrow_mut();
                        ratacat::ui::draw(f, &mut app, &[]); // Empty marks list for web
                    });
                });
            });

            // Release guard
            DRAWING.store(false, Ordering::Release);
        }

        /// Wait for web fonts to load (prevents 0x0 cell sizing)
        async fn fonts_ready(doc: &Document) {
            let fonts = doc.fonts();
            if let Ok(promise) = fonts.ready() {
                let _ = JsFuture::from(promise).await;
                log::debug!("✅ Fonts ready");
            } else {
                log::warn!("Failed to get fonts.ready promise");
            }
        }

        /// Force early and follow-up redraws to avoid "black until resize"
        /// This handles timing issues with DOM layout and font loading
        fn boot_draws() {
            // Immediate draw (before fonts fully loaded)
            draw_once();
            log::debug!("🎨 Boot draw #1 (immediate)");

            // Follow-up draws after font/layout stabilize
            Timeout::new(32, || {
                draw_once();
                log::debug!("🎨 Boot draw #2 (32ms - fonts stabilizing)");
            }).forget();

            Timeout::new(128, || {
                draw_once();
                log::debug!("🎨 Boot draw #3 (128ms - layout stable)");
            }).forget();
        }

        /// Install ResizeObserver to redraw on container size changes
        fn install_resize_observer(root: &HtmlElement) {
            let cb = Closure::<dyn FnMut(js_sys::Array, ResizeObserver)>::wrap(Box::new(move |_, _| {
                log::debug!("📐 Resize detected, redrawing");
                draw_once();
            }));
            let ro = ResizeObserver::new(cb.as_ref().unchecked_ref()).expect("ResizeObserver");
            ro.observe(root);
            cb.forget();
            // Keep ResizeObserver alive by attaching it to the element via JS expando
            js_sys::Reflect::set(root, &JsValue::from_str("__ro"), ro.as_ref()).ok();
        }

        /// Prevent default browser behavior for terminal-centric keys
        fn prevent_default_for_terminal_keys(ev: &KeyboardEvent) {
            match ev.key().as_str() {
                "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" |
                "PageUp" | "PageDown" | "Home" | "End" |
                "Tab" | " " | "Backspace" => ev.prevent_default(),
                _ => {
                    // Also prevent Ctrl+F (browser find)
                    if (ev.ctrl_key() || ev.meta_key()) && ev.key() == "f" {
                        ev.prevent_default();
                    }
                }
            }
        }

        // ---------------------------
        // Event Loop & Initialization
        // ---------------------------

        /// Install DOM event listeners for keyboard, mouse, resize, focus
        fn install_event_listeners(doc: &Document, root: &HtmlElement) {
            // Focus management: click to focus
            {
                let root_click = {
                    let root = root.clone();
                    Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::MouseEvent| {
                        let _ = root.focus();
                    }))
                };
                root.add_event_listener_with_callback("mousedown", root_click.as_ref().unchecked_ref())
                    .expect("add mousedown listener");
                root_click.forget();
            }

            // Blur (unfocus)
            {
                let blur = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::FocusEvent| {
                    // Could update app focus state if needed
                    draw_once();
                }));
                root.add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())
                    .expect("add blur listener");
                blur.forget();
            }

            // Focus
            {
                let focus = Closure::<dyn FnMut(_)>::wrap(Box::new(move |_e: web_sys::FocusEvent| {
                    // Could update app focus state if needed
                    draw_once();
                }));
                root.add_event_listener_with_callback("focus", focus.as_ref().unchecked_ref())
                    .expect("add focus listener");
                focus.forget();
            }

            // Keyboard (keydown)
            {
                let keydown = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: KeyboardEvent| {
                    prevent_default_for_terminal_keys(&e);
                    if let Some(k) = translate_key(&e) {
                        handle_key(k, is_ctrl(&e));
                        draw_once();
                    }
                }));
                doc.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())
                    .expect("add keydown listener");
                keydown.forget();
            }

            // Wheel (scroll)
            {
                let wheel = Closure::<dyn FnMut(_)>::wrap(Box::new(move |e: WheelEvent| {
                    e.prevent_default();
                    handle_wheel(e.delta_y());
                    draw_once();
                }));
                doc.add_event_listener_with_callback("wheel", wheel.as_ref().unchecked_ref())
                    .expect("add wheel listener");
                wheel.forget();
            }

            // Resize
            {
                let resize = Closure::<dyn FnMut()>::wrap(Box::new(move || {
                    draw_once();
                }));
                window().unwrap()
                    .add_event_listener_with_callback("resize", resize.as_ref().unchecked_ref())
                    .expect("add resize listener");
                resize.forget();
            }
        }

        /// Start RAF (requestAnimationFrame) loop for smooth rendering
        fn start_raf_loop(event_rx: Rc<RefCell<tokio::sync::mpsc::UnboundedReceiver<AppEvent>>>) {
            // Track first render for logging
            use std::sync::atomic::{AtomicBool, Ordering};
            static FIRST_RENDER: AtomicBool = AtomicBool::new(true);

            // Create named function that can recursively schedule itself
            fn render_frame(event_rx: Rc<RefCell<tokio::sync::mpsc::UnboundedReceiver<AppEvent>>>) {
                // Process any pending blockchain events
                if let Ok(mut rx) = event_rx.try_borrow_mut() {
                    let mut processed = 0;
                    while let Ok(event) = rx.try_recv() {
                        APP.with(|cell| {
                            let app_rc = cell.get().expect("APP not initialized");
                            app_rc.borrow_mut().on_event(event);
                        });
                        processed += 1;
                    }
                    if processed > 0 {
                        log::debug!("Processed {processed} events this frame");
                    }
                }

                // Always redraw (terminal will detect if nothing changed)
                draw_once();

                // Log first render for debugging
                if FIRST_RENDER.load(Ordering::Relaxed) {
                    FIRST_RENDER.store(false, Ordering::Relaxed);
                    log::info!("🎨 First render completed via RAF");
                }

                // Schedule next frame (recursive call)
                let event_rx_clone = event_rx.clone();
                let _ = request_animation_frame(move |_timestamp| {
                    render_frame(event_rx_clone);
                });
            }

            // Kick off RAF loop (will call draw_once() in first frame)
            // Note: Don't call draw_once() here - let RAF handle the first render
            // to ensure Ratzilla's DomBackend is fully attached to the DOM
            render_frame(event_rx);
        }

        /// Configure root element sizing and focus
        fn configure_root_element(root: &HtmlElement) {
            let style = root.style();
            style.set_property("width", "100%").ok();
            style.set_property("height", "100%").ok();
            style.set_property("outline", "none").ok();

            // Make focusable
            root.set_tab_index(0);

            // Auto-focus on load
            let _ = root.focus();
        }

        /// Load configuration from URL parameters and localStorage
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

            let network = search_params
                .as_ref()
                .and_then(|p| p.get("network"))
                .unwrap_or_else(|| "mainnet".to_string());

            log::info!("RPC: {}, Network: {}, Filter: {}, Token: {} (from {})",
                rpc_url, network, filter,
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

        /// WASM entry point (async to await fonts.ready)
        #[wasm_bindgen(start)]
        pub async fn main() -> Result<(), JsValue> {
            use web_sys::console;

            // Setup panic hook and logging
            console_error_panic_hook::set_once();
            wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

            console::time_with_label("🦀 Total WASM init");
            log::info!("🦀 Ratacat Web starting (production harness v0.4.0)...");

            // Get DOM elements
            console::time_with_label("⏱️ DOM query");
            let win = window().ok_or_else(|| JsValue::from_str("no window"))?;
            let doc: Document = win.document().ok_or_else(|| JsValue::from_str("no document"))?;
            let root: HtmlElement = doc
                .get_element_by_id("canvas_parent")
                .ok_or_else(|| JsValue::from_str("#canvas_parent missing"))?
                .dyn_into()?;
            console::time_end_with_label("⏱️ DOM query");

            configure_root_element(&root);

            // Load configuration
            console::time_with_label("⏱️ Config load");
            let config = Rc::new(load_web_config());
            log::info!("Config: RPC={}, Filter={}", &config.near_node_url, config.default_filter);
            console::time_end_with_label("⏱️ Config load");

            // Create app state
            console::time_with_label("⏱️ App::new");
            let app = App::new(
                config.render_fps,
                config.render_fps_choices.clone(),
                config.keep_blocks,
                config.default_filter.clone(),
                None, // No archival fetch for web
            );
            console::time_end_with_label("⏱️ App::new");

            // Create Ratzilla terminal with DOM backend
            console::time_with_label("⏱️ Ratzilla DomBackend::new");
            let backend = DomBackend::new().map_err(|e| JsValue::from_str(&format!("Backend error: {e}")))?;
            console::time_end_with_label("⏱️ Ratzilla DomBackend::new");

            console::time_with_label("⏱️ Terminal::new");
            let terminal = Terminal::new(backend).map_err(|e| JsValue::from_str(&format!("Terminal error: {e}")))?;
            console::time_end_with_label("⏱️ Terminal::new");

            // Initialize global state (must happen before event listeners)
            console::time_with_label("⏱️ init_globals");
            init_globals(app, terminal);
            console::time_end_with_label("⏱️ init_globals");

            // Setup event channel for blockchain data
            let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<AppEvent>();
            let event_rx = Rc::new(RefCell::new(event_rx));

            // Start RPC polling task
            console::time_with_label("⏱️ spawn RPC polling");
            let config_clone = config.clone();
            wasm_bindgen_futures::spawn_local(async move {
                if let Err(e) = source_rpc::run_rpc(&config_clone, event_tx).await {
                    log::error!("RPC polling failed: {e}");
                }
            });
            console::time_end_with_label("⏱️ spawn RPC polling");

            // Install DOM event listeners
            console::time_with_label("⏱️ install_event_listeners");
            install_event_listeners(&doc, &root);
            install_resize_observer(&root);
            console::time_end_with_label("⏱️ install_event_listeners");

            // Boot draws: immediate + timed follow-ups (fixes black screen)
            console::time_with_label("⏱️ boot_draws");
            boot_draws();
            console::time_end_with_label("⏱️ boot_draws");

            // Wait for fonts to load (prevents 0x0 cell sizing)
            console::time_with_label("⏱️ fonts_ready");
            fonts_ready(&doc).await;
            console::time_end_with_label("⏱️ fonts_ready");

            // Draw again after fonts are ready
            draw_once();
            log::info!("🎨 Post-font draw complete");

            // Start RAF loop
            console::time_with_label("⏱️ start_raf_loop");
            start_raf_loop(event_rx);
            console::time_end_with_label("⏱️ start_raf_loop");

            console::time_end_with_label("🦀 Total WASM init");
            log::info!("🚀 Ratacat Web running (RAF loop active)");

            // Hide loading screen now that we're fully initialized
            hide_loading_screen();

            Ok(())
        }

        /// Call JavaScript to hide loading screen
        #[wasm_bindgen(inline_js = "export function hide_loading_js() { if (window.hideLoading) window.hideLoading(); }")]
        extern "C" {
            fn hide_loading_js();
        }

        fn hide_loading_screen() {
            hide_loading_js();
        }
    }
}
