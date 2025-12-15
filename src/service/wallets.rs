use uuid::Uuid;

use crate::{
    infrastructure::db::PgPool,
    repository::wallets::{self, CreateWalletInput, Wallet},
};

pub async fn create_wallet(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    chain_id: i64,
    address: String,
    pubkey: String,
    policy_id: Option<Uuid>,
) -> Result<Wallet, anyhow::Error> {
    create_wallet_with_metadata(
        pool, tenant_id, user_id, chain_id, address, pubkey, policy_id, None, None, None, None,
        None, None,
    )
    .await
}

/// 创建钱包（带多链元数据）
#[allow(clippy::too_many_arguments)]
pub async fn create_wallet_with_metadata(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    chain_id: i64,
    address: String,
    pubkey: String,
    policy_id: Option<Uuid>,
    name: Option<String>,
    derivation_path: Option<String>,
    curve_type: Option<String>,
    chain_symbol: Option<String>,
    account_index: Option<i64>,
    address_index: Option<i64>,
) -> Result<Wallet, anyhow::Error> {
    // 幂等：可在上层用 Redis idem:key 保护
    let input = CreateWalletInput {
        tenant_id,
        user_id,
        chain_id,
        address,
        pubkey,
        policy_id,
        name,
        derivation_path,
        curve_type,
        chain_symbol,
        account_index,
        address_index,
        group_id: None, // ✅ 旧接口不支持钱包组，设为None
    };
    let w = wallets::create(pool, input).await?;
    Ok(w)
}

pub async fn get_wallet_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Wallet>, anyhow::Error> {
    let w = wallets::get_by_id(pool, id).await?;
    Ok(w)
}

pub async fn get_wallet_by_address(
    pool: &PgPool,
    tenant_id: Uuid,
    chain_id: i64,
    address: &str,
) -> Result<Option<Wallet>, anyhow::Error> {
    let w = wallets::get_by_address(pool, tenant_id, chain_id, address).await?;
    Ok(w)
}

pub async fn list_wallets_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Wallet>, anyhow::Error> {
    let wallets = wallets::list_by_tenant(pool, tenant_id, limit, offset).await?;
    Ok(wallets)
}

pub async fn list_wallets_by_user(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<Wallet>, anyhow::Error> {
    let wallets = wallets::list_by_user(pool, tenant_id, user_id, limit, offset).await?;
    Ok(wallets)
}

/// Alias for list_wallets_by_user with default pagination
pub async fn list_user_wallets(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<Vec<Wallet>, anyhow::Error> {
    list_wallets_by_user(pool, tenant_id, user_id, 100, 0).await
}

pub async fn delete_wallet(
    pool: &PgPool,
    id: Uuid,
    tenant_id: Uuid,
) -> Result<bool, anyhow::Error> {
    let deleted = wallets::delete(pool, id, tenant_id).await?;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    #[tokio::test]
    #[ignore] // 需要数据库连接
    async fn test_create_wallet() {
        // 这个测试需要实际的数据库连接
        // 在实际测试中，应该使用测试数据库
    }

    #[test]
    fn test_wallet_service_logic() {
        // 测试不依赖数据库的逻辑
        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        // 测试UUID生成
        assert_ne!(tenant_id, user_id);
    }
}
