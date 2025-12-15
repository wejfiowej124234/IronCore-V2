//! 审计日志API
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::response::{convert_error, success_response},
    app_state::AppState,
    error::AppError,
    service::audit_service::AuditService,
};

/// GET /api/audit/logs - 获取审计日志
#[derive(Debug, Deserialize)]
pub struct AuditLogsQuery {
    pub user_id: Option<String>,
    pub order_id: Option<String>,
    pub action: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogsResponse {
    pub logs: Vec<AuditLogResponse>,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub id: String,
    pub user_id: Option<String>,
    pub order_id: Option<String>,
    pub action: String,
    pub amount: Option<String>,
    pub status: Option<String>,
    pub provider: Option<String>,
    pub created_at: String,
}

pub async fn get_audit_logs(
    State(state): State<Arc<AppState>>,
    auth: crate::api::middleware::jwt_extractor::JwtAuthContext,
    Query(query): Query<AuditLogsQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<AuditLogsResponse>>, AppError> {
    let audit_service = AuditService::new(state.pool.clone(), state.immu.clone());

    let user_id = query.user_id.and_then(|id| Uuid::parse_str(&id).ok());

    let order_id = query.order_id.and_then(|id| Uuid::parse_str(&id).ok());

    let start_date = query
        .start_date
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
        .map(|d| d.with_timezone(&chrono::Utc));

    let end_date = query
        .end_date
        .and_then(|d| chrono::DateTime::parse_from_rfc3339(&d).ok())
        .map(|d| d.with_timezone(&chrono::Utc));

    let logs = audit_service
        .get_audit_logs(
            Some(auth.tenant_id),
            user_id,
            order_id,
            query.action.as_deref(),
            start_date,
            end_date,
            query.limit,
            query.offset,
        )
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let log_responses: Vec<AuditLogResponse> = logs
        .iter()
        .map(|log| AuditLogResponse {
            id: log.id.to_string(),
            user_id: log.user_id.map(|id| id.to_string()),
            order_id: log.order_id.map(|id| id.to_string()),
            action: log.action.clone(),
            amount: log.amount.map(|a| a.to_string()),
            status: log.status.clone(),
            provider: log.provider.clone(),
            created_at: log.created_at.to_rfc3339(),
        })
        .collect();

    success_response(AuditLogsResponse {
        logs: log_responses,
        total: logs.len() as i64,
    })
}

/// GET /api/audit/compliance-report - 生成合规报告
#[derive(Debug, Deserialize)]
pub struct ComplianceReportQuery {
    pub report_type: String, // 'daily', 'weekly', 'monthly'
    pub start_date: String,  // YYYY-MM-DD
    pub end_date: String,    // YYYY-MM-DD
}

#[derive(Debug, Serialize)]
pub struct ComplianceReportResponse {
    pub report_id: String,
    pub report_type: String,
    pub start_date: String,
    pub end_date: String,
    pub total_orders: i64,
    pub total_amount: String,
    pub completed_orders: i64,
    pub failed_orders: i64,
    pub kyc_verified_users: i64,
    pub suspicious_transactions: i64,
    pub generated_at: String,
    pub data: serde_json::Value,
}

pub async fn generate_compliance_report(
    State(state): State<Arc<AppState>>,
    auth: crate::api::middleware::jwt_extractor::JwtAuthContext,
    Query(query): Query<ComplianceReportQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<ComplianceReportResponse>>, AppError> {
    let start_date =
        chrono::NaiveDate::parse_from_str(&query.start_date, "%Y-%m-%d").map_err(|_| {
            convert_error(
                StatusCode::BAD_REQUEST,
                "Invalid start_date format".to_string(),
            )
        })?;

    let end_date =
        chrono::NaiveDate::parse_from_str(&query.end_date, "%Y-%m-%d").map_err(|_| {
            convert_error(
                StatusCode::BAD_REQUEST,
                "Invalid end_date format".to_string(),
            )
        })?;

    let audit_service = AuditService::new(state.pool.clone(), state.immu.clone());

    let report = audit_service
        .generate_compliance_report(auth.tenant_id, &query.report_type, start_date, end_date)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(ComplianceReportResponse {
        report_id: report.report_id.to_string(),
        report_type: report.report_type,
        start_date: report.start_date.to_string(),
        end_date: report.end_date.to_string(),
        total_orders: report.total_orders,
        total_amount: report.total_amount.to_string(),
        completed_orders: report.completed_orders,
        failed_orders: report.failed_orders,
        kyc_verified_users: report.kyc_verified_users,
        suspicious_transactions: report.suspicious_transactions,
        generated_at: report.generated_at.to_rfc3339(),
        data: report.data,
    })
}
