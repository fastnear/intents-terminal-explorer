use copypasta::{ClipboardContext, ClipboardProvider};

pub fn copy_to_clipboard(s: &str) -> bool {
    match ClipboardContext::new() {
        Ok(mut ctx) => ctx.set_contents(s.to_string()).is_ok(),
        Err(_) => false,
    }
}
