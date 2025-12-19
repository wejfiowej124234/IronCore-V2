use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::infrastructure::db::PgPool;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub chain_id: i64,
    pub address: String,
    pub pubkey: String,
    pub policy_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    // 多链扩展字段
    pub name: Option<String>,
    pub derivation_path: Option<String>,
    pub curve_type: Option<String>,
    pub chain_symbol: Option<String>,
    pub account_index: Option<i64>,
    pub address_index: Option<i64>,
    // 钱包组ID（1个助记词 → 1个钱包组 → 4个链账户）
    pub group_id: Option<Uuid>,
}

#[derive(Debug)]
pub struct CreateWalletInput {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub chain_id: i64,
    pub address: String,
    pub pubkey: String,
    pub policy_id: Option<Uuid>,
    // 多链扩展字段
    pub name: Option<String>,
    pub derivation_path: Option<String>,
    pub curve_type: Option<String>,
    pub chain_symbol: Option<String>,
    pub account_index: Option<i64>,
    pub address_index: Option<i64>,
    // 钱包组ID
    pub group_id: Option<Uuid>,
}

pub async fn create(pool: &PgPool, input: CreateWalletInput) -> Result<Wallet, sqlx::Error> {
    // 使用数据库事务确保原子性
    let mut tx = pool.begin().await?;

    let rec = sqlx::query_as::<_, Wallet>(
        r#"
        INSERT INTO wallets (
            tenant_id, user_id, chain_id, address, pubkey, policy_id,
            name, derivation_path, curve_type, chain_symbol, account_index, address_index, group_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        RETURNING
            id,
            tenant_id,
            user_id,
            chain_id::BIGINT as chain_id,
            address,
            pubkey,
            policy_id,
            created_at,
            name,
            derivation_path,
            curve_type,
            chain_symbol,
            account_index::BIGINT as account_index,
            address_index::BIGINT as address_index,
            group_id
        "#,
    )
    .bind(input.tenant_id)
    .bind(input.user_id)
    .bind(input.chain_id)
    .bind(input.address)
    .bind(input.pubkey)
    .bind(input.policy_id)
    .bind(input.name)
    .bind(input.derivation_path)
    .bind(input.curve_type)
    .bind(input.chain_symbol)
    .bind(input.account_index)
    .bind(input.address_index)
    .bind(input.group_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(rec)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Wallet>, sqlx::Error> {
    let rec = sqlx::query_as::<_, Wallet>(
        r#"
        SELECT
            id,
            tenant_id,
            user_id,
            chain_id::BIGINT as chain_id,
            address,
            pubkey,
            policy_id,
            created_at,
            name,
            derivation_path,
            curve_type,
            chain_symbol,
            account_index::BIGINT as account_index,
            address_index::BIGINT as address_index,
            group_id
        FROM wallets
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn get_by_address(
    pool: &PgPool,
    tenant_id: Uuid,
    chain_id: i64,
    address: &str,
) -> Result<Option<Wallet>, sqlx::Error> {
    let rec = sqlx::query_as::<_, Wallet>(
        r#"
        SELECT
            id,
            tenant_id,
            user_id,
            chain_id::BIGINT as chain_id,
            address,
            pubkey,
            policy_id,
            created_at,
            name,
            derivation_path,
            curve_type,
            chain_symbol,
            account_index::BIGINT as account_index,
            address_index::BIGINT as address_index,
            group_id
        FROM wallets
        WHERE tenant_id = $1 AND chain_id = $2 AND address = $3
        "#,
    )
    .bind(tenant_id)
    .bind(chain_id)
    .bind(address)
    .fetch_optional(pool)
    .await?;
    Ok(rec)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Wallet>, sqlx::Error> {
    let recs = sqlx::query_as::<_, Wallet>(
        r#"
        SELECT
            id,
            tenant_id,
            user_id,
            chain_id::BIGINT as chain_id,
            address,
            pubkey,
            policy_id,
            created_at,
            name,
            derivation_path,
            curve_type,
            chain_symbol,
            account_index::BIGINT as account_index,
            address_index::BIGINT as address_index,
            group_id
        FROM wallets
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

pub async fn list_by_user(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Wallet>, sqlx::Error> {
    let recs = sqlx::query_as::<_, Wallet>(
        r#"
        SELECT
            id,
            tenant_id,
            user_id,
            chain_id::BIGINT as chain_id,
            address,
            pubkey,
            policy_id,
            created_at,
            name,
            derivation_path,
            curve_type,
            chain_symbol,
            account_index::BIGINT as account_index,
            address_index::BIGINT as address_index,
            group_id
        FROM wallets
        WHERE tenant_id = $1 AND user_id = $2
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;
    Ok(recs)
}

pub async fn delete(pool: &PgPool, id: Uuid, tenant_id: Uuid) -> Result<bool, sqlx::Error> {
    let rows_affected = sqlx::query(
        r#"
        DELETE FROM wallets
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
        SELECT COUNT(*) FROM wallets WHERE tenant_id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}

pub async fn count_by_user(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<i64, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM wallets WHERE tenant_id = $1 AND user_id = $2
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}
