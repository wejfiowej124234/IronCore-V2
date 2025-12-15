//! Nonce 管理器
//! 企业级实现：使用分布式锁防止多实例nonce冲突

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use sqlx::{PgPool, Row};
use tokio::sync::RwLock;

use crate::infrastructure::distributed_lock::DistributedLock;

/// Nonce 记录
#[derive(Debug, Clone)]
struct NonceRecord {
    #[allow(dead_code)]
    address: String,
    #[allow(dead_code)]
    chain: String,
    current_nonce: u64,
    pending_nonces: Vec<u64>,
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// Nonce 管理器（企业级：分布式锁保护）
pub struct NonceManager {
    pool: PgPool,
    cache: Arc<RwLock<HashMap<String, NonceRecord>>>,
    distributed_lock: Arc<DistributedLock>,
}

impl NonceManager {
    pub fn new(pool: PgPool, distributed_lock: Arc<DistributedLock>) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
            distributed_lock,
        }
    }

    /// 获取下一个可用的nonce（企业级：分布式锁保护）
    /// 从链上获取当前nonce，并考虑本地pending的交易
    pub async fn get_next_nonce(
        &self,
        chain: &str,
        address: &str,
        blockchain_client: &crate::service::blockchain_client::BlockchainClient,
    ) -> Result<u64> {
        let key = format!("{}:{}", chain, address);
        let lock_key = format!("nonce_lock:{}:{}", chain, address);

        // ✅ 企业级：获取分布式锁（最多等待10秒）
        let _guard = self
            .distributed_lock
            .acquire(&lock_key, 30, Duration::from_secs(10))
            .await
            .map_err(|e| anyhow!("Failed to acquire nonce lock: {}", e))?;

        tracing::debug!(
            chain = %chain,
            address = %address,
            "Acquired nonce lock"
        );

        // 从缓存获取
        let mut cache = self.cache.write().await;
        if let Some(record) = cache.get(&key) {
            // 检查缓存是否过期（5分钟）
            let cache_age = chrono::Utc::now().signed_duration_since(record.last_updated);
            if cache_age.num_seconds() < 300 {
                // 使用缓存中的nonce，但需要检查pending
                let next_nonce = record.current_nonce + 1;
                if !record.pending_nonces.contains(&next_nonce) {
                    // 更新pending列表
                    let mut new_record = record.clone();
                    new_record.pending_nonces.push(next_nonce);
                    new_record.last_updated = chrono::Utc::now();
                    cache.insert(key.clone(), new_record);
                    return Ok(next_nonce);
                }
            }
        }

        // 从链上获取当前nonce
        let chain_nonce = blockchain_client
            .get_transaction_count(chain, address)
            .await
            .map_err(|e| anyhow!("Failed to get nonce from chain: {}", e))?;

        // 从数据库获取pending nonces
        let pending_nonces = self.get_pending_nonces_from_db(chain, address).await?;

        // ✅ 企业级：检测并修复Nonce Gap
        self.detect_and_fix_nonce_gap(chain, address, chain_nonce, &pending_nonces)
            .await?;

        // 计算下一个可用nonce
        let mut next_nonce = chain_nonce;
        for pending in &pending_nonces {
            if *pending >= next_nonce {
                next_nonce = pending + 1;
            }
        }

        // 更新缓存
        let record = NonceRecord {
            address: address.to_string(),
            chain: chain.to_string(),
            current_nonce: chain_nonce,
            pending_nonces: {
                let mut p = pending_nonces.clone();
                p.push(next_nonce);
                p
            },
            last_updated: chrono::Utc::now(),
        };
        cache.insert(key, record);

        // 保存到数据库
        self.save_nonce_to_db(chain, address, next_nonce).await?;

        Ok(next_nonce)
    }

    /// 标记nonce为已使用
    pub async fn mark_nonce_used(&self, chain: &str, address: &str, nonce: u64) -> Result<()> {
        let key = format!("{}:{}", chain, address);

        // 更新缓存
        let mut cache = self.cache.write().await;
        if let Some(record) = cache.get_mut(&key) {
            record.pending_nonces.retain(|&n| n != nonce);
            record.current_nonce = nonce;
            record.last_updated = chrono::Utc::now();
        }

        // 更新数据库
        self.update_nonce_in_db(chain, address, nonce).await?;

        Ok(())
    }

    /// 从数据库获取pending nonces
    async fn get_pending_nonces_from_db(&self, chain: &str, address: &str) -> Result<Vec<u64>> {
        // 查询pending状态的交易（从transactions表）
        let rows = sqlx::query(
            "SELECT nonce FROM transactions
             WHERE chain = $1
             AND from_address = $2
             AND status = 'pending'
             AND nonce IS NOT NULL
             ORDER BY nonce",
        )
        .bind(chain)
        .bind(address)
        .fetch_all(&self.pool)
        .await?;

        let mut nonces = Vec::new();
        for row in rows {
            // 使用try_get方法获取nonce字段（可能为NULL）
            match row.try_get::<Option<i64>, _>("nonce") {
                Ok(Some(n)) => nonces.push(n as u64),
                Ok(None) => continue, // nonce为NULL，跳过
                Err(_) => continue,   // 字段不存在或类型不匹配，跳过
            }
        }

        Ok(nonces)
    }

    /// 保存nonce到数据库
    async fn save_nonce_to_db(&self, chain: &str, address: &str, nonce: u64) -> Result<()> {
        sqlx::query(
            "INSERT INTO nonce_tracking (chain, address, last_nonce, updated_at)
             VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
             ON CONFLICT (chain, address)
             DO UPDATE SET last_nonce = $3, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(chain)
        .bind(address)
        .bind(nonce as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 更新数据库中的nonce
    async fn update_nonce_in_db(&self, chain: &str, address: &str, nonce: u64) -> Result<()> {
        sqlx::query(
            "UPDATE nonce_tracking
             SET last_nonce = $1, updated_at = CURRENT_TIMESTAMP
             WHERE chain = $2 AND address = $3",
        )
        .bind(nonce as i64)
        .bind(chain)
        .bind(address)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 从链上同步nonce（用于恢复）
    pub async fn sync_nonce_from_chain(
        &self,
        chain: &str,
        address: &str,
        blockchain_client: &crate::service::blockchain_client::BlockchainClient,
    ) -> Result<u64> {
        let chain_nonce = blockchain_client
            .get_transaction_count(chain, address)
            .await
            .map_err(|e| anyhow!("Failed to sync nonce from chain: {}", e))?;

        let key = format!("{}:{}", chain, address);
        let mut cache = self.cache.write().await;
        if let Some(record) = cache.get_mut(&key) {
            record.current_nonce = chain_nonce;
            record.last_updated = chrono::Utc::now();
        }

        self.save_nonce_to_db(chain, address, chain_nonce).await?;

        Ok(chain_nonce)
    }

    /// ✅ 企业级：检测并修复Nonce Gap
    ///
    /// # Gap场景
    /// - 链上nonce = 10
    /// - 数据库pending nonces = [8, 9, 11]
    /// - nonce 8, 9 已被链上确认，需要标记为replaced
    async fn detect_and_fix_nonce_gap(
        &self,
        chain: &str,
        address: &str,
        chain_nonce: u64,
        pending_nonces: &[u64],
    ) -> Result<()> {
        // 找出所有小于chain_nonce的pending nonces（这些已经被链确认或被替换）
        let stale_nonces: Vec<u64> = pending_nonces
            .iter()
            .copied()
            .filter(|&n| n < chain_nonce)
            .collect();

        if !stale_nonces.is_empty() {
            tracing::warn!(
                chain = %chain,
                address = %address,
                chain_nonce = chain_nonce,
                stale_nonces = ?stale_nonces,
                "Detected nonce gap, cleaning up stale pending transactions"
            );

            // 标记这些交易为"replaced"
            for nonce in stale_nonces {
                let result = sqlx::query(
                    "UPDATE transactions
                     SET status = 'replaced', updated_at = CURRENT_TIMESTAMP
                     WHERE chain = $1 AND from_address = $2
                       AND nonce = $3 AND status = 'pending'",
                )
                .bind(chain)
                .bind(address)
                .bind(nonce as i64)
                .execute(&self.pool)
                .await;

                match result {
                    Ok(rows) if rows.rows_affected() > 0 => {
                        tracing::info!(
                            chain = %chain,
                            address = %address,
                            nonce = nonce,
                            "Marked stale transaction as replaced"
                        );
                    }
                    Ok(_) => {
                        tracing::debug!(
                            chain = %chain,
                            address = %address,
                            nonce = nonce,
                            "No matching pending transaction found"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            chain = %chain,
                            address = %address,
                            nonce = nonce,
                            "Failed to mark transaction as replaced"
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
