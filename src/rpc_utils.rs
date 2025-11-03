use anyhow::{Result, anyhow};
use serde_json::{json, Value};
use std::sync::OnceLock;
use crate::types::{BlockRow, TxLite, TxDetailed, ActionSummary};

// Platform-specific imports
#[cfg(not(target_arch = "wasm32"))]
use tokio::task::JoinSet;

use crate::platform::{Duration, Instant, sleep};

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP.get_or_init(|| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            log::debug!("🌐 Initializing reqwest client (native)...");
            reqwest::Client::builder()
                .pool_max_idle_per_host(8)
                .tcp_nodelay(true)
                .build()
                .expect("reqwest client")
        }

        #[cfg(target_arch = "wasm32")]
        {
            log::info!("╔═══════════════════════════════════════════════════════════╗");
            log::info!("║  🌐 Initializing reqwest WASM HTTP client               ║");
            log::info!("╚═══════════════════════════════════════════════════════════╝");
            match reqwest::Client::builder().build() {
                Ok(client) => {
                    log::info!("  ✅ HTTP client created successfully!");
                    client
                }
                Err(e) => {
                    log::error!("  ❌ Failed to create HTTP client: {}", e);
                    panic!("Failed to create reqwest WASM client: {}", e);
                }
            }
        }
    })
}

pub async fn rpc_post(url:&str, body:&Value, timeout_ms:u64, auth_token: Option<&str>) -> Result<Value> {
    #[cfg(target_arch = "wasm32")]
    log::info!("═══════════════════════════════════════");

    #[cfg(not(target_arch = "wasm32"))]
    log::debug!("═══════════════════════════════════════");

    log::debug!("📡 RPC POST to: {}", url);
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("unknown");
    log::debug!("📋 Method: {}", method);
    log::debug!("⏱️  Timeout: {}ms", timeout_ms);

    // Small, bounded retry on transient HTTP failures
    let mut attempt = 0u32;
    loop {
        #[cfg(target_arch = "wasm32")]
        log::info!("🔧 Getting HTTP client...");

        let client = http_client();

        #[cfg(target_arch = "wasm32")]
        log::info!("🔧 Building POST request...");

        let mut req = client
            .post(url)
            .json(body)
            .timeout(Duration::from_millis(timeout_ms));

        if let Some(token) = auth_token {
            let token_preview = if token.len() > 8 {
                format!("{}...{}",
                    &token.chars().take(4).collect::<String>(),
                    &token.chars().skip(token.len().saturating_sub(4)).collect::<String>())
            } else {
                format!("{}...", &token.chars().take(4).collect::<String>())
            };
            log::debug!("🔑 Auth: Bearer {} ({} chars total)", token_preview, token.len());
            req = req.header("Authorization", format!("Bearer {token}"));
        } else {
            log::warn!("⚠️  NO AUTH TOKEN - May hit rate limits (HTTP 429)!");
        }

        log::debug!("🚀 Sending HTTP request (attempt {})...", attempt + 1);
        let start = Instant::now();

        let res = req.send().await?;
        let elapsed = start.elapsed();

        log::debug!("📨 Response: {} ({:.2}ms)", res.status(), elapsed.as_secs_f64() * 1000.0);

        if res.status().is_success() {
            let v: Value = res.json().await?;

            if let Some(err) = v.get("error") {
                log::error!("❌ RPC error response:");
                log::error!("   {}", serde_json::to_string_pretty(err).unwrap_or_default());
                let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or_default();
                let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("rpc error");
                log::debug!("═══════════════════════════════════════");
                return Err(anyhow!("rpc {code} {msg}"));
            }

            if let Some(r) = v.get("result") {
                // Log result summary based on method
                match method {
                    "block" => {
                        let height = r.get("header").and_then(|h| h.get("height")).and_then(|h| h.as_u64());
                        let chunks = r.get("chunks").and_then(|c| c.as_array()).map(|a| a.len());
                        log::debug!("✅ Block: height={:?}, chunks={:?}", height, chunks);
                    },
                    "chunk" => {
                        let txs = r.get("transactions").and_then(|t| t.as_array()).map(|a| a.len());
                        log::debug!("✅ Chunk: transactions={:?}", txs);
                    },
                    _ => {
                        log::debug!("✅ RPC success!");
                    }
                }
                log::debug!("═══════════════════════════════════════");
                return Ok(r.clone());
            }

            log::error!("❌ Invalid RPC response - no 'result' field");
            log::error!("   Full response: {}", serde_json::to_string(&v).unwrap_or_default());
            log::debug!("═══════════════════════════════════════");
            return Err(anyhow!("invalid rpc payload (no result)"));
        } else {
            log::warn!("⚠️  HTTP {} {}", res.status(), res.status().canonical_reason().unwrap_or(""));

            // Retry only on transient statuses
            if matches!(res.status().as_u16(), 429 | 500 | 502 | 503 | 504) && attempt < 2 {
                attempt += 1;
                log::info!("🔄 Retrying in {}ms... (attempt {}/3)", 150 * attempt, attempt + 1);
                sleep(Duration::from_millis(150 * attempt as u64)).await;
                continue;
            }

            log::error!("❌ Giving up after {} attempts", attempt + 1);
            log::debug!("═══════════════════════════════════════");
            return Err(anyhow!("http {}", res.status()));
        }
    }
}

