use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="type")]
pub enum WsPayload {
    #[serde(rename="block")]
    Block { data: u64 },
    #[serde(rename="tx")]
    Tx { identifier: Option<String>, data: Option<TxSummary> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxSummary {
    pub hash: String,
    pub signer: Option<String>,
    pub receiver: Option<String>,
    pub actions: Vec<TxAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxAction {
    pub r#type: String,
    pub method: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BlockRow {
    pub height: u64,
    pub hash: String,
    pub timestamp: u64,
    pub tx_count: usize,
    pub when: String,
    pub transactions: Vec<TxLite>,
}

#[derive(Debug, Clone)]
pub struct TxLite {
    pub hash: String,
    // Optional detailed fields (populated when available)
    pub signer_id: Option<String>,
    pub receiver_id: Option<String>,
    pub actions: Option<Vec<ActionSummary>>,
    pub nonce: Option<u64>,
}

/// Rich transaction details parsed from near-primitives
#[derive(Debug, Clone)]
pub struct TxDetailed {
    pub hash: String,
    pub signer_id: String,
    pub receiver_id: String,
    pub actions: Vec<ActionSummary>,
    pub nonce: u64,
    #[allow(dead_code)]
    pub public_key: String,
    #[allow(dead_code)]
    pub raw_transaction: Option<Vec<u8>>,  // For debugging/export
}

#[derive(Debug, Clone)]
pub enum ActionSummary {
    CreateAccount,
    DeployContract { code_len: usize },
    FunctionCall { method_name: String, _args_base64: String, args_decoded: crate::near_args::DecodedArgs, gas: u64, deposit: u128 },
    Transfer { deposit: u128 },
    Stake { stake: u128, public_key: String },
    AddKey { public_key: String, access_key: String },
    DeleteKey { public_key: String },
    DeleteAccount { beneficiary_id: String },
    Delegate { sender_id: String, receiver_id: String, actions: Vec<ActionSummary> },
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    FromWs(WsPayload),
    NewBlock(BlockRow),
    Quit,
}
