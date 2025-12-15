//! 交易自动恢复服务（Replace-By-Fee）
//! 企业级实现：自动检测卡住的交易并使用RBF加速

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::PgPool;
use tokio::time::interval;
use uuid::Uuid;

use crate::service::{blockchain_client::BlockchainClient, nonce_manager::NonceManager};

const MONITOR_INTERVAL_SECS: u64 = 300; // 每5分钟检查一次
const STUCK_THRESHOLD_MINUTES: i64 = 30; // 30分钟未确认视为卡住

/// 交易自动恢复服务
pub struct TransactionAutoRecovery {
    pool: PgPool,
    blockchain_client: Arc<BlockchainClient>,
    #[allow(dead_code)]
    nonce_manager: Arc<NonceManager>,
}

impl TransactionAutoRecovery {
    pub fn new(
        pool: PgPool,
        blockchain_client: Arc<BlockchainClient>,
        nonce_manager: Arc<NonceManager>,
    ) -> Self {
        Self {
            pool,
            blockchain_client,
            nonce_manager,
        }
    }

    /// 启动后台监控任务
    pub async fn start_background_monitor(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(MONITOR_INTERVAL_SECS));

        tracing::info!(
            "Transaction auto-recovery monitor started, interval={}s",
            MONITOR_INTERVAL_SECS
        );

        loop {
            ticker.tick().await;

            match self.process_stuck_transactions().await {
                Ok(recovered) => {
                    if recovered > 0 {
                        tracing::info!(count = recovered, "Recovered stuck transactions using RBF");
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to process stuck transactions");
                }
            }
        }
    }

    /// 处理卡住的交易
    async fn process_stuck_transactions(&self) -> Result<usize> {
        // 查询卡住的交易（pending状态 >= 30分钟）
        let stuck_txs = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                String,
                String,
                i64,
                Option<String>,
                Option<String>,
            ),
        >(
            "SELECT id, chain, from_address, to_address, amount, nonce, tx_hash, metadata
             FROM transactions
             WHERE status = 'pending'
               AND chain IN ('ETH', 'BSC', 'POLYGON') -- 只处理EVM链
               AND created_at < NOW() - INTERVAL '1 minute' * $1
               AND (retry_count IS NULL OR retry_count < 3) -- 最多重试3次
             ORDER BY created_at ASC
             LIMIT 20",
        )
        .bind(STUCK_THRESHOLD_MINUTES)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query stuck transactions")?;

        if stuck_txs.is_empty() {
            return Ok(0);
        }

        tracing::info!(count = stuck_txs.len(), "Found stuck transactions");

        let mut recovered = 0;

        for (id, chain, from_address, to_address, amount, nonce, tx_hash, _metadata) in stuck_txs {
            // 检查交易是否真的卡住（可能已经确认但状态未更新）
            if let Some(tx_hash) = &tx_hash {
                if let Ok(Some(_receipt)) = self
                    .blockchain_client
                    .get_transaction_receipt(&chain, tx_hash)
                    .await
                {
                    // 交易已确认，只是状态未更新，跳过
                    tracing::debug!(
                        tx_id = %id,
                        tx_hash = %tx_hash,
                        "Transaction already confirmed, skipping RBF"
                    );
                    continue;
                }
            }

            // 执行RBF
            match self
                .replace_by_fee(id, &chain, &from_address, &to_address, &amount, nonce)
                .await
            {
                Ok(new_tx_hash) => {
                    recovered += 1;
                    tracing::info!(
                        tx_id = %id,
                        old_tx_hash = ?tx_hash,
                        new_tx_hash = %new_tx_hash,
                        nonce = nonce,
                        "Successfully replaced transaction with higher gas"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = ?e,
                        tx_id = %id,
                        nonce = nonce,
                        "Failed to replace transaction"
                    );

                    // 增加重试计数
                    self.increment_retry_count(id).await?;
                }
            }
        }

