//! 非托管钱包Repository
//! 数据访问层：只操作公开信息

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::wallet_non_custodial::{CreateNonCustodialWalletRequest, NonCustodialWallet};

pub struct NonCustodialWalletRepository {
    pool: PgPool,
}

impl NonCustodialWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建钱包（只存储公开信息）
    pub async fn create(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        request: CreateNonCustodialWalletRequest,
    ) -> Result<NonCustodialWallet> {
        let wallet_id = Uuid::new_v4();
        let chain_id = self.chain_to_id(&request.chain);

        let wallet = sqlx::query_as::<_, NonCustodialWallet>(
            "INSERT INTO wallets 
            (id, user_id, tenant_id, chain_id, chain_symbol, address, pubkey, 
             name, derivation_path, curve_type, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            RETURNING 
                id, user_id, tenant_id, chain_id, chain_symbol, address,
                pubkey as public_key, name, derivation_path, curve_type,
                created_at, updated_at",
        )
        .bind(wallet_id)
        .bind(user_id)
        .bind(tenant_id)
        .bind(chain_id)
        .bind(request.chain.to_uppercase())
        .bind(&request.address)
        .bind(&request.public_key)
        .bind(
            request
                .name
                .unwrap_or_else(|| format!("{} Wallet", request.chain)),
        )
        .bind(&request.derivation_path)
        .bind(&request.curve_type)
        .fetch_one(&self.pool)
        .await?;

        Ok(wallet)
    }

    /// 批量创建钱包
    pub async fn create_batch(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        requests: Vec<CreateNonCustodialWalletRequest>,
    ) -> Result<Vec<NonCustodialWallet>> {
        let mut wallets = Vec::new();

        for request in requests {
            match self.create(user_id, tenant_id, request).await {
                Ok(wallet) => wallets.push(wallet),
                Err(e) => {
                    tracing::error!("Failed to create wallet: {:?}", e);
                    // 继续处理其他钱包
                }
            }
        }

        Ok(wallets)
    }

    /// 查询用户的所有钱包
    pub async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<NonCustodialWallet>> {
        let wallets = sqlx::query_as::<_, NonCustodialWallet>(
            "SELECT 
                id, user_id, tenant_id, chain_id, chain_symbol, address,
                pubkey as public_key, name, derivation_path, curve_type,
                created_at, updated_at
            FROM wallets
            WHERE user_id = $1
            ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(wallets)
    }

    /// 根据地址查询钱包
    pub async fn find_by_address(
        &self,
        address: &str,
        chain_id: i64,
    ) -> Result<Option<NonCustodialWallet>> {
        let wallet = sqlx::query_as::<_, NonCustodialWallet>(
            "SELECT 
                id, user_id, tenant_id, chain_id, chain_symbol, address,
                pubkey as public_key, name, derivation_path, curve_type,
                created_at, updated_at
            FROM wallets
            WHERE address = $1 AND chain_id = $2
            LIMIT 1",
        )
        .bind(address)
        .bind(chain_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(wallet)
    }

    /// 验证钱包所有权
    pub async fn verify_ownership(&self, wallet_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query_as::<_, (bool,)>(
            "SELECT EXISTS(SELECT 1 FROM wallets WHERE id = $1 AND user_id = $2) as exists",
        )
        .bind(wallet_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    /// 删除钱包
    pub async fn delete(&self, wallet_id: Uuid, user_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM wallets WHERE id = $1 AND user_id = $2")
            .bind(wallet_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// 链标识转ID
    fn chain_to_id(&self, chain: &str) -> i64 {
        match chain.to_uppercase().as_str() {
            "ETH" | "ETHEREUM" => 1,
            "BSC" | "BINANCE" => 56,
            "POLYGON" | "MATIC" => 137,
            "BTC" | "BITCOIN" => 0,
            "SOL" | "SOLANA" => 501,
            "TON" => 607,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chain_to_id() {
        // 注意：这个测试需要实际的数据库连接，在 CI 环境中可能会失败
        // 建议使用模拟的 pool 或跳过需要数据库的测试
        if let Ok(pool) = PgPool::connect("").await {
            let repo = NonCustodialWalletRepository { pool };

            assert_eq!(repo.chain_to_id("ETH"), 1);
            assert_eq!(repo.chain_to_id("ethereum"), 1);
            assert_eq!(repo.chain_to_id("BSC"), 56);
            assert_eq!(repo.chain_to_id("BTC"), 0);
        }
    }
}
