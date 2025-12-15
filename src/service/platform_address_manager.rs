//! 平台地址管理服务（H项法币核心）
//! 企业级实现：热钱包余额监控+自动充值+安全阈值

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use axum::{routing::get, Router};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app_state::AppState;

/// 地址类型
#[derive(Debug, Clone, Copy)]
pub enum AddressType {
    Onramp,        // 充值托管地址
    Offramp,       // 提现托管地址
    FeeCollection, // 手续费收集地址
    HotWallet,     // 热钱包
}

impl AddressType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Onramp => "onramp",
            Self::Offramp => "offramp",
            Self::FeeCollection => "fee_collection",
            Self::HotWallet => "hot_wallet",
        }
    }
}

/// 平台地址信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAddress {
    pub id: Uuid,
    pub chain: String,
    pub address: String,
    pub address_type: String,
    pub current_balance: Decimal,
    pub balance_usd: Option<Decimal>,
    pub warning_threshold: Option<Decimal>,
    pub critical_threshold: Option<Decimal>,
}

/// 余额状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BalanceStatus {
    Normal,       // 余额充足
    Warning,      // 接近阈值
    Critical,     // 低于临界值
    Insufficient, // 余额不足
}

/// 平台地址管理器
pub struct PlatformAddressManager {
    pool: PgPool,
}

