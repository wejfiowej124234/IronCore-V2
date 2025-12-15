use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::tenants::{self, CreateTenantInput, Tenant},
};

pub async fn create_tenant(pool: &PgPool, name: String) -> Result<Tenant, anyhow::Error> {
    let input = CreateTenantInput { name };
    let t = tenants::create(pool, input).await?;
    Ok(t)
}

pub async fn get_tenant_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Tenant>, anyhow::Error> {
    let t = tenants::get_by_id(pool, id).await?;
    Ok(t)
}

pub async fn list_tenants(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<Tenant>, anyhow::Error> {
    let t = tenants::list(pool, limit, offset).await?;
    Ok(t)
}

pub async fn update_tenant(
    pool: &PgPool,
    id: Uuid,
    name: String,
) -> Result<Option<Tenant>, anyhow::Error> {
    let t = tenants::update(pool, id, name).await?;
    Ok(t)
}

pub async fn delete_tenant(pool: &PgPool, id: Uuid) -> Result<bool, anyhow::Error> {
    let deleted = tenants::delete(pool, id).await?;
    Ok(deleted)
}

pub async fn count_tenants(pool: &PgPool) -> Result<i64, anyhow::Error> {
    let count = tenants::count(pool).await?;
    Ok(count)
}
