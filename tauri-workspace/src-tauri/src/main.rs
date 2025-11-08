#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager};

/// Keeps deep-link URLs that arrive before the webview is ready.
#[derive(Default, Clone)]
struct PendingLinks(Arc<Mutex<Vec<String>>>);

fn main() {
    // Logger initialized by tauri-plugin-log
    log::info!("NEARx Tauri starting");

    let pending = PendingLinks::default();

    let builder = tauri::Builder::default()
        .manage(pending.clone())
        // Deep-link plugin: registers the "nearx" scheme and forwards URLs to the webview
        .plugin(tauri_plugin_deep_link::init())
        // Single-instance plugin: if a second process starts with nearx://..., forward it
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            log::info!("Single-instance: received argv: {:?}", argv);
            // argv example (Windows): ["nearx.exe", "nearx://v1/tx/ABC123"]
            // macOS may call via deep_link plugin instead; this is an extra guard.
            for arg in argv {
                let arg = arg.trim().to_string();
                if arg.starts_with("nearx://") {
                    log::info!("Single-instance: forwarding deep link: {}", arg);
                    let _ = app.emit("nearx://open", arg.clone());
                } else if arg.starts_with("/v1/") || arg.starts_with("v1/") || arg.contains("#/v1/") {
                    // Normalize non-scheme route to a nearx:// URL for consistency
                    let norm = format!("nearx://{}", arg.trim_start_matches('/'));
                    log::info!("Single-instance: normalized {} to {}", arg, norm);
                    let _ = app.emit("nearx://open", norm);
                }
            }
        }))
        .plugin(tauri_plugin_log::Builder::default().build())
        .setup(move |app| {
            log::info!("Tauri setup starting");

            let pending_clone = pending.clone();
            // Register the custom scheme for this app.
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                // Register deep link handler
                #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
                app.deep_link().register_all()?;

                log::info!("Deep link registration complete");

                let app_handle = app.handle().clone();

                // When a deep link arrives (warm or cold), buffer it and *try* to emit.
                app.deep_link().on_open_url(move |event| {
                    let urls = event.urls();
                    log::info!("Deep link received: {:?}", urls);

                    for url in urls {
                        let s = url.to_string();
                        {
                            let mut q = pending_clone.0.lock().expect("pending lock");
                            q.push(s.clone());
                        }
                        // Try to deliver immediately to any open windows; if none exist yet,
                        // the page-load hook will flush.
                        log::info!("Emitting deep link event: {}", s);
                        let _ = app_handle.emit("nearx://open", s);
                    }
                });

                // Check for initial deep links on cold start
                if let Some(urls) = app.deep_link().get_current()? {
                    log::info!("Initial deep links: {:?}", urls);
                    for url in urls {
                        let s = url.to_string();
                        let mut q = pending.0.lock().expect("pending lock");
                        q.push(s);
                    }
                }
            }

            log::info!("Tauri setup complete");

            // Auto-open DevTools in debug builds
            #[cfg(debug_assertions)]
            {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                    log::info!("DevTools opened");
                }
            }

            Ok(())
        })
        // Command to flush queued URLs when frontend is ready
        .invoke_handler(tauri::generate_handler![get_queued_links]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn get_queued_links(state: tauri::State<PendingLinks>) -> Vec<String> {
    let mut q = state.0.lock().expect("pending lock");
    let links = q.drain(..).collect();
    log::info!("Frontend requested queued links: {:?}", links);
    links
}
