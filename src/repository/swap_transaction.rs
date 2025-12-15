//! Swap Transaction Repository
//! 企业级实现，支持swap交易记录的CRUD操作

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SwapTransaction {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub wallet_id: Option<Uuid>, // ✅ 改为Option
    pub chain: Option<String>,   // ✅ 新增
    pub network: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: Decimal,
    pub to_amount: Option<Decimal>,
    pub to_amount_min: Option<Decimal>, // ✅ 新增（滑点保护）
    pub slippage: Option<Decimal>,
    pub swap_id: String,
    pub tx_hash: Option<String>,
    pub wallet_address: Option<String>, // ✅ 新增
    pub status: String,
    pub fiat_order_id: Option<Uuid>, // ✅ 新增
    pub gas_used: Option<String>,
    pub confirmations: i32,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SwapTransactionRepository {
    pool: PgPool,
}

impl SwapTransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建swap交易记录
    pub async fn create(&self, tx: &SwapTransaction) -> Result<SwapTransaction, sqlx::Error> {
        let result = sqlx::query_as::<_, SwapTransaction>(
            r#"
            INSERT INTO swap_transactions (
                id, tenant_id, user_id, wallet_id, network, from_token, to_token,
                from_amount, to_amount, slippage, swap_id, tx_hash, status,
                gas_used, confirmations, metadata, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id, tenant_id, user_id, wallet_id, network, from_token, to_token,
                      from_amount, to_amount, slippage, swap_id, tx_hash, status,
                      gas_used, confirmations, metadata, created_at, updated_at
            "#,
        )
        .bind(tx.id)
        .bind(tx.tenant_id)
        .bind(tx.user_id)
        .bind(tx.wallet_id)
        .bind(&tx.network)
        .bind(&tx.from_token)
        .bind(&tx.to_token)
        .bind(tx.from_amount)
        .bind(tx.to_amount)
        .bind(tx.slippage)
        .bind(&tx.swap_id)
        .bind(&tx.tx_hash)
        .bind(&tx.status)
        .bind(&tx.gas_used)
        .bind(tx.confirmations)
        .bind(&tx.metadata)
        .bind(tx.created_at)
        .bind(tx.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    /// 根据swap_id查找交易
    pub async fn find_by_swap_id(
        &self,
        swap_id: &str,
    ) -> Result<Option<SwapTransaction>, sqlx::Error> {
        let result = sqlx::query_as::<_, SwapTransaction>(
            r#"
            SELECT id, tenant_id, user_id, wallet_id, network, from_token, to_token,
                   from_amount, to_amount, slippage, swap_id, tx_hash, status,
                   gas_used, confirmations, metadata, created_at, updated_at
            FROM swap_transactions
            WHERE swap_id = $1
            "#,
        )
        .bind(swap_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    /// 根据用户ID和tenant_id列出交易（支持状态过滤）
    pub async fn list_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        status: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<SwapTransaction>, sqlx::Error> {
        let query = if let Some(status_filter) = status {
            sqlx::query_as::<_, SwapTransaction>(
                r#"
                SELECT id, tenant_id, user_id, wallet_id, network, from_token, to_token,
                       from_amount, to_amount, slippage, swap_id, tx_hash, status,
                       gas_used, confirmations, metadata, created_at, updated_at
                FROM swap_transactions
                WHERE tenant_id = $1 AND user_id = $2 AND status = $3
                ORDER BY created_at DESC
                LIMIT $4 OFFSET $5
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(status_filter)
            .bind(limit)
            .bind(offset)
        } else {
            sqlx::query_as::<_, SwapTransaction>(
                r#"
                SELECT id, tenant_id, user_id, wallet_id, network, from_token, to_token,
                       from_amount, to_amount, slippage, swap_id, tx_hash, status,
                       gas_used, confirmations, metadata, created_at, updated_at
                FROM swap_transactions
                WHERE tenant_id = $1 AND user_id = $2
                ORDER BY created_at DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(limit)
            .bind(offset)
        };

        let results = query.fetch_all(&self.pool).await?;
        Ok(results)
    }

    /// 统计用户交易数量（支持状态过滤）
    pub async fn count_by_user(
        &self,
        tenant_id: Uuid,
        user_id: Uuid,
        status: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let count = if let Some(status_filter) = status {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM swap_transactions
                WHERE tenant_id = $1 AND user_id = $2 AND status = $3
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
            .bind(status_filter)
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM swap_transactions
                WHERE tenant_id = $1 AND user_id = $2
                "#,
            )
            .bind(tenant_id)
            .bind(user_id)
        };

        let result = count.fetch_one(&self.pool).await?;
        Ok(result)
    }

    /// 更新swap交易状态
    ///
    /// CockroachDB兼容：
    /// - 使用CURRENT_TIMESTAMP更新updated_at字段（CockroachDB不支持触发器）
    /// - 使用CASE WHEN确保只有非None值才会更新字段
    pub async fn update_status(
        &self,
        swap_id: &str,
        status: &str,
        tx_hash: Option<&str>,
        to_amount: Option<Decimal>,
        gas_used: Option<&str>,
        confirmations: Option<i32>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE swap_transactions
            SET status = $1,
                tx_hash = CASE WHEN $2 IS NOT NULL THEN $2 ELSE tx_hash END,
                to_amount = CASE WHEN $3 IS NOT NULL THEN $3 ELSE to_amount END,
                gas_used = CASE WHEN $4 IS NOT NULL THEN $4 ELSE gas_used END,
                confirmations = CASE WHEN $5 IS NOT NULL THEN $5 ELSE confirmations END,
                updated_at = CURRENT_TIMESTAMP
            WHERE swap_id = $6
            "#,
        )
        .bind(status)
        .bind(tx_hash)
        .bind(to_amount)
        .bind(gas_used)
        .bind(confirmations)
        .bind(swap_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }
}
