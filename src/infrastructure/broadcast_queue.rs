//! 交易广播队列持久化
//! 企业级实现：使用数据库持久化待广播交易

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// 广播队列项
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BroadcastQueueItem {
    pub id: Uuid,
    pub chain: String,
    pub signed_tx: String,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub retry_count: i32,
    pub max_retries: i32,
    pub status: BroadcastStatus,
    pub created_at: DateTime<Utc>,
    pub next_retry_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BroadcastStatus {
    Pending,
    Broadcasting,
    Success,
    Failed,
    Cancelled,
}

impl ToString for BroadcastStatus {
    fn to_string(&self) -> String {
        match self {
            BroadcastStatus::Pending => "pending".to_string(),
            BroadcastStatus::Broadcasting => "broadcasting".to_string(),
            BroadcastStatus::Success => "success".to_string(),
            BroadcastStatus::Failed => "failed".to_string(),
            BroadcastStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

impl BroadcastStatus {
    fn from_str(s: &str) -> Self {
        match s {
            "pending" => BroadcastStatus::Pending,
            "broadcasting" => BroadcastStatus::Broadcasting,
            "success" => BroadcastStatus::Success,
            "failed" => BroadcastStatus::Failed,
            "cancelled" => BroadcastStatus::Cancelled,
            _ => BroadcastStatus::Pending,
        }
    }
}

/// 广播队列管理器
pub struct BroadcastQueue {
    pool: PgPool,
}

impl BroadcastQueue {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 添加到队列
    pub async fn enqueue(
        &self,
        chain: String,
        signed_tx: String,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Uuid> {
        let id = Uuid::new_v4();
        
        let _ = sqlx::query(
            "INSERT INTO broadcast_queue 
             (id, chain, signed_tx, user_id, tenant_id, retry_count, max_retries, status, created_at)
             VALUES ($1, $2, $3, $4, $5, 0, 3, 'pending', CURRENT_TIMESTAMP)"
        )
        .bind(id)
        .bind(chain)
        .bind(signed_tx)
        .bind(user_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    /// 获取待广播项目（按优先级）
    pub async fn get_pending_items(&self, limit: i32) -> Result<Vec<BroadcastQueueItem>> {
        #[derive(sqlx::FromRow)]
        struct QueueRow {
            id: Uuid,
            chain: String,
            signed_tx: String,
            user_id: Uuid,
            tenant_id: Uuid,
            retry_count: i32,
            max_retries: i32,
            status: String,
            created_at: DateTime<Utc>,
            next_retry_at: Option<DateTime<Utc>>,
            error_message: Option<String>,
        }
        
        let rows = sqlx::query_as::<_, QueueRow>(
            "SELECT id, chain, signed_tx, user_id, tenant_id, retry_count, max_retries, 
                    status, created_at, next_retry_at, error_message
             FROM broadcast_queue
             WHERE status = 'pending' 
               AND (next_retry_at IS NULL OR next_retry_at <= CURRENT_TIMESTAMP)
             ORDER BY created_at ASC
             LIMIT $1
             FOR UPDATE SKIP LOCKED"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| BroadcastQueueItem {
            id: row.id,
            chain: row.chain,
            signed_tx: row.signed_tx,
            user_id: row.user_id,
            tenant_id: row.tenant_id,
            retry_count: row.retry_count,
            max_retries: row.max_retries,
            status: BroadcastStatus::from_str(&row.status),
            created_at: row.created_at,
            next_retry_at: row.next_retry_at,
            error_message: row.error_message,
        }).collect())
    }

    /// 标记为正在广播
    pub async fn mark_broadcasting(&self, id: Uuid) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE broadcast_queue 
             SET status = 'broadcasting', updated_at = CURRENT_TIMESTAMP
             WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 标记为成功
    pub async fn mark_success(&self, id: Uuid, tx_hash: String) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE broadcast_queue 
             SET status = 'success', 
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 标记为失败并安排重试
    pub async fn mark_failed_with_retry(&self, id: Uuid, error: String) -> Result<()> {
        let item = self.get_item(id).await?;
        
        if item.retry_count + 1 >= item.max_retries {
            // 超过最大重试次数，标记为最终失败
            let _ = sqlx::query(
                "UPDATE broadcast_queue 
                 SET status = 'failed', 
                     error_message = $2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = $1"
            )
            .bind(id)
            .bind(error)
            .execute(&self.pool)
            .await?;
        } else {
            // 安排下次重试（指数退避）
            let retry_delay_seconds = 2_i64.pow((item.retry_count + 1) as u32) * 60; // 2min, 4min, 8min
            
            let _ = sqlx::query(
                "UPDATE broadcast_queue 
                 SET status = 'pending',
                     retry_count = retry_count + 1,
                     error_message = $2,
                     next_retry_at = CURRENT_TIMESTAMP + ($3 || ' seconds')::interval,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = $1"
            )
            .bind(id)
            .bind(error)
            .bind(retry_delay_seconds.to_string())
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// 获取单个项目
    async fn get_item(&self, id: Uuid) -> Result<BroadcastQueueItem> {
        #[derive(sqlx::FromRow)]
        struct QueueItemRow {
            id: Uuid,
            chain: String,
            signed_tx: String,
            user_id: Uuid,
            tenant_id: Uuid,
            retry_count: i32,
            max_retries: i32,
            status: String,
            created_at: DateTime<Utc>,
            next_retry_at: Option<DateTime<Utc>>,
            error_message: Option<String>,
        }
        
        let row = sqlx::query_as::<_, QueueItemRow>(
            "SELECT id, chain, signed_tx, user_id, tenant_id, retry_count, max_retries,
                    status, created_at, next_retry_at, error_message
             FROM broadcast_queue WHERE id = $1"
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(BroadcastQueueItem {
            id: row.id,
            chain: row.chain,
            signed_tx: row.signed_tx,
            user_id: row.user_id,
            tenant_id: row.tenant_id,
            retry_count: row.retry_count,
            max_retries: row.max_retries,
            status: BroadcastStatus::from_str(&row.status),
            created_at: row.created_at,
            next_retry_at: row.next_retry_at,
            error_message: row.error_message,
        })
    }

    /// 清理旧的成功记录（超过7天）
    pub async fn cleanup_old_items(&self) -> Result<i64> {
        let result = sqlx::query(
            "DELETE FROM broadcast_queue 
             WHERE status = 'success' 
               AND created_at < CURRENT_TIMESTAMP - INTERVAL '7 days'"
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    /// 取消待处理项目
    pub async fn cancel_pending(&self, id: Uuid) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE broadcast_queue 
             SET status = 'cancelled', updated_at = CURRENT_TIMESTAMP
             WHERE id = $1 AND status = 'pending'"
        )
        .bind(id)
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// 创建broadcast_queue表的迁移SQL
pub const MIGRATION_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS broadcast_queue (
    id UUID PRIMARY KEY,
    chain TEXT NOT NULL,
    signed_tx TEXT NOT NULL,
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    next_retry_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_broadcast_queue_status_retry 
ON broadcast_queue(status, next_retry_at) 
WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_broadcast_queue_created 
ON broadcast_queue(created_at) 
WHERE status = 'success';

COMMENT ON TABLE broadcast_queue IS '交易广播队列：持久化待广播交易，支持重启恢复';
"#;

