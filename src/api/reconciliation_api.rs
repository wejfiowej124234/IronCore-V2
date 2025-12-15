//! 对账和监控API
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::response::{convert_error, success_response},
    app_state::AppState,
    error::AppError,
    service::reconciliation_service::ReconciliationService,
};

/// POST /api/reconciliation/daily - 执行每日对账
#[derive(Debug, Deserialize)]
pub struct DailyReconciliationRequest {
    pub date: Option<String>, // YYYY-MM-DD format
    pub provider: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReconciliationResponse {
    pub reconciliation_id: String,
    pub date: String,
    pub provider: String,
    pub total_orders: i64,
    pub matched_orders: i64,
    pub unmatched_orders: i64,
    pub status: String,
    pub completed_at: String,
}

pub async fn run_daily_reconciliation(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DailyReconciliationRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<ReconciliationResponse>>, AppError> {
    let date = if let Some(date_str) = req.date {
        chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
            convert_error(StatusCode::BAD_REQUEST, "Invalid date format".to_string())
        })?
    } else {
        chrono::Utc::now().date_naive()
    };

    let reconciliation_service = ReconciliationService::new(state.pool.clone());

    let record = reconciliation_service
        .run_daily_reconciliation(Some(date), req.provider.as_deref())
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(ReconciliationResponse {
        reconciliation_id: record.id.to_string(),
        date: record.reconciliation_date.to_string(),
        provider: record.provider,
        total_orders: record.total_orders,
        matched_orders: record.matched_orders,
        unmatched_orders: record.unmatched_orders,
        status: record.status,
        completed_at: record
            .completed_at
            .map(|d| d.to_rfc3339())
            .unwrap_or_default(),
    })
}

/// POST /api/reconciliation/sync-orders - 同步订单状态
#[derive(Debug, Deserialize)]
pub struct SyncOrdersRequest {
    pub order_ids: Option<Vec<String>>,
    pub provider: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct SyncOrdersResponse {
    pub updated_orders: Vec<OrderStatusUpdate>,
}

#[derive(Debug, Serialize)]
pub struct OrderStatusUpdate {
    pub order_id: String,
    pub new_status: String,
}

pub async fn sync_order_status(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SyncOrdersRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SyncOrdersResponse>>, AppError> {
    let reconciliation_service = ReconciliationService::new(state.pool.clone());

    let order_id = if let Some(ref ids) = req.order_ids {
        ids.first().and_then(|id| Uuid::parse_str(id).ok())
    } else {
        None
    };

    let updated = reconciliation_service
        .sync_order_status(order_id, req.provider.as_deref(), req.limit)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let updates: Vec<OrderStatusUpdate> = updated
        .iter()
        .map(|(id, status)| OrderStatusUpdate {
            order_id: id.to_string(),
            new_status: status.clone(),
        })
        .collect();

    success_response(SyncOrdersResponse {
        updated_orders: updates,
    })
}

/// GET /api/reconciliation/alerts - 获取告警列表
#[derive(Debug, Deserialize)]
pub struct AlertsQuery {
    pub tenant_id: Option<String>,
    pub status: Option<String>,
    pub severity: Option<String>,
    pub limit: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct AlertsResponse {
    pub alerts: Vec<AlertResponse>,
}

#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub id: String,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub order_id: Option<String>,
    pub provider: Option<String>,
    pub status: String,
    pub created_at: String,
}

pub async fn get_alerts(
    State(state): State<Arc<AppState>>,
    Query(query): Query<AlertsQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<AlertsResponse>>, AppError> {
    let reconciliation_service = ReconciliationService::new(state.pool.clone());

    let tenant_id = query.tenant_id.and_then(|id| Uuid::parse_str(&id).ok());

    let alerts = reconciliation_service
        .get_alerts(
            tenant_id,
            query.status.as_deref(),
            query.severity.as_deref(),
            query.limit,
        )
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let alert_responses: Vec<AlertResponse> = alerts
        .into_iter()
        .map(|a| AlertResponse {
            id: a.id.to_string(),
            alert_type: a.alert_type,
            severity: a.severity,
            message: a.message,
            order_id: a.order_id.map(|id| id.to_string()),
            provider: a.provider,
            status: a.status,
            created_at: a.created_at.to_rfc3339(),
        })
        .collect();

    success_response(AlertsResponse {
        alerts: alert_responses,
    })
}