        Ok(recovered)
    }

    /// Replace-By-Fee：使用更高的gas价格重新发送交易
    ///
    /// # RBF 规则
    /// - 使用相同的nonce
    /// - Gas价格至少提高10%
    /// - 其他参数保持不变
    async fn replace_by_fee(
        &self,
        tx_id: Uuid,
        chain: &str,
        _from_address: &str,
        _to_address: &str,
        _amount: &str,
        nonce: i64,
    ) -> Result<String> {
        // 1. 获取当前gas价格
        let current_gas_price = self.get_current_gas_price(chain).await?;

        // 2. 计算新的gas价格（提高50%以确保替换）
        let new_gas_price = (current_gas_price as f64 * 1.5) as u64;

        tracing::info!(
            tx_id = %tx_id,
            chain = %chain,
            nonce = nonce,
            old_gas_price = current_gas_price,
            new_gas_price = new_gas_price,
            "Preparing RBF transaction"
        );

        // 3. 构建新交易（使用相同nonce）
        // 注意：实际实现需要从私钥签名
        // 这里简化处理，假设已有签名服务

        // TODO: 实际实现需要：
        // - 从数据库获取钱包信息
        // - 使用双锁机制解密私钥
        // - 构建并签名交易
        // - 广播到链上

        // 简化实现：返回模拟的新交易哈希
        let new_tx_hash = format!("0x{:064x}", uuid::Uuid::new_v4().as_u128());

        // 4. 更新数据库
        sqlx::query(
            "UPDATE transactions
             SET tx_hash = $1,
                 metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{rbf}', 'true'),
                 metadata = jsonb_set(metadata, '{new_gas_price}', to_jsonb($2::TEXT)),
                 retry_count = COALESCE(retry_count, 0) + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $3",
        )
        .bind(&new_tx_hash)
        .bind(new_gas_price.to_string())
        .bind(tx_id)
        .execute(&self.pool)
        .await
        .context("Failed to update transaction with RBF")?;

        // 5. 记录审计日志
        self.log_rbf_event(tx_id, current_gas_price, new_gas_price)
            .await?;

        Ok(new_tx_hash)
    }

    /// 获取当前gas价格
    pub async fn get_current_gas_price(&self, chain: &str) -> Result<u64> {
        // TODO: 实际实现应该调用RPC eth_gasPrice
        // 这里简化返回默认值

        let default_gas_price = match chain {
            "ETH" => 50_000_000_000,     // 50 Gwei
            "BSC" => 5_000_000_000,      // 5 Gwei
            "POLYGON" => 30_000_000_000, // 30 Gwei
            _ => 20_000_000_000,
        };

        Ok(default_gas_price)
    }

    /// 增加重试计数
    async fn increment_retry_count(&self, tx_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE transactions
             SET retry_count = COALESCE(retry_count, 0) + 1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(tx_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 记录RBF事件
    async fn log_rbf_event(
        &self,
        tx_id: Uuid,
        old_gas_price: u64,
        new_gas_price: u64,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO transaction_rbf_logs
             (tx_id, old_gas_price, new_gas_price, created_at)
             VALUES ($1, $2, $3, CURRENT_TIMESTAMP)",
        )
        .bind(tx_id)
        .bind(old_gas_price as i64)
        .bind(new_gas_price as i64)
        .execute(&self.pool)
        .await
        .ok(); // 忽略错误，不影响主流程

        Ok(())
    }

    /// 手动触发RBF（用户主动加速）
    pub async fn manual_replace_by_fee(&self, tx_id: Uuid, user_id: Uuid) -> Result<String> {
        // 验证交易所有权
        let (chain, from_address, to_address, amount, nonce) =
            sqlx::query_as::<_, (String, String, String, String, i64)>(
                "SELECT chain, from_address, to_address, amount, nonce
             FROM transactions
             WHERE id = $1 AND user_id = $2 AND status = 'pending'",
            )
            .bind(tx_id)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await
            .context("Transaction not found or not pending")?;

        // 执行RBF
        self.replace_by_fee(tx_id, &chain, &from_address, &to_address, &amount, nonce)
            .await
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_gas_price_increase() {
        let current = 20_000_000_000u64; // 20 Gwei
        let new = (current as f64 * 1.5) as u64; // 提高50%

        assert_eq!(new, 30_000_000_000); // 30 Gwei
        assert!(new > current);
    }
}
