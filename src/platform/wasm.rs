use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    // Try the Tauri/Extension-aware bridge (if the page provides it).
    // `catch` prevents a panic when the function is missing.
    #[wasm_bindgen(js_namespace = window, js_name = __copy_text, catch)]
    fn __copy_text_js(s: &str) -> Result<js_sys::Promise, JsValue>;
}

pub fn copy_to_clipboard(s: &str) -> bool {
    // 1) Preferred: bridge present (Tauri / Extension / Web).
    if let Ok(promise) = __copy_text_js(s) {
        wasm_bindgen_futures::spawn_local(async move { let _ = JsFuture::from(promise).await; });
        return true;
    }

    // 2) Fallback: plain web clipboard (secure contexts).
    let Some(win) = web_sys::window() else { return false; };
    let clip = win.navigator().clipboard();
    if let Some(clip) = clip {
        let p = clip.write_text(s);
        wasm_bindgen_futures::spawn_local(async move { let _ = JsFuture::from(p).await; });
        return true;
    }

    // No path available (old WebViews etc.).
    false
}