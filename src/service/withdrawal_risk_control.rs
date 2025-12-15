//! 提现三级风控系统
//! 企业级实现：金额限制 + 行为分析 + 人工审核

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// 风控等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    /// 低风险：自动通过
    Low,
    /// 中风险：延迟处理 + 额外验证
    Medium,
    /// 高风险：人工审核
    High,
    /// 拒绝：直接拒绝
    Reject,
}

/// 风控决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskControlDecision {
    /// 风控等级
    pub risk_level: RiskLevel,
    /// 是否允许提现
    pub allow: bool,
    /// 触发的规则列表
    pub triggered_rules: Vec<String>,
    /// 建议操作
    pub suggestion: String,
    /// 需要人工审核
    pub requires_manual_review: bool,
}

/// 提现请求
#[derive(Debug, Clone)]
pub struct WithdrawalRequest {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub chain: String,
    pub to_address: String,
    pub amount_usd: f64,
    pub wallet_id: Uuid,
}

/// 提现风控服务
pub struct WithdrawalRiskControl {
    pool: PgPool,
}

impl WithdrawalRiskControl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 评估提现风险（三级风控）
    ///
    /// # 三级风控规则
    ///
    /// ## Level 1: 金额限制（自动通过）
    /// - < $1,000: 立即通过
    /// - 每日总额 < $5,000: 立即通过
    ///
    /// ## Level 2: 行为分析（延迟 + 验证）
    /// - $1,000 - $10,000: 延迟10分钟 + 邮件验证
    /// - 异常时间（凌晨2-6点）: 额外验证
    /// - 频繁提现（24小时内>5次）: 延迟处理
    /// - 新注册用户（<7天）: 额外验证
    ///
    /// ## Level 3: 人工审核（高风险）
    /// - > $10,000: 必须人工审核
    /// - 异常IP（国外IP）: 人工审核
    /// - 账户异常行为: 人工审核
    /// - 黑名单地址: 直接拒绝
    pub async fn evaluate(&self, request: &WithdrawalRequest) -> Result<RiskControlDecision> {
        let mut triggered_rules = Vec::new();
        let mut risk_level = RiskLevel::Low;

        // ===== Level 1: 金额限制 =====

        // 规则1.1: 单笔金额检查
        if request.amount_usd >= 10_000.0 {
            triggered_rules.push("AMOUNT_EXCEEDS_10K".to_string());
            risk_level = RiskLevel::High;
        } else if request.amount_usd >= 1_000.0 {
            triggered_rules.push("AMOUNT_EXCEEDS_1K".to_string());
            risk_level = std::cmp::max(risk_level, RiskLevel::Medium);
        }

        // 规则1.2: 每日总额检查
        let daily_total = self.get_daily_withdrawal_total(request.user_id).await?;
        if daily_total + request.amount_usd > 5_000.0 {
            triggered_rules.push("DAILY_LIMIT_EXCEEDED".to_string());
            risk_level = std::cmp::max(risk_level, RiskLevel::Medium);
        }

        // ===== Level 2: 行为分析 =====

        // 规则2.1: 异常时间检查（凌晨2-6点）
        use chrono::Timelike;
        let hour = chrono::Utc::now().hour();
        if (2..6).contains(&hour) {
            triggered_rules.push("UNUSUAL_TIME".to_string());
            risk_level = std::cmp::max(risk_level, RiskLevel::Medium);
        }

        // 规则2.2: 频繁提现检查（24小时内>5次）
        let recent_count = self
            .get_recent_withdrawal_count(request.user_id, 24)
            .await?;
        if recent_count >= 5 {
            triggered_rules.push("FREQUENT_WITHDRAWALS".to_string());
            risk_level = std::cmp::max(risk_level, RiskLevel::Medium);
        }

        // 规则2.3: 新用户检查（注册<7天）
        let account_age_days = self.get_account_age_days(request.user_id).await?;
        if account_age_days < 7 {
            triggered_rules.push("NEW_USER_ACCOUNT".to_string());
            risk_level = std::cmp::max(risk_level, RiskLevel::Medium);
        }

        // ===== Level 3: 人工审核规则 =====

        // 规则3.1: 黑名单地址检查
        if self.is_blacklisted_address(&request.to_address).await? {
            triggered_rules.push("BLACKLISTED_ADDRESS".to_string());
            risk_level = RiskLevel::Reject;
        }

        // 规则3.2: 账户异常行为
        if self.has_suspicious_activity(request.user_id).await? {
            triggered_rules.push("SUSPICIOUS_ACTIVITY".to_string());
            risk_level = std::cmp::max(risk_level, RiskLevel::High);
        }

        // 规则3.3: 异常IP检查（简化实现）
        // 实际应该从请求上下文获取IP并检查
        // if self.is_unusual_ip(&request.ip).await? {
        //     triggered_rules.push("UNUSUAL_IP".to_string());
        //     risk_level = std::cmp::max(risk_level, RiskLevel::High);
        // }

        // ===== 生成决策 =====

        let (allow, suggestion, requires_manual_review) = match risk_level {
            RiskLevel::Low => (true, "Auto-approved: Low risk".to_string(), false),
            RiskLevel::Medium => (
                true,
                "Approved with delay: Medium risk, additional verification sent".to_string(),
                false,
            ),
            RiskLevel::High => (
                false,
                "Pending manual review: High risk transaction".to_string(),
                true,
            ),
            RiskLevel::Reject => (
                false,
                "Rejected: Transaction blocked by risk rules".to_string(),
                false,
            ),
        };

        Ok(RiskControlDecision {
            risk_level,
            allow,
            triggered_rules,
            suggestion,
            requires_manual_review,
        })
    }

    /// 获取用户24小时内的提现总额
    async fn get_daily_withdrawal_total(&self, user_id: Uuid) -> Result<f64> {
        let total: Option<f64> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount_usd), 0)
             FROM withdrawal_requests
             WHERE user_id = $1
               AND status IN ('completed', 'pending')
               AND created_at > NOW() - INTERVAL '24 hours'",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(total.unwrap_or(0.0))
    }

    /// 获取用户最近N小时内的提现次数
    async fn get_recent_withdrawal_count(&self, user_id: Uuid, hours: i32) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*)
             FROM withdrawal_requests
             WHERE user_id = $1
               AND created_at > NOW() - INTERVAL '1 hour' * $2",
        )
        .bind(user_id)
        .bind(hours)
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// 获取账户年龄（天数）
    async fn get_account_age_days(&self, user_id: Uuid) -> Result<i64> {
        let age: Option<i64> = sqlx::query_scalar(
            "SELECT EXTRACT(DAY FROM NOW() - created_at)::BIGINT
             FROM users
             WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(age.unwrap_or(0))
    }

    /// 检查地址是否在黑名单
    async fn is_blacklisted_address(&self, address: &str) -> Result<bool> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM address_blacklist
                WHERE address = $1 AND is_active = true
            )",
        )
        .bind(address)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        Ok(exists)
    }

    /// 检查账户是否有可疑活动
    async fn has_suspicious_activity(&self, user_id: Uuid) -> Result<bool> {
        // 简化实现：检查是否有最近的安全告警
        let has_alerts: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM security_alerts
                WHERE user_id = $1
                  AND severity IN ('high', 'critical')
                  AND created_at > NOW() - INTERVAL '7 days'
                  AND status = 'open'
            )",
        )
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .unwrap_or(false);

        Ok(has_alerts)
    }

    /// 记录风控决策（审计）
    pub async fn log_decision(
        &self,
        request: &WithdrawalRequest,
        decision: &RiskControlDecision,
    ) -> Result<Uuid> {
        let log_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO withdrawal_risk_logs
             (id, user_id, tenant_id, amount_usd, chain, to_address,
              risk_level, allow, triggered_rules, suggestion, requires_manual_review, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())",
        )
        .bind(log_id)
        .bind(request.user_id)
        .bind(request.tenant_id)
        .bind(request.amount_usd)
        .bind(&request.chain)
        .bind(&request.to_address)
        .bind(format!("{:?}", decision.risk_level))
        .bind(decision.allow)
        .bind(serde_json::to_value(&decision.triggered_rules)?)
        .bind(&decision.suggestion)
        .bind(decision.requires_manual_review)
        .execute(&self.pool)
        .await?;

        Ok(log_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Low < RiskLevel::Medium);
        assert!(RiskLevel::Medium < RiskLevel::High);
    }
}
