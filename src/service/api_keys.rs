use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::api_keys::{self, ApiKey, CreateApiKeyInput},
};

pub async fn create_api_key(
    pool: &PgPool,
    tenant_id: Uuid,
    name: String,
    key_hash: String,
    status: String,
) -> Result<ApiKey, anyhow::Error> {
    let input = CreateApiKeyInput {
        tenant_id,
        name,
        key_hash,
        status,
    };
    let k = api_keys::create(pool, input).await?;
    Ok(k)
}

pub async fn get_api_key_by_id(pool: &PgPool, id: Uuid) -> Result<Option<ApiKey>, anyhow::Error> {
    let k = api_keys::get_by_id(pool, id).await?;
    Ok(k)
}

pub async fn get_api_key_by_hash(
    pool: &PgPool,
    key_hash: &str,
) -> Result<Option<ApiKey>, anyhow::Error> {
    let k = api_keys::get_by_hash(pool, key_hash).await?;
    Ok(k)
}

pub async fn list_api_keys_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ApiKey>, anyhow::Error> {
    let k = api_keys::list_by_tenant(pool, tenant_id, status, limit, offset).await?;
    Ok(k)
}

pub async fn update_api_key_status(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    status: String,
) -> Result<Option<ApiKey>, anyhow::Error> {
    let k = api_keys::update_status(pool, id, tenant_id, status).await?;
    Ok(k)
}

pub async fn delete_api_key(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
) -> Result<bool, anyhow::Error> {
    let deleted = api_keys::delete(pool, id, tenant_id).await?;
    Ok(deleted)
}

pub async fn count_api_keys_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<String>,
) -> Result<i64, anyhow::Error> {
    let count = api_keys::count_by_tenant(pool, tenant_id, status).await?;
    Ok(count)
}
