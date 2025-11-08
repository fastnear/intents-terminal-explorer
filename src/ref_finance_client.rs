// RPC client for fetching live pool data from Ref Finance
// Polls v2.ref-finance.near contract for real-time pool state

use anyhow::{Context, Result};
use futures_util::future;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time;

use crate::arb_engine::PoolInfo;

const REF_FINANCE_CONTRACT: &str = "v2.ref-finance.near";
const DEFAULT_POLL_INTERVAL_MS: u64 = 1000; // Poll every 1 second

/// Ref Finance RPC client
pub struct RefFinanceClient {
    rpc_url: String,
    http_client: reqwest::Client,
    poll_interval: Duration,
}

/// RPC request for view function calls
#[derive(Serialize)]
struct ViewFunctionRequest {
    jsonrpc: String,
    id: String,
    method: String,
    params: ViewFunctionParams,
}

#[derive(Serialize)]
struct ViewFunctionParams {
    request_type: String,
    finality: String,
    account_id: String,
    method_name: String,
    args_base64: String,
}

/// RPC response
#[derive(Deserialize)]
struct RpcResponse {
    result: RpcResult,
}

#[derive(Deserialize)]
struct RpcResult {
    result: Vec<u8>,
}

impl RefFinanceClient {
    /// Create new client
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            poll_interval: Duration::from_millis(DEFAULT_POLL_INTERVAL_MS),
        }
    }

    /// Set polling interval
    pub fn with_poll_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval = Duration::from_millis(interval_ms);
        self
    }

    /// Call view function on Ref Finance contract
    async fn view_function(&self, method_name: &str, args: serde_json::Value) -> Result<Vec<u8>> {
        use base64::{engine::general_purpose, Engine as _};
        let args_json = serde_json::to_string(&args).unwrap();
        let args_base64 = general_purpose::STANDARD.encode(args_json.as_bytes());

        let request = ViewFunctionRequest {
            jsonrpc: "2.0".to_string(),
            id: "ratacat".to_string(),
            method: "query".to_string(),
            params: ViewFunctionParams {
                request_type: "call_function".to_string(),
                finality: "final".to_string(),
                account_id: REF_FINANCE_CONTRACT.to_string(),
                method_name: method_name.to_string(),
                args_base64,
            },
        };

        let response = self
            .http_client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let rpc_response: RpcResponse = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        Ok(rpc_response.result.result)
    }

    /// Get number of pools in Ref Finance
    pub async fn get_number_of_pools(&self) -> Result<u64> {
        let result = self.view_function("get_number_of_pools", json!({})).await?;
        let result_str = String::from_utf8(result).context("Invalid UTF-8 in response")?;
        let num_pools: u64 = result_str
            .trim_matches('"')
            .parse()
            .context("Failed to parse pool count")?;
        Ok(num_pools)
    }

    /// Get pools by range
    pub async fn get_pools(&self, from_index: u64, limit: u64) -> Result<Vec<PoolInfo>> {
        let result = self
            .view_function(
                "get_pools",
                json!({
                    "from_index": from_index,
                    "limit": limit
                }),
            )
            .await?;

        let result_str = String::from_utf8(result).context("Invalid UTF-8 in response")?;
        let pools: Vec<RefPoolInfo> =
            serde_json::from_str(&result_str).context("Failed to parse pools JSON")?;

        // Convert to our PoolInfo format
        Ok(pools
            .into_iter()
            .enumerate()
            .map(|(idx, p)| PoolInfo {
                pool_id: from_index + idx as u64,
                token_account_ids: p.token_account_ids,
                amounts: p
                    .amounts
                    .into_iter()
                    .map(|s| s.trim_matches('"').parse::<u128>().unwrap_or(0))
                    .collect(),
                total_fee: p.total_fee,
                shares_total_supply: p
                    .shares_total_supply
                    .trim_matches('"')
                    .parse::<u128>()
                    .unwrap_or(0),
            })
            .collect())
    }

    /// Get specific pool by ID
    pub async fn get_pool(&self, pool_id: u64) -> Result<PoolInfo> {
        let result = self
            .view_function(
                "get_pool",
                json!({
                    "pool_id": pool_id
                }),
            )
            .await?;

        let result_str = String::from_utf8(result).context("Invalid UTF-8 in response")?;
        let pool: RefPoolInfo =
            serde_json::from_str(&result_str).context("Failed to parse pool JSON")?;

        Ok(PoolInfo {
            pool_id,
            token_account_ids: pool.token_account_ids,
            amounts: pool
                .amounts
                .into_iter()
                .map(|s| s.trim_matches('"').parse::<u128>().unwrap_or(0))
                .collect(),
            total_fee: pool.total_fee,
            shares_total_supply: pool
                .shares_total_supply
                .trim_matches('"')
                .parse::<u128>()
                .unwrap_or(0),
        })
    }

    /// Poll specific pools continuously and yield updates
    pub async fn poll_pools(
        &self,
        pool_ids: Vec<u64>,
        mut callback: impl FnMut(PoolInfo),
    ) -> Result<()> {
        let mut interval = time::interval(self.poll_interval);

        loop {
            interval.tick().await;

            // Fetch all pools concurrently
            let futures: Vec<_> = pool_ids
                .iter()
                .map(|&pool_id| self.get_pool(pool_id))
                .collect();

            let results = future::join_all(futures).await;

            for result in results {
                if let Ok(pool_info) = result {
                    callback(pool_info);
                }
            }
        }
    }

    /// Auto-discover interesting pools (NEAR-* pairs with high liquidity)
    pub async fn discover_near_pools(&self) -> Result<Vec<u64>> {
        let num_pools = self.get_number_of_pools().await?;
        let mut interesting_pools = Vec::new();

        // Fetch pools in batches of 100
        for batch_start in (0..num_pools).step_by(100) {
            let batch_size = 100.min(num_pools - batch_start);
            let pools = self.get_pools(batch_start, batch_size).await?;

            for pool in pools {
                // Filter for NEAR pairs with decent liquidity
                if pool.token_account_ids.iter().any(|t| t.contains("near"))
                    && pool.liquidity_usd() > 1000.0
                {
                    interesting_pools.push(pool.pool_id);
                }
            }
        }

        Ok(interesting_pools)
    }
}

