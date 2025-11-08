use serde_json::Value;

pub fn pretty(v: &Value, space: usize) -> String {
    // Plain JSON formatting without ANSI codes
    // Ratatui doesn't interpret ANSI escape codes - they appear as literal characters
    fn fmt(v: &Value, ind: usize, sp: usize, out: &mut String) {
        let pad = " ".repeat(ind);
        match v {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(&format!("{b}")),
            Value::Number(n) => out.push_str(&format!("{n}")),
            Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap()),
            Value::Array(a) => {
                if a.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push('[');
                out.push('\n');
                for (i, it) in a.iter().enumerate() {
                    out.push_str(&" ".repeat(ind + sp));
                    fmt(it, ind + sp, sp, out);
                    if i + 1 != a.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push(']');
            }
            Value::Object(m) => {
                if m.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push('{');
                out.push('\n');
                let mut keys: Vec<&String> = m.keys().collect();
                keys.sort_unstable();
                for (i, k) in keys.iter().enumerate() {
                    out.push_str(&" ".repeat(ind + sp));
                    out.push_str(&format!("{}: ", serde_json::to_string(k.as_str()).unwrap()));
                    fmt(&m[*k], ind + sp, sp, out);
                    if i + 1 != keys.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&pad);
                out.push('}');
            }
        }
    }
    let mut out = String::new();
    fmt(v, 0, space, &mut out);
    out
}
