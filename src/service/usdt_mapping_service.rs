//! USDT到各链资产映射服务
//!
//! 企业级实现：法币充值后自动将USDT映射到目标链资产

use std::sync::Arc;

use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::service::cross_chain_bridge_service::{CrossChainBridgeService, CrossChainSwapRequest};

/// USDT映射服务
pub struct UsdtMappingService {
    pool: PgPool,
    bridge_service: Arc<CrossChainBridgeService>,
}

impl UsdtMappingService {
    /// 创建新的USDT映射服务
    pub fn new(pool: PgPool, bridge_service: Arc<CrossChainBridgeService>) -> Self {
        Self {
            pool,
            bridge_service,
        }
    }

    /// 映射USDT到目标链✅完整实现
    pub async fn map_usdt_to_chain(
        &self,
        user_id: Uuid,
        order_id: Uuid,
        target_chain: String,
    ) -> Result<String> {
        // ✅验证输入
        if target_chain.trim().is_empty() {
            anyhow::bail!("Target chain required");
        }

        // 1. 获取订单
        let order = self.get_fiat_order(order_id).await?;
        if order.status != "completed" {
            anyhow::bail!("Order not completed: status={}", order.status);
        }

        // 2. 获取目标链钱包
        let wallet = self.get_user_wallet(user_id, &target_chain).await?;

        // 3. 获取源链钱包(Ethereum USDT)✅
        let source_wallet = match self.get_user_wallet(user_id, "ethereum").await {
            Ok(w) => w,
            Err(_) => self.get_user_wallet(user_id, "eth").await?,
        };

        let swap_request = CrossChainSwapRequest {
            user_id,
            source_chain: "ethereum".to_string(), // USDT默认在Ethereum主网
            source_token: "USDT".to_string(),
            source_amount: order
                .crypto_amount
                .parse::<f64>()
                .map_err(|e| anyhow::anyhow!("Invalid crypto amount: {}", e))?,
            source_wallet_id: source_wallet.id,
            target_chain: target_chain.clone(),
            target_token: "USDT".to_string(), // 目标链也使用USDT
            target_wallet_id: Some(wallet.id),
        };

        // 4. 执行跨链桥接
        let result = self.bridge_service.execute_swap(swap_request).await?;

        // 5. 更新订单映射状态
        self.update_order_mapping_status(order_id, &result.swap_id, &target_chain)
            .await?;

        Ok(result.swap_id)
    }

    /// 获取法币订单✅简化
    async fn get_fiat_order(&self, order_id: Uuid) -> Result<FiatOrder> {
        #[derive(sqlx::FromRow)]
        struct OrderRow {
            id: Uuid,
            user_id: Uuid,
            tenant_id: Uuid,
            fiat_amount: rust_decimal::Decimal,
            crypto_amount: rust_decimal::Decimal,
            crypto_token: String,
            status: String,
            created_at: chrono::DateTime<chrono::Utc>,
        }

        let row = sqlx::query_as::<_, OrderRow>(
            "SELECT id, user_id, tenant_id, fiat_amount, crypto_amount, crypto_token, status, created_at FROM fiat.orders WHERE id = $1"
        ).bind(order_id).fetch_one(&self.pool).await?;

        Ok(FiatOrder {
            id: row.id,
            user_id: row.user_id,
            tenant_id: row.tenant_id,
            fiat_amount: row.fiat_amount.to_string(),
            crypto_amount: row.crypto_amount.to_string(),
            token: row.crypto_token,
            status: row.status,
            created_at: row.created_at,
        })
    }

    /// 获取用户钱包✅完整
    async fn get_user_wallet(&self, user_id: Uuid, chain: &str) -> Result<UserWallet> {
        #[derive(sqlx::FromRow)]
        struct WalletRow {
            id: Uuid,
            address: String,
            chain_symbol: Option<String>,
        }

        let row = sqlx::query_as::<_, WalletRow>(
            "SELECT id, address, chain_symbol FROM wallets WHERE user_id = $1 AND (chain = $2 OR LOWER(chain_symbol) = LOWER($2)) LIMIT 1"
        ).bind(user_id).bind(chain).fetch_one(&self.pool).await?;

        Ok(UserWallet {
            id: row.id,
            address: row.address,
            chain: row.chain_symbol.unwrap_or_else(|| chain.to_string()),
        })
    }

    /// 更新订单映射状态
    async fn update_order_mapping_status(
        &self,
        order_id: Uuid,
        swap_id: &str,
        target_chain: &str,
    ) -> Result<()> {
        // ✅修复表名和字段
        sqlx::query("UPDATE fiat.orders SET swap_tx_hash = $1, updated_at = NOW() WHERE id = $2")
            .bind(swap_id)
            .bind(order_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("UPDATE fiat.asset_mappings SET status = 'completed', swap_tx_hash = $1, target_chain = $2, completed_at = NOW() WHERE order_id = $3")
            .bind(swap_id).bind(target_chain).bind(order_id).execute(&self.pool).await.ok();
        Ok(())
    }

    /// 自动映射USDT（根据用户默认设置或订单信息）
    pub async fn auto_map_usdt(&self, user_id: Uuid, order_id: Uuid) -> Result<Option<String>> {
        // 获取用户默认目标链（从用户设置或订单信息）
        let target_chain = self.get_user_default_chain(user_id).await?;

        if let Some(chain) = target_chain {
            let swap_id = self.map_usdt_to_chain(user_id, order_id, chain).await?;
            Ok(Some(swap_id))
        } else {
            // 如果没有默认链，返回None，需要用户选择
            Ok(None)
        }
    }

    /// 获取用户默认目标链
    async fn get_user_default_chain(&self, _user_id: Uuid) -> Result<Option<String>> {
        // 从用户设置中获取，或返回用户最常用的链
        // 简化实现：返回None，需要用户选择
        Ok(None)
    }
}

/// 法币订单✅
#[allow(dead_code)]
struct FiatOrder {
    id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
    fiat_amount: String,
    crypto_amount: String,
    token: String,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// 用户钱包（简化结构）
#[allow(dead_code)]
struct UserWallet {
    id: Uuid,
    address: String,
    chain: String,
}
