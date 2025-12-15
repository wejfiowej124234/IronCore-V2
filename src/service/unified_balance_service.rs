//! 统一余额获取服务
//! 企业级实现：所有链的余额获取使用统一接口

use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    infrastructure::rpc_selector::RpcSelector, service::blockchain_client::BlockchainClient,
};

/// 余额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceInfo {
    /// 链标识
    pub chain: String,
    /// 地址
    pub address: String,
    /// 原生币余额（最小单位）
    pub native_balance: String,
    /// 原生币余额（显示单位，如ETH）
    pub native_balance_formatted: String,
    /// 原生币符号
    pub native_symbol: String,
    /// Token余额列表
    pub tokens: Vec<TokenBalance>,
}

/// Token余额
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    /// 合约地址
    pub contract_address: String,
    /// Token符号
    pub symbol: String,
    /// Token名称
    pub name: String,
    /// 余额（最小单位）
    pub balance: String,
    /// 余额（显示单位）
    pub balance_formatted: String,
    /// 精度
    pub decimals: u8,
}

/// 统一余额服务
pub struct UnifiedBalanceService {
    blockchain_client: Arc<BlockchainClient>,
    rpc_selector: Arc<RpcSelector>,
}

impl UnifiedBalanceService {
    pub fn new(blockchain_client: Arc<BlockchainClient>, rpc_selector: Arc<RpcSelector>) -> Self {
        Self {
            blockchain_client,
            rpc_selector,
        }
    }

    /// 获取地址余额（统一接口）
    ///
    /// # 支持的链
    /// - EVM: ETH, BSC, Polygon, Arbitrum, Optimism
    /// - Solana: SOL
    /// - Bitcoin: BTC
    /// - TON: TON
    pub async fn get_balance(&self, chain: &str, address: &str) -> Result<BalanceInfo> {
        let chain_normalized = crate::utils::chain_normalizer::normalize_chain_identifier(chain)?;

        if crate::utils::chain_normalizer::is_evm_chain(&chain_normalized) {
            self.get_evm_balance(&chain_normalized, address).await
        } else {
            match chain_normalized.as_str() {
                "solana" | "sol" => self.get_solana_balance(address).await,
                "bitcoin" | "btc" => self.get_bitcoin_balance(address).await,
                "ton" => self.get_ton_balance(address).await,
                _ => anyhow::bail!("Unsupported chain: {}", chain),
            }
        }
    }

    /// 批量获取多个地址余额
    pub async fn get_balances_batch(
        &self,
        requests: Vec<(String, String)>, // (chain, address)
    ) -> HashMap<String, Result<BalanceInfo>> {
        let mut results = HashMap::new();

        // 并发获取所有余额
        let futures: Vec<_> = requests
            .into_iter()
            .map(|(chain, address)| {
                let service = self.clone();
                let key = format!("{}:{}", chain, address);

                async move {
                    let result = service.get_balance(&chain, &address).await;
                    (key, result)
                }
            })
            .collect();

        let outcomes = futures::future::join_all(futures).await;

        for (key, result) in outcomes {
            results.insert(key, result);
        }

        results
    }

    // ===== EVM 链余额 =====

    async fn get_evm_balance(&self, chain: &str, address: &str) -> Result<BalanceInfo> {
        // 验证地址格式
        if !address.starts_with("0x") || address.len() != 42 {
            anyhow::bail!("Invalid EVM address format: {}", address);
        }

        // 选择RPC端点
        let endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No available RPC endpoint for chain: {}", chain))?;

        // 调用eth_getBalance
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBalance",
            "params": [address, "latest"]
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&endpoint.url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send RPC request")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = json.get("error") {
            anyhow::bail!("RPC error: {:?}", error);
        }

        let balance_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid RPC response"))?;

        // 解析余额（Wei）
        let balance_wei = u128::from_str_radix(balance_hex.trim_start_matches("0x"), 16)
            .context("Failed to parse balance")?;

        // 转换为ETH（18位精度）
        let balance_eth = balance_wei as f64 / 1e18;

        // 获取原生币符号
        let native_symbol = self.get_native_symbol(chain);

