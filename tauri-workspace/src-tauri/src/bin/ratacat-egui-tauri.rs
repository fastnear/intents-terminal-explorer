//! Ratacat egui + Tauri Integration
//!
//! Production-grade TUI embedded in egui, running inside Tauri for:
//! - Native desktop (OpenGL/glow)
//! - Deep link handling via Tauri
//! - Bearer token UI with localStorage persistence
//!
//! This binary uses egui_ratatui to render the full Ratacat TUI
//! inside an egui canvas, providing a single codebase for desktop.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use egui::{Align, CentralPanel, Layout, TopBottomPanel};
use egui_ratatui::RataguiBackend;
use ratacat::{App, AppEvent};
use ratatui::Terminal;
use embedded_graphics_unicodefonts::{
    mono_8x13_atlas, mono_8x13_bold_atlas, mono_8x13_italic_atlas,
};
use soft_ratatui::{EmbeddedGraphics, SoftBackend};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};
use tauri_plugin_deep_link::DeepLinkExt;


// ============================================================================
// State Management
// ============================================================================

/// Shared state between Tauri backend and egui frontend
#[derive(Clone)]
struct AppState {
    /// Ratacat app instance
    app: Arc<Mutex<App>>,
    /// Channel for blockchain events
    #[allow(dead_code)]
    event_rx: Arc<Mutex<tokio::sync::mpsc::UnboundedReceiver<AppEvent>>>,
    /// Bearer token (for UI display/edit)
    bearer_token: Arc<Mutex<Option<String>>>,
    /// RPC URL (for UI display/edit)
    rpc_url: Arc<Mutex<String>>,
}

// ============================================================================
// Tauri Commands
// ============================================================================

#[tauri::command]
fn get_bearer_token(state: State<AppState>) -> Option<String> {
    state.bearer_token.lock().unwrap().clone()
}

#[tauri::command]
fn set_bearer_token(state: State<AppState>, token: Option<String>) {
    *state.bearer_token.lock().unwrap() = token;
    // TODO: Update RPC client with new token
}

#[tauri::command]
fn get_rpc_url(state: State<AppState>) -> String {
    state.rpc_url.lock().unwrap().clone()
}

#[tauri::command]
fn set_rpc_url(state: State<AppState>, url: String) {
    *state.rpc_url.lock().unwrap() = url;
    // TODO: Restart RPC client with new URL
}

// ============================================================================
// egui Application (placeholder for future egui-based UI)
// ============================================================================

#[allow(dead_code)]
struct RatacatEguiApp {
    /// Ratatui terminal with egui_ratatui backend
    terminal: Terminal<RataguiBackend<EmbeddedGraphics>>,
    /// Shared app state
    state: AppState,
    /// Settings panel expanded
    settings_open: bool,
    /// Bearer token editor (temporary, synced on save)
    bearer_edit: String,
    /// RPC URL editor (temporary, synced on save)
    rpc_edit: String,
}

impl RatacatEguiApp {
    #[allow(dead_code)]
    fn new(_cc: &eframe::CreationContext<'_>, state: AppState) -> Self {
        // Create software-rendered ratatui backend using SoftBackend + EmbeddedGraphics
        let font_regular = mono_8x13_atlas();
        let font_bold = Some(mono_8x13_bold_atlas());
        let font_italic = Some(mono_8x13_italic_atlas());

        let soft_backend = SoftBackend::<EmbeddedGraphics>::new(
            120,  // width in columns (desktop-optimized, larger than web's 85)
            40,   // height in rows (desktop-optimized, larger than web's 30)
            font_regular,
            font_bold,
            font_italic,
        );

        let backend = RataguiBackend::new("ratacat", soft_backend);
        let terminal = Terminal::new(backend).expect("Failed to create terminal");

        let bearer_edit = state.bearer_token.lock().unwrap().clone().unwrap_or_default();
        let rpc_edit = state.rpc_url.lock().unwrap().clone();

        Self {
            terminal,
            state,
            settings_open: false,
            bearer_edit,
            rpc_edit,
        }
    }

