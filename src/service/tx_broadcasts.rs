use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::tx_broadcasts::{self, CreateTxBroadcastInput, TxBroadcast},
};

pub async fn create_tx_broadcast(
    pool: &PgPool,
    tenant_id: Uuid,
    tx_request_id: Uuid,
    tx_hash: Option<String>,
    receipt: Option<serde_json::Value>,
) -> Result<TxBroadcast, anyhow::Error> {
    let input = CreateTxBroadcastInput {
        tenant_id,
        tx_request_id,
        tx_hash,
        receipt,
    };
    let b = tx_broadcasts::create(pool, input).await?;
    Ok(b)
}

pub async fn get_tx_broadcast_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<TxBroadcast>, anyhow::Error> {
    let b = tx_broadcasts::get_by_id(pool, id).await?;
    Ok(b)
}

pub async fn get_tx_broadcast_by_tx_request_id(
    pool: &PgPool,
    tx_request_id: Uuid,
) -> Result<Option<TxBroadcast>, anyhow::Error> {
    let b = tx_broadcasts::get_by_tx_request_id(pool, tx_request_id).await?;
    Ok(b)
}

pub async fn get_tx_broadcast_by_tx_hash(
    pool: &PgPool,
    tenant_id: Uuid,
    tx_hash: &str,
) -> Result<Option<TxBroadcast>, anyhow::Error> {
    let b = tx_broadcasts::get_by_tx_hash(pool, tenant_id, tx_hash).await?;
    Ok(b)
}

pub async fn list_tx_broadcasts_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<TxBroadcast>, anyhow::Error> {
    let b = tx_broadcasts::list_by_tenant(pool, tenant_id, limit, offset).await?;
    Ok(b)
}

pub async fn update_tx_broadcast(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    tx_hash: Option<String>,
    receipt: Option<serde_json::Value>,
) -> Result<Option<TxBroadcast>, anyhow::Error> {
    let b = tx_broadcasts::update(pool, id, tenant_id, tx_hash, receipt).await?;
    Ok(b)
}

pub async fn count_tx_broadcasts_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
) -> Result<i64, anyhow::Error> {
    let count = tx_broadcasts::count_by_tenant(pool, tenant_id).await?;
    Ok(count)
}
