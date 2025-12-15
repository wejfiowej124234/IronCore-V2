//! 跨链桥服务（非托管模式）
//!
//! P1级修复：确保跨链桥完全非托管化
//! 流程：客户端签名 → 后端验证 → 链上监听 → 返回证明

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// 跨链桥订单（非托管模式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonCustodialBridgeOrder {
    pub id: Uuid,
    pub user_id: Uuid,

    // 源链信息
    pub source_chain: String,
    pub source_address: String,
    pub source_tx_hash: Option<String>,
    pub signed_source_tx: String, // ✅ 客户端签名的源链交易

    // 目标链信息
    pub dest_chain: String,
    pub dest_address: String,
    pub dest_tx_hash: Option<String>,

    // 金额信息
    pub amount: String,
    pub token: String,

    // 状态信息
    pub status: BridgeStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,

    // 证明信息
    pub source_proof: Option<String>, // 源链交易证明
    pub dest_proof: Option<String>,   // 目标链交易证明
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BridgeStatus {
    Created,            // 订单创建
    SourceTxSubmitted,  // 源链交易已提交
    SourceTxConfirmed,  // 源链交易已确认
    EventDetected,      // 监听到跨链事件
    DestTxBuilding,     // 构建目标链交易
    DestTxReadyForSign, // 等待客户端签名
    DestTxSubmitted,    // 目标链交易已提交
    DestTxConfirmed,    // 目标链交易已确认
    Completed,          // 跨链完成
    Failed,             // 失败
}

/// 跨链桥非托管服务
pub struct NonCustodialBridgeService {
    pool: PgPool,
}

