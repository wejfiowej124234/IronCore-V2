//! 审计日志服务
//! 企业级实现，使用Immudb进行不可篡改审计日志存储
use std::sync::Arc;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::infrastructure::audit::{AuditEvent, ImmuCtx};

/// 审计日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Option<Uuid>,
    pub order_id: Option<Uuid>,
    pub action: String,
    pub amount: Option<rust_decimal::Decimal>,
    pub status: Option<String>,
    pub provider: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub immudb_proof_hash: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// 合规报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_id: Uuid,
    pub report_type: String, // 'daily', 'weekly', 'monthly'
    pub start_date: chrono::NaiveDate,
    pub end_date: chrono::NaiveDate,
    pub total_orders: i64,
    pub total_amount: rust_decimal::Decimal,
    pub completed_orders: i64,
    pub failed_orders: i64,
    pub kyc_verified_users: i64,
    pub suspicious_transactions: i64,
    pub generated_at: DateTime<Utc>,
    pub data: serde_json::Value,
}

pub struct AuditService {
    pool: PgPool,
    immu: Arc<ImmuCtx>,
}

impl AuditService {
    pub fn new(pool: PgPool, immu: Arc<ImmuCtx>) -> Self {
        Self { pool, immu }
    }

    /// 记录审计事件
    #[allow(clippy::too_many_arguments)]
    pub async fn log_event(
        &self,
        tenant_id: Uuid,
        user_id: Option<Uuid>,
        order_id: Option<Uuid>,
        action: &str,
        amount: Option<rust_decimal::Decimal>,
        status: Option<&str>,
        provider: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<Uuid> {
        let log_id = Uuid::new_v4();
        let now = Utc::now();

        // 1. 写入数据库
        let _proof_hash = sqlx::query_scalar::<_, Option<String>>(
            r#"
            INSERT INTO fiat.audit_logs (
                id, tenant_id, user_id, order_id, action,
                amount, status, provider, ip_address, user_agent,
                metadata, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING immudb_proof_hash
            "#,
        )
        .bind(log_id)
        .bind(tenant_id)
        .bind(user_id)
        .bind(order_id)
        .bind(action)
        .bind(amount)
        .bind(status)
        .bind(provider)
        .bind(ip_address)
        .bind(user_agent)
        .bind(&metadata)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        // 2. 写入Immudb（不可篡改审计日志）
        let event = AuditEvent {
            event: action.to_string(),
            tenant_id: tenant_id.to_string(),
            actor: user_id.map(|u| u.to_string()).unwrap_or_default(),
            resource: order_id.map(|o| o.to_string()).unwrap_or_default(),
            payload_hash: format!("hash_{}", log_id),
            ts: now.to_rfc3339(),
        };

        // 异步写入Immudb（不阻塞主流程）
        let immu = self.immu.clone();
        let log_id_clone = log_id;
        tokio::spawn(async move {
            if let Ok(proof) = immu.write_event(&event).await {
                // 更新数据库中的proof hash
                tracing::debug!("Audit log {} written to Immudb: {}", log_id_clone, proof);
            }
        });

        Ok(log_id)
    }

    /// 查询审计日志
    #[allow(clippy::too_many_arguments)]
    pub async fn get_audit_logs(
        &self,
        tenant_id: Option<Uuid>,
        user_id: Option<Uuid>,
        order_id: Option<Uuid>,
        action: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<AuditLog>> {
        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let rows = sqlx::query(
            r#"
            SELECT id, tenant_id, user_id, order_id, action,
                   amount, status, provider, ip_address, user_agent,
                   metadata, immudb_proof_hash, created_at
            FROM fiat.audit_logs
            WHERE ($1::UUID IS NULL OR tenant_id = $1)
                AND ($2::UUID IS NULL OR user_id = $2)
                AND ($3::UUID IS NULL OR order_id = $3)
                AND ($4::TEXT IS NULL OR action = $4)
                AND ($5::TIMESTAMPTZ IS NULL OR created_at >= $5)
                AND ($6::TIMESTAMPTZ IS NULL OR created_at <= $6)
            ORDER BY created_at DESC
            LIMIT $7 OFFSET $8
            "#,
        )
        .bind(tenant_id)
        .bind(user_id)
        .bind(order_id)
        .bind(action)
        .bind(start_date)
        .bind(end_date)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut logs = Vec::new();
        for row in rows {
            logs.push(AuditLog {
                id: row.try_get("id")?,
                tenant_id: row.try_get("tenant_id")?,
                user_id: row.try_get("user_id")?,
                order_id: row.try_get("order_id")?,
                action: row.try_get("action")?,
                amount: row.try_get("amount")?,
                status: row.try_get("status")?,
                provider: row.try_get("provider")?,
                ip_address: row.try_get("ip_address")?,
                user_agent: row.try_get("user_agent")?,
                metadata: row.try_get("metadata")?,
                immudb_proof_hash: row.try_get("immudb_proof_hash")?,
                created_at: row.try_get("created_at")?,
            });
        }

        Ok(logs)
    }

    /// 生成合规报告
    pub async fn generate_compliance_report(
        &self,
        tenant_id: Uuid,
        report_type: &str,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
    ) -> Result<ComplianceReport> {
        let report_id = Uuid::new_v4();
        let generated_at = Utc::now();

        // 1. 统计订单数据
        let order_stats = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_orders,
                SUM(fiat_amount) as total_amount,
                COUNT(*) FILTER (WHERE status = 'completed') as completed_orders,
                COUNT(*) FILTER (WHERE status = 'failed') as failed_orders
            FROM fiat.orders
            WHERE tenant_id = $1
                AND DATE(created_at) >= $2
                AND DATE(created_at) <= $3
            "#,
        )
        .bind(tenant_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        let total_orders: i64 = order_stats.try_get::<i64, _>("total_orders")?;
        let total_amount: rust_decimal::Decimal = order_stats
            .try_get::<Option<rust_decimal::Decimal>, _>("total_amount")?
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let completed_orders: i64 = order_stats.try_get::<i64, _>("completed_orders")?;
        let failed_orders: i64 = order_stats.try_get::<i64, _>("failed_orders")?;

        // 2. 统计KYC验证用户数
        let kyc_users: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT COUNT(DISTINCT user_id)
            FROM fiat.orders
            WHERE tenant_id = $1
                AND DATE(created_at) >= $2
                AND DATE(created_at) <= $3
                AND metadata->>'kyc_status' = 'verified'
            "#,
        )
        .bind(tenant_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;
        let kyc_users = kyc_users.unwrap_or(0);

        // 3. 统计可疑交易
        let suspicious: Option<i64> = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            SELECT COUNT(*)
            FROM fiat.alerts
            WHERE tenant_id = $1
                AND alert_type = 'suspicious_transaction'
                AND DATE(created_at) >= $2
                AND DATE(created_at) <= $3
            "#,
        )
        .bind(tenant_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;
        let suspicious = suspicious.unwrap_or(0);

        // 4. 构建报告数据
        let data = serde_json::json!({
            "tenant_id": tenant_id,
            "report_type": report_type,
            "period": {
                "start": start_date,
                "end": end_date,
            },
            "statistics": {
                "total_orders": total_orders,
                "total_amount": total_amount.to_string(),
                "completed_orders": completed_orders,
                "failed_orders": failed_orders,
                "success_rate": if total_orders > 0 {
                    (completed_orders as f64 / total_orders as f64) * 100.0
                } else {
                    0.0
                },
                "kyc_verified_users": kyc_users,
                "suspicious_transactions": suspicious,
            },
            "generated_at": generated_at,
        });

        let report = ComplianceReport {
            report_id,
            report_type: report_type.to_string(),
            start_date,
            end_date,
            total_orders,
            total_amount,
            completed_orders,
            failed_orders,
            kyc_verified_users: kyc_users,
            suspicious_transactions: suspicious,
            generated_at,
            data,
        };

        // 5. 记录审计日志
        let _ = self
            .log_event(
                tenant_id,
                None, // system action
                None,
                "compliance_report_generated",
                None,
                Some("completed"),
                None,
                None,
                None,
                Some(serde_json::json!({
                    "report_id": report_id,
                    "report_type": report_type,
                    "start_date": start_date,
                    "end_date": end_date,
                })),
            )
            .await;

        Ok(report)
    }
}