pub async fn get_latest_block(url:&str, t:u64, auth_token: Option<&str>) -> Result<Value> {
    rpc_post(url, &json!({"jsonrpc":"2.0","id":"ratacat","method":"block","params":{"finality":"final"}}), t, auth_token).await
}

pub async fn get_block_by_height(url:&str, h:u64, t:u64, auth_token: Option<&str>) -> Result<Value> {
    rpc_post(url, &json!({"jsonrpc":"2.0","id":"ratacat","method":"block","params":{"block_id":h}}), t, auth_token).await
}

pub async fn get_chunk(url:&str, hash:&str, t:u64, auth_token: Option<&str>) -> Result<Value> {
    rpc_post(url, &json!({"jsonrpc":"2.0","id":"ratacat","method":"chunk","params":{"chunk_id":hash}}), t, auth_token).await
}

/// Extract transactions from a chunk JSON response
fn extract_transactions_from_chunk(chunk: &Value, txs: &mut Vec<TxLite>) {
    if let Some(arr) = chunk["transactions"].as_array() {
        for t in arr {
            // Try to parse full transaction details
            if let Some(detailed) = parse_transaction_detailed(t) {
                txs.push(TxLite {
                    hash: detailed.hash,
                    signer_id: Some(detailed.signer_id),
                    receiver_id: Some(detailed.receiver_id),
                    actions: Some(detailed.actions),
                    nonce: Some(detailed.nonce),
                });
            } else if let Some(hh) = t["hash"].as_str() {
                // Fallback to just hash if parsing fails
                txs.push(TxLite {
                    hash: hh.to_string(),
                    signer_id: None,
                    receiver_id: None,
                    actions: None,
                    nonce: None,
                });
            }
        }
    }
}