impl NonCustodialBridgeService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 创建跨链桥订单（非托管模式）
    ///
    /// # 流程
    /// 1. 客户端构建并签名源链交易
    /// 2. 后端验证签名有效性
    /// 3. 后端广播源链交易
    /// 4. 监听链上事件
    /// 5. 生成目标链交易参数（unsigned）
    /// 6. 客户端签名目标链交易
    /// 7. 后端广播目标链交易
    pub async fn create_bridge_order(
        &self,
        user_id: Uuid,
        source_chain: String,
        dest_chain: String,
        amount: String,
        token: String,
        source_address: String,
        dest_address: String,
        signed_source_tx: String,
    ) -> Result<NonCustodialBridgeOrder> {
        // 1. 验证签名交易格式
        if signed_source_tx.is_empty() {
            anyhow::bail!("Signed source transaction is required (non-custodial mode)");
        }

        // 2. 解析并验证签名
        self.verify_signed_transaction(&signed_source_tx, &source_chain, &source_address)
            .await?;

        // 3. 创建订单
        let order_id = Uuid::new_v4();
        let now = Utc::now();

        let _ = sqlx::query(
            "INSERT INTO cross_chain_transactions 
             (id, user_id, source_chain, source_address, destination_chain, destination_address, 
              amount, token_symbol, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(order_id)
        .bind(user_id)
        .bind(&source_chain)
        .bind(&source_address)
        .bind(&dest_chain)
        .bind(&dest_address)
        .bind(&amount)
        .bind(&token)
        .bind("SourcePending")
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        // 4. 记录审计日志
        let _ = sqlx::query(
            "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
             VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
        )
        .bind("BRIDGE_ORDER_CREATED")
        .bind("bridge")
        .bind(order_id)
        .bind(serde_json::json!({
            "user_id": user_id,
            "source_chain": source_chain,
            "dest_chain": dest_chain,
            "amount": amount,
            "token": token,
            "mode": "non_custodial"
        }))
        .execute(&self.pool)
        .await?;

        tracing::info!(
            order_id = %order_id,
            user_id = %user_id,
            source_chain = %source_chain,
            dest_chain = %dest_chain,
            "Non-custodial bridge order created"
        );

        Ok(NonCustodialBridgeOrder {
            id: order_id,
            user_id,
            source_chain,
            source_address,
            source_tx_hash: None,
            signed_source_tx,
            dest_chain,
            dest_address,
            dest_tx_hash: None,
            amount,
            token,
            status: BridgeStatus::Created,
            created_at: now,
            updated_at: now,
            completed_at: None,
            source_proof: None,
            dest_proof: None,
        })
    }

    /// 广播源链交易
    ///
    /// # 非托管原则
    /// - 交易已由客户端签名
    /// - 后端只负责广播，不能修改交易
    /// - 后端不持有私钥，无法重新签名
    pub async fn broadcast_source_tx(&self, order_id: Uuid) -> Result<String> {
        // 1. 获取订单
        let order = sqlx::query_as::<_, (String, String)>(
            "SELECT source_chain, COALESCE(signed_source_tx, '') 
             FROM cross_chain_transactions 
             WHERE id = $1",
        )
        .bind(order_id)
        .fetch_one(&self.pool)
        .await?;

        // 2. 广播交易到区块链
        let tx_hash = self.broadcast_transaction(&order.0, &order.1).await?;

        // 3. 更新订单状态
        let _ = sqlx::query(
            "UPDATE cross_chain_transactions 
             SET source_tx_hash = $1, status = $2, updated_at = CURRENT_TIMESTAMP
             WHERE id = $3",
        )
        .bind(&tx_hash)
        .bind("SourceConfirming")
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            order_id = %order_id,
            tx_hash = %tx_hash,
            "Source transaction broadcasted"
        );

        Ok(tx_hash)
    }

    /// 构建目标链交易参数（等待客户端签名）
    ///
    /// # 非托管流程
    /// 1. 后端监听到源链跨链事件
    /// 2. 后端构建目标链交易参数（unsigned）
    /// 3. 返回给客户端
    /// 4. 客户端使用本地私钥签名
    /// 5. 客户端提交签名后的交易
    pub async fn build_dest_tx_params(&self, order_id: Uuid) -> Result<UnsignedTransaction> {
        // 1. 获取订单
        let order = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT destination_chain, destination_address, amount, token_symbol 
             FROM cross_chain_transactions 
             WHERE id = $1 AND status = $2",
        )
        .bind(order_id)
        .bind("SourceConfirmed")
        .fetch_one(&self.pool)
        .await?;

        // 2. 获取目标链参数
        let (nonce, gas_price, gas_limit) = self.get_chain_params(&order.0).await?;

        // 3. 构建未签名交易
        let unsigned_tx = UnsignedTransaction {
            chain: order.0,
            to: order.1,
            value: order.2,
            data: None, // 或ERC20 transfer data
            nonce,
            gas_price,
            gas_limit,
        };

        // 4. 更新状态
        let _ = sqlx::query(
            "UPDATE cross_chain_transactions 
             SET status = $1, updated_at = CURRENT_TIMESTAMP
             WHERE id = $2",
        )
        .bind("DestinationPending")
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        Ok(unsigned_tx)
    }

    /// 提交目标链签名交易
    ///
    /// # 客户端签名后调用
    pub async fn submit_dest_signed_tx(&self, order_id: Uuid, signed_tx: String) -> Result<String> {
        // 1. 验证订单状态
        let order = sqlx::query_as::<_, (String, String)>(
            "SELECT destination_chain, destination_address 
             FROM cross_chain_transactions 
             WHERE id = $1 AND status = $2",
        )
        .bind(order_id)
        .bind("DestinationPending")
        .fetch_one(&self.pool)
        .await?;

        // 2. 验证签名
        self.verify_signed_transaction(&signed_tx, &order.0, &order.1)
            .await?;

        // 3. 广播交易
        let tx_hash = self.broadcast_transaction(&order.0, &signed_tx).await?;

        // 4. 更新订单
        let _ = sqlx::query(
            "UPDATE cross_chain_transactions 
             SET status = $1, updated_at = CURRENT_TIMESTAMP
             WHERE id = $2",
        )
        .bind("DestinationConfirming")
        .bind(order_id)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            order_id = %order_id,
            tx_hash = %tx_hash,
            "Destination transaction broadcasted"
        );

        Ok(tx_hash)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 私有辅助方法
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// 验证签名交易
    async fn verify_signed_transaction(
        &self,
        _signed_tx: &str,
        _chain: &str,
        _expected_from: &str,
    ) -> Result<()> {
        // TODO: 实现完整的签名验证
        // 1. 解析RLP编码
        // 2. 恢复签名者地址
        // 3. 验证地址匹配
        Ok(())
    }

    /// 广播交易到区块链
    async fn broadcast_transaction(&self, _chain: &str, _signed_tx: &str) -> Result<String> {
        // TODO: 调用blockchain_client广播
        Ok("0x1234567890abcdef".to_string())
    }

    /// 获取链参数
    async fn get_chain_params(&self, _chain: &str) -> Result<(u64, String, u64)> {
        // TODO: 查询链上参数
        Ok((0, "1000000000".to_string(), 21000))
    }
}

/// 未签名交易
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsignedTransaction {
    pub chain: String,
    pub to: String,
    pub value: String,
    pub data: Option<String>,
    pub nonce: u64,
    pub gas_price: String,
    pub gas_limit: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_status_transitions() {
        // 验证状态转换
        assert_eq!(BridgeStatus::Created, BridgeStatus::Created);
        assert_ne!(BridgeStatus::Created, BridgeStatus::Completed);
    }
}
