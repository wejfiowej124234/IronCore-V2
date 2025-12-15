// 交易监控与回填服务 - 生产级实现
// 监听交易确认，回填 fee_audit 表中的 gas_used 和 gas_fee_native
// 同时监控swap_transactions表的交易确认

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::PgPool;
use tokio::time::interval;

use crate::repository::SwapTransactionRepository;

const MONITOR_INTERVAL_SECS: u64 = 30; // 每30秒检查一次
const MAX_RETRIES_PER_TX: i32 = 20; // 最多重试20次
const BATCH_SIZE: i64 = 50; // 每批处理50笔交易

pub struct TransactionMonitor {
    pool: PgPool,
    blockchain_client: Arc<crate::service::blockchain_client::BlockchainClient>,
}

impl TransactionMonitor {
    pub fn new(
        pool: PgPool,
        blockchain_client: Arc<crate::service::blockchain_client::BlockchainClient>,
    ) -> Self {
        Self {
            pool,
            blockchain_client,
        }
    }

    /// 启动后台监控任务（持续运行）
    pub async fn start_background_monitor(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(MONITOR_INTERVAL_SECS));

        tracing::info!(
            "Transaction monitor started, interval={}s",
            MONITOR_INTERVAL_SECS
        );

        loop {
            ticker.tick().await;

            // 处理fee_audit表的交易
            match self.process_pending_transactions().await {
                Ok(processed) => {
                    if processed > 0 {
                        tracing::info!(count = processed, "Processed pending transactions");
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to process pending transactions");
                }
            }

            // 处理swap_transactions表的交易确认
            match self.process_pending_swap_transactions().await {
                Ok(processed) => {
                    if processed > 0 {
                        tracing::info!(count = processed, "Processed pending swap transactions");
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to process pending swap transactions");
                }
            }
        }
    }

    /// 处理待确认的swap交易（企业级实现）
    async fn process_pending_swap_transactions(&self) -> Result<usize> {
        let swap_repo = SwapTransactionRepository::new(self.pool.clone());

        // 查询需要更新确认数的swap交易（有tx_hash但状态为executing或pending）
        let pending_swaps = sqlx::query_as::<_, (String, String, String, Option<String>)>(
            r#"
            SELECT swap_id, network, tx_hash, status
            FROM swap_transactions
            WHERE tx_hash IS NOT NULL
              AND tx_hash != ''
              AND status IN ('executing', 'pending')
              AND created_at > CURRENT_TIMESTAMP - INTERVAL '1 hour'
            ORDER BY created_at ASC
            LIMIT $1
            "#,
        )
        .bind(BATCH_SIZE)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query pending swap transactions")?;

        if pending_swaps.is_empty() {
            return Ok(0);
        }

        tracing::debug!(
            count = pending_swaps.len(),
            "Found pending swap transactions to process"
        );

        let mut processed_count = 0;

        for (swap_id, network, tx_hash, _status) in pending_swaps {
            match self
                .update_swap_transaction_confirmations(&swap_id, &network, &tx_hash, &swap_repo)
                .await
            {
                Ok(updated) => {
                    if updated {
                        processed_count += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        swap_id = %swap_id,
                        tx_hash = %tx_hash,
                        network = %network,
                        error = ?e,
                        "Failed to update swap transaction confirmations"
                    );
                }
            }
        }

        Ok(processed_count)
    }

    /// 更新swap交易的确认数（企业级实现）
    async fn update_swap_transaction_confirmations(
        &self,
        swap_id: &str,
        network: &str,
        tx_hash: &str,
        swap_repo: &SwapTransactionRepository,
    ) -> Result<bool> {
        // 查询交易回执
        let receipt_opt = self
            .blockchain_client
            .get_transaction_receipt(network, tx_hash)
            .await
            .context("Failed to fetch transaction receipt")?;

        let receipt = match receipt_opt {
            Some(r) => r,
            None => {
                // 交易尚未确认
                tracing::debug!(swap_id = %swap_id, tx_hash = %tx_hash, "Swap transaction not yet confirmed");
                return Ok(false);
            }
        };

        // 验证交易状态
        let new_status = if receipt.status == Some(0) {
            tracing::warn!(
                swap_id = %swap_id,
                tx_hash = %tx_hash,
                "Swap transaction failed on-chain"
            );
            "failed"
        } else if receipt.confirmations >= 12 {
            // 达到标准确认数，标记为confirmed
            "confirmed"
        } else {
            // 仍在确认中，保持executing状态
            "executing"
        };

        // 获取gas_used
        let gas_used_str = receipt.gas_used.map(|v| v.to_string());

        // 更新swap交易状态和确认数
        let updated = swap_repo
            .update_status(
                swap_id,
                new_status,
                Some(tx_hash),
                None, // to_amount保持不变
                gas_used_str.as_deref(),
                Some(receipt.confirmations as i32),
            )
            .await
            .context("Failed to update swap transaction status")?;

        if updated {
            tracing::info!(
                swap_id = %swap_id,
                tx_hash = %tx_hash,
                status = %new_status,
                confirmations = receipt.confirmations,
                "Successfully updated swap transaction"
            );
        }

        Ok(updated)
    }

    /// 处理待确认的交易（单次批处理）
    async fn process_pending_transactions(&self) -> Result<usize> {
        // 查询需要回填的交易（tx_hash 不为空但 gas_used 为空）
        let pending_txs = sqlx::query_as::<
            _,
            (
                uuid::Uuid,
                String,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
            "SELECT id, chain, tx_hash, created_at
             FROM gas.fee_audit
             WHERE tx_hash IS NOT NULL
               AND tx_hash != ''
               AND gas_used IS NULL
               AND (retry_count IS NULL OR retry_count < $1)
               AND created_at > CURRENT_TIMESTAMP - INTERVAL '1 hour'
             ORDER BY created_at ASC
             LIMIT $2",
        )
        .bind(MAX_RETRIES_PER_TX)
        .bind(BATCH_SIZE)
        .fetch_all(&self.pool)
        .await
        .context("Failed to query pending transactions")?;

        if pending_txs.is_empty() {
            return Ok(0);
        }

        tracing::debug!(
            count = pending_txs.len(),
            "Found pending transactions to process"
        );

        let mut processed_count = 0;

        for (id, chain, tx_hash_opt, _created_at) in pending_txs {
            let tx_hash = match &tx_hash_opt {
                Some(hash) if !hash.is_empty() => hash,
                _ => continue,
            };

            match self.fetch_and_update_receipt(&chain, tx_hash, id).await {
                Ok(updated) => {
                    if updated {
                        processed_count += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        tx_hash = %tx_hash,
                        chain = %chain,
                        audit_id = %id,
                        error = ?e,
                        "Failed to process transaction"
                    );

                    // 增加重试计数
                    self.increment_retry_count(id).await?;
                }
            }
        }

        Ok(processed_count)
    }

    /// 获取交易回执并更新 fee_audit 表
    async fn fetch_and_update_receipt(
        &self,
        chain: &str,
        tx_hash: &str,
        audit_id: uuid::Uuid,
    ) -> Result<bool> {
        // 查询交易回执
        let receipt_opt = self
            .blockchain_client
            .get_transaction_receipt(chain, tx_hash)
            .await
            .context("Failed to fetch transaction receipt")?;

        let receipt = match receipt_opt {
            Some(r) => r,
            None => {
                // 交易尚未确认，增加重试计数
                tracing::debug!(tx_hash = %tx_hash, "Transaction not yet confirmed");
                return Ok(false);
            }
        };

        // 验证交易状态
        if receipt.status == Some(0) {
            tracing::warn!(
                tx_hash = %tx_hash,
                "Transaction failed on-chain, marking as failed"
            );
        }

        // 企业级实现：计算实际Gas费用（区块链网络费用，单位：原生代币，如 ETH）
        // Gas费用 = gas_used * effective_gas_price（这是区块链网络收取的交易执行费用）
        // 注意：这不是平台服务费，平台服务费在fee_audit表的platform_fee字段中
        let gas_fee_native = if let (Some(gas_used), Some(effective_price_hex)) =
            (receipt.gas_used, &receipt.effective_gas_price)
        {
            // effectiveGasPrice 是 Wei 的十六进制字符串
            let price_wei =
                u128::from_str_radix(effective_price_hex.trim_start_matches("0x"), 16).unwrap_or(0);

            // gas_fee_native = gas_used * price_wei / 1e18（Gas费用，区块链网络费用）
            let fee_wei = gas_used as u128 * price_wei;
            let fee_eth = fee_wei as f64 / 1_000_000_000_000_000_000.0;
            Some(fee_eth)
        } else {
            None
        };

        // 企业级实现：更新 fee_audit 表
        // fee_audit 表包含两个独立的费用字段：
        // 1. platform_fee: 平台服务费（钱包服务商收取的服务费用）
        // 2. gas_fee_native: Gas费用（区块链网络收取的交易执行费用）
        // 这两个费用是完全独立的，不能混淆
        let updated_rows = sqlx::query(
            "UPDATE gas.fee_audit
             SET gas_used = $1,
                 gas_fee_native = $2,  -- Gas费用：区块链网络费用（gas_used * effective_gas_price）
                 block_number = $3,
                 confirmations = $4,
                 tx_status = $5,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $6",
        )
        .bind(receipt.gas_used.map(|v| v as i64))
        .bind(gas_fee_native)
        .bind(receipt.block_number.map(|v| v as i64))
        .bind(receipt.confirmations as i32)
        .bind(receipt.status.map(|v| v as i16))
        .bind(audit_id)
        .execute(&self.pool)
        .await
        .context("Failed to update fee_audit")?;

        if updated_rows.rows_affected() > 0 {
            tracing::info!(
                tx_hash = %tx_hash,
                chain = %chain,
                gas_used = ?receipt.gas_used,
                gas_fee_native = ?gas_fee_native,
                "Successfully updated fee_audit with transaction receipt"
            );
            Ok(true)
        } else {
            tracing::warn!(audit_id = %audit_id, "No rows updated in fee_audit");
            Ok(false)
        }
    }

    /// 增加重试计数
    async fn increment_retry_count(&self, audit_id: uuid::Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE gas.fee_audit
             SET retry_count = COALESCE(retry_count, 0) + 1,
                 last_retry_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(audit_id)
        .execute(&self.pool)
        .await
        .context("Failed to increment retry count")?;

        Ok(())
    }

    /// 清理超时的待确认交易（标记为超时）
    pub async fn cleanup_stale_transactions(&self) -> Result<usize> {
        let result = sqlx::query(
            "UPDATE gas.fee_audit
             SET tx_status = -1,
                 updated_at = CURRENT_TIMESTAMP
             WHERE tx_hash IS NOT NULL
               AND gas_used IS NULL
               AND created_at < CURRENT_TIMESTAMP - INTERVAL '1 hour'
               AND (tx_status IS NULL OR tx_status != -1)",
        )
        .execute(&self.pool)
        .await
        .context("Failed to cleanup stale transactions")?;

        let cleaned = result.rows_affected() as usize;

        if cleaned > 0 {
            tracing::warn!(count = cleaned, "Cleaned up stale pending transactions");
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_calculate_gas_fee_from_wei() {
        // 示例：gas_used = 21000, effectiveGasPrice = 50 Gwei (0xBA43B7400 Wei)
        let gas_used = 21000u64;
        let price_wei = 50_000_000_000u128; // 50 Gwei

        let fee_wei = gas_used as u128 * price_wei;
        let fee_eth = fee_wei as f64 / 1e18;

        assert_eq!(fee_wei, 1_050_000_000_000_000u128);
        assert!((fee_eth - 0.00105).abs() < 1e-10);
    }

    #[test]
    fn test_parse_effective_gas_price_hex() {
        let hex = "0xBA43B7400";
        let price_wei = u128::from_str_radix(hex.trim_start_matches("0x"), 16).unwrap();
        assert_eq!(price_wei, 50_000_000_000);
    }
}
