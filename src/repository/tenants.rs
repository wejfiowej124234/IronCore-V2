use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreateTenantInput {
    pub name: String,
}

pub async fn create(pool: &PgPool, input: CreateTenantInput) -> Result<Tenant, sqlx::Error> {
    let rec = sqlx::query_as::<_, Tenant>(
        r#"
        INSERT INTO tenants (name)
        VALUES ($1)
        RETURNING id, name, created_at
        "#,
    )
    .bind(input.name)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
    let rec = sqlx::query_as::<_, Tenant>(
        r#"
        SELECT id, name, created_at
        FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list(pool: &PgPool, limit: i64, offset: i64) -> Result<Vec<Tenant>, sqlx::Error> {
    let recs = sqlx::query_as::<_, Tenant>(
        r#"
        SELECT id, name, created_at
        FROM tenants
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn update(pool: &PgPool, id: Uuid, name: String) -> Result<Option<Tenant>, sqlx::Error> {
    let rec = sqlx::query_as::<_, Tenant>(
        r#"
        UPDATE tenants SET name = $2
        WHERE id = $1
        RETURNING id, name, created_at
        "#,
    )
    .bind(id)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(id)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected > 0)
}

pub async fn count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM tenants
        "#,
    )
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}
