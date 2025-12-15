//! 代币注册表 Repository
//! 企业级实现，从数据库读取代币信息，替代硬编码

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};

/// 代币信息
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Token {
    pub id: uuid::Uuid,
    pub symbol: String,
    pub name: String,
    pub chain_id: i64,
    pub address: String,
    pub decimals: i64, // INT8 in CockroachDB
    pub is_native: bool,
    pub is_stablecoin: bool,
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    pub is_enabled: bool,
    pub priority: i64, // INT8 in CockroachDB
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Repository Trait
#[async_trait]
pub trait TokenRepository: Send + Sync {
    /// 根据符号和链ID获取代币
    async fn get_by_symbol_and_chain(&self, symbol: &str, chain_id: u64) -> Result<Option<Token>>;

    /// 根据地址和链ID获取代币
    async fn get_by_address_and_chain(&self, address: &str, chain_id: u64)
        -> Result<Option<Token>>;

    /// 获取链上所有启用的代币
    async fn list_by_chain(&self, chain_id: u64) -> Result<Vec<Token>>;

    /// 获取链上的稳定币列表
    async fn list_stablecoins_by_chain(&self, chain_id: u64) -> Result<Vec<Token>>;
}

/// PostgreSQL 实现
pub struct PgTokenRepository {
    pool: PgPool,
}

impl PgTokenRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TokenRepository for PgTokenRepository {
    async fn get_by_symbol_and_chain(&self, symbol: &str, chain_id: u64) -> Result<Option<Token>> {
        // 使用sqlx::query_as（运行时查询，避免编译时数据库连接要求）
        let token = sqlx::query_as::<_, Token>(
            r#"
            SELECT 
                id,
                symbol,
                name,
                chain_id,
                address,
                decimals,
                is_native,
                is_stablecoin,
                logo_url,
                coingecko_id,
                is_enabled,
                priority,
                created_at,
                updated_at
            FROM tokens.registry
            WHERE symbol = $1 AND chain_id = $2 AND is_enabled = TRUE
            "#,
        )
        .bind(symbol.to_uppercase())
        .bind(chain_id as i64)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query token by symbol and chain")?;

        Ok(token)
    }

    async fn get_by_address_and_chain(
        &self,
        address: &str,
        chain_id: u64,
    ) -> Result<Option<Token>> {
        // 使用sqlx::query_as（运行时查询，避免编译时数据库连接要求）
        // ✅ 使用 LOWER() 进行不区分大小写的地址匹配（Ethereum 地址标准）
        let token = sqlx::query_as::<_, Token>(
            r#"
            SELECT 
                id,
                symbol,
                name,
                chain_id,
                address,
                decimals,
                is_native,
                is_stablecoin,
                logo_url,
                coingecko_id,
                is_enabled,
                priority,
                created_at,
                updated_at
            FROM tokens.registry
            WHERE LOWER(address) = LOWER($1) AND chain_id = $2 AND is_enabled = TRUE
            "#,
        )
        .bind(address)
        .bind(chain_id as i64)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to query token by address and chain")?;

        Ok(token)
    }

    async fn list_by_chain(&self, chain_id: u64) -> Result<Vec<Token>> {
        // 使用sqlx::query_as（运行时查询，避免编译时数据库连接要求）
        let tokens = sqlx::query_as::<_, Token>(
            r#"
            SELECT 
                id,
                symbol,
                name,
                chain_id,
                address,
                decimals,
                is_native,
                is_stablecoin,
                logo_url,
                coingecko_id,
                is_enabled,
                priority,
                created_at,
                updated_at
            FROM tokens.registry
            WHERE chain_id = $1 AND is_enabled = TRUE
            ORDER BY priority ASC, symbol ASC
            "#,
        )
        .bind(chain_id as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list tokens by chain")?;

        Ok(tokens)
    }

    async fn list_stablecoins_by_chain(&self, chain_id: u64) -> Result<Vec<Token>> {
        // 使用sqlx::query_as（运行时查询，避免编译时数据库连接要求）
        let tokens = sqlx::query_as::<_, Token>(
            r#"
            SELECT 
                id,
                symbol,
                name,
                chain_id,
                address,
                decimals,
                is_native,
                is_stablecoin,
                logo_url,
                coingecko_id,
                is_enabled,
                priority,
                created_at,
                updated_at
            FROM tokens.registry
            WHERE chain_id = $1 AND is_stablecoin = TRUE AND is_enabled = TRUE
            ORDER BY priority ASC, symbol ASC
            "#,
        )
        .bind(chain_id as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list stablecoins by chain")?;

        Ok(tokens)
    }
}
