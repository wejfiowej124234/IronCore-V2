// 钱包数据访问 Repository

use anyhow::Result;
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

// ============ 领域模型 ============

/// 钱包领域模型（100%对齐数据库Schema - 企业级最佳实践）
#[derive(Debug, Clone)]
pub struct Wallet {
    pub id: Uuid,
    pub tenant_id: Uuid, // ✅ 企业级：租户隔离
    pub user_id: Uuid,
    pub chain_id: i64, // ✅ 数字链ID (1=ETH, 56=BSC, 137=Polygon) - INT8/BIGINT
    pub chain_symbol: Option<String>, // ✅ 链符号 ("ETH", "BSC", "POLYGON")
    pub address: String,
    pub pubkey: Option<String>,          // ✅ 公钥（非托管核心字段）
    pub name: Option<String>,            // ✅ 钱包名称（对齐DB字段名）
    pub derivation_path: Option<String>, // ✅ BIP44派生路径
    pub curve_type: Option<String>,      // ✅ 曲线类型 (secp256k1/ed25519)
    pub account_index: i64,              // ✅ 账户索引（默认0）- INT8/BIGINT
    pub address_index: i64,              // ✅ 地址索引（默认0）- INT8/BIGINT
    pub policy_id: Option<Uuid>,         // ✅ 策略ID（企业级审批）
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 创建钱包参数（对齐数据库Schema）
#[derive(Debug, Clone)]
pub struct CreateWalletParams {
    pub tenant_id: Uuid, // ✅ 租户ID（企业级必需）
    pub user_id: Uuid,
    pub chain_id: i64,                // ✅ 数字链ID - INT8/BIGINT
    pub chain_symbol: Option<String>, // ✅ 链符号
    pub address: String,
    pub pubkey: Option<String>, // ✅ 公钥
    pub name: Option<String>,   // ✅ 钱包名称
    pub derivation_path: Option<String>,
    pub curve_type: Option<String>, // ✅ 曲线类型
    pub account_index: Option<i64>, // ✅ 账户索引 - INT8/BIGINT
    pub address_index: Option<i64>, // ✅ 地址索引 - INT8/BIGINT
    pub policy_id: Option<Uuid>,    // ✅ 策略ID
}

#[derive(Debug, Clone)]
pub struct WalletBalance {
    pub wallet_id: Uuid,
    pub chain_type: String,
    pub native_balance: String,
    pub tokens: Vec<TokenBalance>,
}

#[derive(Debug, Clone)]
pub struct TokenBalance {
    pub contract_address: String,
    pub symbol: String,
    pub balance: String,
    pub decimals: u8,
}

// ============ Repository Trait ============

#[async_trait]
pub trait WalletRepository: Send + Sync {
    /// 根据 ID 查询钱包
    async fn find_by_id(&self, wallet_id: Uuid) -> Result<Option<Wallet>>;

    /// 根据地址查询钱包
    async fn find_by_address(&self, address: &str) -> Result<Option<Wallet>>;

    /// 创建新钱包
    async fn create(&self, params: CreateWalletParams) -> Result<Wallet>;

    /// 更新钱包名称
    async fn update_name(&self, wallet_id: Uuid, new_name: &str) -> Result<()>;

    /// 删除钱包
    async fn delete(&self, wallet_id: Uuid) -> Result<()>;

    /// 列出用户的所有钱包
    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Wallet>>;

    /// 列出用户指定链类型的钱包
    async fn list_by_user_and_chain(&self, user_id: Uuid, chain_type: &str) -> Result<Vec<Wallet>>;

    /// 获取用户在目标链的首个钱包地址（用于跨链收款）
    async fn get_user_address_for_chain(
        &self,
        user_id: Uuid,
        chain_type: &str,
    ) -> Result<Option<String>>;

