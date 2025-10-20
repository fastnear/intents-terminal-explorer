use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum DecodedArgs {
    Json(Value),
    Text(String),
    Bytes { _hex: String, preview: String },
    Empty,
    Error(String),
}

pub fn decode_args_base64(b64: Option<&str>, preview_len: usize) -> DecodedArgs {
    let s = match b64 {
        Some(s) if !s.trim().is_empty() => s,
        _ => return DecodedArgs::Empty,
    };

    let bytes = match B64.decode(s) {
        Ok(v) => v,
        Err(e) => return DecodedArgs::Error(format!("base64: {e}")),
    };

    if bytes.is_empty() {
        return DecodedArgs::Empty;
    }

    // Try JSON first
    if let Ok(v) = serde_json::from_slice::<Value>(&bytes) {
        return DecodedArgs::Json(v);
    }

    // Try UTF-8 text if mostly printable
    let text = String::from_utf8_lossy(&bytes).to_string();
    let printable = text.chars().filter(|&ch| ch >= ' ' && ch <= '~').count();
    if printable as f32 / (text.len().max(1) as f32) > 0.85 {
        return DecodedArgs::Text(text);
    }

    // Fallback: hex with ASCII preview
    let n = preview_len.min(bytes.len());
    let hex = bytes[..n]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>();
    let preview = bytes[..n]
        .iter()
        .map(|&b| {
            if (0x20..=0x7e).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect();
    DecodedArgs::Bytes { _hex: hex, preview }
}