    #[allow(dead_code)]
    fn process_events(&mut self) {
        // Drain blockchain events and update app state
        if let Ok(mut rx) = self.state.event_rx.try_lock() {
            let mut count = 0;
            while let Ok(event) = rx.try_recv() {
                if let Ok(mut app) = self.state.app.try_lock() {
                    app.on_event(event);
                    count += 1;
                }
            }
            if count > 0 {
                log::debug!("Processed {} blockchain events", count);
            }
        }
    }

    #[allow(dead_code)]
    fn draw_settings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("⚙ Settings");
        ui.separator();

        // RPC URL
        ui.label("RPC URL:");
        ui.text_edit_singleline(&mut self.rpc_edit);
        if ui.button("💾 Save URL").clicked() {
            *self.state.rpc_url.lock().unwrap() = self.rpc_edit.clone();
            log::info!("RPC URL updated: {}", self.rpc_edit);
        }

        ui.add_space(8.0);

        // Bearer Token
        ui.label("Bearer Token:");
        let mut masked = self.bearer_edit.clone();
        if !masked.is_empty() {
            masked = format!("{}...", masked.chars().take(12).collect::<String>());
        }
        ui.label(format!("Current: {}", if masked.is_empty() { "none" } else { &masked }));

        ui.horizontal(|ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.bearer_edit)
                    .password(true)
                    .hint_text("Paste token here"),
            );

            if ui.button("💾 Save Token").clicked() || response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let token = if self.bearer_edit.is_empty() {
                    None
                } else {
                    Some(self.bearer_edit.clone())
                };
                *self.state.bearer_token.lock().unwrap() = token;
                log::info!("Bearer token updated");
            }
        });

        ui.add_space(8.0);
        ui.separator();
        ui.label("📝 Token persists to localStorage on web, env var on native");
    }
}

impl eframe::App for RatacatEguiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process blockchain events
        self.process_events();

        // Top panel: controls
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label("🐱 Ratacat");
                ui.separator();

                // Settings toggle
                if ui.button(if self.settings_open { "⚙ Hide Settings" } else { "⚙ Settings" }).clicked() {
                    self.settings_open = !self.settings_open;
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let bearer_status = if self.state.bearer_token.lock().unwrap().is_some() {
                        "🔐 Authenticated"
                    } else {
                        "⚠️  No Bearer Token"
                    };
                    ui.label(bearer_status);
                });
            });
        });

        // Settings panel (collapsible)
        if self.settings_open {
            TopBottomPanel::top("settings_panel")
                .resizable(false)
                .show(ctx, |ui| {
                    self.draw_settings_panel(ui);
                });
        }

        // Central panel: Ratatui TUI
        CentralPanel::default().show(ctx, |ui| {
            // Handle keyboard input
            ctx.input(|i| {
                // Check for 'c' key to copy
                if i.key_pressed(egui::Key::C) && !i.modifiers.ctrl {
                    if let Ok(app) = self.state.app.try_lock() {
                        let content = app.get_copy_content();

                        // Use platform abstraction (JavaScript bridge to Tauri plugin)
                        if ratacat::platform::copy_to_clipboard(&content) {
                            let msg = match app.pane() {
                                0 => "Copied block JSON",
                                1 => "Copied transaction JSON",
                                2 => "Copied details JSON",
                                _ => "Copied",
                            };
                            log::info!("{}", msg);
                        } else {
                            log::error!("Copy failed");
                        }
                    }
                }
            });

            // Draw ratatui UI
            if let Ok(mut app) = self.state.app.try_lock() {
                let _ = self.terminal.draw(|f| {
                    ratacat::ui::draw(f, &mut app, &[]); // Empty marks for now
                });
            }

            // Render the egui_ratatui widget
            ui.add(self.terminal.backend_mut());
        });

        // Request repaint for smooth animation
        ctx.request_repaint();
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract account name from deep link URL
/// Examples:
///   ratacat://account/alice.near -> Some("alice.near")
///   ratacat://tx/ABC123 -> None
fn extract_account_from_url(url: &tauri::Url) -> Option<String> {
    // Get path segments from Tauri Url type
    let path = url.path();
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    // Look for account pattern: ratacat://account/alice.near
    if segments.len() >= 2 && segments[0] == "account" {
        return Some(segments[1].to_string());
    }
    None
}