    /// 获取钱包余额（模拟接口，实际可能需要查询链上）
    async fn get_balance(&self, wallet_id: Uuid) -> Result<WalletBalance>;
}

// ============ PostgreSQL 实现 ============

pub struct PgWalletRepository {
    pool: PgPool,
}

impl PgWalletRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WalletRepository for PgWalletRepository {
    async fn find_by_id(&self, wallet_id: Uuid) -> Result<Option<Wallet>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                i64,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i64,
                i64,
                Option<Uuid>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
                "SELECT id, tenant_id, user_id, chain_id::BIGINT as chain_id, chain_symbol, address, pubkey, name,
                    derivation_path, curve_type,
                    account_index::BIGINT as account_index,
                    address_index::BIGINT as address_index,
                    policy_id, created_at
             FROM wallets WHERE id = $1",
        )
        .bind(wallet_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                id,
                tenant_id,
                user_id,
                chain_id,
                chain_symbol,
                address,
                pubkey,
                name,
                derivation_path,
                curve_type,
                account_index,
                address_index,
                policy_id,
                created_at,
            )| {
                Wallet {
                    id,
                    tenant_id,
                    user_id,
                    chain_id,
                    chain_symbol,
                    address,
                    pubkey,
                    name,
                    derivation_path,
                    curve_type,
                    account_index,
                    address_index,
                    policy_id,
                    created_at,
                }
            },
        ))
    }

    async fn find_by_address(&self, address: &str) -> Result<Option<Wallet>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                i64,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i64,
                i64,
                Option<Uuid>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
                "SELECT id, tenant_id, user_id, chain_id::BIGINT as chain_id, chain_symbol, address, pubkey, name,
                    derivation_path, curve_type,
                    account_index::BIGINT as account_index,
                    address_index::BIGINT as address_index,
                    policy_id, created_at
             FROM wallets WHERE address = $1",
        )
        .bind(address)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                id,
                tenant_id,
                user_id,
                chain_id,
                chain_symbol,
                address,
                pubkey,
                name,
                derivation_path,
                curve_type,
                account_index,
                address_index,
                policy_id,
                created_at,
            )| {
                Wallet {
                    id,
                    tenant_id,
                    user_id,
                    chain_id,
                    chain_symbol,
                    address,
                    pubkey,
                    name,
                    derivation_path,
                    curve_type,
                    account_index,
                    address_index,
                    policy_id,
                    created_at,
                }
            },
        ))
    }

    async fn create(&self, params: CreateWalletParams) -> Result<Wallet> {
        let wallet_id = Uuid::new_v4();

        // 使用RETURNING子句，CockroachDB完全支持，避免额外的查询
        let row = sqlx::query_as::<_, (
            Uuid, Uuid, Uuid, i64, Option<String>, String, Option<String>, Option<String>,
            Option<String>, Option<String>, i64, i64, Option<Uuid>,
            chrono::DateTime<chrono::Utc>
        )>(
            "INSERT INTO wallets (id, tenant_id, user_id, chain_id, chain_symbol, address, pubkey, name,
                                  derivation_path, curve_type, account_index, address_index, policy_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             RETURNING id, tenant_id, user_id, chain_id::BIGINT as chain_id, chain_symbol, address, pubkey, name,
                       derivation_path, curve_type,
                       account_index::BIGINT as account_index,
                       address_index::BIGINT as address_index,
                       policy_id, created_at"
        )
        .bind(wallet_id)
        .bind(params.tenant_id)
        .bind(params.user_id)
        .bind(params.chain_id)
        .bind(&params.chain_symbol)
        .bind(&params.address)
        .bind(&params.pubkey)
        .bind(&params.name)
        .bind(&params.derivation_path)
        .bind(&params.curve_type)
        .bind(params.account_index.unwrap_or(0))
        .bind(params.address_index.unwrap_or(0))
        .bind(params.policy_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Wallet {
            id: row.0,
            tenant_id: row.1,
            user_id: row.2,
            chain_id: row.3, // 已经是i64，不需要as转换
            chain_symbol: row.4,
            address: row.5,
            pubkey: row.6,
            name: row.7,
            derivation_path: row.8,
            curve_type: row.9,
            account_index: row.10,
            address_index: row.11,
            policy_id: row.12,
            created_at: row.13,
        })
    }

    async fn update_name(&self, wallet_id: Uuid, new_name: &str) -> Result<()> {
        sqlx::query("UPDATE wallets SET name = $1 WHERE id = $2")
            .bind(new_name)
            .bind(wallet_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete(&self, wallet_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM wallets WHERE id = $1")
            .bind(wallet_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_by_user(&self, user_id: Uuid) -> Result<Vec<Wallet>> {
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                i64,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i64,
                i64,
                Option<Uuid>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
                "SELECT id, tenant_id, user_id, chain_id::BIGINT as chain_id, chain_symbol, address, pubkey, name,
                    derivation_path, curve_type,
                    account_index::BIGINT as account_index,
                    address_index::BIGINT as address_index,
                    policy_id, created_at
             FROM wallets
             WHERE user_id = $1
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    tenant_id,
                    user_id,
                    chain_id,
                    chain_symbol,
                    address,
                    pubkey,
                    name,
                    derivation_path,
                    curve_type,
                    account_index,
                    address_index,
                    policy_id,
                    created_at,
                )| {
                    Wallet {
                        id,
                        tenant_id,
                        user_id,
                        chain_id,
                        chain_symbol,
                        address,
                        pubkey,
                        name,
                        derivation_path,
                        curve_type,
                        account_index,
                        address_index,
                        policy_id,
                        created_at,
                    }
                },
            )
            .collect())
    }

    async fn list_by_user_and_chain(&self, user_id: Uuid, chain_type: &str) -> Result<Vec<Wallet>> {
        // 注意：保留chain_type参数名以兼容调用方，但使用chain_symbol查询
        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                i64,
                Option<String>,
                String,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<String>,
                i64,
                i64,
                Option<Uuid>,
                chrono::DateTime<chrono::Utc>,
            ),
        >(
                "SELECT id, tenant_id, user_id, chain_id::BIGINT as chain_id, chain_symbol, address, pubkey, name,
                    derivation_path, curve_type,
                    account_index::BIGINT as account_index,
                    address_index::BIGINT as address_index,
                    policy_id, created_at
             FROM wallets
             WHERE user_id = $1 AND chain_symbol = $2
             ORDER BY created_at DESC",
        )
        .bind(user_id)
        .bind(chain_type)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    tenant_id,
                    user_id,
                    chain_id,
                    chain_symbol,
                    address,
                    pubkey,
                    name,
                    derivation_path,
                    curve_type,
                    account_index,
                    address_index,
                    policy_id,
                    created_at,
                )| {
                    Wallet {
                        id,
                        tenant_id,
                        user_id,
                        chain_id,
                        chain_symbol,
                        address,
                        pubkey,
                        name,
                        derivation_path,
                        curve_type,
                        account_index,
                        address_index,
                        policy_id,
                        created_at,
                    }
                },
            )
            .collect())
    }

    async fn get_user_address_for_chain(
        &self,
        user_id: Uuid,
        chain_type: &str,
    ) -> Result<Option<String>> {
        // PRODUCTION: Query user's first wallet address on target chain for cross-chain recipient
        // Returns first created wallet address for the specified chain
        let row = sqlx::query_as::<_, (String,)>(
            "SELECT address
             FROM wallets
             WHERE user_id = $1 AND chain_symbol = $2
             ORDER BY created_at ASC
             LIMIT 1",
        )
        .bind(user_id)
        .bind(chain_type)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(address,)| address))
    }

    async fn get_balance(&self, wallet_id: Uuid) -> Result<WalletBalance> {
        // 注意：此方法返回空余额，真实余额通过AssetService查询
        let wallet = self
            .find_by_id(wallet_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Wallet not found"))?;

        Ok(WalletBalance {
            wallet_id,
            chain_type: wallet
                .chain_symbol
                .unwrap_or_else(|| format!("chain_{}", wallet.chain_id)),
            native_balance: "0.0".to_string(),
            tokens: vec![],
        })
    }
}
