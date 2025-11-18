#[derive(Default, Debug, Clone)]
pub struct CompiledFilter {
    pub signer: Vec<String>,
    pub receiver: Vec<String>,
    pub acct: Vec<String>,
    pub action: Vec<String>,
    pub method: Vec<String>,
    pub raw: Vec<String>,
    pub hash: Vec<String>,
    pub free: Vec<String>,
}

pub fn compile_filter(q: &str) -> CompiledFilter {
    let mut f = CompiledFilter::default();
    for tok in q.split_whitespace() {
        let mut it = tok.splitn(2, ':');
        if let (Some(k), Some(v)) = (it.next(), it.next()) {
            push(&mut f, k, v);
        } else if !tok.is_empty() {
            // Smart auto-detection for bare tokens
            if is_likely_hash(tok) {
                f.hash.push(tok.to_lowercase());
            } else if is_likely_account(tok) {
                f.acct.push(tok.to_lowercase());
            } else {
                f.free.push(tok.to_lowercase());
            }
        }
    }
    f
}

/// Detect if token looks like a NEAR transaction hash
/// NEAR tx hashes are base58 encoded, typically 43-44 characters
fn is_likely_hash(tok: &str) -> bool {
    (tok.len() >= 43 && tok.len() <= 44) && tok.chars().all(|c| c.is_ascii_alphanumeric())
}

/// Detect if token looks like a NEAR account
fn is_likely_account(tok: &str) -> bool {
    tok.ends_with(".near") ||
    tok.ends_with(".testnet") ||
    // Implicit accounts (64-char hex)
    (tok.len() == 64 && tok.chars().all(|c| c.is_ascii_hexdigit()))
}

fn push(f: &mut CompiledFilter, k: &str, v: &str) {
    // Split comma-separated values (comma = OR logic)
    let values: Vec<String> = v
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    match &*k.to_lowercase() {
        "acct" | "account" => f.acct.extend(values),
        "signer" => f.signer.extend(values),
        "receiver" | "rcv" => f.receiver.extend(values),
        "action" => f.action.extend(values),
        "method" => f.method.extend(values),
        "raw" => f.raw.extend(values),
        "hash" | "tx" | "txn" | "transaction" => f.hash.extend(values),
        _ => f
            .free
            .extend(values.into_iter().map(|v| format!("{k}:{v}"))),
    }
}

pub fn tx_matches_filter(tx: &serde_json::Value, f: &CompiledFilter) -> bool {
    if is_empty(f) {
        return true;
    }

    let signer = tx
        .pointer("/signer_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let receiver = tx
        .pointer("/receiver_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let hash = tx
        .pointer("/hash")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let actions = tx
        .pointer("/actions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let action_types: Vec<String> = actions
        .iter()
        .map(|a| {
            a.as_object()
                .and_then(|o| o.keys().next().cloned())
                .unwrap_or_default()
                .to_lowercase()
        })
        .collect();
    let methods: Vec<String> = actions
        .iter()
        .filter_map(|a| {
            a.pointer("/FunctionCall/method_name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_lowercase())
        })
        .collect();

    let raw = tx.to_string().to_lowercase();

    let any = |vals: &[String], hay: &str| vals.is_empty() || vals.iter().any(|v| hay.contains(v));
    let any_in = |vals: &[String], arr: &[String]| {
        vals.is_empty() || vals.iter().any(|v| arr.iter().any(|x| x.contains(v)))
    };

    // acct matches signer OR receiver
    if !(any(&f.acct, &signer) || any(&f.acct, &receiver)) {
        return false;
    }
    if !any(&f.signer, &signer) {
        return false;
    }
    if !any(&f.receiver, &receiver) {
        return false;
    }
    if !any_in(&f.action, &action_types) {
        return false;
    }
    if !any_in(&f.method, &methods) {
        return false;
    }
    if !any(&f.raw, &raw) {
        return false;
    }
    // hash field check (exact match on full hash)
    if !any(&f.hash, &hash) {
        return false;
    }

    // free text matches signer/receiver/hash/methods
    if !f.free.is_empty() {
        let hay = [signer, receiver, hash, methods.join(" ")].join(" ");
        if !f.free.iter().any(|v| hay.contains(v)) {
            return false;
        }
    }
    true
}

// Optimized version that works directly with TxLite without JSON serialization
pub fn tx_lite_matches_filter(tx: &crate::types::TxLite, f: &CompiledFilter) -> bool {
    if is_empty(f) {
        return true;
    }

    let signer = tx.signer_id.as_deref().unwrap_or("").to_lowercase();
    let receiver = tx.receiver_id.as_deref().unwrap_or("").to_lowercase();
    let hash = tx.hash.to_lowercase();

    let action_types: Vec<String> = if let Some(actions) = &tx.actions {
        actions.iter().map(|a| {
            use crate::types::ActionSummary;
            match a {
                ActionSummary::CreateAccount => "createaccount",
                ActionSummary::DeployContract { .. } => "deploycontract",
                ActionSummary::FunctionCall { .. } => "functioncall",
                ActionSummary::Transfer { .. } => "transfer",
                ActionSummary::Stake { .. } => "stake",
                ActionSummary::AddKey { .. } => "addkey",
                ActionSummary::DeleteKey { .. } => "deletekey",
                ActionSummary::DeleteAccount { .. } => "deleteaccount",
                ActionSummary::Delegate { .. } => "delegate",
            }.to_string()
        }).collect()
    } else {
        vec![]
    };

    let methods: Vec<String> = if let Some(actions) = &tx.actions {
        actions.iter().filter_map(|a| {
            if let crate::types::ActionSummary::FunctionCall { method_name, .. } = a {
                Some(method_name.to_lowercase())
            } else {
                None
            }
        }).collect()
    } else {
        vec![]
    };

    let any = |vals: &[String], hay: &str| vals.is_empty() || vals.iter().any(|v| hay.contains(v));
    let any_in = |vals: &[String], arr: &[String]| {
        vals.is_empty() || vals.iter().any(|v| arr.iter().any(|x| x.contains(v)))
    };

    // acct matches signer OR receiver
    if !(any(&f.acct, &signer) || any(&f.acct, &receiver)) {
        return false;
    }
    if !any(&f.signer, &signer) {
        return false;
    }
    if !any(&f.receiver, &receiver) {
        return false;
    }
    if !any_in(&f.action, &action_types) {
        return false;
    }
    if !any_in(&f.method, &methods) {
        return false;
    }

    // For raw filter, only compute the expensive string if needed
    if !f.raw.is_empty() {
        // Only serialize to JSON if raw filter is actually being used
        let json_val = serde_json::to_value(tx).unwrap_or(serde_json::Value::Null);
        let raw = json_val.to_string().to_lowercase();
        if !any(&f.raw, &raw) {
            return false;
        }
    }

    // hash field check (exact match on full hash)
    if !any(&f.hash, &hash) {
        return false;
    }

    // free text matches signer/receiver/hash/methods
    if !f.free.is_empty() {
        let hay = [signer, receiver, hash, methods.join(" ")].join(" ");
        if !f.free.iter().any(|v| hay.contains(v)) {
            return false;
        }
    }
    true
}

pub fn is_empty(f: &CompiledFilter) -> bool {
    f.signer.is_empty()
        && f.receiver.is_empty()
        && f.acct.is_empty()
        && f.action.is_empty()
        && f.method.is_empty()
        && f.raw.is_empty()
        && f.hash.is_empty()
        && f.free.is_empty()
}
