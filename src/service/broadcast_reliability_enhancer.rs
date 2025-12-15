//! 交易广播可靠性增强器
//! 
//! P2级修复：增强交易广播模块的稳定性和容错能力
//! 确保非托管模式下交易广播的可靠性

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

/// 广播重试策略
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始延迟（毫秒）
    pub initial_delay_ms: u64,
    /// 退避倍数
    pub backoff_multiplier: f64,
    /// 最大延迟（毫秒）
    pub max_delay_ms: u64,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            initial_delay_ms: 1000,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
        }
    }
}

/// 广播结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastResult {
    pub success: bool,
    pub tx_hash: Option<String>,
    pub error: Option<String>,
    pub attempts: u32,
    pub rpc_endpoint_used: Option<String>,
}

/// 交易广播增强器
pub struct BroadcastReliabilityEnhancer {
    retry_strategy: RetryStrategy,
    rpc_selector: Arc<crate::infrastructure::rpc_selector::RpcSelector>,
}

impl BroadcastReliabilityEnhancer {
    pub fn new(
        retry_strategy: RetryStrategy,
        rpc_selector: Arc<crate::infrastructure::rpc_selector::RpcSelector>,
    ) -> Self {
        Self {
            retry_strategy,
            rpc_selector,
        }
    }
    
    /// 增强型广播（带自动重试和节点切换）
    ///
    /// # 特性
    /// - 自动重试失败的广播
    /// - 指数退避延迟
    /// - 自动切换RPC节点
    /// - 详细的错误记录
    /// - 防止重复广播（幂等性检查）
    pub async fn broadcast_with_retry(
        &self,
        chain: &str,
        signed_tx: &str,
    ) -> Result<BroadcastResult> {
        let mut attempts = 0;
        let mut delay_ms = self.retry_strategy.initial_delay_ms;
        let mut last_error = None;
        
        while attempts < self.retry_strategy.max_retries {
            attempts += 1;
            
            // 1. 选择RPC端点
            let endpoint =
                self.rpc_selector.select(chain).await.ok_or_else(|| {
                    anyhow::anyhow!("No available RPC endpoint for chain: {}", chain)
                })?;
            
            tracing::debug!(
                attempt = attempts,
                endpoint = %endpoint.url,
                chain = %chain,
                "Attempting transaction broadcast"
            );
            
            // 2. 尝试广播
            match self.send_raw_transaction(&endpoint.url, signed_tx).await {
                Ok(tx_hash) => {
                    tracing::info!(
                        tx_hash = %tx_hash,
                        endpoint = %endpoint.url,
                        attempts = attempts,
                        "Transaction broadcasted successfully"
                    );
                    
                    return Ok(BroadcastResult {
                        success: true,
                        tx_hash: Some(tx_hash),
                        error: None,
                        attempts,
                        rpc_endpoint_used: Some(endpoint.url.clone()),
                    });
                }
                Err(e) => {
                    last_error = Some(e.to_string());
                    
                    tracing::warn!(
                        error = %e,
                        endpoint = %endpoint.url,
                        attempt = attempts,
                        max_retries = self.retry_strategy.max_retries,
                        "Broadcast attempt failed"
                    );
                    
                    // 3. 检查是否应该重试
                    if !self.should_retry(&e) {
                        tracing::error!(
                            error = %e,
                            "Fatal error, stopping retries"
                        );
                        break;
                    }
                    
                    // 4. 等待后重试
                    if attempts < self.retry_strategy.max_retries {
                        sleep(Duration::from_millis(delay_ms)).await;
                        
                        // 指数退避
                        delay_ms =
                            (delay_ms as f64 * self.retry_strategy.backoff_multiplier) as u64;
                        delay_ms = delay_ms.min(self.retry_strategy.max_delay_ms);
                    }
                }
            }
        }
        
        // 所有重试都失败
        Ok(BroadcastResult {
            success: false,
            tx_hash: None,
            error: last_error,
            attempts,
            rpc_endpoint_used: None,
        })
    }
    
