use std::sync::Once;
use std::time::Duration as StdDuration;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

pub type Duration = StdDuration;

#[derive(Clone, Copy, Debug)]
pub struct Instant(f64);

impl Instant {
    pub fn now() -> Self {
        if let Some(window) = web_sys::window() {
            if let Ok(perf) = window.performance() {
                return Self(perf.now());
            }
        }
        Self(js_sys::Date::now())
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        let delta = (self.0 - earlier.0).max(0.0);
        Duration::from_secs_f64(delta / 1000.0)
    }

    pub fn elapsed(&self) -> Duration {
        Self::now().duration_since(*self)
    }
}

pub async fn sleep(duration: Duration) {
    let millis = duration.as_millis().min(i32::MAX as u128) as i32;
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let resolve_fn = resolve.clone();
        let closure = Closure::once(move || {
            let _ = resolve_fn.call0(&JsValue::UNDEFINED);
        });

        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                millis,
            );
        }

        closure.forget();
    });

    let _ = JsFuture::from(promise).await;
}

struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let msg = format!("{} {}", record.target(), record.args());
        let value = JsValue::from_str(&msg);
        match record.level() {
            log::Level::Error => web_sys::console::error_1(&value),
            log::Level::Warn => web_sys::console::warn_1(&value),
            log::Level::Info => web_sys::console::info_1(&value),
            log::Level::Debug | log::Level::Trace => web_sys::console::log_1(&value),
        }
    }

    fn flush(&self) {}
}

static LOGGER: ConsoleLogger = ConsoleLogger;
static LOGGER_INIT: Once = Once::new();
static PANIC_HOOK: Once = Once::new();

pub fn init_logging(level: log::Level) {
    LOGGER_INIT.call_once(|| {
        let _ = log::set_logger(&LOGGER);
    });
    log::set_max_level(level.to_level_filter());
}

pub fn install_panic_hook() {
    PANIC_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let msg = info.to_string();
            web_sys::console::error_1(&JsValue::from_str(&format!("panic: {msg}")));
        }));
    });
}
