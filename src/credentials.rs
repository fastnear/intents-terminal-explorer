//! Credentials watcher for owned account filtering
//!
//! This module is only available on native targets (file system access not available in WASM).

use anyhow::Result;
use notify::{Error as NotifyError, Event, EventKind, RecursiveMode, Watcher};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc::UnboundedSender;

/// Start watching credentials directory for NEAR account files
/// Scans initially and watches for changes, emitting account_id set via channel
pub async fn start_credentials_watcher(
    base_dir: PathBuf,
    network: String,
    tx: UnboundedSender<HashSet<String>>,
) -> Result<()> {
    let creds_path = base_dir.join(network.to_lowercase());

    // Create directory if it doesn't exist
    tokio::fs::create_dir_all(&creds_path).await?;

    // Initial scan
    let accounts = scan_directory(&creds_path).await?;
    let _ = tx.send(accounts.clone());

    // Start watching in background
    tokio::spawn(async move {
        let _ = watch_directory(creds_path, tx).await;
    });

    Ok(())
}

/// Scan directory for all credential files and extract account IDs
async fn scan_directory(path: &Path) -> Result<HashSet<String>> {
    let mut accounts = HashSet::new();

    if !path.exists() {
        return Ok(accounts);
    }

    let mut entries = tokio::fs::read_dir(path).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        if path.is_file() {
            if let Some(account) = parse_account_file(&path).await {
                accounts.insert(account.to_lowercase());
            }
        }
    }

    Ok(accounts)
}

/// Parse account_id from a NEAR credentials JSON file
/// Format: {"account_id": "alice.near", "private_key": "ed25519:...", ...}
/// Fallback: use filename without .json extension
async fn parse_account_file(path: &Path) -> Option<String> {
    // Only process .json files
    if path.extension()?.to_str()? != "json" {
        return None;
    }

    // Try to parse JSON and extract account_id
    if let Ok(content) = tokio::fs::read_to_string(path).await {
        if let Ok(json) = serde_json::from_str::<Value>(&content) {
            if let Some(account_id) = json["account_id"].as_str() {
                if !account_id.is_empty() {
                    return Some(account_id.to_string());
                }
            }
        }
    }

    // Fallback: use filename without extension
    // e.g., "alice.near.json" -> "alice.near"
    let filename = path.file_stem()?.to_str()?;

    // Only use filename if it looks like an account (contains dot, not a key)
    if filename.contains('.') && !filename.starts_with("ed25519:") {
        return Some(filename.to_string());
    }

    None
}

/// Watch directory for changes and rescan on modifications
async fn watch_directory(path: PathBuf, tx: UnboundedSender<HashSet<String>>) -> Result<()> {
    let (notify_tx, mut notify_rx) = tokio::sync::mpsc::unbounded_channel();

    // Create watcher
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, NotifyError>| {
        if let Ok(event) = res {
            let _ = notify_tx.send(event);
        }
    })?;

    // Watch directory (non-recursive)
    watcher.watch(&path, RecursiveMode::NonRecursive)?;

    // Process events
    while let Some(event) = notify_rx.recv().await {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                // Debounce: wait a bit for file writes to complete
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // Rescan directory
                if let Ok(accounts) = scan_directory(&path).await {
                    let _ = tx.send(accounts);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