pub async fn fetch_block_with_txs(
    url: &str,
    height: u64,
    timeout_ms: u64,
    #[allow(unused_variables)] chunk_concurrency: usize,
    auth_token: Option<&str>
) -> Result<BlockRow> {
    log::debug!("🏗️  fetch_block_with_txs: Starting for block #{}", height);

    let b = get_block_by_height(url, height, timeout_ms, auth_token).await?;

    let chunks = b["chunks"].as_array().cloned().unwrap_or_default();
    log::debug!("📦 Block #{} has {} chunks to fetch", height, chunks.len());

    let mut txs = Vec::<TxLite>::new();

    // Native: Use JoinSet for concurrent chunk fetching
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut set = JoinSet::new();
        for c in chunks.iter() {
            if let Some(hash) = c["chunk_hash"].as_str() {
                let url = url.to_string();
                let hash = hash.to_string();
                let t = timeout_ms;
                let token = auth_token.map(|s| s.to_string());
                set.spawn(async move { get_chunk(&url, &hash, t, token.as_deref()).await });
                if set.len() >= chunk_concurrency.max(1) {
                    let _ = set.join_next().await;
                }
            }
        }

        while let Some(res) = set.join_next().await {
            if let Ok(Ok(chunk)) = res {
                extract_transactions_from_chunk(&chunk, &mut txs);
            }
        }
    }

    // WASM: Concurrent chunk fetching using buffer_unordered (bounded pool)
    #[cfg(target_arch = "wasm32")]
    {
        use futures::stream::{self, StreamExt};

        let start = Instant::now();
        let max_concurrent = chunk_concurrency.max(1).min(6); // Cap at 6 (browser per-origin limit)

        log::debug!("🚀 WASM: Fetching {} chunks with concurrency={}", chunks.len(), max_concurrent);

        // Build list of chunk hashes to fetch
        let chunk_hashes: Vec<String> = chunks.iter()
            .filter_map(|c| c["chunk_hash"].as_str().map(|s| s.to_string()))
            .collect();

        // Create stream of futures
        let url_s = url.to_string();
        let auth_s = auth_token.map(|s| s.to_string());

        let chunk_futures = stream::iter(chunk_hashes).map(move |hash| {
            let url = url_s.clone();
            let auth = auth_s.clone();
            async move {
                get_chunk(&url, &hash, timeout_ms, auth.as_deref()).await
            }
        });

        // Execute with bounded concurrency
        let mut results = chunk_futures.buffer_unordered(max_concurrent);

        while let Some(result) = results.next().await {
            match result {
                Ok(chunk) => extract_transactions_from_chunk(&chunk, &mut txs),
                Err(e) => log::warn!("⚠️  Failed to fetch chunk: {}", e),
            }
        }

        let elapsed = start.elapsed();
        log::info!("✅ WASM: Fetched {} chunks in {:.0}ms (concurrency={})",
            chunks.len(), elapsed.as_secs_f64() * 1000.0, max_concurrent);
    }

    let timestamp = b["header"]["timestamp_nanosec"].as_str()
        .and_then(|s| s.parse::<u128>().ok())
        .unwrap_or(0);

    let when = if timestamp > 0 {
        chrono_fmt(timestamp as i64)
    } else {
        "-".into()
    };

    let hash = b["header"]["hash"].as_str().unwrap_or("").to_string();

    log::debug!("📊 Block #{} summary:", height);
    log::debug!("   Hash: {}", hash);
    log::debug!("   Timestamp: {}", when);
    log::debug!("   Total transactions: {}", txs.len());
    log::debug!("   Chunks processed: {}", chunks.len());

    let row = BlockRow {
        height,
        hash,
        timestamp: (timestamp / 1_000_000) as u64,
        tx_count: txs.len(),
        when,
        transactions: txs
    };

    log::info!("✅ Block #{} fetched successfully ({} txs)", height, row.tx_count);

    Ok(row)
}

fn chrono_fmt(nano: i64) -> String {
    use chrono::{TimeZone, Utc, Local, Timelike};
    let secs = nano / 1_000_000_000;
    let nsec = (nano % 1_000_000_000) as u32;
    let dt = Utc.timestamp_opt(secs, nsec).single().unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());
    let local_dt = dt.with_timezone(&Local);
    format!(
        "{} · {}",
        local_dt.format("%Y-%m-%d %H:%M:%S%.3f"),
        time_of_day_fmt(local_dt.hour() as u8, local_dt.minute() as u8, local_dt.second() as u8)
    )
}

