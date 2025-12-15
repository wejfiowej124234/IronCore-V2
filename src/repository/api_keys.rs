use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreateApiKeyInput {
    pub tenant_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub status: String,
}

pub async fn create(pool: &PgPool, input: CreateApiKeyInput) -> Result<ApiKey, sqlx::Error> {
    let rec = sqlx::query_as::<_, ApiKey>(
        r#"
        INSERT INTO api_keys (tenant_id, name, key_hash, status)
        VALUES ($1, $2, $3, $4)
        RETURNING id, tenant_id, name, key_hash, status, created_at
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.name)
    .bind(input.key_hash)
    .bind(input.status)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<ApiKey>, sqlx::Error> {
    let rec = sqlx::query_as::<_, ApiKey>(
        r#"
        SELECT id, tenant_id, name, key_hash, status, created_at
        FROM api_keys
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_hash(pool: &PgPool, key_hash: &str) -> Result<Option<ApiKey>, sqlx::Error> {
    let rec = sqlx::query_as::<_, ApiKey>(
        r#"
        SELECT id, tenant_id, name, key_hash, status, created_at
        FROM api_keys
        WHERE key_hash = $1
        "#,
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ApiKey>, sqlx::Error> {
    let recs = if let Some(s) = status {
        sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT id, tenant_id, name, key_hash, status, created_at
            FROM api_keys
            WHERE tenant_id = $1 AND status = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(s)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, ApiKey>(
            r#"
            SELECT id, tenant_id, name, key_hash, status, created_at
            FROM api_keys
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    };
    Ok(recs)
}

pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    status: String,
) -> Result<Option<ApiKey>, sqlx::Error> {
    let rec = sqlx::query_as::<_, ApiKey>(
        r#"
        UPDATE api_keys SET status = $3
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id, name, key_hash, status, created_at
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(status)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn delete(pool: &PgPool, id: Uuid, tenant_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM api_keys
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected > 0)
}

pub async fn count_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<String>,
) -> Result<i64, sqlx::Error> {
    let count: (i64,) = if let Some(s) = status {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM api_keys WHERE tenant_id = $1 AND status = $2
            "#,
        )
        .bind(tenant_id)
        .bind(s)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM api_keys WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(pool)
        .await?
    };
    Ok(count.0)
}
