use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AuditIndex {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub event_type: String,
    pub business_id: Option<Uuid>,
    pub proof_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreateAuditIndexInput {
    pub tenant_id: Uuid,
    pub event_type: String,
    pub business_id: Option<Uuid>,
    pub proof_hash: String,
}

pub async fn create(
    pool: &PgPool,
    input: CreateAuditIndexInput,
) -> Result<AuditIndex, sqlx::Error> {
    let rec = sqlx::query_as::<_, AuditIndex>(
        r#"
        INSERT INTO audit_index (tenant_id, event_type, business_id, proof_hash)
        VALUES ($1, $2, $3, $4)
        RETURNING id, tenant_id, event_type, business_id, proof_hash, created_at
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.event_type)
    .bind(input.business_id)
    .bind(input.proof_hash)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<AuditIndex>, sqlx::Error> {
    let rec = sqlx::query_as::<_, AuditIndex>(
        r#"
        SELECT id, tenant_id, event_type, business_id, proof_hash, created_at
        FROM audit_index
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    event_type: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditIndex>, sqlx::Error> {
    let recs = if let Some(et) = event_type {
        sqlx::query_as::<_, AuditIndex>(
            r#"
            SELECT id, tenant_id, event_type, business_id, proof_hash, created_at
            FROM audit_index
            WHERE tenant_id = $1 AND event_type = $2
            ORDER BY created_at DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(et)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, AuditIndex>(
            r#"
            SELECT id, tenant_id, event_type, business_id, proof_hash, created_at
            FROM audit_index
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

pub async fn get_by_business_id(
    pool: &PgPool,
    tenant_id: Uuid,
    business_id: Uuid,
) -> Result<Vec<AuditIndex>, sqlx::Error> {
    let recs = sqlx::query_as::<_, AuditIndex>(
        r#"
        SELECT id, tenant_id, event_type, business_id, proof_hash, created_at
        FROM audit_index
        WHERE tenant_id = $1 AND business_id = $2
        ORDER BY created_at DESC
        "#,
    )
    .bind(tenant_id)
    .bind(business_id)
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn count_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    event_type: Option<String>,
) -> Result<i64, sqlx::Error> {
    let count: (i64,) = if let Some(et) = event_type {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM audit_index WHERE tenant_id = $1 AND event_type = $2
            "#,
        )
        .bind(tenant_id)
        .bind(et)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM audit_index WHERE tenant_id = $1
            "#,
        )
        .bind(tenant_id)
        .fetch_one(pool)
        .await?
    };
    Ok(count.0)
}
