//! WASM-specific JavaScript bridge functions
//!
//! This module provides Rust bindings for JavaScript functions exposed via window.NEARxAuth.
//! Only available when building for WebAssembly targets (web and Tauri).

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[cfg(target_arch = "wasm32")]
use js_sys::{Function, Promise, Reflect};

#[cfg(target_arch = "wasm32")]
use web_sys::window;

// ----------------------- EXTERNAL BROWSER OPENER ----------------------------

/// Open URL in system browser (Tauri) or new tab (Web)
/// Prefers Tauri's open_external command if available, fallback to window.open
#[cfg(target_arch = "wasm32")]
#[inline]
pub fn open_external(url: &str) {
    use js_sys::{Object, Reflect};
    use wasm_bindgen::JsValue;
    if let Some(win) = window() {
        // Try Tauri invoke first
        if let Ok(tauri) = Reflect::get(&JsValue::from(&win), &JsValue::from_str("__TAURI__")) {
            if let Ok(invoke) = Reflect::get(&tauri, &JsValue::from_str("invoke")) {
                let f = Function::from(invoke);
                // Build args object: { url: "..." }
                let args = Object::new();
                let _ = Reflect::set(&args, &JsValue::from_str("url"), &JsValue::from_str(url));
                let _ = f.call2(
                    &tauri,
                    &JsValue::from_str("open_external"),
                    &JsValue::from(args),
                );
                return;
            }
        }
        // Fallback to window.open (Web)
        let _ = win.open_with_url_and_target(url, "_blank");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_external(_url: &str) {}

// ----------------------- AUTH JS BRIDGE (wasm32) ----------------------------

/// Initiate Google OAuth login flow
/// Calls window.NEARxAuth.loginGoogle() which opens OAuth provider in system browser
#[cfg(target_arch = "wasm32")]
#[inline]
pub fn auth_login_google() {
    if let Some(win) = window() {
        if let Ok(obj) = Reflect::get(&JsValue::from(win), &JsValue::from_str("NEARxAuth")) {
            if let Ok(f) = Reflect::get(&obj, &JsValue::from_str("loginGoogle")) {
                let f = Function::from(f);
                let _ = f.call0(&obj);
                return;
            }
        }
    }
    // Final fallback if NEARxAuth not loaded
    open_external("about:blank");
}

/// Initiate magic link (passwordless email) login flow
/// Calls window.NEARxAuth.loginMagic() which prompts for email and sends magic link
#[cfg(target_arch = "wasm32")]
#[inline]
pub fn auth_login_magic() {
    if let Some(win) = window() {
        if let Ok(obj) = Reflect::get(&JsValue::from(win), &JsValue::from_str("NEARxAuth")) {
            if let Ok(f) = Reflect::get(&obj, &JsValue::from_str("loginMagic")) {
                let f = Function::from(f);
                let _ = f.call0(&obj);
            }
        }
    }
}

/// Exchange an auth code for a token via JS bridge.
/// Calls window.NEARxAuth.exchangeCode(code) which performs PKCE token exchange
#[cfg(target_arch = "wasm32")]
pub fn auth_exchange_code<F: 'static + FnOnce(Option<String>)>(code: &str, cb: F) {
    if let Some(win) = window() {
        if let Ok(obj) = Reflect::get(&JsValue::from(win), &JsValue::from_str("NEARxAuth")) {
            if let Ok(f) = Reflect::get(&obj, &JsValue::from_str("exchangeCode")) {
                let f = Function::from(f);
                let this = obj.clone();
                let promise = f.call1(&this, &JsValue::from_str(code)).ok();
                if let Some(p) = promise {
                    wasm_bindgen_futures::spawn_local(async move {
                        let out = JsFuture::from(Promise::from(p)).await.ok();
                        let s = out.and_then(|v| v.as_string());
                        cb(s);
                    });
                    return;
                }
            }
        }
    }
    cb(None);
}

// No-op implementations for non-WASM builds (native terminal)
#[cfg(not(target_arch = "wasm32"))]
pub fn auth_login_google() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn auth_login_magic() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn auth_exchange_code<F: 'static + FnOnce(Option<String>)>(_c: &str, cb: F) {
    cb(None)
}
