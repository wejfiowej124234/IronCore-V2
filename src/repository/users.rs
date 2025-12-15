use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email_cipher: String,
    pub phone_cipher: Option<String>,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct CreateUserInput {
    pub tenant_id: Uuid,
    pub email_cipher: String,
    pub phone_cipher: Option<String>,
    pub role: String,
}

pub async fn create(pool: &PgPool, input: CreateUserInput) -> Result<User, sqlx::Error> {
    let rec = sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (tenant_id, email_cipher, phone_cipher, role)
        VALUES ($1, $2, $3, $4)
        RETURNING id, tenant_id, email_cipher, phone_cipher, role, created_at
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.email_cipher)
    .bind(input.phone_cipher)
    .bind(input.role)
    .fetch_one(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    let rec = sqlx::query_as::<_, User>(
        r#"
        SELECT id, tenant_id, email_cipher, phone_cipher, role, created_at
        FROM users
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
    limit: i64,
    offset: i64,
) -> Result<Vec<User>, sqlx::Error> {
    let recs = sqlx::query_as::<_, User>(
        r#"
        SELECT id, tenant_id, email_cipher, phone_cipher, role, created_at
        FROM users
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
    email_cipher: Option<String>,
    phone_cipher: Option<String>,
    role: Option<String>,
) -> Result<Option<User>, sqlx::Error> {
    // 如果所有字段都是None，直接返回当前记录
    if email_cipher.is_none() && phone_cipher.is_none() && role.is_none() {
        return get_by_id(pool, id).await;
    }

    // 使用COALESCE处理可选字段
    let rec = sqlx::query_as::<_, User>(
        r#"
        UPDATE users 
        SET email_cipher = COALESCE($3, email_cipher),
            phone_cipher = COALESCE($4, phone_cipher),
            role = COALESCE($5, role)
        WHERE id = $1 AND tenant_id = $2
        RETURNING id, tenant_id, email_cipher, phone_cipher, role, created_at
        "#,
    )
    .bind(id)
    .bind(tenant_id)
    .bind(email_cipher)
    .bind(phone_cipher)
    .bind(role)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn delete(pool: &PgPool, id: Uuid, tenant_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM users
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

pub async fn count_by_tenant(pool: &PgPool, tenant_id: Uuid) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM users WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}
