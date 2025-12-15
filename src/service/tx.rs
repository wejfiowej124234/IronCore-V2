use rust_decimal::Decimal;
use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::tx::{self, CreateTxInput, TxRequest},
};

pub async fn create_tx_request(
    pool: &PgPool,
    tenant_id: Uuid,
    wallet_id: Uuid,
    chain_id: i64,
    to_addr: String,
    amount_wei: Decimal,
    metadata: Option<serde_json::Value>,
) -> Result<TxRequest, anyhow::Error> {
    let input = CreateTxInput {
        tenant_id,
        wallet_id,
        chain_id,
        to_addr,
        amount: amount_wei,
        metadata,
    };
    let r = tx::create(pool, input).await?;
    Ok(r)
}

pub async fn get_tx_by_id(pool: &PgPool, id: Uuid) -> Result<Option<TxRequest>, anyhow::Error> {
    let r = tx::get_by_id(pool, id).await?;
    Ok(r)
}

pub async fn list_tx_by_wallet(
    pool: &PgPool,
    tenant_id: Uuid,
    wallet_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<TxRequest>, anyhow::Error> {
    let r = tx::list_by_wallet(pool, tenant_id, wallet_id, limit, offset).await?;
    Ok(r)
}

pub async fn list_tx_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<TxRequest>, anyhow::Error> {
    let r = tx::list_by_tenant(pool, tenant_id, limit, offset).await?;
    Ok(r)
}

pub async fn advance_tx_status(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
    next_status: &str,
) -> Result<Option<TxRequest>, anyhow::Error> {
    let r = tx::update_status(pool, id, tenant_id, next_status).await?;
    Ok(r)
}
