//! 余额同步事件驱动服务
//!
//! 企业级实现：交易确认后实时同步余额
//! 解决问题：G.2 - 钱包余额同步未实时

use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;

use crate::service::{
    balance_sync_service::BalanceSyncService, blockchain_client::BlockchainClient,
};

/// 余额同步事件类型
#[derive(Debug, Clone)]
pub enum BalanceSyncEvent {
    /// 交易确认（需要同步余额）
    TransactionConfirmed {
        chain: String,
        address: String,
        tx_hash: String,
    },
    /// 手动触发同步
    ManualSync { chain: String, address: String },
}

/// 余额同步事件处理器
pub struct BalanceSyncEventHandler {
    pool: PgPool,
    balance_sync_service: Arc<BalanceSyncService>,
}

impl BalanceSyncEventHandler {
    /// 创建事件处理器
    pub fn new(pool: PgPool, blockchain_client: Arc<BlockchainClient>) -> Self {
        let balance_sync_service =
            Arc::new(BalanceSyncService::new(pool.clone(), blockchain_client));

        Self {
            pool,
            balance_sync_service,
        }
    }

    /// 处理余额同步事件
    pub async fn handle_event(&self, event: BalanceSyncEvent) -> Result<()> {
        match event {
            BalanceSyncEvent::TransactionConfirmed {
                chain,
                address,
                tx_hash,
            } => {
                tracing::info!(
                    "Transaction confirmed, syncing balance: chain={}, address={}, tx_hash={}",
                    chain,
                    address,
                    tx_hash
                );

                // ✅ 查找钱包ID
                let wallet_id_opt = sqlx::query_scalar::<_, Option<uuid::Uuid>>(
                    "SELECT id FROM wallets WHERE address = $1 AND chain_id = (SELECT chain_id FROM public.transactions WHERE tx_hash = $2 LIMIT 1)"
                )
                .bind(&address)
                .bind(&tx_hash)
                .fetch_optional(&self.pool)
                .await?
                .flatten();

                if let Some(wallet_id) = wallet_id_opt {
                    // 同步余额（需要3个参数：wallet_id, address, chain）
                    self.balance_sync_service
                        .sync_wallet_balance(wallet_id, &address, &chain)
                        .await
                        .ok();
                } else {
                    tracing::warn!("Wallet not found for address: {}", address);
                }

                // ✅ 更新交易元数据标记已同步（使用metadata字段）
                sqlx::query(
                    r#"UPDATE public.transactions 
                       SET metadata = jsonb_set(COALESCE(metadata, '{}'::jsonb), '{balance_synced}', 'true'::jsonb), 
                           updated_at = CURRENT_TIMESTAMP 
                       WHERE tx_hash = $1"#
                )
                .bind(&tx_hash)
                .execute(&self.pool)
                .await?;

                Ok(())
            }
            BalanceSyncEvent::ManualSync { chain, address } => {
                tracing::info!("Manual balance sync: chain={}, address={}", chain, address);

                // ✅ 查找钱包ID
                let wallet_id_opt = sqlx::query_scalar::<_, Option<uuid::Uuid>>(
                    "SELECT id FROM wallets WHERE address = $1 LIMIT 1",
                )
                .bind(&address)
                .fetch_optional(&self.pool)
                .await?
                .flatten();

                if let Some(wallet_id) = wallet_id_opt {
                    self.balance_sync_service
                        .sync_wallet_balance(wallet_id, &address, &chain)
                        .await
                        .ok();
                    Ok(())
                } else {
                    anyhow::bail!("Wallet not found for address: {}", address)
                }
            }
        }
    }

    /// 批量处理待同步的交易
    pub async fn process_pending_syncs(&self) -> Result<()> {
        // ✅ 查询已确认但未同步余额的交易（使用metadata检查）
        #[derive(sqlx::FromRow)]
        struct PendingTx {
            chain_id: i64,
            from_addr: String,
            tx_hash: String,
        }

        let pending_txs = sqlx::query_as::<_, PendingTx>(
            r#"
            SELECT chain_id, from_addr, tx_hash
            FROM public.transactions
            WHERE status = 'confirmed' 
              AND (metadata->>'balance_synced')::boolean IS NOT TRUE
              AND created_at > NOW() - INTERVAL '24 hours'
            LIMIT 50
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        if pending_txs.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "Found {} transactions pending balance sync",
            pending_txs.len()
        );

        for tx in pending_txs {
            let chain = format!("{}", tx.chain_id); // 简化：使用chain_id
            let tx_hash_clone = tx.tx_hash.clone();

            if let Err(e) = self
                .handle_event(BalanceSyncEvent::TransactionConfirmed {
                    chain,
                    address: tx.from_addr,
                    tx_hash: tx.tx_hash,
                })
                .await
            {
                tracing::warn!("Failed to sync balance for tx {}: {}", tx_hash_clone, e);
            }
        }

        Ok(())
    }
}