fn time_of_day_fmt(h: u8, m: u8, s: u8) -> String {
    let am = h < 12;
    let hour12 = match h % 12 { 0 => 12, x => x as u32 };
    format!("{:01}:{:02}:{:02} {}", hour12, m, s, if am { "AM" } else { "PM" })
}

/// Parse a transaction from chunk response JSON into TxDetailed
fn parse_transaction_detailed(tx_json: &Value) -> Option<TxDetailed> {
    let hash = tx_json.get("hash")?.as_str()?.to_string();
    let signer_id = tx_json.get("signer_id")?.as_str()?.to_string();
    let receiver_id = tx_json.get("receiver_id")?.as_str()?.to_string();
    let nonce = tx_json.get("nonce").and_then(|n| n.as_u64()).unwrap_or(0);
    let public_key = tx_json.get("public_key").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let actions_array = tx_json.get("actions")?.as_array()?;
    let actions = parse_actions(actions_array);

    Some(TxDetailed {
        hash,
        signer_id,
        receiver_id,
        actions,
        nonce,
        public_key,
        raw_transaction: None,
    })
}

/// Parse actions array from transaction JSON
fn parse_actions(actions_json: &[Value]) -> Vec<ActionSummary> {
    actions_json
        .iter()
        .filter_map(|action| {
            if action.get("CreateAccount").is_some() {
                Some(ActionSummary::CreateAccount)
            } else if let Some(deploy) = action.get("DeployContract") {
                let code_len = deploy.get("code").and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0);
                Some(ActionSummary::DeployContract { code_len })
            } else if let Some(fc) = action.get("FunctionCall") {
                let method_name = fc.get("method_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let args_base64 = fc.get("args").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let gas = fc.get("gas").and_then(|v| v.as_u64()).unwrap_or(0);
                let deposit = fc.get("deposit").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0);

                // Decode args using near_args module
                let args_decoded = crate::near_args::decode_args_base64(
                    if args_base64.is_empty() { None } else { Some(&args_base64) },
                    64  // preview length for binary data
                );

                Some(ActionSummary::FunctionCall {
                    method_name,
                    _args_base64: args_base64,
                    args_decoded,
                    gas,
                    deposit,
                })
            } else if let Some(transfer) = action.get("Transfer") {
                let deposit = transfer.get("deposit").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0u128);
                Some(ActionSummary::Transfer { deposit })
            } else if let Some(stake) = action.get("Stake") {
                let stake_amt = stake.get("stake").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(0u128);
                let pk = stake.get("public_key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                Some(ActionSummary::Stake { stake: stake_amt, public_key: pk })
            } else if let Some(add_key) = action.get("AddKey") {
                let pk = add_key.get("public_key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let ak = if let Some(access_key) = add_key.get("access_key") {
                    access_key.to_string()
                } else { String::new() };
                Some(ActionSummary::AddKey { public_key: pk, access_key: ak })
            } else if let Some(del_key) = action.get("DeleteKey") {
                let pk = del_key.get("public_key").and_then(|v| v.as_str()).unwrap_or("").to_string();
                Some(ActionSummary::DeleteKey { public_key: pk })
            } else if let Some(del) = action.get("DeleteAccount") {
                let ben = del.get("beneficiary_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                Some(ActionSummary::DeleteAccount { beneficiary_id: ben })
            } else if let Some(delegate) = action.get("Delegate") {
                // Delegate wraps SignedDelegateAction, which has delegate_action field
                let delegate_action = delegate.get("delegate_action").unwrap_or(delegate);
                let sender_id = delegate_action.get("sender_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let receiver_id = delegate_action.get("receiver_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                // Recursively parse nested actions
                let nested_actions = delegate_action
                    .get("actions")
                    .and_then(|a| a.as_array())
                    .map(|arr| parse_actions(arr))
                    .unwrap_or_default();
                Some(ActionSummary::Delegate { sender_id, receiver_id, actions: nested_actions })
            } else {
                None
            }
        })
        .collect()
}
