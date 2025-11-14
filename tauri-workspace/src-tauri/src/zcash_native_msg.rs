//! Native messaging handler for Chrome extension communication
//! Reads JSON messages from STDIN and writes responses to STDOUT
//! following the Chrome Native Messaging protocol

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Debug, Deserialize)]
pub struct NativeRequest {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub session: String,
}

#[derive(Debug, Serialize)]
pub struct NativeResponse {
    pub status: String, // "approved", "denied", or "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub txid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub session: String,
}

#[derive(Debug, Clone)]
pub struct TransactionRequest {
    pub to: String,
    pub amount: f64,
    pub memo: String,
    pub session: String,
}

pub struct NativeMessagingHandler {
    tx: Sender<NativeRequest>,
    rx: Receiver<NativeRequest>,
}

impl NativeMessagingHandler {
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self { tx, rx }
    }

    /// Start listening on STDIN for native messaging requests
    /// This runs in a background thread
    pub fn start_listener(&self) -> Sender<NativeRequest> {
        let tx = self.tx.clone();

        thread::spawn(move || {
            log::info!("🔵 [Native Messaging] Starting STDIN listener thread");
            let stdin = io::stdin();
            let reader = stdin.lock();

            for line in reader.lines() {
                match line {
                    Ok(input) => {
                        log::info!("🔵 [Native Messaging] Received input: {}", input);

                        // Try to parse as JSON
                        match serde_json::from_str::<NativeRequest>(&input) {
                            Ok(request) => {
                                log::info!("🔵 [Native Messaging] Parsed request: {:?}", request);
                                if let Err(e) = tx.send(request) {
                                    log::error!("🔴 [Native Messaging] Failed to send request to main thread: {}", e);
                                }
                            }
                            Err(e) => {
                                log::error!("🔴 [Native Messaging] Failed to parse JSON: {}", e);
                                log::error!("🔴 [Native Messaging] Raw input: {}", input);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("🔴 [Native Messaging] Error reading from STDIN: {}", e);
                        break;
                    }
                }
            }

            log::info!("🔵 [Native Messaging] STDIN listener thread exiting");
        });

        self.tx.clone()
    }

    /// Get the receiver to process incoming requests
    pub fn receiver(&self) -> &Receiver<NativeRequest> {
        &self.rx
    }

    /// Send a response back to the extension via STDOUT
    /// Follows Chrome Native Messaging protocol: 4-byte length prefix + JSON
    pub fn send_response(response: &NativeResponse) -> io::Result<()> {
        log::info!("🟢 [Native Messaging] Sending response: {:?}", response);

        let json = serde_json::to_vec(response)?;
        let len = (json.len() as u32).to_le_bytes();

        let mut stdout = io::stdout();
        stdout.write_all(&len)?;
        stdout.write_all(&json)?;
        stdout.flush()?;

        log::info!("🟢 [Native Messaging] Response sent successfully");
        Ok(())
    }

    /// Parse transaction details from request params
    pub fn parse_transaction(params: &serde_json::Value, session: String) -> Result<TransactionRequest, String> {
        let to = params
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'to' field")?
            .to_string();

        let amount = params
            .get("amount")
            .and_then(|v| v.as_f64())
            .ok_or("Missing or invalid 'amount' field")?;

        let memo = params
            .get("memo")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(TransactionRequest {
            to,
            amount,
            memo,
            session,
        })
    }
}

impl Default for NativeMessagingHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_transaction() {
        let params = serde_json::json!({
            "to": "zs1test123",
            "amount": 1.5,
            "memo": "Coffee"
        });

        let result = NativeMessagingHandler::parse_transaction(&params, "session123".to_string());
        assert!(result.is_ok());

        let tx = result.unwrap();
        assert_eq!(tx.to, "zs1test123");
        assert_eq!(tx.amount, 1.5);
        assert_eq!(tx.memo, "Coffee");
        assert_eq!(tx.session, "session123");
    }

    #[test]
    fn test_parse_transaction_missing_fields() {
        let params = serde_json::json!({"amount": 1.5});
        let result = NativeMessagingHandler::parse_transaction(&params, "session123".to_string());
        assert!(result.is_err());
    }
}
