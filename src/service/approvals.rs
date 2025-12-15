use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::approvals::{self, Approval, CreateApprovalInput},
};

pub async fn create_approval(
    pool: &PgPool,
    tenant_id: Uuid,
    policy_id: Uuid,
    requester: Uuid,
    status: String,
    payload: serde_json::Value,
) -> Result<Approval, anyhow::Error> {
    let input = CreateApprovalInput {
        tenant_id,
        policy_id,
        requester,
        status,
        payload,
    };
    let a = approvals::create(pool, input).await?;
    Ok(a)
}

pub async fn get_approval_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<Approval>, anyhow::Error> {
    let a = approvals::get_by_id(pool, id).await?;
    Ok(a)
}

pub async fn list_approvals_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<String>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Approval>, anyhow::Error> {
    let a = approvals::list_by_tenant(pool, tenant_id, status, limit, offset).await?;
    Ok(a)
}

pub async fn update_approval_status(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    status: String,
) -> Result<Option<Approval>, anyhow::Error> {
    let a = approvals::update_status(pool, id, tenant_id, status).await?;
    Ok(a)
}

pub async fn delete_approval(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
) -> Result<bool, anyhow::Error> {
    let deleted = approvals::delete(pool, id, tenant_id).await?;
    Ok(deleted)
}

pub async fn count_approvals_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    status: Option<String>,
) -> Result<i64, anyhow::Error> {
    let count = approvals::count_by_tenant(pool, tenant_id, status).await?;
    Ok(count)
}
