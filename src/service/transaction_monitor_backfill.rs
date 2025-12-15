//! 交易监控Gas费用回填服务
//!
//! 企业级实现：监控交易确认后回填实际Gas费用到审计日志
//! 解决问题：D.2 - 费用审计日志缺少回填机制

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::PgPool;
use tokio::time::{sleep, Duration};

use crate::service::blockchain_client::BlockchainClient;

/// Gas费用回填服务
pub struct GasBackfillService {
    pool: PgPool,
    blockchain_client: Arc<BlockchainClient>,
}

impl GasBackfillService {
    /// 创建回填服务
    pub fn new(pool: PgPool, blockchain_client: Arc<BlockchainClient>) -> Self {
        Self {
            pool,
            blockchain_client,
        }
    }

    /// 启动后台回填任务
    ///
    /// # 功能
    /// 1. 定期扫描未回填的审计记录
    /// 2. 查询链上交易receipt
    /// 3. 回填实际Gas费用
    pub async fn start_background_backfill(self: Arc<Self>) {
        tracing::info!("Starting gas fee backfill service...");

        loop {
            if let Err(e) = self.backfill_batch().await {
                tracing::error!("Gas fee backfill error: {}", e);
            }

            // 每30秒执行一次
            sleep(Duration::from_secs(30)).await;
        }
    }

    /// 批量回填
    async fn backfill_batch(&self) -> Result<()> {
        // ✅ 查询未回填的记录（使用sqlx::query_as避免编译时验证）
        #[derive(sqlx::FromRow)]
        #[allow(dead_code)]
        struct AuditRecord {
            id: uuid::Uuid,
            chain: String,
            tx_hash: Option<String>,
            wallet_address: Option<String>,
        }

        let records = sqlx::query_as::<_, AuditRecord>(
            r#"
            SELECT id, chain, tx_hash, wallet_address
            FROM gas.fee_audit
            WHERE tx_hash IS NOT NULL 
              AND gas_fee_native IS NULL
              AND created_at > NOW() - INTERVAL '7 days'
            ORDER BY created_at DESC
            LIMIT 100
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch audit records")?;

        if records.is_empty() {
            return Ok(());
        }

        tracing::info!("Found {} audit records to backfill", records.len());

        for record in records {
            let tx_hash = record.tx_hash.unwrap();
            let chain = record.chain;

            // 查询交易receipt
            match self
                .blockchain_client
                .get_transaction_receipt(&chain, &tx_hash)
                .await
            {
                Ok(Some(receipt)) => {
                    // 计算实际Gas费用
                    if let (Some(gas_used), Some(gas_price_str)) =
                        (receipt.gas_used, receipt.effective_gas_price)
                    {
                        // gas_price_str是wei单位的字符串
                        if let Ok(gas_price) = gas_price_str.parse::<u128>() {
                            let gas_fee_wei = gas_used as u128 * gas_price;
                            let gas_fee_eth = gas_fee_wei as f64 / 1e18;

                            // ✅ 回填到数据库（不使用backfilled_at字段，使用metadata）
                            let result = sqlx::query(
                                r#"
                                UPDATE gas.fee_audit
                                SET gas_fee_native = $1, 
                                    metadata = CASE 
                                        WHEN metadata IS NULL THEN jsonb_build_object('backfilled_at', CURRENT_TIMESTAMP)
                                        ELSE jsonb_set(metadata, '{backfilled_at}', to_jsonb(CURRENT_TIMESTAMP))
                                    END
                                WHERE id = $2
                                "#
                            )
                            .bind(gas_fee_eth)
                            .bind(record.id)
                            .execute(&self.pool)
                            .await;

                            if result.is_err() {
                                tracing::warn!(
                                    "Failed to backfill gas fee for record {}: {:?}",
                                    record.id,
                                    result.err()
                                );
                                continue;
                            }

                            tracing::info!(
                                "Backfilled gas fee: audit_id={}, tx_hash={}, gas_fee={}",
                                record.id,
                                tx_hash,
                                gas_fee_eth
                            );
                        }
                    }
                }
                Ok(None) => {
                    // 交易还未确认，跳过
                    tracing::debug!("Transaction not yet confirmed: tx_hash={}", tx_hash);
                }
                Err(e) => {
                    tracing::warn!("Failed to get receipt for tx_hash={}: {}", tx_hash, e);
                }
            }
        }

        Ok(())
    }

    /// 手动回填单个交易
    pub async fn backfill_single(&self, tx_hash: &str, chain: &str) -> Result<()> {
        let receipt = self
            .blockchain_client
            .get_transaction_receipt(chain, tx_hash)
            .await?
            .context("Transaction not found")?;

        if let (Some(gas_used), Some(gas_price_str)) =
            (receipt.gas_used, receipt.effective_gas_price)
        {
            if let Ok(gas_price) = gas_price_str.parse::<u128>() {
                let gas_fee_wei = gas_used as u128 * gas_price;
                let gas_fee_eth = gas_fee_wei as f64 / 1e18;

                sqlx::query(
                    r#"
                    UPDATE gas.fee_audit
                    SET gas_fee_native = $1, 
                        metadata = CASE 
                            WHEN metadata IS NULL THEN jsonb_build_object('backfilled_at', CURRENT_TIMESTAMP)
                            ELSE jsonb_set(metadata, '{backfilled_at}', to_jsonb(CURRENT_TIMESTAMP))
                        END
                    WHERE tx_hash = $2
                    "#
                )
                .bind(gas_fee_eth)
                .bind(tx_hash)
                .execute(&self.pool)
                .await?;

                tracing::info!(
                    "Backfilled gas fee for tx_hash={}: {}",
                    tx_hash,
                    gas_fee_eth
                );
            }
        }

        Ok(())
    }
}
