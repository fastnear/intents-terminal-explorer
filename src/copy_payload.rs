use serde_json::{json, Value};
use crate::types::{BlockRow, TxLite, ActionSummary};
use crate::util_text::{format_gas, format_near};

pub fn block_json(block: &BlockRow, txs_override: Option<&[TxLite]>) -> Value {
    let txs: Vec<&TxLite> = if let Some(slice) = txs_override {
        slice.iter().collect()
    } else {
        block.transactions.iter().collect()
    };
    let transactions: Vec<Value> = txs.into_iter()
        .map(|tx| tx_summary_json(Some(block), tx))
        .collect();

    json!({
        "block": {
            "height": block.height,
            "hash": block.hash,
            "timestamp": block.timestamp,
            "when": block.when,
            "tx_count": block.tx_count,
            "transactions": transactions
        }
    })
}

pub fn tx_summary_json(block: Option<&BlockRow>, tx: &TxLite) -> Value {
    let mut v = json!({ "hash": tx.hash });
    if let Some(b) = block {
        v["block_height"] = json!(b.height);
        v["block_hash"]  = json!(b.hash.clone());
        v["timestamp"]   = json!(b.timestamp);
        v["when"]        = json!(b.when.clone());
    }
    if let Some(ref signer)   = tx.signer_id   { v["signer"]   = json!(signer); }
    if let Some(ref receiver) = tx.receiver_id { v["receiver"] = json!(receiver); }
    if let Some(nonce) = tx.nonce { v["nonce"] = json!(nonce); }

    if let Some(ref actions) = tx.actions {
        let arr: Vec<Value> = actions.iter().map(|a| {
            let mut obj = json!({
                "type": action_type(a),
                "description": action_description(a),
            });
            if let Some(extra) = action_extra(a) {
                if let Value::Object(map) = &mut obj {
                    for (k, vv) in extra.as_object().unwrap().iter() {
                        map.insert(k.clone(), vv.clone());
                    }
                }
            }
            obj
        }).collect();
        v["actions"] = json!(arr);
    }
    v
}

fn action_type(a: &ActionSummary) -> &'static str {
    use ActionSummary::*;
    match a {
        CreateAccount => "CreateAccount",
        DeployContract { .. } => "DeployContract",
        FunctionCall   { .. } => "FunctionCall",
        Transfer       { .. } => "Transfer",
        Stake          { .. } => "Stake",
        AddKey         { .. } => "AddKey",
        DeleteKey      { .. } => "DeleteKey",
        DeleteAccount  { .. } => "DeleteAccount",
        Delegate       { .. } => "Delegate",
    }
}

fn action_description(a: &ActionSummary) -> String {
    use ActionSummary::*;
    match a {
        CreateAccount => "CreateAccount".into(),
        DeployContract { code_len } =>
            format!("DeployContract ({} bytes)", code_len),
        FunctionCall { method_name, gas, deposit, .. } =>
            format!("FunctionCall: {}() [gas: {}, deposit: {}]", method_name, format_gas(*gas), format_near(*deposit)),
        Transfer { deposit } =>
            format!("Transfer: {}", format_near(*deposit)),
        Stake { stake, public_key } =>
            format!("Stake: {} ({})", format_near(*stake), public_key),
        AddKey { public_key, .. } =>
            format!("AddKey: {}", public_key),
        DeleteKey { public_key } =>
            format!("DeleteKey: {}", public_key),
        DeleteAccount { beneficiary_id } =>
            format!("DeleteAccount → {}", beneficiary_id),
        Delegate { sender_id, receiver_id, actions } =>
            format!("Delegate: {} → {} ({} actions)", sender_id, receiver_id, actions.len()),
    }
}

fn action_extra(a: &ActionSummary) -> Option<Value> {
    use crate::near_args::DecodedArgs;
    use ActionSummary::*;
    match a {
        FunctionCall { method_name, args_decoded, gas, deposit, .. } => {
            let args_display = match args_decoded {
                DecodedArgs::Json(v)        => crate::json_auto_parse::auto_parse_nested_json(v.clone(), 5, 0),
                DecodedArgs::Text(t)        => json!(t),
                DecodedArgs::Bytes { preview, .. } => json!(format!("[binary: {}]", preview)),
                DecodedArgs::Empty          => json!({}),
                DecodedArgs::Error(e)       => json!(format!("<decode error: {}>", e)),
            };
            Some(json!({
                "method": method_name,
                "args": args_display,
                "gas": format_gas(*gas),
                "deposit": format_near(*deposit)
            }))
        }
        Transfer { deposit } => Some(json!({ "amount": format_near(*deposit) })),
        Stake { stake, public_key } => Some(json!({ "amount": format_near(*stake), "public_key": public_key })),
        AddKey { public_key, access_key } => {
            let parsed = serde_json::from_str::<Value>(access_key)
                .map(|v| crate::json_auto_parse::auto_parse_nested_json(v, 5, 0))
                .unwrap_or_else(|_| json!(access_key));
            Some(json!({ "public_key": public_key, "access_key": parsed }))
        }
        DeleteKey { public_key } => Some(json!({ "public_key": public_key })),
        DeleteAccount { beneficiary_id } => Some(json!({ "beneficiary": beneficiary_id })),
        DeployContract { code_len } => Some(json!({ "code_size": format!("{} bytes", code_len) })),
        Delegate { sender_id, receiver_id, actions } => {
            let nested: Vec<Value> = actions.iter().map(|a| {
                let mut obj = json!({ "type": action_type(a), "description": action_description(a) });
                if let Some(extra) = action_extra(a) {
                    if let Value::Object(map) = &mut obj {
                        for (k, vv) in extra.as_object().unwrap().iter() {
                            map.insert(k.clone(), vv.clone());
                        }
                    }
                }
                obj
            }).collect();
            Some(json!({ "sender": sender_id, "receiver": receiver_id, "actions": nested }))
        }
        CreateAccount => None,
    }
}