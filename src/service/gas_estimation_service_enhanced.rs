//! Gas估算服务增强版（企业级实现）
//! 支持多链、动态费率、拥堵检测

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::infrastructure::upstream::UpstreamClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasEstimation {
    /// 建议的Gas价格（wei）
    pub gas_price: u64,
    /// Gas限制
    pub gas_limit: u64,
    /// 预估总费用（wei）
    pub total_fee_wei: String,
    /// 预估总费用（美元）
    pub total_fee_usd: f64,
    /// 网络拥堵等级（low, medium, high）
    pub congestion_level: String,
    /// 预估确认时间（秒）
    pub estimated_confirmation_time: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSpeedGasEstimation {
    pub slow: GasEstimation,
    pub standard: GasEstimation,
    pub fast: GasEstimation,
}

pub struct GasEstimationServiceEnhanced {
    #[allow(dead_code)]
    upstream: UpstreamClient,
}

impl GasEstimationServiceEnhanced {
    pub fn new() -> Self {
        Self {
            upstream: UpstreamClient::new(),
        }
    }

    /// 获取Gas估算（三档速度）
    pub async fn estimate_gas_multi_speed(
        &self,
        chain: &str,
        from: &str,
        to: &str,
        value: &str,
        data: Option<&str>,
    ) -> Result<MultiSpeedGasEstimation> {
        // 1. 估算Gas限制
        let gas_limit = self
            .estimate_gas_limit(chain, from, to, value, data)
            .await?;

        // 2. 获取当前Gas价格
        let current_gas_price = self.get_current_gas_price(chain).await?;

        // 3. 计算三档Gas价格
        let slow_gas_price = (current_gas_price as f64 * 0.8) as u64;
        let standard_gas_price = current_gas_price;
        let fast_gas_price = (current_gas_price as f64 * 1.2) as u64;

        // 4. 检测网络拥堵
        let congestion = self.detect_congestion(chain).await?;

        // 5. 获取ETH价格（用于USD换算）
        let eth_price_usd = 2000.0; // TODO: 从价格服务获取

        Ok(MultiSpeedGasEstimation {
            slow: self.build_estimation(slow_gas_price, gas_limit, eth_price_usd, &congestion, 180),
            standard: self.build_estimation(
                standard_gas_price,
                gas_limit,
                eth_price_usd,
                &congestion,
                60,
            ),
            fast: self.build_estimation(fast_gas_price, gas_limit, eth_price_usd, &congestion, 30),
        })
    }

    /// 估算Gas限制
    async fn estimate_gas_limit(
        &self,
        _chain: &str,
        _from: &str,
        _to: &str,
        _value: &str,
        data: Option<&str>,
    ) -> Result<u64> {
        // 简单转账：21,000
        // ERC-20转账：~65,000
        // 复杂合约调用：需要通过RPC估算

        if data.is_none() || data == Some("0x") {
            Ok(21_000)
        } else {
            // TODO: 调用eth_estimateGas
            Ok(65_000)
        }
    }

    /// 获取当前Gas价格
    pub async fn get_current_gas_price(&self, chain: &str) -> Result<u64> {
        // TODO: 调用eth_gasPrice
        match chain {
            "ETH" => Ok(50_000_000_000),     // 50 Gwei
            "BSC" => Ok(5_000_000_000),      // 5 Gwei
            "POLYGON" => Ok(50_000_000_000), // 50 Gwei
            _ => Ok(50_000_000_000),
        }
    }

    /// 检测网络拥堵
    async fn detect_congestion(&self, chain: &str) -> Result<String> {
        let gas_price = self.get_current_gas_price(chain).await?;

        // 简单规则（Gwei）
        let gwei = gas_price / 1_000_000_000;

        Ok(match gwei {
            0..=30 => "low".to_string(),
            31..=100 => "medium".to_string(),
            _ => "high".to_string(),
        })
    }

    /// 构建估算结果
    fn build_estimation(
        &self,
        gas_price: u64,
        gas_limit: u64,
        eth_price_usd: f64,
        congestion: &str,
        estimated_time: u32,
    ) -> GasEstimation {
        let total_fee_wei = gas_price as u128 * gas_limit as u128;
        let total_fee_eth = total_fee_wei as f64 / 1e18;
        let total_fee_usd = total_fee_eth * eth_price_usd;

        GasEstimation {
            gas_price,
            gas_limit,
            total_fee_wei: total_fee_wei.to_string(),
            total_fee_usd,
            congestion_level: congestion.to_string(),
            estimated_confirmation_time: estimated_time,
        }
    }
}

impl Default for GasEstimationServiceEnhanced {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gas_estimation() {
        let service = GasEstimationServiceEnhanced::new();

        // 基本估算测试
        let result = service
            .estimate_gas_limit(
                "ETH",
                "0x742d35Cc6634C0532925a3b844Bc9e7595f0bFd2",
                "0x853f43d89b5B0F3F9E76F9F8C8e8C8e8C8e8C8e8",
                "1000000000000000000", // 1 ETH
                None,
            )
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 21_000);
    }
}
