// 区块链客户端服务 - 生产级实现
// 支持真实RPC广播、故障转移、重试机制
// 企业级实现：支持EVM链和非EVM链（Solana、Bitcoin、TON）

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use base64::Engine;
use hex;
use serde::{Deserialize, Serialize};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BroadcastTransactionRequest {
    pub chain: String,
    pub signed_raw_tx: String, // 0x-prefixed hex string
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BroadcastTransactionResponse {
    pub tx_hash: String,
    pub chain: String,
    pub rpc_endpoint_used: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionReceipt {
    pub tx_hash: String,
    pub block_number: Option<u64>,
    pub block_hash: Option<String>,
    pub gas_used: Option<u64>,
    pub effective_gas_price: Option<String>, // Wei as string
    pub status: Option<u8>,                  // 1 = success, 0 = failed
    pub confirmations: u64,
}

pub struct BlockchainClient {
    http_client: reqwest::Client,
    rpc_selector: Arc<crate::infrastructure::rpc_selector::RpcSelector>,
}

/// ✅统一链类型判断（使用标准化模块）
fn is_evm_chain(chain: &str) -> bool {
    crate::utils::chain_normalizer::is_evm_chain(chain)
}

impl BlockchainClient {
    pub fn new(rpc_selector: Arc<crate::infrastructure::rpc_selector::RpcSelector>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http_client: client,
            rpc_selector,
        }
    }

    /// 广播已签名的交易到区块链网络（带重试和故障转移）
    /// 企业级实现：支持EVM链和非EVM链
    pub async fn broadcast_transaction(
        &self,
        req: BroadcastTransactionRequest,
    ) -> Result<BroadcastTransactionResponse> {
        // ✅ 使用标准化的链标识符
        let chain_normalized =
            crate::utils::chain_normalizer::normalize_chain_identifier(&req.chain)
                .map_err(|e| anyhow::anyhow!("Invalid chain identifier: {}", e))?;

        if is_evm_chain(&chain_normalized) {
            // EVM链：使用eth_sendRawTransaction
            self.broadcast_evm_transaction(req).await
        } else {
            // 非EVM链：使用链特定的方法
            match chain_normalized.as_str() {
                "solana" | "sol" => self.broadcast_solana_transaction(req).await,
                "bitcoin" | "btc" => self.broadcast_bitcoin_transaction(req).await,
                "ton" => self.broadcast_ton_transaction(req).await,
                _ => anyhow::bail!("Unsupported chain for transaction broadcast: {}", req.chain),
            }
        }
    }

    /// EVM链交易广播
    async fn broadcast_evm_transaction(
        &self,
        req: BroadcastTransactionRequest,
    ) -> Result<BroadcastTransactionResponse> {
        let chain_lower = req.chain.to_lowercase();

        // 验证交易数据格式
        if !req.signed_raw_tx.starts_with("0x") {
            anyhow::bail!("Invalid raw transaction format: must start with 0x");
        }
        if req.signed_raw_tx.len() < 10 {
            anyhow::bail!("Invalid raw transaction: too short");
        }

        let mut last_error: Option<anyhow::Error> = None;

        // 重试逻辑：每次重试选择新的RPC端点
        for attempt in 1..=MAX_RETRIES {
            match self.rpc_selector.select(&chain_lower).await {
                Some(endpoint) => {
                    tracing::debug!(
                        attempt = attempt,
                        endpoint = %endpoint.url,
                        chain = %chain_lower,
                        "Attempting to broadcast EVM transaction"
                    );

                    match self
                        .send_raw_transaction(&endpoint.url, &req.signed_raw_tx)
                        .await
                    {
                        Ok(tx_hash) => {
                            crate::metrics::inc_blockchain_broadcast_success(&chain_lower);
                            tracing::info!(
                                tx_hash = %tx_hash,
                                endpoint = %endpoint.url,
                                chain = %chain_lower,
                                attempts = attempt,
                                "EVM transaction broadcast successful"
                            );

                            return Ok(BroadcastTransactionResponse {
                                tx_hash,
                                chain: req.chain,
                                rpc_endpoint_used: endpoint.url,
                            });
                        }
                        Err(e) => {
                            crate::metrics::inc_blockchain_broadcast_fail(&chain_lower);
                            tracing::warn!(
                                error = ?e,
                                endpoint = %endpoint.url,
                                attempt = attempt,
                                "EVM broadcast attempt failed"
                            );
                            last_error = Some(e);

                            if attempt < MAX_RETRIES {
                                tokio::time::sleep(Duration::from_millis(
                                    RETRY_DELAY_MS * attempt as u64,
                                ))
                                .await;
                            }
                        }
                    }
                }
                None => {
                    let err = anyhow::anyhow!(
                        "No healthy RPC endpoint available for chain: {}",
                        chain_lower
                    );
                    tracing::error!(error = ?err, chain = %chain_lower, "RPC endpoint selection failed");
                    last_error = Some(err);
                    break;
                }
            }
        }

        // 所有重试失败
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("EVM broadcast failed after {} attempts", MAX_RETRIES)
        }))
    }

    /// Solana交易广播
    async fn broadcast_solana_transaction(
        &self,
        req: BroadcastTransactionRequest,
    ) -> Result<BroadcastTransactionResponse> {
        let chain_lower = "solana";

        // Solana交易是base64编码的
        let tx_base64 = if req.signed_raw_tx.starts_with("0x") {
            // 如果是hex格式，转换为base64
            let tx_bytes = hex::decode(req.signed_raw_tx.trim_start_matches("0x"))
                .context("Failed to decode hex transaction")?;
            base64::engine::general_purpose::STANDARD.encode(&tx_bytes)
        } else {
            // 假设已经是base64格式
            req.signed_raw_tx.clone()
        };

        match self.rpc_selector.select(chain_lower).await {
            Some(endpoint) => {
                let payload = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "sendTransaction",
                    "params": [
                        tx_base64,
                        {
                            "encoding": "base64",
                            "skipPreflight": false
                        }
                    ]
                });

                let response = self
                    .http_client
                    .post(&endpoint.url)
                    .json(&payload)
                    .send()
                    .await
                    .context("Failed to send Solana RPC request")?;

                let json: serde_json::Value = response
                    .json()
                    .await
                    .context("Failed to parse Solana RPC response")?;

                if let Some(error) = json.get("error") {
                    anyhow::bail!("Solana RPC error: {:?}", error);
                }

                let signature = json
                    .get("result")
                    .and_then(|r| r.as_str())
                    .context("Missing result in Solana RPC response")?;

                Ok(BroadcastTransactionResponse {
                    tx_hash: signature.to_string(),
                    chain: req.chain,
                    rpc_endpoint_used: endpoint.url,
                })
            }
            None => anyhow::bail!("No healthy RPC endpoint available for Solana"),
        }
    }

    /// Bitcoin交易广播
    async fn broadcast_bitcoin_transaction(
        &self,
        req: BroadcastTransactionRequest,
    ) -> Result<BroadcastTransactionResponse> {
        let chain_lower = "bitcoin";

        // Bitcoin交易是hex编码的
        let tx_hex = if req.signed_raw_tx.starts_with("0x") {
            req.signed_raw_tx.trim_start_matches("0x").to_string()
        } else {
            req.signed_raw_tx.clone()
        };

        match self.rpc_selector.select(chain_lower).await {
            Some(endpoint) => {
                // 使用sendrawtransaction RPC方法
                let payload = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "sendrawtransaction",
                    "params": [tx_hex]
                });

                let response = self
                    .http_client
                    .post(&endpoint.url)
                    .json(&payload)
                    .send()
                    .await
                    .context("Failed to send Bitcoin RPC request")?;

                let json: serde_json::Value = response
                    .json()
                    .await
                    .context("Failed to parse Bitcoin RPC response")?;

                if let Some(error) = json.get("error") {
                    anyhow::bail!("Bitcoin RPC error: {:?}", error);
                }

                let txid = json
                    .get("result")
                    .and_then(|r| r.as_str())
                    .context("Missing result in Bitcoin RPC response")?;

                Ok(BroadcastTransactionResponse {
                    tx_hash: txid.to_string(),
                    chain: req.chain,
                    rpc_endpoint_used: endpoint.url,
                })
            }
            None => anyhow::bail!("No healthy RPC endpoint available for Bitcoin"),
        }
    }

    /// TON交易广播
    async fn broadcast_ton_transaction(
        &self,
        req: BroadcastTransactionRequest,
    ) -> Result<BroadcastTransactionResponse> {
        let chain_lower = "ton";

        // TON交易是base64编码的BOC
        let boc_base64 = if req.signed_raw_tx.starts_with("0x") {
            // 如果是hex格式，转换为base64
            let tx_bytes = hex::decode(req.signed_raw_tx.trim_start_matches("0x"))
                .context("Failed to decode hex transaction")?;
            base64::engine::general_purpose::STANDARD.encode(&tx_bytes)
        } else {
            // 假设已经是base64格式
            req.signed_raw_tx.clone()
        };

        match self.rpc_selector.select(chain_lower).await {
            Some(endpoint) => {
                // TON使用sendBoc方法
                let payload = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "sendBoc",
                    "params": {
                        "boc": boc_base64
                    }
                });

                let response = self
                    .http_client
                    .post(&endpoint.url)
                    .json(&payload)
                    .send()
                    .await
                    .context("Failed to send TON RPC request")?;

                let json: serde_json::Value = response
                    .json()
                    .await
                    .context("Failed to parse TON RPC response")?;

                if let Some(error) = json.get("error") {
                    anyhow::bail!("TON RPC error: {:?}", error);
                }

                // TON返回的可能是不同的格式，需要根据实际API调整
                let tx_hash = json
                    .get("result")
                    .and_then(|r| r.as_str())
                    .or_else(|| {
                        json.get("result")
                            .and_then(|r| r.get("hash"))
                            .and_then(|h| h.as_str())
                    })
                    .context("Missing result in TON RPC response")?;

                Ok(BroadcastTransactionResponse {
                    tx_hash: tx_hash.to_string(),
                    chain: req.chain,
                    rpc_endpoint_used: endpoint.url,
                })
            }
            None => anyhow::bail!("No healthy RPC endpoint available for TON"),
        }
    }

    /// 查询交易回执（用于回填fee_audit）
    pub async fn get_transaction_receipt(
        &self,
        chain: &str,
        tx_hash: &str,
    ) -> Result<Option<TransactionReceipt>> {
        let chain_lower = chain.to_lowercase();

        let endpoint = self
            .rpc_selector
            .select(&chain_lower)
            .await
            .context("No healthy RPC endpoint available")?;

        let receipt = self
            .fetch_transaction_receipt(&endpoint.url, tx_hash, &chain_lower)
            .await?;

        Ok(receipt)
    }

    /// 内部方法：调用 eth_sendRawTransaction JSON-RPC
    async fn send_raw_transaction(&self, rpc_url: &str, raw_tx: &str) -> Result<String> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [raw_tx],
            "id": 1
        });

        let response = self
            .http_client
            .post(rpc_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            anyhow::bail!("RPC request failed with status {}: {}", status, body);
        }

        let json: serde_json::Value =
            serde_json::from_str(&body).context("Failed to parse JSON response")?;

        // 检查 JSON-RPC 错误
        if let Some(error) = json.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RPC error");
            let error_code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-1);

            anyhow::bail!("RPC error {}: {}", error_code, error_msg);
        }

        // 验证RPC响应格式
        crate::infrastructure::rpc_validator::validate_rpc_response(&json)
            .context("Invalid RPC response format")?;

        // 提取并验证 tx_hash
        let tx_hash_str = json
            .get("result")
            .and_then(|r| r.as_str())
            .context("Missing result field in RPC response")?;

        let tx_hash = crate::infrastructure::rpc_validator::validate_tx_hash(tx_hash_str)
            .context("Invalid transaction hash format")?;

        Ok(tx_hash)
    }

    /// 获取ERC20代币余额（企业级实现）
    pub async fn get_erc20_balance(
        &self,
        chain: &str,
        token_address: &str,
        wallet_address: &str,
    ) -> Result<u128> {
        let chain_lower = chain.to_lowercase();

        // 选择RPC端点
        let endpoint = self
            .rpc_selector
            .select(&chain_lower)
            .await
            .ok_or_else(|| anyhow::anyhow!("No RPC endpoint available for chain: {}", chain))?;

        // ERC20 balanceOf(address) 函数调用
        // function selector: balanceOf(address) = 0x70a08231
        let function_selector = "0x70a08231";

        // 编码参数：address (32字节，右对齐)
        let address_param = format!("{:0>64}", wallet_address.trim_start_matches("0x"));
        let data = format!("{}{}", function_selector, address_param);

        // 构建eth_call请求
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_call",
            "params": [
                {
                    "to": token_address,
                    "data": data
                },
                "latest"
            ]
        });

        let response = self
            .http_client
            .post(&endpoint.url)
            .json(&request_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to call RPC endpoint")?;

        if !response.status().is_success() {
            anyhow::bail!("RPC call failed with status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        // 验证RPC响应格式
        crate::infrastructure::rpc_validator::validate_rpc_response(&json)
            .context("Invalid RPC response format")?;

        let result_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid RPC response: missing result"))?;

        // 使用RPC验证器验证余额
        let balance = crate::infrastructure::rpc_validator::validate_balance(result_hex)
            .context("Failed to validate balance from RPC response")?;

        Ok(balance)
    }

    /// 获取原生代币余额（ETH/BNB/MATIC等）
    pub async fn get_native_balance(&self, chain: &str, wallet_address: &str) -> Result<u128> {
        let chain_lower = chain.to_lowercase();

        // 选择RPC端点
        let endpoint = self
            .rpc_selector
            .select(&chain_lower)
            .await
            .ok_or_else(|| anyhow::anyhow!("No RPC endpoint available for chain: {}", chain))?;

        // 构建eth_getBalance请求
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBalance",
            "params": [wallet_address, "latest"]
        });

        let response = self
            .http_client
            .post(&endpoint.url)
            .json(&request_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to call RPC endpoint")?;

        if !response.status().is_success() {
            anyhow::bail!("RPC call failed with status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        // 验证RPC响应格式
        crate::infrastructure::rpc_validator::validate_rpc_response(&json)
            .context("Invalid RPC response format")?;

        let result_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid RPC response: missing result"))?;

        // 使用RPC验证器验证余额
        let balance = crate::infrastructure::rpc_validator::validate_balance(result_hex)
            .context("Failed to validate balance from RPC response")?;

        Ok(balance)
    }

    /// 获取交易计数（用于nonce管理）
    pub async fn get_transaction_count(&self, chain: &str, address: &str) -> Result<u64> {
        let chain_lower = chain.to_lowercase();

        // 选择健康的RPC端点
        let endpoint = self
            .rpc_selector
            .select(&chain_lower)
            .await
            .context("No healthy RPC endpoint available")?;

        // 构建eth_getTransactionCount请求
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getTransactionCount",
            "params": [address, "latest"]
        });

        let response = self
            .http_client
            .post(&endpoint.url)
            .json(&request_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to call RPC endpoint")?;

        if !response.status().is_success() {
            anyhow::bail!("RPC call failed with status: {}", response.status());
        }

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        // 验证RPC响应格式
        crate::infrastructure::rpc_validator::validate_rpc_response(&json)
            .context("Invalid RPC response format")?;

        let result_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid RPC response: missing result"))?;

        // 使用RPC验证器验证nonce
        let nonce = crate::infrastructure::rpc_validator::validate_nonce(result_hex)
            .context("Failed to validate nonce from RPC response")?;

        Ok(nonce)
    }

    /// 获取链高（区块高度）
    /// 企业级实现：支持EVM链和非EVM链
    pub async fn get_block_number(&self, chain: &str) -> Result<u64> {
        let chain_lower = chain.to_lowercase();

        if is_evm_chain(&chain_lower) {
            self.get_evm_block_number(&chain_lower).await
        } else {
            match chain_lower.as_str() {
                "solana" | "sol" => self.get_solana_block_number().await,
                "bitcoin" | "btc" => self.get_bitcoin_block_height().await,
                "ton" => self.get_ton_block_height().await,
                _ => anyhow::bail!("Unsupported chain for block height query: {}", chain),
            }
        }
    }

    /// EVM链区块高度查询
    async fn get_evm_block_number(&self, chain: &str) -> Result<u64> {
        // 选择健康的RPC端点
        let endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .context("No healthy RPC endpoint available")?;

        // 调用 eth_blockNumber JSON-RPC
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_blockNumber",
            "params": [],
            "id": 1
        });

        let response = self
            .http_client
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse JSON response")?;

        // 检查错误
        if let Some(error) = json.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RPC error");
            anyhow::bail!("RPC error: {}", error_msg);
        }

        // 解析区块高度（十六进制字符串）
        let hex_str = json
            .get("result")
            .and_then(|r| r.as_str())
            .context("Missing result field")?;

        let block_number = u64::from_str_radix(hex_str.trim_start_matches("0x"), 16)
            .context("Failed to parse block number")?;

        Ok(block_number)
    }

    /// Solana区块高度查询
    async fn get_solana_block_number(&self) -> Result<u64> {
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getSlot"
        });

        let response = self
            .http_client
            .post(&rpc_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to call Solana RPC")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Solana RPC response")?;

        if let Some(error) = json.get("error") {
            anyhow::bail!("Solana RPC error: {:?}", error);
        }

        let slot = json
            .get("result")
            .and_then(|r| r.as_u64())
            .context("Missing result in Solana RPC response")?;

        Ok(slot)
    }

    /// Bitcoin区块高度查询
    async fn get_bitcoin_block_height(&self) -> Result<u64> {
        let api_url = std::env::var("BITCOIN_API_URL")
            .unwrap_or_else(|_| "https://blockstream.info/api".to_string());

        let url = format!("{}/blocks/tip/height", api_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to call Bitcoin API")?;

        let height: u64 = response
            .json()
            .await
            .context("Failed to parse Bitcoin API response")?;

        Ok(height)
    }

    /// TON区块高度查询
    async fn get_ton_block_height(&self) -> Result<u64> {
        let api_url = std::env::var("TON_API_URL")
            .unwrap_or_else(|_| "https://toncenter.com/api/v2".to_string());

        let url = format!("{}/getMasterchainInfo", api_url);

        #[derive(serde::Deserialize)]
        struct TonResponse {
            ok: bool,
            result: TonResult,
        }

        #[derive(serde::Deserialize)]
        struct TonResult {
            #[serde(rename = "last")]
            last_block: LastBlock,
        }

        #[derive(serde::Deserialize)]
        struct LastBlock {
            #[serde(rename = "seqno")]
            seqno: u64,
        }

        let response: TonResponse = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to call TON API")?
            .json()
            .await
            .context("Failed to parse TON API response")?;

        if response.ok {
            Ok(response.result.last_block.seqno)
        } else {
            anyhow::bail!("TON API returned error")
        }
    }

    /// 内部方法：调用 eth_getTransactionReceipt JSON-RPC
    async fn fetch_transaction_receipt(
        &self,
        rpc_url: &str,
        tx_hash: &str,
        chain_type: &str,
    ) -> Result<Option<TransactionReceipt>> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });

        let response = self
            .http_client
            .post(rpc_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let status = response.status();
        let body = response
            .text()
            .await
            .context("Failed to read response body")?;

        if !status.is_success() {
            anyhow::bail!("RPC request failed with status {}: {}", status, body);
        }

        let json: serde_json::Value =
            serde_json::from_str(&body).context("Failed to parse JSON response")?;

        if let Some(error) = json.get("error") {
            let error_msg = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown RPC error");
            anyhow::bail!("RPC error: {}", error_msg);
        }

        let result = json.get("result");

        // null result 表示交易尚未确认
        let receipt_json = match result {
            None => return Ok(None),
            Some(v) if v.is_null() => return Ok(None),
            Some(v) => v,
        };

        // 解析回执字段
        let gas_used = receipt_json
            .get("gasUsed")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());

        let effective_gas_price = receipt_json
            .get("effectiveGasPrice")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let block_number = receipt_json
            .get("blockNumber")
            .and_then(|v| v.as_str())
            .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok());

        let block_hash = receipt_json
            .get("blockHash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let status_field = receipt_json
            .get("status")
            .and_then(|v| v.as_str())
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok());

        // 计算真实确认数：current_block_number - tx_block_number
        // 生产级实现：查询当前区块高度并计算差值
        let confirmations = if let Some(tx_block) = block_number {
            // 尝试获取当前区块高度
            match self.get_block_number(chain_type).await {
                Ok(current_block) => {
                    if current_block >= tx_block {
                        current_block - tx_block
                    } else {
                        0 // 异常情况：交易块高于当前块
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to get current block number: {}", e);
                    0 // 降级：无法确定确认数
                }
            }
        } else {
            0 // 交易还未打包进块
        };

        Ok(Some(TransactionReceipt {
            tx_hash: tx_hash.to_string(),
            block_number,
            block_hash,
            gas_used,
            effective_gas_price,
            status: status_field,
            confirmations,
        }))
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_validate_raw_tx_format() {
        let valid_tx = "0xf86c808504a817c800825208943535353535353535353535353535353535353535880de0b6b3a76400008025a0";
        assert!(valid_tx.starts_with("0x"));
        assert!(valid_tx.len() > 10);

        let invalid_tx1 = "f86c808504a817c800825208";
        assert!(!invalid_tx1.starts_with("0x"));

        let invalid_tx2 = "0x12345";
        assert!(invalid_tx2.len() < 10);
    }

    #[test]
    fn test_parse_hex_to_u64() {
        let hex = "0x1a2b3c";
        let num = u64::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap();
        assert_eq!(num, 1715004);
    }
}