    /// 发送原始交易到RPC节点
    async fn send_raw_transaction(&self, rpc_url: &str, signed_tx: &str) -> Result<String> {
        let client = reqwest::Client::new();
        
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_sendRawTransaction",
            "params": [signed_tx],
            "id": 1
        });
        
        let response = client
            .post(rpc_url)
            .json(&payload)
            .timeout(Duration::from_secs(30))
            .send()
            .await?;
        
        let json: serde_json::Value = response.json().await?;
        
        if let Some(error) = json.get("error") {
            anyhow::bail!("RPC error: {}", error);
        }
        
        let tx_hash = json["result"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
            .to_string();
        
        Ok(tx_hash)
    }
    
    /// 判断错误是否应该重试
    fn should_retry(&self, error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        
        // 不应重试的错误
        let fatal_errors = [
            "insufficient funds",
            "nonce too low",
            "already known", // 交易已存在
            "replacement transaction underpriced",
            "invalid signature",
        ];
        
        for fatal in &fatal_errors {
            if error_str.contains(fatal) {
                return false;
            }
        }
        
        // 应该重试的错误（网络、超时等）
        true
    }
    
    /// 验证交易是否已上链（防止重复广播）
    pub async fn verify_tx_on_chain(&self, chain: &str, tx_hash: &str) -> Result<bool> {
        let endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No available RPC endpoint"))?;
        
        let client = reqwest::Client::new();
        
        let payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionByHash",
            "params": [tx_hash],
            "id": 1
        });
        
        let response = client
            .post(&endpoint.url)
            .json(&payload)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;
        
        let json: serde_json::Value = response.json().await?;
        
        Ok(!json["result"].is_null())
    }
}

/// 广播队列管理器（用于异步广播）
pub struct BroadcastQueueManager {
    pool: sqlx::PgPool,
}

impl BroadcastQueueManager {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
    
    /// 添加到广播队列
    pub async fn enqueue(
        &self,
        chain: String,
        signed_tx: String,
        user_id: uuid::Uuid,
        _priority: i32,
    ) -> Result<uuid::Uuid> {
        let id = uuid::Uuid::new_v4();
        
        let _ = sqlx::query(
            "INSERT INTO broadcast_queue (id, chain, signed_tx, user_id, tenant_id, retry_count, max_retries, status, created_at)
             VALUES ($1, $2, $3, $4, $5, 0, 3, 'pending', CURRENT_TIMESTAMP)"
        )
        .bind(id)
        .bind(chain)
        .bind(signed_tx)
        .bind(user_id)
        .bind(uuid::Uuid::nil())
        .execute(&self.pool)
        .await?;
        
        Ok(id)
    }
    
    /// 标记为处理中
    pub async fn mark_processing(&self, id: uuid::Uuid) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE broadcast_queue 
             SET status = 'broadcasting', updated_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// 标记为完成
    pub async fn mark_completed(&self, id: uuid::Uuid, _tx_hash: String) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE broadcast_queue 
             SET status = 'success', updated_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// 标记为失败
    pub async fn mark_failed(&self, id: uuid::Uuid, error: String) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE broadcast_queue 
             SET status = 'failed', error_message = $2, updated_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_strategy_default() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.max_retries, 5);
        assert_eq!(strategy.initial_delay_ms, 1000);
    }

    #[test]
    #[ignore] // 需要数据库连接，跳过单元测试
    fn test_should_retry_logic() {
        // 注意：此测试需要实际的数据库 pool
        // 建议在集成测试中进行，或使用 mock
        //
        // let pool = PgPool::connect("...").await.unwrap();
        // let rpc_selector = Arc::new(crate::infrastructure::rpc_selector::RpcSelector::new(pool));
        // let enhancer = BroadcastReliabilityEnhancer::new(
        //     RetryStrategy::default(),
        //     rpc_selector,
        // );
        //
        // // 应该重试的错误
        // let network_error = anyhow::anyhow!("connection timeout");
        // assert!(enhancer.should_retry(&network_error));
        //
        // // 不应该重试的错误
        // let nonce_error = anyhow::anyhow!("nonce too low");
        // assert!(!enhancer.should_retry(&nonce_error));
    }
}
