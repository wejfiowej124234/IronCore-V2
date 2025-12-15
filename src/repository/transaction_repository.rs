// 交易数据访问 Repository

use std::str::FromStr;

use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

// ============ 领域模型 ============

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>, // ✅ 新增
    pub user_id: Uuid,
    pub wallet_id: Option<Uuid>, // ✅ 改为Option
    pub chain: Option<String>,   // ✅ 统一使用chain字段
    pub tx_hash: Option<String>, // ✅ 改为Option
    pub tx_type: String,         // send / receive / swap / approve
    pub status: String,          // pending / confirmed / failed
    pub from_address: String,
    pub to_address: String,
    pub amount: Option<String>, // ✅ 改为Option
    pub token_symbol: Option<String>,
    /// Gas费用：区块链网络收取的交易执行费用（gas_used * gas_price）
    /// 注意：这不是平台服务费，平台服务费在fee_audit表的platform_fee字段中
    pub gas_fee: Option<String>,
    pub nonce: Option<i64>,
    pub metadata: Option<serde_json::Value>, // ✅ 新增
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>, // ✅ 新增
    pub confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct CreateTransactionParams {
    pub user_id: Uuid,
    pub wallet_id: Uuid,
    pub chain: String,
    pub tx_hash: String,
    pub tx_type: String,
    pub from_address: String,
    pub to_address: String,
    pub amount: String,
    pub token_symbol: Option<String>,
    /// Gas费用：区块链网络收取的交易执行费用（gas_used * gas_price）
    /// 注意：这不是平台服务费，平台服务费在fee_audit表的platform_fee字段中
    pub gas_fee: Option<String>,
    pub nonce: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct TransactionFilter {
    pub user_id: Option<Uuid>,
    pub wallet_id: Option<Uuid>,
    pub chain: Option<String>,
    pub tx_type: Option<String>,
    pub status: Option<String>,
}

// ============ Repository Trait ============

#[async_trait]
pub trait TransactionRepository: Send + Sync {
    /// 根据 ID 查询交易
    async fn find_by_id(&self, tx_id: Uuid) -> Result<Option<Transaction>>;

    /// 根据交易哈希查询
    async fn find_by_hash(&self, tx_hash: &str) -> Result<Option<Transaction>>;

    /// 创建新交易记录
    async fn create(&self, params: CreateTransactionParams) -> Result<Transaction>;

    /// 更新交易状态
    async fn update_status(&self, tx_id: Uuid, status: &str) -> Result<()>;

    /// 标记交易为已确认
    async fn mark_confirmed(&self, tx_id: Uuid) -> Result<()>;

    /// 列出用户的交易历史
    async fn list_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>>;

    /// 列出钱包的交易历史
    async fn list_by_wallet(
        &self,
        wallet_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>>;

    /// 根据过滤条件查询
    async fn list_by_filter(
        &self,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>>;

    /// 获取待确认交易数量
    async fn count_pending_by_user(&self, user_id: Uuid) -> Result<i64>;
}

// ============ PostgreSQL 实现 ============

pub struct PgTransactionRepository {
    pool: PgPool,
}

impl PgTransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionRepository for PgTransactionRepository {
    async fn find_by_id(&self, tx_id: Uuid) -> Result<Option<Transaction>> {
        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            tenant_id: Option<Uuid>, // ✅ 匹配 Transaction 结构
            user_id: Uuid,
            wallet_id: Uuid,
            chain: String,
            tx_hash: String,
            tx_type: String,
            status: String,
            from_address: String,
            to_address: String, // ✅ 必填字段
            amount: String,     // Decimal as TEXT
            token_symbol: Option<String>,
            gas_fee: Option<String>,
            nonce: Option<i64>,
            metadata: Option<serde_json::Value>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let row = sqlx::query_as::<_, TransactionRow>(
            r#"SELECT id, tenant_id, user_id, wallet_id, chain, tx_hash,
                    tx_type, status, from_address, to_address, amount::TEXT as amount, token_symbol,
                    gas_fee, nonce, metadata, created_at, updated_at, confirmed_at
             FROM transactions WHERE id = $1"#,
        )
        .bind(tx_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Transaction {
            id: r.id,
            tenant_id: r.tenant_id,
            user_id: r.user_id,
            wallet_id: Some(r.wallet_id), // ✅ 包装为 Option
            chain: Some(r.chain),         // ✅ 包装为 Option
            tx_hash: Some(r.tx_hash),     // ✅ 包装为 Option
            tx_type: r.tx_type,
            status: r.status,
            from_address: r.from_address,
            to_address: r.to_address,
            amount: Some(r.amount), // ✅ 包装为 Option
            token_symbol: r.token_symbol,
            gas_fee: r.gas_fee,
            nonce: r.nonce,
            metadata: r.metadata,
            created_at: r.created_at,
            updated_at: r.updated_at,
            confirmed_at: r.confirmed_at,
        }))
    }

    async fn find_by_hash(&self, tx_hash: &str) -> Result<Option<Transaction>> {
        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            tenant_id: Option<Uuid>,
            user_id: Uuid,
            wallet_id: Uuid,
            chain: String,
            tx_hash: String,
            tx_type: String,
            status: String,
            from_address: String,
            to_address: String,
            amount: String,
            token_symbol: Option<String>,
            gas_fee: Option<String>,
            nonce: Option<i64>,
            metadata: Option<serde_json::Value>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let row = sqlx::query_as::<_, TransactionRow>(
            r#"SELECT id, tenant_id, user_id, wallet_id, chain, tx_hash,
                    tx_type, status, from_address, to_address, amount::TEXT as amount, token_symbol,
                    gas_fee, nonce, metadata, created_at, updated_at, confirmed_at
             FROM transactions WHERE tx_hash = $1"#,
        )
        .bind(tx_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Transaction {
            id: r.id,
            tenant_id: r.tenant_id,
            user_id: r.user_id,
            wallet_id: Some(r.wallet_id),
            chain: Some(r.chain),
            tx_hash: Some(r.tx_hash),
            tx_type: r.tx_type,
            status: r.status,
            from_address: r.from_address,
            to_address: r.to_address,
            amount: Some(r.amount),
            token_symbol: r.token_symbol,
            gas_fee: r.gas_fee,
            nonce: r.nonce,
            metadata: r.metadata,
            created_at: r.created_at,
            updated_at: r.updated_at,
            confirmed_at: r.confirmed_at,
        }))
    }

    async fn create(&self, params: CreateTransactionParams) -> Result<Transaction> {
        let tx_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // 将 String 解析为 Decimal
        let amount_decimal = Decimal::from_str(&params.amount)?;

        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            tenant_id: Option<Uuid>,
            user_id: Uuid,
            wallet_id: Uuid,
            chain: String,
            tx_hash: String,
            tx_type: String,
            status: String,
            from_address: String,
            to_address: String,
            amount: String,
            token_symbol: Option<String>,
            gas_fee: Option<String>,
            nonce: Option<i64>,
            metadata: Option<serde_json::Value>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let row = sqlx::query_as::<_, TransactionRow>(
            r#"INSERT INTO transactions
             (id, tenant_id, user_id, wallet_id, chain, tx_hash, tx_type, status,
              from_address, to_address, amount, token_symbol, gas_fee, nonce, metadata,
              created_at, updated_at)
             VALUES ($1, NULL, $2, $3, $4, $5, $6, 'pending', $7, $8, $9, $10, $11, $12, NULL, $13, $13)
             RETURNING id, tenant_id, user_id, wallet_id, chain, tx_hash, tx_type, status,
                       from_address, to_address, amount::TEXT as amount, token_symbol, gas_fee, nonce, metadata,
                       created_at, updated_at, confirmed_at"#
        )
        .bind(tx_id)
        .bind(params.user_id)
        .bind(params.wallet_id)
        .bind(&params.chain)
        .bind(&params.tx_hash)
        .bind(&params.tx_type)
        .bind(&params.from_address)
        .bind(&params.to_address)
        .bind(amount_decimal)
        .bind(params.token_symbol.as_deref())
        .bind(params.gas_fee.as_deref())
        .bind(params.nonce)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        Ok(Transaction {
            id: row.id,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            wallet_id: Some(row.wallet_id),
            chain: Some(row.chain),
            tx_hash: Some(row.tx_hash),
            tx_type: row.tx_type,
            status: row.status,
            from_address: row.from_address,
            to_address: row.to_address,
            amount: Some(row.amount),
            token_symbol: row.token_symbol,
            gas_fee: row.gas_fee,
            nonce: row.nonce,
            metadata: row.metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
            confirmed_at: row.confirmed_at,
        })
    }

    async fn update_status(&self, tx_id: Uuid, status: &str) -> Result<()> {
        // CockroachDB兼容：手动更新updated_at字段（CockroachDB不支持触发器）
        sqlx::query(
            "UPDATE transactions SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(status)
        .bind(tx_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_confirmed(&self, tx_id: Uuid) -> Result<()> {
        // CockroachDB兼容：同时更新updated_at字段（CockroachDB不支持触发器）
        sqlx::query("UPDATE transactions SET status = 'confirmed', confirmed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(tx_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn list_by_user(
        &self,
        user_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>> {
        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            tenant_id: Option<Uuid>,
            user_id: Uuid,
            wallet_id: Uuid,
            chain: String,
            tx_hash: String,
            tx_type: String,
            status: String,
            from_address: String,
            to_address: String,
            amount: String,
            token_symbol: Option<String>,
            gas_fee: Option<String>,
            nonce: Option<i64>,
            metadata: Option<serde_json::Value>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let rows = sqlx::query_as::<_, TransactionRow>(
            r#"SELECT id, tenant_id, user_id, wallet_id, chain, tx_hash, tx_type, status,
                    from_address, to_address, amount::TEXT as amount, token_symbol, gas_fee, nonce, metadata,
                    created_at, updated_at, confirmed_at
             FROM transactions
             WHERE user_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3"#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Transaction {
                id: r.id,
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                wallet_id: Some(r.wallet_id),
                chain: Some(r.chain),
                tx_hash: Some(r.tx_hash),
                tx_type: r.tx_type,
                status: r.status,
                from_address: r.from_address,
                to_address: r.to_address,
                amount: Some(r.amount),
                token_symbol: r.token_symbol,
                gas_fee: r.gas_fee,
                nonce: r.nonce,
                metadata: r.metadata,
                created_at: r.created_at,
                updated_at: r.updated_at,
                confirmed_at: r.confirmed_at,
            })
            .collect())
    }

    async fn list_by_wallet(
        &self,
        wallet_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>> {
        #[derive(sqlx::FromRow)]
        struct TransactionRow {
            id: Uuid,
            tenant_id: Option<Uuid>,
            user_id: Uuid,
            wallet_id: Uuid,
            chain: String,
            tx_hash: String,
            tx_type: String,
            status: String,
            from_address: String,
            to_address: String,
            amount: String,
            token_symbol: Option<String>,
            gas_fee: Option<String>,
            nonce: Option<i64>,
            metadata: Option<serde_json::Value>,
            created_at: chrono::DateTime<chrono::Utc>,
            updated_at: chrono::DateTime<chrono::Utc>,
            confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
        }

        let rows = sqlx::query_as::<_, TransactionRow>(
            r#"SELECT id, tenant_id, user_id, wallet_id, chain, tx_hash, tx_type, status,
                    from_address, to_address, amount::TEXT as amount, token_symbol, gas_fee, nonce, metadata,
                    created_at, updated_at, confirmed_at
             FROM transactions
             WHERE wallet_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3"#
        )
        .bind(wallet_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| Transaction {
                id: r.id,
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                wallet_id: Some(r.wallet_id),
                chain: Some(r.chain),
                tx_hash: Some(r.tx_hash),
                tx_type: r.tx_type,
                status: r.status,
                from_address: r.from_address,
                to_address: r.to_address,
                amount: Some(r.amount),
                token_symbol: r.token_symbol,
                gas_fee: r.gas_fee,
                nonce: r.nonce,
                metadata: r.metadata,
                created_at: r.created_at,
                updated_at: r.updated_at,
                confirmed_at: r.confirmed_at,
            })
            .collect())
    }

    async fn list_by_filter(
        &self,
        filter: TransactionFilter,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Transaction>> {
        use sqlx::QueryBuilder;

        let mut query_builder = QueryBuilder::new(
            "SELECT id, tenant_id, user_id, wallet_id, chain, chain_type, tx_hash, tx_type, status,
                    from_address, to_address, amount::TEXT, token_symbol, gas_fee, nonce, metadata,
                    created_at, updated_at, confirmed_at
             FROM transactions WHERE 1=1",
        );

        let _param_index = 1;

        if let Some(user_id) = filter.user_id {
            query_builder.push(" AND user_id = ");
            query_builder.push_bind(user_id);
        }
        if let Some(wallet_id) = filter.wallet_id {
            query_builder.push(" AND wallet_id = ");
            query_builder.push_bind(wallet_id);
        }
        if let Some(chain) = filter.chain {
            query_builder.push(" AND chain = ");
            query_builder.push_bind(chain);
        }
        if let Some(tx_type) = filter.tx_type {
            query_builder.push(" AND tx_type = ");
            query_builder.push_bind(tx_type);
        }
        if let Some(status) = filter.status {
            query_builder.push(" AND status = ");
            query_builder.push_bind(status);
        }

        query_builder.push(" ORDER BY created_at DESC LIMIT ");
        query_builder.push_bind(limit);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset);

        let query = query_builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| Transaction {
                id: row.get("id"),
                tenant_id: row.get("tenant_id"),
                user_id: row.get("user_id"),
                wallet_id: row.get("wallet_id"),
                chain: row.get("chain"),
                tx_hash: row.get("tx_hash"),
                tx_type: row.get("tx_type"),
                status: row.get("status"),
                from_address: row.get("from_address"),
                to_address: row.get("to_address"),
                amount: row.get("amount"),
                token_symbol: row.get("token_symbol"),
                gas_fee: row.get("gas_fee"),
                nonce: row.get("nonce"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                confirmed_at: row.get("confirmed_at"),
            })
            .collect())
    }

    async fn count_pending_by_user(&self, user_id: Uuid) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transactions WHERE user_id = $1 AND status = 'pending'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }
}
