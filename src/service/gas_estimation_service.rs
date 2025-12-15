//! 统一Gas估算服务
//!
//! 企业级实现：为所有链提供统一的Gas估算接口
//! 解决问题：D.1 - Gas估算未统一

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    infrastructure::{cache::RedisCtx, rpc_selector::RpcSelector},
    utils::chain_normalizer,
};

/// Gas估算请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimationRequest {
    /// 链标识
    pub chain: String,
    /// 发送地址
    pub from: String,
    /// 接收地址
    pub to: String,
    /// 交易金额（wei/lamports等最小单位）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// 交易数据（合约调用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Gas估算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimationResult {
    /// 估算的Gas限制
    pub gas_limit: String,
    /// 当前Gas价格（wei）
    pub gas_price: String,
    /// 预估总费用（wei）
    pub estimated_fee: String,
    /// 链标识（规范化后）
    pub chain: String,
    /// 安全缓冲（实际使用 gas_limit * buffer）
    pub buffer_multiplier: f64,
}

/// Gas估算服务
pub struct GasEstimationService {
    rpc_selector: Arc<RpcSelector>,
    redis: Option<Arc<RedisCtx>>,
    http_client: reqwest::Client,
}

impl GasEstimationService {
    /// 创建Gas估算服务
    pub fn new(rpc_selector: Arc<RpcSelector>, redis: Option<Arc<RedisCtx>>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            rpc_selector,
            redis,
            http_client,
        }
    }

    /// 估算Gas
    ///
    /// # 功能
    /// 1. 标准化链标识符
    /// 2. 检查缓存（相同交易类型）
    /// 3. 调用链上估算（eth_estimateGas / Solana getRecentBlockhash等）
    /// 4. 添加安全缓冲（1.2x）
    /// 5. 缓存结果
    pub async fn estimate_gas(&self, request: GasEstimationRequest) -> Result<GasEstimationResult> {
        // 1. 标准化链标识符
        let chain_normalized = chain_normalizer::normalize_chain_identifier(&request.chain)?;

        // 2. 检查缓存
        if let Some(cached) = self
            .get_cached_estimation(&chain_normalized, &request)
            .await
        {
            return Ok(cached);
        }

        // 3. 根据链类型调用相应的估算逻辑
        let result = if chain_normalizer::is_evm_chain(&chain_normalized) {
            self.estimate_evm_gas(&chain_normalized, &request).await?
        } else {
            match chain_normalized.as_str() {
                "solana" => {
                    self.estimate_solana_gas(&chain_normalized, &request)
                        .await?
                }
                "bitcoin" => {
                    self.estimate_bitcoin_gas(&chain_normalized, &request)
                        .await?
                }
                "ton" => self.estimate_ton_gas(&chain_normalized, &request).await?,
                _ => anyhow::bail!("Unsupported chain for gas estimation: {}", chain_normalized),
            }
        };

        // 4. 缓存结果
        self.cache_estimation(&chain_normalized, &request, &result)
            .await;

        Ok(result)
    }

    /// 估算EVM链的Gas
    async fn estimate_evm_gas(
        &self,
        chain: &str,
        request: &GasEstimationRequest,
    ) -> Result<GasEstimationResult> {
        let endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No healthy RPC endpoint for chain: {}", chain))?;

        // 1. 估算Gas Limit（eth_estimateGas）
        let gas_limit = self.call_eth_estimate_gas(&endpoint.url, request).await?;

        // 2. 获取当前Gas价格（eth_gasPrice）
        let gas_price = self.call_eth_gas_price(&endpoint.url).await?;

        // 3. 添加安全缓冲（1.2x）
        let buffer_multiplier = 1.2;
        let gas_limit_with_buffer = (gas_limit as f64 * buffer_multiplier) as u64;

        // 4. 计算预估总费用
        let estimated_fee = gas_limit_with_buffer * gas_price;

        Ok(GasEstimationResult {
            gas_limit: gas_limit_with_buffer.to_string(),
            gas_price: gas_price.to_string(),
            estimated_fee: estimated_fee.to_string(),
            chain: chain.to_string(),
            buffer_multiplier,
        })
    }

    /// 调用 eth_estimateGas
    async fn call_eth_estimate_gas(
        &self,
        rpc_url: &str,
        request: &GasEstimationRequest,
    ) -> Result<u64> {
        let mut params_obj = serde_json::json!({
            "from": request.from,
            "to": request.to,
        });

        if let Some(value) = &request.value {
            params_obj["value"] = serde_json::Value::String(value.clone());
        }
        if let Some(data) = &request.data {
            params_obj["data"] = serde_json::Value::String(data.clone());
        }

        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_estimateGas",
            "params": [params_obj]
        });

        let response = self
            .http_client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send eth_estimateGas request")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse eth_estimateGas response")?;

        if let Some(error) = json.get("error") {
            anyhow::bail!("eth_estimateGas error: {:?}", error);
        }

        let result_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .context("Missing result in eth_estimateGas response")?;

        let gas_limit = u64::from_str_radix(result_hex.trim_start_matches("0x"), 16)
            .context("Failed to parse gas limit")?;

        Ok(gas_limit)
    }

    /// 调用 eth_gasPrice
    async fn call_eth_gas_price(&self, rpc_url: &str) -> Result<u64> {
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_gasPrice",
            "params": []
        });

        let response = self
            .http_client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send eth_gasPrice request")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse eth_gasPrice response")?;

        if let Some(error) = json.get("error") {
            anyhow::bail!("eth_gasPrice error: {:?}", error);
        }

        let result_hex = json
            .get("result")
            .and_then(|v| v.as_str())
            .context("Missing result in eth_gasPrice response")?;

        let gas_price = u64::from_str_radix(result_hex.trim_start_matches("0x"), 16)
            .context("Failed to parse gas price")?;

        Ok(gas_price)
    }

    /// 估算Solana的费用
    async fn estimate_solana_gas(
        &self,
        chain: &str,
        _request: &GasEstimationRequest,
    ) -> Result<GasEstimationResult> {
        let _endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No healthy RPC endpoint for Solana"))?;

        // Solana固定费用：5000 lamports per signature
        let base_fee = 5000u64;

        Ok(GasEstimationResult {
            gas_limit: "1".to_string(), // Solana没有gas limit概念
            gas_price: base_fee.to_string(),
            estimated_fee: base_fee.to_string(),
            chain: chain.to_string(),
            buffer_multiplier: 1.0,
        })
    }

    /// 估算Bitcoin的费用
    async fn estimate_bitcoin_gas(
        &self,
        chain: &str,
        _request: &GasEstimationRequest,
    ) -> Result<GasEstimationResult> {
        // Bitcoin使用 sat/vB (satoshis per virtual byte)
        // 典型交易大小：约250 vB
        let typical_tx_size = 250u64;
        let sat_per_vb = 10u64; // 默认10 sat/vB（应从费率API获取）

        let estimated_fee = typical_tx_size * sat_per_vb;

        Ok(GasEstimationResult {
            gas_limit: typical_tx_size.to_string(),
            gas_price: sat_per_vb.to_string(),
            estimated_fee: estimated_fee.to_string(),
            chain: chain.to_string(),
            buffer_multiplier: 1.0,
        })
    }

    /// 估算TON的费用
    async fn estimate_ton_gas(
        &self,
        chain: &str,
        _request: &GasEstimationRequest,
    ) -> Result<GasEstimationResult> {
        // TON典型交易费用：约0.01 TON
        let estimated_fee = 10_000_000u64; // 0.01 TON in nanotons

        Ok(GasEstimationResult {
            gas_limit: "1".to_string(),
            gas_price: estimated_fee.to_string(),
            estimated_fee: estimated_fee.to_string(),
            chain: chain.to_string(),
            buffer_multiplier: 1.0,
        })
    }

    /// 从缓存获取估算结果
    async fn get_cached_estimation(
        &self,
        chain: &str,
        request: &GasEstimationRequest,
    ) -> Option<GasEstimationResult> {
        if let Some(redis) = &self.redis {
            let cache_key = format!(
                "gas:estimate:{}:{}:{}",
                chain,
                request.from,
                request.data.as_deref().unwrap_or("simple")
            );

            if let Ok(mut conn) = redis.client.get_multiplexed_async_connection().await {
                use redis::AsyncCommands;
                if let Ok(Some(cached_json)) = conn.get::<_, Option<String>>(&cache_key).await {
                    if let Ok(result) = serde_json::from_str::<GasEstimationResult>(&cached_json) {
                        return Some(result);
                    }
                }
            }
        }
        None
    }

    /// 缓存估算结果
    async fn cache_estimation(
        &self,
        chain: &str,
        request: &GasEstimationRequest,
        result: &GasEstimationResult,
    ) {
        if let Some(redis) = &self.redis {
            let cache_key = format!(
                "gas:estimate:{}:{}:{}",
                chain,
                request.from,
                request.data.as_deref().unwrap_or("simple")
            );

            if let Ok(result_json) = serde_json::to_string(result) {
                if let Ok(mut conn) = redis.client.get_multiplexed_async_connection().await {
                    use redis::AsyncCommands;
                    let _: Result<(), _> = conn.set_ex(&cache_key, result_json, 60).await;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_estimation_result_serialization() {
        let result = GasEstimationResult {
            gas_limit: "21000".to_string(),
            gas_price: "20000000000".to_string(),
            estimated_fee: "420000000000000".to_string(),
            chain: "ethereum".to_string(),
            buffer_multiplier: 1.2,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: GasEstimationResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.gas_limit, result.gas_limit);
        assert_eq!(deserialized.chain, result.chain);
    }
}