// ============================================================================
// Main Entry Point
// ============================================================================

fn main() -> Result<()> {
    env_logger::init();

    log::info!("🚀 Ratacat egui-tauri starting...");

    // Create blockchain event channel
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create tokio runtime for RPC polling
    let rt = tokio::runtime::Runtime::new()?;

    // Create config for RPC polling
    let rpc_url = std::env::var("NEAR_NODE_URL")
        .unwrap_or_else(|_| "https://rpc.mainnet.fastnear.com".to_string());
    let fastnear_auth_token = std::env::var("FASTNEAR_AUTH_TOKEN").ok();

    let config = ratacat::Config {
        source: ratacat::Source::Rpc,
        ws_url: String::new(),
        ws_fetch_blocks: false,
        render_fps: 30,
        render_fps_choices: vec![20, 30, 60],
        poll_interval_ms: 2000,
        poll_max_catchup: 5,
        poll_chunk_concurrency: 4,
        keep_blocks: 100,
        near_node_url: rpc_url.clone(),
        near_node_url_explicit: true,
        archival_rpc_url: None,
        rpc_timeout_ms: 8000,
        rpc_retries: 2,
        fastnear_auth_token: fastnear_auth_token.clone(),
        default_filter: "intents.near".to_string(),
        theme: ratacat::theme::Theme::default(),
    };

    // Spawn RPC polling task
    let event_tx_clone = event_tx.clone();
    let config_clone = config.clone();
    rt.spawn(async move {
        log::info!("📞 Starting RPC polling task...");
        if let Err(e) = ratacat::source_rpc::run_rpc(&config_clone, event_tx_clone).await {
            log::error!("❌ RPC polling failed: {}", e);
        }
    });

    // Create app state with defaults
    let app = App::new(
        30,                                   // fps
        vec![20, 30, 60],                    // fps_choices
        100,                                  // keep_blocks
        "intents.near".to_string(),          // default_filter
        None,                                 // No archival fetch
        ratacat::theme::ColorScheme::default(), // theme
    );

    let app_state = AppState {
        app: Arc::new(Mutex::new(app)),
        event_rx: Arc::new(Mutex::new(event_rx)),
        bearer_token: Arc::new(Mutex::new(fastnear_auth_token)),
        rpc_url: Arc::new(Mutex::new(rpc_url)),
    };

    // Start Tauri
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .manage(app_state.clone())
        .invoke_handler(tauri::generate_handler![
            get_bearer_token,
            set_bearer_token,
            get_rpc_url,
            set_rpc_url
        ])
        .setup(|app| {
            log::info!("✅ Tauri setup complete");

            // Register deep link handler
            #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
            app.deep_link().register_all()?;

            // Handle initial deep link
            let state = app.state::<AppState>();
            if let Some(urls) = app.deep_link().get_current()? {
                log::info!("📎 Initial deep links: {:?}", urls);
                for url in urls {
                    if let Some(account) = extract_account_from_url(&url) {
                        log::info!("🎯 Setting filter to account: {}", account);
                        if let Ok(mut app_instance) = state.app.try_lock() {
                            // Set filter using public API
                            app_instance.start_filter();
                            let filter_str = format!("acct:{}", account);
                            for ch in filter_str.chars() {
                                app_instance.filter_add_char(ch);
                            }
                            app_instance.apply_filter();
                        }
                    }
                }
            }

            // Handle runtime deep links
            let app_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let urls = event.urls();
                log::info!("📎 Runtime deep link: {:?}", urls);
                let state_clone = app_handle.state::<AppState>();
                for url in urls {
                    if let Some(account) = extract_account_from_url(&url) {
                        log::info!("🎯 Setting filter to account: {}", account);
                        if let Ok(mut app_instance) = state_clone.app.try_lock() {
                            // Set filter using public API
                            app_instance.start_filter();
                            let filter_str = format!("acct:{}", account);
                            for ch in filter_str.chars() {
                                app_instance.filter_add_char(ch);
                            }
                            app_instance.apply_filter();
                        }
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                log::info!("👋 Ratacat shutting down");
            }
        });

    Ok(())
}
