//! 跨链桥事件监听服务
//! 企业级实现：源链和目标链双向监听，实时更新跨链状态

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::time::interval;
use uuid::Uuid;

use crate::service::blockchain_client::BlockchainClient;

const POLL_INTERVAL_SECS: u64 = 30;
#[allow(dead_code)]
const MAX_RETRIES: u32 = 5;

/// 跨链交易状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossChainStatus {
    /// 源链交易待确认
    SourcePending,
    /// 源链交易已确认，等待跨链桥处理
    SourceConfirmed,
    /// 跨链桥处理中
    BridgeProcessing,
    /// 目标链交易待确认
    DestinationPending,
    /// 目标链交易已确认（完成）
    DestinationConfirmed,
    /// 失败
    Failed,
}

/// 跨链交易记录
#[derive(Debug, Clone)]
struct CrossChainTransaction {
    id: Uuid,
    user_id: Uuid,
    source_chain: String,
    source_tx_hash: String,
    destination_chain: String,
    destination_tx_hash: Option<String>,
    status: CrossChainStatus,
    #[allow(dead_code)]
    amount: String,
    #[allow(dead_code)]
    token_symbol: String,
}

/// 跨链事件监听服务
pub struct CrossChainEventListener {
    pool: PgPool,
    blockchain_client: Arc<BlockchainClient>,
}

impl CrossChainEventListener {
    pub fn new(pool: PgPool, blockchain_client: Arc<BlockchainClient>) -> Self {
        Self {
            pool,
            blockchain_client,
        }
    }

