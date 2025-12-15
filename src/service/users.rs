use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::users::{self, CreateUserInput, User},
};

pub async fn create_user(
    pool: &PgPool,
    tenant_id: Uuid,
    email_cipher: String,
    phone_cipher: Option<String>,
    role: String,
) -> Result<User, anyhow::Error> {
    let input = CreateUserInput {
        tenant_id,
        email_cipher,
        phone_cipher,
        role,
    };
    let u = users::create(pool, input).await?;
    Ok(u)
}

pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, anyhow::Error> {
    let u = users::get_by_id(pool, id).await?;
    Ok(u)
}

pub async fn list_users_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<User>, anyhow::Error> {
    let u = users::list_by_tenant(pool, tenant_id, limit, offset).await?;
    Ok(u)
}

pub async fn update_user(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    email_cipher: Option<String>,
    phone_cipher: Option<String>,
    role: Option<String>,
) -> Result<Option<User>, anyhow::Error> {
    let u = users::update(pool, id, tenant_id, email_cipher, phone_cipher, role).await?;
    Ok(u)
}

pub async fn delete_user(pool: &PgPool, id: Uuid, tenant_id: Uuid) -> Result<bool, anyhow::Error> {
    let deleted = users::delete(pool, id, tenant_id).await?;
    Ok(deleted)
}

pub async fn count_users_by_tenant(pool: &PgPool, tenant_id: Uuid) -> Result<i64, anyhow::Error> {
    let count = users::count_by_tenant(pool, tenant_id).await?;
    Ok(count)
}
