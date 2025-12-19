use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TxRequest {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub wallet_id: Uuid,
    pub chain_id: i64,
    pub to_addr: String,
    pub amount: Decimal,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreateTxInput {
    pub tenant_id: Uuid,
    pub wallet_id: Uuid,
    pub chain_id: i64,
    pub to_addr: String,
    pub amount: Decimal,
    pub metadata: Option<serde_json::Value>,
}

pub async fn create(pool: &PgPool, input: CreateTxInput) -> Result<TxRequest, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxRequest>(
        r#"
        INSERT INTO tx_requests (tenant_id, wallet_id, chain_id, to_addr, amount, status, metadata)
        VALUES ($1, $2, $3, $4, $5, 'draft', $6)
        RETURNING id, tenant_id, wallet_id, chain_id::BIGINT AS chain_id, to_addr, amount, status, metadata, created_at
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.wallet_id)
    .bind(input.chain_id)
    .bind(input.to_addr)
    .bind(input.amount)
    .bind(input.metadata)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TxRequest>, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxRequest>(
        r#"
        SELECT id, tenant_id, wallet_id, chain_id::BIGINT AS chain_id, to_addr, amount, status, metadata, created_at
        FROM tx_requests
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list_by_wallet(
    pool: &PgPool,
    tenant_id: Uuid,
    wallet_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<TxRequest>, sqlx::Error> {
    let recs = sqlx::query_as::<_, TxRequest>(
        r#"
        SELECT id, tenant_id, wallet_id, chain_id::BIGINT AS chain_id, to_addr, amount, status, metadata, created_at
        FROM tx_requests
        WHERE tenant_id = $1 AND wallet_id = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(tenant_id)
    .bind(wallet_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<TxRequest>, sqlx::Error> {
    let recs = sqlx::query_as::<_, TxRequest>(
        r#"
        SELECT id, tenant_id, wallet_id, chain_id::BIGINT AS chain_id, to_addr, amount, status, metadata, created_at
        FROM tx_requests
        WHERE tenant_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    next: &str,
) -> Result<Option<TxRequest>, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxRequest>(
        r#"
        UPDATE tx_requests SET status = $3
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id, wallet_id, chain_id::BIGINT AS chain_id, to_addr, amount, status, metadata, created_at
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(next)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn count_by_wallet(
    pool: &PgPool,
    tenant_id: Uuid,
    wallet_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM tx_requests WHERE tenant_id = $1 AND wallet_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(wallet_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}