    /// 启动后台监听任务
    pub async fn start_background_listener(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));

        tracing::info!(
            "Cross-chain event listener started, interval={}s",
            POLL_INTERVAL_SECS
        );

        loop {
            ticker.tick().await;

            // 处理待确认的跨链交易
            match self.process_pending_cross_chain_transactions().await {
                Ok(processed) => {
                    if processed > 0 {
                        tracing::info!(
                            count = processed,
                            "Processed pending cross-chain transactions"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        "Failed to process cross-chain transactions"
                    );
                }
            }
        }
    }

    /// 处理待确认的跨链交易
    async fn process_pending_cross_chain_transactions(&self) -> Result<usize> {
        // 查询所有非最终状态的跨链交易
        let pending_txs = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                String,
                String,
                String,
                Option<String>,
                String,
                String,
                String,
            ),
        >(
            "SELECT id, user_id, source_chain, source_tx_hash, destination_chain,
                    destination_tx_hash, status, amount, token_symbol
             FROM cross_chain_transactions
             WHERE status NOT IN ('DestinationConfirmed', 'Failed')
               AND created_at > NOW() - INTERVAL '7 days'
             ORDER BY created_at ASC
             LIMIT 50",
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query pending cross-chain transactions")?;

        if pending_txs.is_empty() {
            return Ok(0);
        }

        let mut processed = 0;

        for (
            id,
            user_id,
            source_chain,
            source_tx_hash,
            destination_chain,
            destination_tx_hash,
            status,
            amount,
            token_symbol,
        ) in pending_txs
        {
            let current_status = self.parse_status(&status);

            let tx = CrossChainTransaction {
                id,
                user_id,
                source_chain: source_chain.clone(),
                source_tx_hash: source_tx_hash.clone(),
                destination_chain: destination_chain.clone(),
                destination_tx_hash: destination_tx_hash.clone(),
                status: current_status.clone(),
                amount,
                token_symbol,
            };

            match self.update_transaction_status(tx).await {
                Ok(updated) => {
                    if updated {
                        processed += 1;
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = ?e,
                        tx_id = %id,
                        source_tx = %source_tx_hash,
                        "Failed to update cross-chain transaction"
                    );
                }
            }
        }

        Ok(processed)
    }

    /// 更新跨链交易状态
    async fn update_transaction_status(&self, tx: CrossChainTransaction) -> Result<bool> {
        match tx.status {
            CrossChainStatus::SourcePending => {
                // 检查源链交易是否已确认
                self.check_source_transaction(&tx).await
            }
            CrossChainStatus::SourceConfirmed => {
                // 查询跨链桥，检查是否已开始处理
                self.check_bridge_processing(&tx).await
            }
            CrossChainStatus::BridgeProcessing => {
                // 检查目标链交易是否已创建
                self.check_destination_transaction(&tx).await
            }
            CrossChainStatus::DestinationPending => {
                // 检查目标链交易是否已确认
                self.check_destination_confirmation(&tx).await
            }
            _ => Ok(false),
        }
    }

    /// 检查源链交易
    async fn check_source_transaction(&self, tx: &CrossChainTransaction) -> Result<bool> {
        let receipt = self
            .blockchain_client
            .get_transaction_receipt(&tx.source_chain, &tx.source_tx_hash)
            .await?;

        if let Some(receipt) = receipt {
            if receipt.status == Some(1) && receipt.confirmations >= 12 {
                // 源链交易已确认
                self.update_status(tx.id, CrossChainStatus::SourceConfirmed)
                    .await?;

                tracing::info!(
                    tx_id = %tx.id,
                    source_tx = %tx.source_tx_hash,
                    "Source transaction confirmed"
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 检查跨链桥处理状态
    async fn check_bridge_processing(&self, tx: &CrossChainTransaction) -> Result<bool> {
        // TODO: 实际实现需要调用跨链桥API
        // 例如：LayerZero, Wormhole, Stargate等

        // 简化实现：假设在24小时内会有目标链交易
        let age_hours = self.get_transaction_age_hours(tx.id).await?;

        if age_hours > 24 {
            // 超时，标记为失败
            self.update_status(tx.id, CrossChainStatus::Failed).await?;

            tracing::warn!(
                tx_id = %tx.id,
                "Cross-chain transaction timeout"
            );

            return Ok(true);
        }

        // 尝试查询目标链是否已有交易
        // 实际实现：通过跨链桥API获取destination_tx_hash

        Ok(false)
    }

    /// 检查目标链交易
    async fn check_destination_transaction(&self, tx: &CrossChainTransaction) -> Result<bool> {
        if let Some(dest_tx_hash) = &tx.destination_tx_hash {
            let receipt = self
                .blockchain_client
                .get_transaction_receipt(&tx.destination_chain, dest_tx_hash)
                .await?;

            if let Some(_receipt) = receipt {
                // 目标链交易已上链
                self.update_status(tx.id, CrossChainStatus::DestinationPending)
                    .await?;

                tracing::info!(
                    tx_id = %tx.id,
                    dest_tx = %dest_tx_hash,
                    "Destination transaction found"
                );

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// 检查目标链交易确认
    async fn check_destination_confirmation(&self, tx: &CrossChainTransaction) -> Result<bool> {
        if let Some(dest_tx_hash) = &tx.destination_tx_hash {
            let receipt = self
                .blockchain_client
                .get_transaction_receipt(&tx.destination_chain, dest_tx_hash)
                .await?;

            if let Some(receipt) = receipt {
                if receipt.status == Some(1) && receipt.confirmations >= 12 {
                    // 目标链交易已确认，跨链完成
                    self.update_status(tx.id, CrossChainStatus::DestinationConfirmed)
                        .await?;

                    tracing::info!(
                        tx_id = %tx.id,
                        dest_tx = %dest_tx_hash,
                        "Cross-chain transaction completed"
                    );

                    // 发送通知给用户
                    self.notify_user_cross_chain_completed(tx).await?;

                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// 更新状态
    async fn update_status(&self, tx_id: Uuid, new_status: CrossChainStatus) -> Result<()> {
        sqlx::query(
            "UPDATE cross_chain_transactions
             SET status = $1, updated_at = CURRENT_TIMESTAMP
             WHERE id = $2",
        )
        .bind(format!("{:?}", new_status))
        .bind(tx_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 获取交易年龄（小时）
    async fn get_transaction_age_hours(&self, tx_id: Uuid) -> Result<i64> {
        let age: Option<i64> = sqlx::query_scalar(
            "SELECT EXTRACT(HOUR FROM NOW() - created_at)::BIGINT
             FROM cross_chain_transactions
             WHERE id = $1",
        )
        .bind(tx_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(age.unwrap_or(0))
    }

    /// 通知用户跨链完成
    async fn notify_user_cross_chain_completed(&self, tx: &CrossChainTransaction) -> Result<()> {
        // TODO: 实际实现：发送推送通知、邮件、短信等
        tracing::info!(
            user_id = %tx.user_id,
            tx_id = %tx.id,
            "Notifying user of cross-chain completion"
        );

        Ok(())
    }

    /// 解析状态字符串
    fn parse_status(&self, status: &str) -> CrossChainStatus {
        match status {
            "SourcePending" => CrossChainStatus::SourcePending,
            "SourceConfirmed" => CrossChainStatus::SourceConfirmed,
            "BridgeProcessing" => CrossChainStatus::BridgeProcessing,
            "DestinationPending" => CrossChainStatus::DestinationPending,
            "DestinationConfirmed" => CrossChainStatus::DestinationConfirmed,
            "Failed" => CrossChainStatus::Failed,
            _ => CrossChainStatus::SourcePending,
        }
    }
}