/// Ref Finance pool info format (as returned by RPC)
#[derive(Debug, Deserialize)]
struct RefPoolInfo {
    token_account_ids: Vec<String>,
    amounts: Vec<String>,
    total_fee: u32,
    shares_total_supply: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_number_of_pools() {
        let client = RefFinanceClient::new("https://rpc.mainnet.fastnear.com/".to_string());
        let num_pools = client.get_number_of_pools().await.unwrap();
        assert!(num_pools > 0);
        println!("Total pools: {}", num_pools);
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_get_pools() {
        let client = RefFinanceClient::new("https://rpc.mainnet.fastnear.com/".to_string());
        let pools = client.get_pools(0, 10).await.unwrap();
        assert!(!pools.is_empty());

        for pool in pools {
            println!(
                "Pool {}: {:?} <-> {:?}",
                pool.pool_id,
                pool.token_account_ids.get(0),
                pool.token_account_ids.get(1)
            );
            println!("  Price: {:.6}", pool.price());
            println!("  Liquidity: ${:.2}", pool.liquidity_usd());
        }
    }

    #[tokio::test]
    #[ignore] // Requires network access
    async fn test_discover_near_pools() {
        let client = RefFinanceClient::new("https://rpc.mainnet.fastnear.com/".to_string());
        let near_pools = client.discover_near_pools().await.unwrap();

        println!("Found {} NEAR pools with >$1k liquidity", near_pools.len());
        for pool_id in near_pools.iter().take(10) {
            println!("  Pool ID: {}", pool_id);
        }
    }
}
