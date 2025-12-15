//! 钱包批量注册服务（多链钱包后端处理）
//! 企业级实现：事务性批量创建+原子性保证

use anyhow::Result;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// 批量钱包注册请求
pub struct BatchWalletRegisterRequest {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub wallets: Vec<WalletRegisterItem>,
}

pub struct WalletRegisterItem {
    pub chain: String,
    pub chain_id: i64,
    pub address: String,
    pub public_key: String,
    pub derivation_path: String,
    pub curve_type: String,
}

/// 批量钱包注册服务
pub struct WalletBatchRegisterService {
    pool: PgPool,
}

impl WalletBatchRegisterService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 批量注册钱包（事务性）
    ///
    /// # 企业级特性
    /// - 使用数据库事务保证原子性
    /// - 全部成功或全部失败
    /// - 防止部分创建导致的不一致
    pub async fn batch_register(
        &self,
        request: BatchWalletRegisterRequest,
    ) -> Result<BatchRegisterResult> {
        // 开启事务
        let mut tx = self.pool.begin().await?;

        let mut created_wallets = Vec::new();
        let mut failed_wallets = Vec::new();

        // 1. 验证所有地址不重复
        for wallet in &request.wallets {
            if let Err(e) = self
                .validate_wallet_unique(&wallet, request.user_id, &mut tx)
                .await
            {
                failed_wallets.push(WalletRegisterError {
                    chain: wallet.chain.clone(),
                    address: wallet.address.clone(),
                    error: e.to_string(),
                });
            }
        }

        // 如果有验证失败，回滚事务
        if !failed_wallets.is_empty() {
            tx.rollback().await?;
            return Ok(BatchRegisterResult {
                success: false,
                created: vec![],
                failed: failed_wallets,
                message: "Validation failed".to_string(),
            });
        }

        // 2. 批量插入钱包
        for wallet in request.wallets {
            match self
                .insert_wallet(&wallet, request.user_id, request.tenant_id, &mut tx)
                .await
            {
                Ok(wallet_id) => {
                    created_wallets.push(CreatedWallet {
                        id: wallet_id.to_string(),
                        chain: wallet.chain.clone(),
                        address: wallet.address.clone(),
                    });
                }
                Err(e) => {
                    // 任何插入失败，回滚整个事务
                    tx.rollback().await?;
                    return Err(e);
                }
            }
        }

        // 3. 提交事务
        tx.commit().await?;

        // 4. 记录审计日志（异步，不影响主流程）
        self.log_batch_creation(request.user_id, &created_wallets)
            .await;

        let wallet_count = created_wallets.len();
        Ok(BatchRegisterResult {
            success: true,
            created: created_wallets,
            failed: vec![],
            message: format!("Successfully created {} wallets", wallet_count),
        })
    }

    /// 验证钱包唯一性
    async fn validate_wallet_unique(
        &self,
        wallet: &WalletRegisterItem,
        user_id: Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let exists = sqlx::query(
            "SELECT id FROM wallets WHERE address = $1 AND chain_id = $2 AND user_id = $3",
        )
        .bind(&wallet.address)
        .bind(wallet.chain_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?;

        if exists.is_some() {
            anyhow::bail!(
                "Wallet already exists: {} on {}",
                wallet.address,
                wallet.chain
            );
        }

        Ok(())
    }

    /// 插入单个钱包
    async fn insert_wallet(
        &self,
        wallet: &WalletRegisterItem,
        user_id: Uuid,
        tenant_id: Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Uuid> {
        let wallet_id = Uuid::new_v4();
        let wallet_name = format!("{} Wallet", wallet.chain);

        let _ = sqlx::query(
            "INSERT INTO wallets 
             (id, tenant_id, user_id, chain_id, chain_symbol, address, pubkey, 
              name, derivation_path, curve_type, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, CURRENT_TIMESTAMP)",
        )
        .bind(wallet_id)
        .bind(tenant_id)
        .bind(user_id)
        .bind(wallet.chain_id)
        .bind(wallet.chain.to_uppercase())
        .bind(&wallet.address)
        .bind(&wallet.public_key)
        .bind(&wallet_name)
        .bind(&wallet.derivation_path)
        .bind(&wallet.curve_type)
        .execute(&mut **tx)
        .await?;

        Ok(wallet_id)
    }

    /// 记录审计日志
    async fn log_batch_creation(&self, user_id: Uuid, wallets: &[CreatedWallet]) {
        let chains: Vec<String> = wallets.iter().map(|w| w.chain.clone()).collect();

        let _ = sqlx::query(
            "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
             VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
        )
        .bind("MULTI_CHAIN_WALLETS_CREATED")
        .bind("wallet")
        .bind(user_id)
        .bind(serde_json::json!({
            "count": wallets.len(),
            "chains": chains
        }))
        .execute(&self.pool)
        .await
        .ok();
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 结果类型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct BatchRegisterResult {
    pub success: bool,
    pub created: Vec<CreatedWallet>,
    pub failed: Vec<WalletRegisterError>,
    pub message: String,
}

pub struct CreatedWallet {
    pub id: String,
    pub chain: String,
    pub address: String,
}

pub struct WalletRegisterError {
    pub chain: String,
    pub address: String,
    pub error: String,
}
