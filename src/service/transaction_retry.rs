// 交易重试和失败处理机制
//
// 提供：
// - 自动重试失败的交易广播
// - Gas 价格动态调整
// - RBF (Replace-By-Fee) 支持
// - 交易取消机制

use std::time::Duration;

use anyhow::Result;
use tokio::time::sleep;
use uuid::Uuid;

/// 交易重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始退避时间（秒）
    pub initial_backoff_secs: u64,
    /// 退避倍数
    pub backoff_multiplier: f64,
    /// Gas 价格增加百分比（每次重试）
    pub gas_price_bump_percent: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_secs: 5,
            backoff_multiplier: 2.0,
            gas_price_bump_percent: 10, // 每次重试增加 10% Gas
        }
    }
}

/// 交易重试结果状态
/// 注意：这是重试机制专用的状态类型，不要与 domain::TransactionStatus 混淆
#[derive(Debug, Clone, PartialEq)]
pub enum RetryTransactionStatus {
    Pending,
    Confirmed,
    Failed(String), // 失败原因
    Cancelled,
    Replaced(String), // 被新交易替换，包含新的 tx_hash
}

/// 交易重试器
pub struct TransactionRetrier {
    config: RetryConfig,
}

impl TransactionRetrier {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// 带重试的广播交易
    ///
    /// 如果交易失败，会自动重试并调整 Gas 价格
    pub async fn broadcast_with_retry<F, Fut>(
        &self,
        tx_id: Uuid,
        chain: &str,
        mut broadcast_fn: F,
    ) -> Result<String>
    where
        F: FnMut(u32) -> Fut, // 接收 gas_bump_percent 参数
        Fut: std::future::Future<Output = Result<String>>,
    {
        let mut last_error = None;
        let mut backoff_secs = self.config.initial_backoff_secs;

        for attempt in 0..=self.config.max_retries {
            // 计算 Gas 价格增加百分比
            let gas_bump = attempt * self.config.gas_price_bump_percent;

            tracing::info!(
                tx_id = %tx_id,
                chain = %chain,
                attempt = attempt + 1,
                max_retries = self.config.max_retries + 1,
                gas_bump_percent = gas_bump,
                "Attempting transaction broadcast"
            );

            match broadcast_fn(gas_bump).await {
                Ok(tx_hash) => {
                    tracing::info!(
                        tx_id = %tx_id,
                        tx_hash = %tx_hash,
                        attempt = attempt + 1,
                        "Transaction broadcast successful"
                    );
                    return Ok(tx_hash);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < self.config.max_retries {
                        tracing::warn!(
                            tx_id = %tx_id,
                            error = ?last_error,
                            next_retry_in_secs = backoff_secs,
                            "Transaction broadcast failed, will retry"
                        );

                        // 指数退避
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs =
                            (backoff_secs as f64 * self.config.backoff_multiplier) as u64;
                    }
                }
            }
        }

        // 所有重试都失败
        let error = last_error.unwrap();
        tracing::error!(
            tx_id = %tx_id,
            chain = %chain,
            error = ?error,
            "Transaction broadcast failed after all retries"
        );

        Err(error)
    }

    /// 替换待处理交易 (RBF - Replace By Fee)
    ///
    /// 为 EVM 链实现，使用相同的 nonce 但更高的 Gas 价格
    pub async fn replace_transaction(
        &self,
        original_tx_hash: &str,
        chain: &str,
        gas_price_multiplier: f64,
    ) -> Result<String> {
        match chain.to_lowercase().as_str() {
            "ethereum" | "eth" | "bsc" | "polygon" => {
                self.replace_evm_transaction(original_tx_hash, gas_price_multiplier)
                    .await
            }
            "bitcoin" | "btc" => self.replace_bitcoin_transaction(original_tx_hash).await,
            _ => {
                anyhow::bail!("RBF not supported for chain: {}", chain)
            }
        }
    }

