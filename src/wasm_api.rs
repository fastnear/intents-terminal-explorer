//! Minimal JS -> Rust surface for the web router shim.
//!
//! This module provides WASM-bindgen exports that allow JavaScript code
//! to call into Rust functionality. Currently used for auth callback handling.

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::prelude::*;

/// JS calls this when the hash route is `#/auth/callback?...`.
///
/// We accept both `code=` and `token=`; the Rust handler exchanges when needed.
/// This is called from web/router_shim.js after parsing the URL hash.
///
/// # Example
/// ```javascript
/// // From router_shim.js:
/// window.wasm_bindgen.nearx_auth_callback("token=abc123&foo=bar");
/// ```
#[wasm_bindgen]
pub fn nearx_auth_callback(qs: String) {
    crate::auth::handle_auth_callback_query(&qs);
}
