//! 返佣收入服务（Referral Commission Service）
//! 生产级实现：追踪和管理来自支付服务商的返佣收入

use std::str::FromStr;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// 返佣状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommissionStatus {
    Pending,   // 待确认（订单刚完成）
    Confirmed, // 服务商已确认
    Paid,      // 已收到返佣
    Failed,    // 失败（如订单退款）
    Cancelled, // 已取消
}

impl ToString for CommissionStatus {
    fn to_string(&self) -> String {
        match self {
            CommissionStatus::Pending => "pending".to_string(),
            CommissionStatus::Confirmed => "confirmed".to_string(),
            CommissionStatus::Paid => "paid".to_string(),
            CommissionStatus::Failed => "failed".to_string(),
            CommissionStatus::Cancelled => "cancelled".to_string(),
        }
    }
}

impl FromStr for CommissionStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(CommissionStatus::Pending),
            "confirmed" => Ok(CommissionStatus::Confirmed),
            "paid" => Ok(CommissionStatus::Paid),
            "failed" => Ok(CommissionStatus::Failed),
            "cancelled" => Ok(CommissionStatus::Cancelled),
            _ => anyhow::bail!("Invalid commission status: {}", s),
        }
    }
}

/// 返佣记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferralCommission {
    pub id: Uuid,
    pub order_id: Uuid,
    pub order_type: String,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub provider_name: String,
    pub provider_order_id: Option<String>,
    pub transaction_amount: Decimal,
    pub provider_fee: Decimal,
    pub provider_fee_percent: Decimal,
    pub commission_rate: Decimal,
    pub commission_amount: Decimal,
    pub commission_currency: String,
    pub status: String,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub payment_reference: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub notes: Option<String>,
}

