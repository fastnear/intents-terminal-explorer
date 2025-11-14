//! Zcash transaction signing module
//! For demo purposes, this creates a simulated transaction
//! In production, this would use a real Zcash SDK library

use crate::zcash_native_msg::TransactionRequest;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub struct SignedTransaction {
    pub txid: String,
    pub raw_tx: String,
    pub to: String,
    pub amount: f64,
    pub memo: String,
}

/// Sign a Zcash transaction (DEMO/STUB implementation)
/// In production, this would:
/// 1. Load private key from secure storage
/// 2. Construct a real Zcash shielded transaction
/// 3. Sign with the private key
/// 4. Return the signed transaction blob
pub fn sign_transaction(request: &TransactionRequest) -> Result<SignedTransaction, String> {
    log::info!("✍️  [Zcash Signer] Signing transaction...");
    log::info!("✍️  [Zcash Signer] To: {}", request.to);
    log::info!("✍️  [Zcash Signer] Amount: {} ZEC", request.amount);
    log::info!("✍️  [Zcash Signer] Memo: {}", request.memo);

    // Validate inputs
    if request.to.is_empty() {
        return Err("Invalid recipient address (empty)".to_string());
    }

    if request.amount <= 0.0 {
        return Err("Invalid amount (must be > 0)".to_string());
    }

    // Check if address looks like a Zcash address (basic validation)
    if !is_valid_zcash_address(&request.to) {
        log::warn!("✍️  [Zcash Signer] Warning: Address doesn't look like a valid Zcash address");
    }

    // DEMO: Generate a fake transaction ID based on request details
    // In production, this would be the actual txid from the signed transaction
    let txid = generate_demo_txid(request);

    // DEMO: Generate a fake raw transaction hex
    // In production, this would be the actual signed transaction bytes
    let raw_tx = generate_demo_raw_tx(request);

    log::info!("✍️  [Zcash Signer] ✅ Transaction signed successfully");
    log::info!("✍️  [Zcash Signer] TX ID: {}", txid);

    Ok(SignedTransaction {
        txid,
        raw_tx,
        to: request.to.clone(),
        amount: request.amount,
        memo: request.memo.clone(),
    })
}

/// Basic Zcash address validation (format check only)
fn is_valid_zcash_address(addr: &str) -> bool {
    // Zcash addresses:
    // - Transparent: starts with 't1' or 't3'
    // - Shielded Sprout: starts with 'zc'
    // - Shielded Sapling: starts with 'zs'
    // - Unified: starts with 'u'

    addr.starts_with("t1")
        || addr.starts_with("t3")
        || addr.starts_with("zc")
        || addr.starts_with("zs")
        || addr.starts_with('u')
}

/// Generate a demo transaction ID (hash of request details)
fn generate_demo_txid(request: &TransactionRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(request.to.as_bytes());
    hasher.update(request.amount.to_string().as_bytes());
    hasher.update(request.memo.as_bytes());
    hasher.update(request.session.as_bytes());

    let result = hasher.finalize();
    format!("{:x}", result)
}

/// Generate a demo raw transaction (would be actual signed bytes in production)
fn generate_demo_raw_tx(request: &TransactionRequest) -> String {
    format!(
        "DEMO_RAW_TX:to={},amount={},memo={},session={}",
        request.to, request.amount, request.memo, request.session
    )
}

/// Broadcast transaction to network (DEMO/STUB)
/// In production, this would submit to Zcash RPC node
pub fn broadcast_transaction(signed_tx: &SignedTransaction) -> Result<String, String> {
    log::info!("📡 [Zcash Signer] Broadcasting transaction...");
    log::info!("📡 [Zcash Signer] TX ID: {}", signed_tx.txid);

    // DEMO: Just log and return success
    // In production, this would call zcashd or lightwalletd RPC
    log::info!("📡 [Zcash Signer] ✅ Transaction broadcast successful (DEMO)");

    Ok(signed_tx.txid.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign_transaction() {
        let request = TransactionRequest {
            to: "zs1test123".to_string(),
            amount: 1.5,
            memo: "Coffee".to_string(),
            session: "session123".to_string(),
        };

        let result = sign_transaction(&request);
        assert!(result.is_ok());

        let tx = result.unwrap();
        assert!(!tx.txid.is_empty());
        assert_eq!(tx.to, "zs1test123");
        assert_eq!(tx.amount, 1.5);
    }

    #[test]
    fn test_invalid_amount() {
        let request = TransactionRequest {
            to: "zs1test123".to_string(),
            amount: 0.0,
            memo: "Coffee".to_string(),
            session: "session123".to_string(),
        };

        let result = sign_transaction(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_address_validation() {
        assert!(is_valid_zcash_address("zs1test123"));
        assert!(is_valid_zcash_address("t1test123"));
        assert!(is_valid_zcash_address("t3test123"));
        assert!(is_valid_zcash_address("u1test123"));
        assert!(!is_valid_zcash_address("invalid"));
    }

    #[test]
    fn test_txid_deterministic() {
        let request = TransactionRequest {
            to: "zs1test123".to_string(),
            amount: 1.5,
            memo: "Coffee".to_string(),
            session: "session123".to_string(),
        };

        let txid1 = generate_demo_txid(&request);
        let txid2 = generate_demo_txid(&request);

        // Same input should produce same txid
        assert_eq!(txid1, txid2);
    }
}
