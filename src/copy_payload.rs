//! Copy payload construction for clipboard operations.
//!
//! This module builds JSON payloads for different panes based on blockchain data.
//! It's used by `copy_api` to provide consistent copy behavior across all targets.
//!
//! ## Payload Formats
//!
//! **Block Summary** (Pane 0):
//! ```json
//! {
//!   "network": "mainnet",
//!   "block_height": 12345,
//!   "block_hash": "...",
//!   "timestamp": 1234567890,
//!   "tx_count": 5,
//!   "txs": [...]
//! }
//! ```
//!
//! **Transaction Summary** (Pane 1 - Dual Format):
//! ```json
//! {
//!   "network": "mainnet",
//!   "block_height": 12345,
//!   "block_timestamp": 1234567890,
//!   "tx_hash": "...",
//!   "chain": {...},    // Raw transaction data
//!   "human": {...}     // Human-readable formatted data
//! }
//! ```

use crate::types::{ActionSummary, BlockRow, TxLite};
use serde_json::{json, Value};

/// Build a JSON value representing a block summary with all transactions.
///
/// Used when copying from the Blocks pane (pane 0).
pub fn block_summary_json(block: &BlockRow, transactions: &[TxLite]) -> Value {
    json!({
        "network": "mainnet",  // TODO: Make configurable
        "block_height": block.height,
        "block_hash": block.hash,
        "timestamp": block.timestamp,
        "tx_count": transactions.len(),
        "txs": transactions
    })
}

/// Build a JSON value representing a transaction summary (dual format: chain + human).
///
/// Used when copying from the Transactions pane (pane 1).
pub fn tx_summary_json(block: &BlockRow, tx: &TxLite) -> Value {
    // Build human-readable view
    let mut human = json!({
        "hash": tx.hash
    });

    if let Some(ref signer) = tx.signer_id {
        human["signer"] = json!(signer);
    }
    if let Some(ref receiver) = tx.receiver_id {
        human["receiver"] = json!(receiver);
    }
    if let Some(nonce) = tx.nonce {
        human["nonce"] = json!(nonce);
    }
    if let Some(ref actions) = tx.actions {
        let formatted_actions: Vec<_> = actions.iter().map(format_action).collect();
        human["actions"] = json!(formatted_actions);
    }

    // Dual format: chain (raw) + human (processed)
    json!({
        "network": "mainnet",  // TODO: Make configurable
        "block_height": block.height,
        "block_timestamp": block.timestamp,
        "tx_hash": tx.hash,
        "chain": tx,      // Raw chain data
        "human": human    // Human-readable formatted data
    })
}

/// Recursively format an action for human-readable display.
///
/// This is the exact same formatter from app.rs for consistency.
pub fn format_action(action: &ActionSummary) -> Value {
    use crate::near_args::DecodedArgs;
    use crate::util_text::{format_gas, format_near};

    match action {
        ActionSummary::CreateAccount => json!({"type": "CreateAccount"}),
        ActionSummary::DeployContract { code_len } => {
            json!({"type": "DeployContract", "code_size": format!("{} bytes", code_len)})
        }
        ActionSummary::FunctionCall {
            method_name,
            args_decoded,
            gas,
            deposit,
            ..
        } => {
            let args_display = match args_decoded {
                DecodedArgs::Json(v) => {
                    // Auto-parse nested JSON-serialized strings for better readability
                    crate::json_auto_parse::auto_parse_nested_json(v.clone(), 5, 0)
                }
                DecodedArgs::Text(t) => json!(t),
                DecodedArgs::Bytes { preview, .. } => json!(format!("[binary: {}]", preview)),
                DecodedArgs::Empty => json!({}),
                DecodedArgs::Error(e) => json!(format!("<decode error: {}>", e)),
            };

            json!({
                "type": "FunctionCall",
                "method": method_name,
                "args": args_display,
                "gas": format_gas(*gas),
                "deposit": format_near(*deposit),
            })
        }
        ActionSummary::Transfer { deposit } => {
            json!({"type": "Transfer", "amount": format_near(*deposit)})
        }
        ActionSummary::Stake { stake, public_key } => {
            json!({"type": "Stake", "amount": format_near(*stake), "public_key": public_key})
        }
        ActionSummary::AddKey {
            public_key,
            access_key,
        } => {
            // Parse access_key if it's stringified JSON (same pattern as FunctionCall args)
            let parsed_access_key = if let Ok(json_val) = serde_json::from_str::<Value>(access_key)
            {
                crate::json_auto_parse::auto_parse_nested_json(json_val, 5, 0)
            } else {
                json!(access_key) // Fallback to string if not valid JSON
            };
            json!({"type": "AddKey", "public_key": public_key, "access_key": parsed_access_key})
        }
        ActionSummary::DeleteKey { public_key } => {
            json!({"type": "DeleteKey", "public_key": public_key})
        }
        ActionSummary::DeleteAccount { beneficiary_id } => {
            json!({"type": "DeleteAccount", "beneficiary": beneficiary_id})
        }
        ActionSummary::Delegate {
            sender_id,
            receiver_id,
            actions,
        } => {
            // Recursively format nested actions
            let nested_formatted: Vec<Value> = actions.iter().map(format_action).collect();
            json!({
                "type": "Delegate",
                "sender": sender_id,
                "receiver": receiver_id,
                "actions": nested_formatted
            })
        }
    }
}
