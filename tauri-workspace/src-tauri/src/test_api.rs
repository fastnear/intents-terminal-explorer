//! Test-only IPC commands for E2E testing
//!
//! This module is only compiled when the `e2e` feature is enabled.
//! It provides commands that allow E2E tests to inject events and query state
//! without exposing these capabilities in production builds.

#[cfg(feature = "e2e")]
use tauri::Emitter;

#[cfg(feature = "e2e")]
#[tauri::command]
pub async fn nearx_test_emit_deeplink(app: tauri::AppHandle, url: String) -> Result<(), String> {
    log::info!("🧪 [E2E-TEST] Emitting deep link: {}", url);
    app.emit("nearx://open", url).map_err(|e| {
        log::error!("🧪 [E2E-TEST] Failed to emit deep link: {}", e);
        e.to_string()
    })
}

#[cfg(feature = "e2e")]
#[tauri::command]
pub async fn nearx_test_get_last_route() -> Result<String, String> {
    // This would read from a global state if we tracked last route
    // For now, just a placeholder showing the pattern
    log::info!("🧪 [E2E-TEST] Getting last route");
    Ok(String::new())
}

#[cfg(feature = "e2e")]
#[tauri::command]
pub async fn nearx_test_clear_storage(window: tauri::WebviewWindow) -> Result<(), String> {
    log::info!("🧪 [E2E-TEST] Clearing storage");
    window
        .eval("localStorage.clear(); sessionStorage.clear();")
        .map_err(|e| e.to_string())
}
