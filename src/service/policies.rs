use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::policies::{self, CreatePolicyInput, Policy},
};

pub async fn create_policy(
    pool: &PgPool,
    tenant_id: Uuid,
    name: String,
    rules: serde_json::Value,
    version: Option<i32>,
) -> Result<Policy, anyhow::Error> {
    let input = CreatePolicyInput {
        tenant_id,
        name,
        rules,
        version,
    };
    let p = policies::create(pool, input).await?;
    Ok(p)
}

pub async fn get_policy_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Policy>, anyhow::Error> {
    let p = policies::get_by_id(pool, id).await?;
    Ok(p)
}

pub async fn list_policies_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Policy>, anyhow::Error> {
    let p = policies::list_by_tenant(pool, tenant_id, limit, offset).await?;
    Ok(p)
}

pub async fn update_policy(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    name: Option<String>,
    rules: Option<serde_json::Value>,
    version: Option<i32>,
) -> Result<Option<Policy>, anyhow::Error> {
    let p = policies::update(pool, id, tenant_id, name, rules, version).await?;
    Ok(p)
}

pub async fn delete_policy(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
) -> Result<bool, anyhow::Error> {
    let deleted = policies::delete(pool, id, tenant_id).await?;
    Ok(deleted)
}

pub async fn count_policies_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<i64, anyhow::Error> {
    let count = policies::count_by_tenant(pool, tenant_id).await?;
    Ok(count)
}