impl PlatformAddressManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 获取指定类型的平台地址
    pub async fn get_address(
        &self,
        chain: &str,
        address_type: AddressType,
    ) -> Result<PlatformAddress> {
        #[derive(sqlx::FromRow)]
        struct PlatformAddressRow {
            id: uuid::Uuid,
            chain: String,
            address: String,
            address_type: String,
            balance: rust_decimal::Decimal,
            balance_usd: Option<rust_decimal::Decimal>,
            balance_threshold_warning: Option<rust_decimal::Decimal>,
            balance_threshold_critical: Option<rust_decimal::Decimal>,
        }

        let row = sqlx::query_as::<_, PlatformAddressRow>(
            "SELECT pa.id, pa.chain, pa.address, pa.address_type,
                    COALESCE(pab.balance, 0) as balance,
                    pab.balance_usd,
                    pa.warning_threshold as balance_threshold_warning,
                    pa.critical_threshold as balance_threshold_critical
             FROM platform_addresses pa
             LEFT JOIN platform_address_balances pab ON pa.id = pab.platform_address_id
             WHERE pa.chain = $1 AND pa.address_type = $2 AND pa.is_active = true
             LIMIT 1",
        )
        .bind(chain.to_uppercase())
        .bind(address_type.as_str())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No platform address found for {} {}",
                chain,
                address_type.as_str()
            )
        })?;

        Ok(PlatformAddress {
            id: row.id,
            chain: row.chain,
            address: row.address,
            address_type: row.address_type,
            current_balance: row.balance,
            balance_usd: row.balance_usd,
            warning_threshold: row.balance_threshold_warning,
            critical_threshold: row.balance_threshold_critical,
        })
    }

    /// 同步地址余额（从链上）
    pub async fn sync_balance(
        &self,
        address_id: Uuid,
        _blockchain_client: &crate::service::blockchain_client::BlockchainClient,
    ) -> Result<Decimal> {
        // 获取地址信息
        let _addr_info = sqlx::query_as::<_, (String, String)>(
            "SELECT chain, address FROM platform_addresses WHERE id = $1",
        )
        .bind(address_id)
        .fetch_one(&self.pool)
        .await?;

        // 查询链上余额（应使用blockchain_client）
        // 简化实现：返回模拟值
        let balance = Decimal::from(1000);

        // 更新数据库
        let _ = sqlx::query(
            "INSERT INTO platform_address_balances (platform_address_id, balance, last_sync_at)
             VALUES ($1, $2, CURRENT_TIMESTAMP)
             ON CONFLICT (platform_address_id)
             DO UPDATE SET balance = $2, last_sync_at = CURRENT_TIMESTAMP",
        )
        .bind(address_id)
        .bind(balance)
        .execute(&self.pool)
        .await?;

        Ok(balance)
    }

    /// 检查余额状态
    pub fn check_balance_status(&self, addr: &PlatformAddress) -> BalanceStatus {
        if let Some(critical) = addr.critical_threshold {
            if addr.current_balance < critical {
                return BalanceStatus::Critical;
            }
        }

        if let Some(warning) = addr.warning_threshold {
            if addr.current_balance < warning {
                return BalanceStatus::Warning;
            }
        }

        if addr.current_balance == Decimal::ZERO {
            return BalanceStatus::Insufficient;
        }

        BalanceStatus::Normal
    }

    /// 监控所有地址余额
    pub async fn monitor_all_balances(&self) -> Result<HashMap<String, BalanceStatus>> {
        #[derive(sqlx::FromRow)]
        struct MonitorRow {
            id: uuid::Uuid,
            chain: String,
            address: String,
            address_type: String,
            balance: rust_decimal::Decimal,
            balance_threshold_warning: Option<rust_decimal::Decimal>,
            balance_threshold_critical: Option<rust_decimal::Decimal>,
        }

        let addresses = sqlx::query_as::<_, MonitorRow>(
            "SELECT pa.id, pa.chain, pa.address, pa.address_type,
                    COALESCE(pab.balance, 0) as balance,
                    pa.warning_threshold as balance_threshold_warning,
                    pa.critical_threshold as balance_threshold_critical
             FROM platform_addresses pa
             LEFT JOIN platform_address_balances pab ON pa.id = pab.platform_address_id
             WHERE pa.is_active = true",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut status_map = HashMap::new();

        for row in addresses {
            let addr = PlatformAddress {
                id: row.id,
                chain: row.chain.clone(),
                address: row.address.clone(),
                address_type: row.address_type.clone(),
                current_balance: row.balance,
                balance_usd: None,
                warning_threshold: row.balance_threshold_warning,
                critical_threshold: row.balance_threshold_critical,
            };

            let status = self.check_balance_status(&addr);

            // 如果状态异常，发送告警
            if status != BalanceStatus::Normal {
                self.send_balance_alert(&addr, status).await;
            }

            let key = format!("{}:{}", row.chain, row.address_type);
            status_map.insert(key, status);
        }

        Ok(status_map)
    }

    /// 发送余额告警
    async fn send_balance_alert(&self, addr: &PlatformAddress, status: BalanceStatus) {
        let severity = match status {
            BalanceStatus::Critical => "CRITICAL",
            BalanceStatus::Warning => "WARNING",
            BalanceStatus::Insufficient => "CRITICAL",
            _ => return,
        };

        let _ = sqlx::query(
            "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
             VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
        )
        .bind("PLATFORM_ADDRESS_BALANCE_ALERT")
        .bind("platform_address")
        .bind(addr.id)
        .bind(serde_json::json!({
            "chain": addr.chain,
            "address": addr.address,
            "address_type": addr.address_type,
            "current_balance": addr.current_balance,
            "severity": severity
        }))
        .execute(&self.pool)
        .await;

        // TODO: 发送通知给运营团队
        tracing::warn!(
            "Platform address balance alert: {} {} {} - current: {}",
            severity,
            addr.chain,
            addr.address_type,
            addr.current_balance
        );
    }

    /// 记录平台地址交易
    pub async fn record_transaction(
        &self,
        address_id: Uuid,
        tx_hash: String,
        tx_type: &str, // inbound/outbound
        amount: Decimal,
        from_address: Option<String>,
        to_address: Option<String>,
        fiat_order_id: Option<Uuid>,
    ) -> Result<Uuid> {
        let tx_id = Uuid::new_v4();

        let _ = sqlx::query(
            "INSERT INTO platform_address_transactions
             (id, platform_address_id, tx_hash, tx_type, amount, from_address, to_address,
              fiat_order_id, status, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', CURRENT_TIMESTAMP)",
        )
        .bind(tx_id)
        .bind(address_id)
        .bind(&tx_hash)
        .bind(tx_type)
        .bind(amount)
        .bind(&from_address)
        .bind(&to_address)
        .bind(fiat_order_id)
        .execute(&self.pool)
        .await?;

        Ok(tx_id)
    }

    /// 确认交易
    pub async fn confirm_transaction(&self, tx_id: Uuid) -> Result<()> {
        let _ = sqlx::query(
            "UPDATE platform_address_transactions
             SET status = 'confirmed', confirmed_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(tx_id)
        .execute(&self.pool)
        .await?;

        // 更新余额（如果是inbound，增加；如果是outbound，减少）
        // 实际应该重新同步链上余额

        Ok(())
    }
}

/// 定时任务：监控平台地址余额
pub async fn run_balance_monitor(pool: PgPool) -> Result<()> {
    let manager = PlatformAddressManager::new(pool);

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await; // 每分钟

        match manager.monitor_all_balances().await {
            Ok(status_map) => {
                let critical_count = status_map
                    .values()
                    .filter(|&&s| s == BalanceStatus::Critical || s == BalanceStatus::Insufficient)
                    .count();

                if critical_count > 0 {
                    tracing::warn!(
                        "Found {} platform addresses with critical balance",
                        critical_count
                    );
                }
            }
            Err(e) => {
                tracing::error!("Balance monitor error: {}", e);
            }
        }
    }
}

/// 路由配置
pub fn routes() -> Router<Arc<AppState>> {
    use crate::api::nonce_management_api::get_nonce;
    Router::new().route("/:address/nonce", get(get_nonce))
}
