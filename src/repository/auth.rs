//! 认证相关的数据访问层
//! 处理用户认证相关的数据库操作

use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

/// 用户认证信息（包含密码哈希）
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AuthUser {
    pub id: Uuid,
    pub tenant_id: Uuid,
    #[sqlx(default)]
    pub email_cipher: String,
    #[sqlx(default)]
    pub email: Option<String>,
    #[sqlx(default)]
    pub phone_cipher: Option<String>,
    #[sqlx(default)]
    pub phone: Option<String>,
    pub role: String,
    pub password_hash: Option<String>, // 密码哈希（可选，用于向后兼容）
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 根据邮箱和租户ID查找用户（用于登录）
pub async fn find_user_by_email(
    pool: &PgPool,
    tenant_id: Uuid,
    email_cipher: &str,
) -> Result<Option<AuthUser>, sqlx::Error> {
    let rec = sqlx::query_as::<_, AuthUser>(
        r#"
        SELECT id, tenant_id, 
               COALESCE(email_cipher, '') as email_cipher,
               email,
               phone_cipher,
               phone,
               role, 
               password_hash, 
               created_at
        FROM users
        WHERE tenant_id = $1 AND (email_cipher = $2 OR email = $2)
        "#,
    )
    .bind(tenant_id)
    .bind(email_cipher)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

/// 更新用户密码哈希
pub async fn update_password_hash(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    password_hash: &str,
) -> Result<bool, sqlx::Error> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $3
        WHERE id = $1 AND tenant_id = $2
        "#,
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(password_hash)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected > 0)
}

/// 根据ID获取认证用户信息
pub async fn get_auth_user_by_id(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<AuthUser>, sqlx::Error> {
    let rec = sqlx::query_as::<_, AuthUser>(
        r#"
        SELECT id, tenant_id, email_cipher, phone_cipher, role, password_hash, created_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}
