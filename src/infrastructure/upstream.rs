//! 上游链数据客户端（EVM 优先）：带超时/重试/降级的最小实现
//! 环境变量：
//! - UPSTREAM_EVM_RPC_URL（默认 http://localhost:8545）
//! - UPSTREAM_TIMEOUT_MS（默认 2000）
//! - UPSTREAM_RETRIES（默认 2 ）

use std::{
    env,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct UpstreamClient {
    pub evm_rpc: String,
    pub timeout: Duration,
    pub retries: usize,
}

impl Default for UpstreamClient {
    fn default() -> Self {
        Self::new()
    }
}

impl UpstreamClient {
    pub fn new() -> Self {
        let evm_rpc =
            env::var("UPSTREAM_EVM_RPC_URL").unwrap_or_else(|_| "http://localhost:8545".into());
        let timeout = env::var("UPSTREAM_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .map(Duration::from_millis)
            .unwrap_or(Duration::from_millis(2000));
        let retries = env::var("UPSTREAM_RETRIES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(2);
        Self {
            evm_rpc,
            timeout,
            retries,
        }
    }

    pub async fn evm_gas_price(&self) -> anyhow::Result<String> {
        // 返回 wei 的十六进制字符串，如 "0x3b9aca00"
        let req = JsonRpcRequest::new("eth_gasPrice", vec![]);
        let v: serde_json::Value = self.rpc_post(&self.evm_rpc, &req).await?;
        let hex = v.get("result").and_then(|r| r.as_str()).unwrap_or("0x0");
        Ok(hex_to_dec_string(hex))
    }

    pub async fn evm_block_number(&self) -> anyhow::Result<u64> {
        let req = JsonRpcRequest::new("eth_blockNumber", vec![]);
        let v: serde_json::Value = self.rpc_post(&self.evm_rpc, &req).await?;
        let hex = v.get("result").and_then(|r| r.as_str()).unwrap_or("0x0");
        u64::from_str_radix(hex.trim_start_matches("0x"), 16)
            .map_err(|e| anyhow::anyhow!("Failed to parse block number: {}", e))
    }

    pub async fn evm_get_balance(&self, address: &str) -> anyhow::Result<String> {
        let params = vec![
            serde_json::Value::String(address.into()),
            serde_json::Value::String("latest".into()),
        ];
        let req = JsonRpcRequest::new("eth_getBalance", params);
        let v: serde_json::Value = self.rpc_post(&self.evm_rpc, &req).await?;
        let hex = v.get("result").and_then(|r| r.as_str()).unwrap_or("0x0");
        Ok(hex_to_dec_string(hex))
    }

    async fn rpc_post<T: for<'de> serde::Deserialize<'de>>(
        &self,
        url: &str,
        body: &JsonRpcRequest<'_>,
    ) -> anyhow::Result<T> {
        let client = reqwest::Client::builder().timeout(self.timeout).build()?;
        let mut attempt = 0usize;
        loop {
            let start = Instant::now();
            let res = client.post(url).json(body).send().await;
            match res {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        crate::metrics::observe_upstream_latency_ms(
                            start.elapsed().as_millis(),
                            false,
                        );
                        attempt += 1;
                    } else {
                        let v = resp.json::<T>().await?;
                        crate::metrics::observe_upstream_latency_ms(
                            start.elapsed().as_millis(),
                            true,
                        );
                        return Ok(v);
                    }
                }
                Err(_) => {
                    crate::metrics::observe_upstream_latency_ms(start.elapsed().as_millis(), false);
                    attempt += 1;
                }
            }
            if attempt > self.retries {
                break;
            }
            let backoff = 50 * (1 << (attempt.min(5))); // 简单指数回退，最大 ~1600ms
            tokio::time::sleep(Duration::from_millis(backoff as u64)).await;
        }
        Err(anyhow::anyhow!("upstream request failed after retries"))
    }
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Vec<serde_json::Value>,
    id: u64,
}

impl<'a> JsonRpcRequest<'a> {
    fn new(method: &'a str, params: Vec<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0",
            method,
            params,
            id: 1,
        }
    }
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<serde_json::Value>,
}

fn hex_to_dec_string(s: &str) -> String {
    let trimmed = s.trim_start_matches("0x");
    if trimmed.is_empty() {
        return "0".into();
    }
    match rust_decimal::Decimal::from_str_exact(
        &u128::from_str_radix(trimmed, 16).unwrap_or(0).to_string(),
    ) {
        Ok(d) => d.to_string(),
        Err(_) => "0".into(),
    }
}
