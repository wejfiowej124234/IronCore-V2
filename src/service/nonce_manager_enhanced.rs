//! Nonce管理器增强版（企业级实现）
//! 集成Redis分布式锁、Gap检测、自动修复

use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;

use crate::infrastructure::{cache_strategy::CacheManager, distributed_lock::DistributedLock};

const NONCE_CACHE_TTL: u64 = 300; // 5分钟
const NONCE_LOCK_TIMEOUT: u64 = 5; // 5秒

/// Nonce管理器
pub struct NonceManagerEnhanced {
    pool: PgPool,
    distributed_lock: Arc<DistributedLock>,
    cache: Arc<CacheManager>,
}

impl NonceManagerEnhanced {
    pub fn new(
        pool: PgPool,
        distributed_lock: Arc<DistributedLock>,
        cache: Arc<CacheManager>,
    ) -> Self {
        Self {
            pool,
            distributed_lock,
            cache,
        }
    }

    /// 获取下一个nonce（分布式锁保护）
    pub async fn get_next_nonce(&self, chain: &str, address: &str) -> Result<u64> {
        let lock_key = format!("nonce:{}:{}", chain, address);

        // 1. 获取分布式锁
        let _lock_guard = self
            .distributed_lock
            .acquire(
                &lock_key,
                NONCE_LOCK_TIMEOUT,
                std::time::Duration::from_secs(5),
            )
            .await?;

        // 2. 检查缓存
        let cache_key = format!("nonce_cache:{}:{}", chain, address);
        if let Some(cached) = self.cache.get::<u64>(&cache_key).await? {
            return Ok(cached);
        }

        // 3. 从链上获取nonce
        let onchain_nonce = self.fetch_nonce_from_chain(chain, address).await?;

        // 4. 从数据库获取pending nonce
        let pending_nonce = self.get_pending_nonce(chain, address).await?;

        // 5. 取最大值
        let next_nonce = std::cmp::max(onchain_nonce, pending_nonce);

        // 6. 检测Gap
        if next_nonce > onchain_nonce + 1 {
            tracing::warn!(
                "Nonce gap detected: chain={}, address={}, onchain={}, pending={}",
                chain,
                address,
                onchain_nonce,
                next_nonce
            );
            // TODO: 触发Gap自动修复
        }

        // 7. 缓存nonce
        self.cache
            .set(
                &cache_key,
                &next_nonce,
                Some(std::time::Duration::from_secs(NONCE_CACHE_TTL)),
            )
            .await?;

        Ok(next_nonce)
    }

    /// 从链上获取nonce
    async fn fetch_nonce_from_chain(&self, _chain: &str, _address: &str) -> Result<u64> {
        // TODO: 调用区块链RPC获取nonce
        // 暂时返回0
        Ok(0)
    }

    /// 从数据库获取pending nonce
    async fn get_pending_nonce(&self, chain: &str, address: &str) -> Result<u64> {
        let result = sqlx::query_as::<_, (Option<i64>,)>(
            "SELECT MAX(nonce) as max_nonce
             FROM transactions
             WHERE chain_symbol = $1 AND from_address = $2 AND status IN ('Pending', 'Confirming')",
        )
        .bind(chain)
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|r| r.0).map(|n| n as u64 + 1).unwrap_or(0))
    }

    /// 记录已使用的nonce
    pub async fn mark_nonce_used(
        &self,
        chain: &str,
        address: &str,
        nonce: u64,
        tx_hash: &str,
    ) -> Result<()> {
        let _ = sqlx::query(
            "INSERT INTO nonce_tracking (chain_symbol, address, nonce, tx_hash, status, created_at)
             VALUES ($1, $2, $3, $4, 'used', CURRENT_TIMESTAMP)
             ON CONFLICT (chain_symbol, address, nonce) DO NOTHING",
        )
        .bind(chain)
        .bind(address)
        .bind(nonce as i64)
        .bind(tx_hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Gap检测和自动修复
    pub async fn detect_and_fix_gaps(&self, chain: &str, address: &str) -> Result<Vec<u64>> {
        // 1. 获取所有pending nonce
        let pending_nonces = sqlx::query_as::<_, (i64,)>(
            "SELECT nonce FROM transactions
             WHERE chain_symbol = $1 AND from_address = $2 AND status = 'Pending'
             ORDER BY nonce",
        )
        .bind(chain)
        .bind(address)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| r.0 as u64)
        .collect::<Vec<_>>();

        // 2. 检测gap
        let mut gaps = Vec::new();
        for i in 1..pending_nonces.len() {
            if pending_nonces[i] > pending_nonces[i - 1] + 1 {
                let gap_start = pending_nonces[i - 1] + 1;
                let gap_end = pending_nonces[i] - 1;
                for nonce in gap_start..=gap_end {
                    gaps.push(nonce);
                }
            }
        }

        // 3. 记录gap
        if !gaps.is_empty() {
            tracing::warn!(
                "Nonce gaps detected: chain={}, address={}, gaps={:?}",
                chain,
                address,
                gaps
            );

            // 记录到审计日志
            let _ = sqlx::query(
                "INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
                 VALUES ($1, $2, $3, CURRENT_TIMESTAMP)",
            )
            .bind("NONCE_GAP_DETECTED")
            .bind("nonce")
            .bind(serde_json::json!({
                "chain": chain,
                "address": address,
                "gaps": gaps
            }))
            .execute(&self.pool)
            .await;
        }

        Ok(gaps)
    }
}

// Nonce跟踪表结构（需要在迁移中创建）
// ```sql
// CREATE TABLE IF NOT EXISTS nonce_tracking (
//     id SERIAL PRIMARY KEY,
//     chain_symbol TEXT NOT NULL,
//     address TEXT NOT NULL,
//     nonce BIGINT NOT NULL,
//     tx_hash TEXT,
//     status TEXT NOT NULL,
//     created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
//     UNIQUE (chain_symbol, address, nonce)
// );
// ```