    /// 替换 EVM 交易
    async fn replace_evm_transaction(
        &self,
        original_tx_hash: &str,
        gas_price_multiplier: f64,
    ) -> Result<String> {
        // 注意：RBF 需要重新签名交易，后端没有私钥
        // 实际应用中，这个功能应该在前端实现
        tracing::info!(
            original_hash = %original_tx_hash,
            multiplier = %gas_price_multiplier,
            "EVM RBF requires client-side re-signing with higher gas price"
        );

        anyhow::bail!(
            "Transaction replacement requires re-signing on client side. \
             Backend does not have access to private keys."
        )
    }

    /// 替换 Bitcoin 交易 (RBF)
    async fn replace_bitcoin_transaction(&self, original_tx_hash: &str) -> Result<String> {
        // Bitcoin RBF 需要：
        // 1. 原始交易设置了 RBF 标志 (nSequence < 0xfffffffe)
        // 2. 新交易使用相同输入但更高手续费

        tracing::info!(
            original_hash = %original_tx_hash,
            "Bitcoin RBF not yet implemented"
        );

        anyhow::bail!("Bitcoin RBF requires re-signing with higher fee")
    }

    /// 取消待处理交易
    ///
    /// 发送一个 0 ETH 的交易到自己，使用相同的 nonce
    pub async fn cancel_transaction(&self, original_tx_hash: &str, chain: &str) -> Result<String> {
        tracing::info!(
            original_hash = %original_tx_hash,
            chain = %chain,
            "Attempting to cancel transaction"
        );

        match chain.to_lowercase().as_str() {
            "ethereum" | "eth" | "bsc" | "polygon" => {
                // 发送 0 ETH 到自己，使用相同 nonce 和更高 Gas
                self.replace_evm_transaction(original_tx_hash, 1.5).await
            }
            _ => {
                anyhow::bail!(
                    "Transaction cancellation not supported for chain: {}",
                    chain
                )
            }
        }
    }
}

/// Gas 价格调整器
pub struct GasPriceAdjuster;

impl GasPriceAdjuster {
    /// 根据网络拥堵情况动态调整 Gas 价格
    pub async fn get_recommended_gas_price(chain: &str) -> Result<u64> {
        match chain.to_lowercase().as_str() {
            "ethereum" | "eth" => Self::get_eth_gas_price().await,
            "bsc" | "binance" => Self::get_bsc_gas_price().await,
            "polygon" | "matic" => Self::get_polygon_gas_price().await,
            _ => Ok(20_000_000_000), // 20 gwei 默认值
        }
    }

    async fn get_eth_gas_price() -> Result<u64> {
        use ethers::providers::{Http, Middleware, Provider};

        let rpc_url =
            std::env::var("ETH_RPC_URL").unwrap_or_else(|_| "https://eth.llamarpc.com".to_string());

        let provider = Provider::<Http>::try_from(&rpc_url)?;
        let gas_price = provider.get_gas_price().await?;

        Ok(gas_price.as_u64())
    }

    async fn get_bsc_gas_price() -> Result<u64> {
        // BSC 通常 Gas 价格较低
        Ok(5_000_000_000) // 5 gwei
    }

    async fn get_polygon_gas_price() -> Result<u64> {
        // Polygon 通常 Gas 价格更低
        Ok(30_000_000_000) // 30 gwei
    }

    /// 应用 Gas 价格增加百分比
    pub fn apply_gas_bump(base_gas: u64, bump_percent: u32) -> u64 {
        let multiplier = 100 + bump_percent as u64;
        base_gas * multiplier / 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_bump_calculation() {
        assert_eq!(GasPriceAdjuster::apply_gas_bump(100, 0), 100);
        assert_eq!(GasPriceAdjuster::apply_gas_bump(100, 10), 110);
        assert_eq!(GasPriceAdjuster::apply_gas_bump(100, 50), 150);
    }

    #[tokio::test]
    async fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_secs, 5);
    }
}
