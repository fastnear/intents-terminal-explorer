//! Filterable debug logging system for Web/Tauri development
//!
//! Categories: UI, INPUT, MOUSE, COPY, ROUTER, RENDER, THEME, DPR
//! Enable via: ?nxdebug=all or localStorage.setItem('nearx.debug','ui,mouse,overlay')
//! Toggle overlay: Ctrl+Shift+D

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

pub mod cat {
    pub const UI: u32 = 1 << 0;
    pub const INPUT: u32 = 1 << 1;
    pub const MOUSE: u32 = 1 << 2;
    pub const COPY: u32 = 1 << 3;
    pub const ROUTER: u32 = 1 << 4;
    pub const RENDER: u32 = 1 << 5;
    pub const THEME: u32 = 1 << 6;
    pub const DPR: u32 = 1 << 7;
    pub const ALL: u32 = 0xffff_ffff;
}

static MASK: AtomicU32 = AtomicU32::new(0);
static OVERLAY: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn mask() -> u32 {
    MASK.load(Ordering::Relaxed)
}

#[inline]
pub fn set(mask: u32) {
    MASK.store(mask, Ordering::Relaxed)
}

#[inline]
pub fn enable(bits: u32) {
    MASK.fetch_or(bits, Ordering::Relaxed);
}

#[inline]
pub fn disable(bits: u32) {
    let m = MASK.load(Ordering::Relaxed);
    MASK.store(m & !bits, Ordering::Relaxed);
}

#[inline]
pub fn is(cat: u32) -> bool {
    (MASK.load(Ordering::Relaxed) & cat) != 0
}

#[inline]
pub fn overlay() -> bool {
    OVERLAY.load(Ordering::Relaxed)
}

#[inline]
pub fn set_overlay(on: bool) {
    OVERLAY.store(on, Ordering::Relaxed)
}

#[inline]
pub fn cat_name(cat: u32) -> &'static str {
    match cat {
        c if c == cat::UI => "ui",
        c if c == cat::INPUT => "input",
        c if c == cat::MOUSE => "mouse",
        c if c == cat::COPY => "copy",
        c if c == cat::ROUTER => "router",
        c if c == cat::RENDER => "render",
        c if c == cat::THEME => "theme",
        c if c == cat::DPR => "dpr",
        _ => "misc",
    }
}

#[inline]
pub fn set_from_list(list: &str) {
    let mut m: u32 = 0;
    for tok in list.split(',').map(|s| s.trim().to_ascii_lowercase()) {
        match tok.as_str() {
            "" | "none" => {
                m = 0;
            }
            "all" => {
                m = cat::ALL;
            }
            "overlay" => {
                set_overlay(true);
            }
            "ui" => {
                m |= cat::UI;
            }
            "input" => {
                m |= cat::INPUT;
            }
            "mouse" => {
                m |= cat::MOUSE;
            }
            "copy" => {
                m |= cat::COPY;
            }
            "router" => {
                m |= cat::ROUTER;
            }
            "render" => {
                m |= cat::RENDER;
            }
            "theme" => {
                m |= cat::THEME;
            }
            "dpr" => {
                m |= cat::DPR;
            }
            _ => {}
        }
    }
    set(m);
}

#[cfg(target_arch = "wasm32")]
pub fn init_from_url_and_storage_once() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        #[allow(unused_imports)]
        use wasm_bindgen::JsValue;
        use web_sys::window;
        if let Some(win) = window() {
            // URL query: ?nxdebug=ui,mouse,overlay  OR  ?nxdbg=all
            if let Ok(search) = win.location().search() {
                let qs = search.trim_start_matches('?');
                for part in qs.split('&') {
                    let mut it = part.splitn(2, '=');
                    let key = it.next().unwrap_or_default();
                    let val = it.next().unwrap_or_default();
                    if key.eq_ignore_ascii_case("nxdebug") || key.eq_ignore_ascii_case("nxdbg") {
                        if let Ok(decoded_js) = js_sys::decode_uri_component(val) {
                            let decoded = decoded_js.as_string().unwrap_or_default();
                            set_from_list(&decoded);
                        }
                    }
                    if key.eq_ignore_ascii_case("nxoverlay") {
                        if let Ok(decoded_js) = js_sys::decode_uri_component(val) {
                            let v = decoded_js.as_string().unwrap_or_default().to_ascii_lowercase();
                            if v == "1" || v == "true" {
                                set_overlay(true);
                            }
                        }
                    }
                }
            }
            // localStorage: nearx.debug = "ui,mouse,overlay"
            if let Ok(Some(storage)) = win.local_storage() {
                if let Ok(Some(v)) = storage.get_item("nearx.debug") {
                    set_from_list(&v);
                }
                if let Ok(Some(v)) = storage.get_item("nearx.debugOverlay") {
                    let v = v.to_ascii_lowercase();
                    if v == "1" || v == "true" {
                        set_overlay(true);
                    }
                }
            }
        }
        log(cat::UI, "debug init (wasm) complete");
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_from_url_and_storage_once() {
    // No-op on native; use env vars or CLI in future if needed.
}

#[cfg(target_arch = "wasm32")]
#[inline]
pub fn log(cat: u32, msg: impl AsRef<str>) {
    if !is(cat) {
        return;
    }
    let s = format!("[NEARx][{}] {}", cat_name(cat), msg.as_ref());
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&s));
}

#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub fn log(cat: u32, msg: impl AsRef<str>) {
    if !is(cat) {
        return;
    }
    eprintln!("[NEARx][{}] {}", cat_name(cat), msg.as_ref());
}
