use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct TxBroadcast {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub tx_request_id: Uuid,
    pub tx_hash: Option<String>,
    pub receipt: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreateTxBroadcastInput {
    pub tenant_id: Uuid,
    pub tx_request_id: Uuid,
    pub tx_hash: Option<String>,
    pub receipt: Option<serde_json::Value>,
}

pub async fn create(
    pool: &PgPool,
    input: CreateTxBroadcastInput,
) -> Result<TxBroadcast, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxBroadcast>(
        r#"
        INSERT INTO tx_broadcasts (tenant_id, tx_request_id, tx_hash, receipt)
        VALUES ($1, $2, $3, $4)
        RETURNING id, tenant_id, tx_request_id, tx_hash, receipt, created_at
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.tx_request_id)
    .bind(input.tx_hash)
    .bind(input.receipt)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TxBroadcast>, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxBroadcast>(
        r#"
        SELECT id, tenant_id, tx_request_id, tx_hash, receipt, created_at
        FROM tx_broadcasts
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_tx_request_id(
    pool: &PgPool,
    tx_request_id: Uuid,
) -> Result<Option<TxBroadcast>, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxBroadcast>(
        r#"
        SELECT id, tenant_id, tx_request_id, tx_hash, receipt, created_at
        FROM tx_broadcasts
        WHERE tx_request_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(tx_request_id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_tx_hash(
    pool: &PgPool,
    tenant_id: Uuid,
    tx_hash: &str,
) -> Result<Option<TxBroadcast>, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxBroadcast>(
        r#"
        SELECT id, tenant_id, tx_request_id, tx_hash, receipt, created_at
        FROM tx_broadcasts
        WHERE tenant_id = $1 AND tx_hash = $2
        "#,
    )
    .bind(tenant_id)
    .bind(tx_hash)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<TxBroadcast>, sqlx::Error> {
    let recs = sqlx::query_as::<_, TxBroadcast>(
        r#"
        SELECT id, tenant_id, tx_request_id, tx_hash, receipt, created_at
        FROM tx_broadcasts
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

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    tx_hash: Option<String>,
    receipt: Option<serde_json::Value>,
) -> Result<Option<TxBroadcast>, sqlx::Error> {
    let rec = sqlx::query_as::<_, TxBroadcast>(
        r#"
        UPDATE tx_broadcasts
        SET tx_hash = COALESCE($3, tx_hash), receipt = COALESCE($4, receipt)
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id, tx_request_id, tx_hash, receipt, created_at
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(tx_hash)
    .bind(receipt)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn count_by_tenant(pool: &PgPool, tenant_id: Uuid) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM tx_broadcasts WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}
