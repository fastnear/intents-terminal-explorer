//! Minimal auth surface for Web/Tauri.
//! - Token lives in webview storage (Tauri shares this), key: `nearx.token`
//! - Upper-right menu: Google OAuth or Magic link
//! - Callback route: `#/auth/callback?...` (Web) or `nearx://auth/callback?...` (Tauri)
//! - Debug category: [NEARx][auth]
use crate::debug::{self, cat};
use std::sync::{Arc, Mutex, OnceLock};
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct AuthState {
    pub token: Option<String>,
    pub email: Option<String>,    // optional, if backend returns it
    pub provider: Option<String>, // "google" | "magic"
}

static STATE: OnceLock<Arc<Mutex<AuthState>>> = OnceLock::new();

fn state_ref() -> &'static Arc<Mutex<AuthState>> {
    STATE.get_or_init(|| Arc::new(Mutex::new(AuthState::default())))
}

#[inline]
pub fn state() -> AuthState {
    if let Ok(guard) = state_ref().lock() {
        guard.clone()
    } else {
        AuthState::default()
    }
}

#[inline]
pub fn set_token(token: String, provider: Option<String>, email: Option<String>) {
    if let Ok(mut s) = state_ref().lock() {
        s.token = Some(token.clone());
        s.provider = provider;
        s.email = email;
    }
    debug::log(cat::AUTH, "token set");
    persist_token_webview(Some(token));
}

#[inline]
pub fn clear() {
    if let Ok(mut s) = state_ref().lock() {
        *s = AuthState::default();
    }
    debug::log(cat::AUTH, "token cleared");
    persist_token_webview(None);
}

/// Returns true if a non-empty token is present.
#[inline]
pub fn has_token() -> bool {
    let s = state();
    matches!(s.token.as_deref(), Some(t) if !t.is_empty())
}

#[inline]
pub fn token_string() -> Option<String> {
    state().token
}

/// Attach Authorization to reqwest request builder (native HTTP path),
/// if you adopt it in your network layer later. NOP on wasm.
#[cfg(not(target_arch = "wasm32"))]
pub fn attach_auth<B: Clone>(rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(tk) = token_string() {
        rb.bearer_auth(tk)
    } else {
        rb
    }
}

#[cfg(target_arch = "wasm32")]
pub fn attach_auth<B: Clone>(rb: B) -> B {
    rb
}

/// Parse auth callback params and stash token. Supports:
/// - token=<jwt>
/// - code=<auth_code> (exchanged via JS bridge if available)
pub fn handle_auth_callback_query(qs: &str) {
    let mut token: Option<String> = None;
    #[cfg_attr(
        not(target_arch = "wasm32"),
        allow(unused_variables, unused_assignments)
    )]
    let mut code: Option<String> = None;
    for kv in qs.split('&') {
        let mut it = kv.splitn(2, '=');
        let k = it.next().unwrap_or_default();
        let v = it.next().unwrap_or_default();
        let k = k.trim().to_ascii_lowercase();
        let v = urlencoding::decode(v)
            .unwrap_or_else(|_| v.into())
            .to_string();
        match k.as_str() {
            "token" => token = Some(v),
            #[cfg_attr(not(target_arch = "wasm32"), allow(unused_assignments))]
            "code" => code = Some(v),
            _ => {}
        }
    }
    if let Some(t) = token {
        set_token(t, None, None);
        // Scrub token from URL immediately to prevent leaks via history/share
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::window;
            if let Some(win) = window() {
                if let Ok(hist) = win.history() {
                    let _ =
                        hist.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some("#/"));
                }
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(c) = code {
            crate::webshim::auth_exchange_code(&c, |res| {
                if let Some(tok) = res {
                    set_token(tok, Some("google".into()), None);
                }
            });
        }
    }
}

// --- Webview persistence ----------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn persist_token_webview(tok: Option<String>) {
    use web_sys::window;
    if let Some(win) = window() {
        if let Ok(Some(ls)) = win.local_storage() {
            let _ = match tok {
                Some(t) => ls.set_item("nearx.token", &t),
                None => ls.remove_item("nearx.token"),
            };
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn persist_token_webview(_tok: Option<String>) {}

#[cfg(target_arch = "wasm32")]
pub fn bootstrap_from_storage() {
    use web_sys::window;
    if let Some(win) = window() {
        if let Ok(Some(ls)) = win.local_storage() {
            if let Ok(Some(t)) = ls.get_item("nearx.token") {
                if !t.is_empty() {
                    set_token(t, None, None);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn bootstrap_from_storage() {}