/// 服务商返佣配置
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderCommissionConfig {
    pub id: Uuid,
    pub provider_name: String,
    pub provider_display_name: String,
    pub default_commission_rate: Decimal,
    pub onramp_commission_rate: Option<Decimal>,
    pub offramp_commission_rate: Option<Decimal>,
    pub tier1_volume: Option<Decimal>,
    pub tier1_rate: Option<Decimal>,
    pub tier2_volume: Option<Decimal>,
    pub tier2_rate: Option<Decimal>,
    pub tier3_volume: Option<Decimal>,
    pub tier3_rate: Option<Decimal>,
    pub partner_id: Option<String>,
    pub settlement_period: String,
    pub minimum_payout: Decimal,
    pub payment_terms_days: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 月度收入汇总
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MonthlyRevenueSummary {
    pub month: DateTime<Utc>,
    pub provider_name: String,
    pub order_type: String,
    pub transaction_count: i64,
    pub total_volume: Decimal,
    pub total_provider_fees: Decimal,
    pub total_commission_income: Decimal,
    pub avg_commission_rate: Decimal,
    pub paid_count: i64,
    pub paid_amount: Decimal,
    pub pending_count: i64,
    pub pending_amount: Decimal,
}

/// 返佣服务
pub struct ReferralCommissionService {
    pool: PgPool,
}

impl ReferralCommissionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 记录返佣（订单完成时自动调用）
    ///
    /// # 使用场景
    /// 当fiat_order状态变为completed时，自动计算并记录返佣
    ///
    /// # 示例
    /// ```rust
    /// let commission_id = service
    ///     .record_commission(
    ///         order_id,
    ///         "offramp",
    ///         user_id,
    ///         "onramper",
    ///         Some("onramper_12345"),
    ///         Decimal::from_str("27225.00")?, // 交易金额
    ///         Decimal::from_str("544.50")?,   // 服务商费用
    ///         Decimal::from_str("2.0")?,      // 服务商费率2%
    ///     )
    ///     .await?;
    /// ```
    pub async fn record_commission(
        &self,
        order_id: Uuid,
        order_type: &str,
        user_id: Uuid,
        provider_name: &str,
        provider_order_id: Option<String>,
        transaction_amount: Decimal,
        provider_fee: Decimal,
        provider_fee_percent: Decimal,
    ) -> Result<Uuid> {
        // 1. 获取服务商返佣配置
        let config = self.get_provider_config(provider_name).await?;

        if !config.is_active {
            anyhow::bail!(
                "Provider {} is not active for commission tracking",
                provider_name
            );
        }

        // 2. 计算返佣率（根据订单类型和月交易量）
        let commission_rate = self
            .calculate_commission_rate(&config, order_type, user_id, transaction_amount)
            .await?;

        // 3. 计算返佣金额
        let commission_amount = transaction_amount * commission_rate / Decimal::from(100);

        // 4. 插入返佣记录
        let record = sqlx::query_as::<_, (Uuid,)>(
            r#"
            INSERT INTO revenue.referral_commissions (
                order_id,
                order_type,
                user_id,
                tenant_id,
                provider_name,
                provider_order_id,
                transaction_amount,
                provider_fee,
                provider_fee_percent,
                commission_rate,
                commission_amount,
                commission_currency,
                status,
                created_at,
                updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'USD', 'pending', NOW(), NOW()
            )
            RETURNING id
            "#,
        )
        .bind(order_id)
        .bind(order_type)
        .bind(user_id)
        .bind(Uuid::nil()) // tenant_id (多租户支持)
        .bind(provider_name)
        .bind(provider_order_id)
        .bind(transaction_amount)
        .bind(provider_fee)
        .bind(provider_fee_percent)
        .bind(commission_rate)
        .bind(commission_amount)
        .fetch_one(&self.pool)
        .await
        .context("Failed to insert referral commission record")?;

        tracing::info!(
            commission_id=%record.0,
            order_id=%order_id,
            provider=%provider_name,
            amount=%commission_amount,
            rate=%commission_rate,
            "Referral commission recorded"
        );

        Ok(record.0)
    }

    /// 获取服务商返佣配置
    pub async fn get_provider_config(
        &self,
        provider_name: &str,
    ) -> Result<ProviderCommissionConfig> {
        let config = sqlx::query_as::<_, ProviderCommissionConfig>(
            r#"
            SELECT
                id, provider_name, provider_display_name,
                default_commission_rate, onramp_commission_rate, offramp_commission_rate,
                tier1_volume, tier1_rate, tier2_volume, tier2_rate, tier3_volume, tier3_rate,
                partner_id, settlement_period, minimum_payout, payment_terms_days,
                is_active, created_at, updated_at
            FROM revenue.provider_commission_config
            WHERE provider_name = $1
            "#,
        )
        .bind(provider_name)
        .fetch_one(&self.pool)
        .await
        .context(format!("Provider config not found: {}", provider_name))?;

        Ok(config)
    }

    /// 计算返佣率（支持阶梯费率）
    async fn calculate_commission_rate(
        &self,
        config: &ProviderCommissionConfig,
        order_type: &str,
        user_id: Uuid,
        transaction_amount: Decimal,
    ) -> Result<Decimal> {
        // 1. 基础费率（根据订单类型）
        let base_rate = if order_type == "onramp" {
            config
                .onramp_commission_rate
                .unwrap_or(config.default_commission_rate)
        } else {
            config
                .offramp_commission_rate
                .unwrap_or(config.default_commission_rate)
        };

        // 2. 查询用户当月交易量（阶梯返佣）
        let monthly_volume: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(transaction_amount), 0)
            FROM revenue.referral_commissions
            WHERE user_id = $1
              AND provider_name = $2
              AND created_at >= DATE_TRUNC('month', NOW())
            "#,
        )
        .bind(user_id)
        .bind(&config.provider_name)
        .fetch_optional(&self.pool)
        .await?;

        let monthly_volume = monthly_volume.unwrap_or(Decimal::ZERO);
        let total_volume = monthly_volume + transaction_amount;

        // 3. 应用阶梯费率
        let tier_rate =
            if let (Some(tier3_vol), Some(tier3_rate)) = (config.tier3_volume, config.tier3_rate) {
                if total_volume >= tier3_vol {
                    tier3_rate
                } else if let (Some(tier2_vol), Some(tier2_rate)) =
                    (config.tier2_volume, config.tier2_rate)
                {
                    if total_volume >= tier2_vol {
                        tier2_rate
                    } else if let (Some(tier1_vol), Some(tier1_rate)) =
                        (config.tier1_volume, config.tier1_rate)
                    {
                        if total_volume >= tier1_vol {
                            tier1_rate
                        } else {
                            base_rate
                        }
                    } else {
                        base_rate
                    }
                } else {
                    base_rate
                }
            } else {
                base_rate
            };

        Ok(tier_rate)
    }

    /// 确认返佣（服务商Webhook回调）
    pub async fn confirm_commission(
        &self,
        commission_id: Uuid,
        payment_reference: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE revenue.referral_commissions
            SET status = 'confirmed',
                confirmed_at = NOW(),
                payment_reference = $2,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(commission_id)
        .bind(payment_reference)
        .execute(&self.pool)
        .await?;

        tracing::info!(commission_id=%commission_id, "Commission confirmed by provider");
        Ok(())
    }

    /// 标记为已支付
    pub async fn mark_as_paid(
        &self,
        commission_id: Uuid,
        payment_method: String,
        payment_reference: String,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE revenue.referral_commissions
            SET status = 'paid',
                paid_at = NOW(),
                payment_method = $2,
                payment_reference = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(commission_id)
        .bind(payment_method)
        .bind(payment_reference)
        .execute(&self.pool)
        .await?;

        tracing::info!(commission_id=%commission_id, "Commission marked as paid");
        Ok(())
    }

    /// 获取月度收入汇总
    pub async fn get_monthly_summary(
        &self,
        year: i32,
        month: u32,
    ) -> Result<Vec<MonthlyRevenueSummary>> {
        let summaries = sqlx::query_as::<_, MonthlyRevenueSummary>(
            r#"
            SELECT * FROM revenue.monthly_revenue_summary
            WHERE EXTRACT(YEAR FROM month) = $1
              AND EXTRACT(MONTH FROM month) = $2
            ORDER BY total_volume DESC
            "#,
        )
        .bind(year)
        .bind(month as i32)
        .fetch_all(&self.pool)
        .await?;

        Ok(summaries)
    }

    /// 获取待支付返佣总额
    pub async fn get_pending_payout(&self, provider_name: &str) -> Result<Decimal> {
        let amount: Option<Decimal> = sqlx::query_scalar(
            r#"
            SELECT COALESCE(SUM(commission_amount), 0)
            FROM revenue.referral_commissions
            WHERE provider_name = $1
              AND status IN ('confirmed', 'pending')
            "#,
        )
        .bind(provider_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(amount.unwrap_or(Decimal::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实数据库
    async fn test_record_commission() {
        // 测试记录返佣
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let service = ReferralCommissionService::new(pool);

        let order_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let commission_id = service
            .record_commission(
                order_id,
                "offramp",
                user_id,
                "onramper",
                Some("onramper_test_123".to_string()),
                Decimal::from_str("27225.00").unwrap(),
                Decimal::from_str("544.50").unwrap(),
                Decimal::from_str("2.0").unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(commission_id, Uuid::nil());
    }

    #[tokio::test]
    #[ignore]
    async fn test_tiered_commission_rate() {
        // 测试阶梯返佣率
        let pool = PgPool::connect(&std::env::var("DATABASE_URL").unwrap())
            .await
            .unwrap();
        let service = ReferralCommissionService::new(pool);

        let config = service.get_provider_config("onramper").await.unwrap();

        // 小额交易：使用默认费率
        let rate = service
            .calculate_commission_rate(
                &config,
                "offramp",
                Uuid::new_v4(),
                Decimal::from_str("1000.00").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rate, config.default_commission_rate);
    }
}
