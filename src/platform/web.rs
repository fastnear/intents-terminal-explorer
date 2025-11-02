/// Web platform clipboard implementation using WASM-bindgen bridge
///
/// This module provides clipboard functionality for WASM targets by calling
/// a JavaScript function that handles the actual clipboard operations.
/// The JavaScript side (`web/platform.js`) provides a unified facade that
/// auto-detects the best available clipboard API:
///
/// 1. Tauri plugin API (if running in Tauri WebView)
/// 2. Browser extension relay (if running in extension context)
/// 3. Navigator Clipboard API (modern browsers)
/// 4. Legacy execCommand fallback
///
/// This approach ensures clipboard works reliably across all web environments.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// JavaScript function provided by web/platform.js
    /// Returns a Promise<boolean> indicating success/failure
    #[wasm_bindgen(js_name = __copy_text)]
    fn __copy_text_js(s: &str) -> js_sys::Promise;
}

/// Copy text to clipboard using the best available web API
///
/// This function spawns an async task to handle the clipboard operation
/// and returns immediately. The JavaScript promise is handled asynchronously.
///
/// # Arguments
///
/// * `s` - The text to copy to the clipboard
///
/// # Returns
///
/// Returns `true` optimistically. The actual result is logged in the browser console.
pub fn copy_to_clipboard(s: &str) -> bool {
    let text = s.to_owned();

    wasm_bindgen_futures::spawn_local(async move {
        let promise = __copy_text_js(&text);

        match wasm_bindgen_futures::JsFuture::from(promise).await {
            Ok(_) => {
                log::debug!("Clipboard copy succeeded");
            }
            Err(e) => {
                log::error!("Clipboard copy failed: {:?}", e);
            }
        }
    });

    // Return true optimistically since the operation is async
    true
}