        Ok(BalanceInfo {
            chain: chain.to_string(),
            address: address.to_string(),
            native_balance: balance_wei.to_string(),
            native_balance_formatted: format!("{:.6}", balance_eth),
            native_symbol,
            tokens: vec![], // TODO: 获取ERC20 token余额
        })
    }

    // ===== Solana 余额 =====

    async fn get_solana_balance(&self, address: &str) -> Result<BalanceInfo> {
        let endpoint = self
            .rpc_selector
            .select("solana")
            .await
            .ok_or_else(|| anyhow::anyhow!("No available Solana RPC endpoint"))?;

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address]
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&endpoint.url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send Solana RPC request")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Solana RPC response")?;

        let balance_lamports = json
            .get("result")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| anyhow::anyhow!("Invalid Solana RPC response"))?;

        // Lamports 转 SOL（9位精度）
        let balance_sol = balance_lamports as f64 / 1e9;

        Ok(BalanceInfo {
            chain: "SOL".to_string(),
            address: address.to_string(),
            native_balance: balance_lamports.to_string(),
            native_balance_formatted: format!("{:.6}", balance_sol),
            native_symbol: "SOL".to_string(),
            tokens: vec![],
        })
    }

    // ===== Bitcoin 余额 =====

    async fn get_bitcoin_balance(&self, address: &str) -> Result<BalanceInfo> {
        // Bitcoin需要使用第三方API（如Blockchair, Blockchain.info）或自建索引器
        // 简化实现：使用Blockchair API

        let url = format!(
            "https://api.blockchair.com/bitcoin/dashboards/address/{}",
            address
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to query Bitcoin balance")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Bitcoin API response")?;

        let balance_satoshi = json
            .get("data")
            .and_then(|v| v.get(address))
            .and_then(|v| v.get("address"))
            .and_then(|v| v.get("balance"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // Satoshi 转 BTC（8位精度）
        let balance_btc = balance_satoshi as f64 / 1e8;

        Ok(BalanceInfo {
            chain: "BTC".to_string(),
            address: address.to_string(),
            native_balance: balance_satoshi.to_string(),
            native_balance_formatted: format!("{:.8}", balance_btc),
            native_symbol: "BTC".to_string(),
            tokens: vec![],
        })
    }

    // ===== TON 余额 =====

    async fn get_ton_balance(&self, address: &str) -> Result<BalanceInfo> {
        // TON需要使用TON Center API或自建节点
        // 简化实现：使用TON Center API

        let url = format!(
            "https://toncenter.com/api/v2/getAddressBalance?address={}",
            address
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .context("Failed to query TON balance")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse TON API response")?;

        let balance_nanoton = json.get("result").and_then(|v| v.as_u64()).unwrap_or(0);

        // NanoTON 转 TON（9位精度）
        let balance_ton = balance_nanoton as f64 / 1e9;

        Ok(BalanceInfo {
            chain: "TON".to_string(),
            address: address.to_string(),
            native_balance: balance_nanoton.to_string(),
            native_balance_formatted: format!("{:.6}", balance_ton),
            native_symbol: "TON".to_string(),
            tokens: vec![],
        })
    }

    // ===== 辅助方法 =====

    fn get_native_symbol(&self, chain: &str) -> String {
        match chain.to_uppercase().as_str() {
            "ETH" | "ETHEREUM" => "ETH".to_string(),
            "BSC" => "BNB".to_string(),
            "POLYGON" | "MATIC" => "MATIC".to_string(),
            "ARBITRUM" => "ETH".to_string(),
            "OPTIMISM" => "ETH".to_string(),
            "AVALANCHE" | "AVAX" => "AVAX".to_string(),
            _ => chain.to_uppercase(),
        }
    }
}

impl Clone for UnifiedBalanceService {
    fn clone(&self) -> Self {
        Self {
            blockchain_client: Arc::clone(&self.blockchain_client),
            rpc_selector: Arc::clone(&self.rpc_selector),
        }
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_wei_to_eth() {
        let wei: u128 = 1_000_000_000_000_000_000; // 1 ETH
        let eth = wei as f64 / 1e18;
        assert_eq!(eth, 1.0);
    }

    #[test]
    fn test_lamports_to_sol() {
        let lamports: u64 = 1_000_000_000; // 1 SOL
        let sol = lamports as f64 / 1e9;
        assert_eq!(sol, 1.0);
    }

    #[test]
    fn test_satoshi_to_btc() {
        let satoshi: u64 = 100_000_000; // 1 BTC
        let btc = satoshi as f64 / 1e8;
        assert_eq!(btc, 1.0);
    }
}
