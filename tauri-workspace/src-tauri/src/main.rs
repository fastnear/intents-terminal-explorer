#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};

#[cfg(feature = "e2e")]
mod test_api;

#[derive(Default, Clone)]
struct PendingLinks(Arc<Mutex<Vec<String>>>);

#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

fn main() {
    let pending = PendingLinks::default();

    let mut builder = tauri::Builder::default()
        .manage(pending.clone())
        // Logging to DevTools console
        .plugin(tauri_plugin_log::Builder::default().build())
        // Deep-link registration for nearx:// scheme
        .plugin(tauri_plugin_deep_link::init())
        // Single instance; forward deep links / CLI args to the running window
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // Forward CLI args that look like deep links
            for arg in argv {
                if arg.starts_with("nearx://") {
                    let _ = app.emit("nearx://open", arg);
                }
            }
        }))
        // System browser for Google OAuth / external links
        .plugin(tauri_plugin_opener::init())
        // Clipboard support (first tier of fallback chain)
        .plugin(tauri_plugin_clipboard_manager::init());

    // Add E2E test commands if feature is enabled
    #[cfg(feature = "e2e")]
    {
        builder = builder.invoke_handler(tauri::generate_handler![
            open_external,
            test_api::nearx_test_emit_deeplink,
            test_api::nearx_test_get_last_route,
            test_api::nearx_test_clear_storage
        ]);
    }

    #[cfg(not(feature = "e2e"))]
    {
        builder = builder.invoke_handler(tauri::generate_handler![open_external]);
    }

    builder
        .setup(move |app| {
            let pending_clone = pending.clone();
            let app_handle = app.handle().clone();

            // Register deep link handler
            #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                // Register scheme on Linux and debug Windows
                #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
                app.deep_link().register_all()?;

                // When a deep link arrives, buffer and emit
                app.deep_link().on_open_url(move |event| {
                    for url in event.urls() {
                        let s = url.to_string();
                        // Buffer in case window isn't ready
                        pending_clone.0.lock().unwrap().push(s.clone());
                        // Try immediate delivery
                        let _ = app_handle.emit("nearx://open", s);
                    }
                });

                // Check for initial deep links on cold start
                if let Some(urls) = app.deep_link().get_current()? {
                    for url in urls {
                        pending.0.lock().unwrap().push(url.to_string());
                    }
                }
            }

            // Auto-open DevTools in debug builds
            #[cfg(debug_assertions)]
            if let Some(win) = app.get_webview_window("main") {
                win.open_devtools();
                let _ = win.set_focus();
            }

            // Flush any buffered deep links to the now-ready window
            if let Some(win) = app.get_webview_window("main") {
                let mut q = pending.0.lock().unwrap();
                for url in q.drain(..) {
                    let _ = win.emit("nearx://open", url);
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("NEARx Tauri failed");
}
