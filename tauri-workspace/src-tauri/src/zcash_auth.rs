//! Authentication module for Touch ID and PIN fallback
//! Provides biometric authentication on macOS using LocalAuthentication framework
//!
//! Uses the idiomatic localauthentication-rs crate for Touch ID integration

use std::process::Command;

#[derive(Debug, Clone)]
pub struct AuthResult {
    pub approved: bool,
    pub method: String, // "touchid", "pin", or "denied"
}

/// Attempt Touch ID authentication on macOS
/// Returns Ok(true) if fingerprint verified, Ok(false) if failed/canceled
///
/// Uses the localauthentication-rs crate for idiomatic Rust access to macOS LocalAuthentication
#[cfg(target_os = "macos")]
pub fn try_touch_id(reason: &str) -> Result<bool, String> {
    log::info!("🔐 [Auth] Attempting Touch ID authentication...");
    log::info!("🔐 [Auth] Reason: {}", reason);

    use localauthentication_rs::{LAPolicy, LocalAuthentication};

    let local_auth = LocalAuthentication::new();

    // Use DeviceOwnerAuthenticationWithBiometrics policy
    // This requires Touch ID/Face ID (no passcode fallback at this level)
    let authenticated = local_auth.evaluate_policy(
        LAPolicy::DeviceOwnerAuthenticationWithBiometrics,
        reason,
    );

    if authenticated {
        log::info!("🔐 [Auth] ✅ Touch ID authentication successful");
        Ok(true)
    } else {
        log::info!("🔐 [Auth] ❌ Touch ID authentication failed or canceled");
        Ok(false)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn try_touch_id(_reason: &str) -> Result<bool, String> {
    log::warn!("🔐 [Auth] Touch ID not available on this platform");
    Ok(false)
}

/// Fallback PIN authentication via dialog
/// For demo purposes, accepts a simple PIN check
/// In production, this would verify against a stored hash
pub fn ask_for_pin(reason: &str) -> bool {
    log::info!("🔑 [Auth] Requesting PIN authentication...");
    log::info!("🔑 [Auth] Reason: {}", reason);

    #[cfg(target_os = "macos")]
    {
        // Use AppleScript to show PIN dialog
        let script = format!(
            r#"
            display dialog "{}\n\nEnter PIN (demo: use '1234'):" default answer "" with hidden answer
            set pin to text returned of result
            return pin
            "#,
            reason.replace("\"", "\\\"")
        );

        if let Ok(output) = Command::new("osascript").arg("-e").arg(&script).output() {
            let pin = String::from_utf8_lossy(&output.stdout).trim().to_string();
            log::info!("🔑 [Auth] PIN entered (length: {})", pin.len());

            // Demo: accept "1234" as valid PIN
            // In production, this would hash and compare against stored PIN
            let valid = pin == "1234";

            if valid {
                log::info!("🔑 [Auth] ✅ PIN valid");
            } else {
                log::warn!("🔑 [Auth] ❌ PIN invalid");
            }

            return valid;
        }
    }

    log::error!("🔑 [Auth] Failed to show PIN dialog");
    false
}

/// Main authentication flow: Try Touch ID first, fall back to PIN
pub fn authenticate(reason: &str) -> AuthResult {
    log::info!("🔐 [Auth] Starting authentication flow...");
    log::info!("🔐 [Auth] Reason: {}", reason);

    // Try Touch ID first
    match try_touch_id(reason) {
        Ok(true) => {
            log::info!("🔐 [Auth] ✅ Authentication successful via Touch ID");
            return AuthResult {
                approved: true,
                method: "touchid".to_string(),
            };
        }
        Ok(false) => {
            log::info!("🔐 [Auth] Touch ID failed/canceled, falling back to PIN");
        }
        Err(e) => {
            log::error!("🔐 [Auth] Touch ID error: {}, falling back to PIN", e);
        }
    }

    // Fall back to PIN
    if ask_for_pin(reason) {
        log::info!("🔐 [Auth] ✅ Authentication successful via PIN");
        AuthResult {
            approved: true,
            method: "pin".to_string(),
        }
    } else {
        log::warn!("🔐 [Auth] ❌ Authentication denied (PIN failed)");
        AuthResult {
            approved: false,
            method: "denied".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_result() {
        let result = AuthResult {
            approved: true,
            method: "touchid".to_string(),
        };
        assert!(result.approved);
        assert_eq!(result.method, "touchid");
    }

    #[test]
    fn test_auth_result_denied() {
        let result = AuthResult {
            approved: false,
            method: "denied".to_string(),
        };
        assert!(!result.approved);
        assert_eq!(result.method, "denied");
    }
}
