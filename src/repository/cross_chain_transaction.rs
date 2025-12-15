// 跨链交易数据访问 Repository

use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

// ============ 领域模型 ============

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CrossChainTransaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub source_chain: String,
    pub source_address: String,
    pub source_tx_hash: Option<String>,
    pub source_confirmations: Option<i64>,
    pub destination_chain: String,
    pub destination_address: String,
    pub destination_tx_hash: Option<String>,
    pub destination_confirmations: Option<i64>,
    pub token_symbol: String,
    pub amount: Decimal,
    pub status: String,
    pub progress_percentage: Option<i64>,
    pub bridge_provider: Option<String>,
    pub bridge_protocol: Option<String>,
    pub fee_paid: Option<Decimal>,
    pub signed_source_tx: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone)]
pub struct CreateCrossChainTransactionParams {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub source_chain: String,
    pub source_address: String,
    pub destination_chain: String,
    pub destination_address: String,
    pub token_symbol: String,
    pub amount: Decimal,
    pub signed_source_tx: Option<String>,
}

// ============ Repository Trait ============

#[async_trait]
pub trait CrossChainTransactionRepository: Send + Sync {
    async fn create(&self, params: CreateCrossChainTransactionParams) -> Result<Uuid>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<CrossChainTransaction>>;
    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<CrossChainTransaction>>;
    async fn update_status(&self, id: Uuid, status: String, progress: i64) -> Result<()>;
    async fn update_source_tx(&self, id: Uuid, tx_hash: String, confirmations: i64) -> Result<()>;
    async fn update_destination_tx(
        &self,
        id: Uuid,
        tx_hash: String,
        confirmations: i64,
    ) -> Result<()>;
}

// ============ PostgreSQL 实现 ============

pub struct PgCrossChainTransactionRepository {
    pool: PgPool,
}

impl PgCrossChainTransactionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrossChainTransactionRepository for PgCrossChainTransactionRepository {
    async fn create(&self, params: CreateCrossChainTransactionParams) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO cross_chain_transactions (
                id, user_id, tenant_id, source_chain, source_address,
                destination_chain, destination_address, token_symbol, amount,
                status, progress_percentage, signed_source_tx,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            "#,
        )
        .bind(id)
        .bind(params.user_id)
        .bind(params.tenant_id)
        .bind(&params.source_chain)
        .bind(&params.source_address)
        .bind(&params.destination_chain)
        .bind(&params.destination_address)
        .bind(&params.token_symbol)
        .bind(params.amount)
        .bind("SourcePending")
        .bind(0i64) // progress_percentage 初始值为0
        .bind(&params.signed_source_tx)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(id)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<CrossChainTransaction>> {
        let result = sqlx::query_as::<_, CrossChainTransaction>(
            r#"
            SELECT id, user_id, tenant_id, source_chain, source_address,
                   source_tx_hash, source_confirmations,
                   destination_chain, destination_address, destination_tx_hash,
                   destination_confirmations,
                   token_symbol, amount, status, progress_percentage,
                   bridge_provider, bridge_protocol, fee_paid, signed_source_tx,
                   created_at, updated_at, completed_at
            FROM cross_chain_transactions
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result)
    }

    async fn find_by_user(&self, user_id: Uuid) -> Result<Vec<CrossChainTransaction>> {
        let results = sqlx::query_as::<_, CrossChainTransaction>(
            r#"
            SELECT id, user_id, tenant_id, source_chain, source_address,
                   source_tx_hash, source_confirmations,
                   destination_chain, destination_address, destination_tx_hash,
                   destination_confirmations,
                   token_symbol, amount, status, progress_percentage,
                   bridge_provider, bridge_protocol, fee_paid, signed_source_tx,
                   created_at, updated_at, completed_at
            FROM cross_chain_transactions
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
    }

    async fn update_status(&self, id: Uuid, status: String, progress: i64) -> Result<()> {
        let now = chrono::Utc::now();
        let completed_at = if status == "Completed" {
            Some(now)
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE cross_chain_transactions
            SET status = $1, progress_percentage = $2, updated_at = $3, completed_at = $4
            WHERE id = $5
            "#,
        )
        .bind(&status)
        .bind(progress)
        .bind(now)
        .bind(completed_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_source_tx(&self, id: Uuid, tx_hash: String, confirmations: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE cross_chain_transactions
            SET source_tx_hash = $1, source_confirmations = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(&tx_hash)
        .bind(confirmations)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_destination_tx(
        &self,
        id: Uuid,
        tx_hash: String,
        confirmations: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE cross_chain_transactions
            SET destination_tx_hash = $1, destination_confirmations = $2, updated_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(&tx_hash)
        .bind(confirmations)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
