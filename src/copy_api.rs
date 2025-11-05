use serde_json::Value;
use crate::App;

/// Pane-aware JSON copy (no UI coupling; works on all targets).
pub fn get_copy_content(app: &App) -> String {
    match app.pane() {
        0 => copy_block_json(app),
        1 => copy_tx_json(app),
        2 => copy_details_json(app),
        _ => String::new(),
    }
}

pub fn copy_block_json(app: &App) -> String {
    if let Some(b) = app.current_block() {
        let (filtered, _start, total) = app.txs();
        let v = if filtered.len() != total {
            crate::copy_payload::block_json(b, Some(&filtered))
        } else {
            crate::copy_payload::block_json(b, None)
        };
        serde_json::to_string_pretty(&v).unwrap_or_default()
    } else {
        String::new()
    }
}

pub fn copy_tx_json(app: &App) -> String {
    // Try to reuse the details pane JSON if it's valid JSON.
    if let Ok(v) = serde_json::from_str::<Value>(app.details()) {
        return serde_json::to_string_pretty(&v).unwrap_or_default();
    }
    // Fallback: best-effort from current block's filtered list.
    if let Some(b) = app.current_block() {
        let (filtered, sel_tx, _total) = app.txs();
        if let Some(tx) = filtered.get(sel_tx) {
            let v = crate::copy_payload::tx_summary_json(Some(b), tx);
            return serde_json::to_string_pretty(&v).unwrap_or_default();
        }
    }
    String::new()
}

pub fn copy_details_json(app: &App) -> String {
    copy_tx_json(app)
}