//! Gas估算API（企业级实现）
//! 支持多速度估算、网络拥堵检测

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    api::response::{success_response, ApiResponse},
    app_state::AppState,
    error::AppError,
    service::gas_estimation_service_enhanced::GasEstimationServiceEnhanced,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct GasEstimationQuery {
    pub chain: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub data: Option<String>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/estimate", get(estimate_gas))
        .route("/price", get(get_gas_price))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/gas/estimate
/// 获取Gas估算（三档速度）
#[utoipa::path(
    get,
    path = "/api/gas/estimate",
    params(GasEstimationQuery),
    responses(
        (status = 200, description = "Gas estimation", body = ApiResponse<serde_json::Value>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn estimate_gas(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<GasEstimationQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 验证地址格式
    if !query.from.starts_with("0x") || query.from.len() != 42 {
        return Err(AppError::bad_request("Invalid from address".to_string()));
    }

    if !query.to.starts_with("0x") || query.to.len() != 42 {
        return Err(AppError::bad_request("Invalid to address".to_string()));
    }

    // 使用增强版Gas估算服务
    let service = GasEstimationServiceEnhanced::new();
    let estimation = service
        .estimate_gas_multi_speed(
            &query.chain,
            &query.from,
            &query.to,
            &query.value,
            query.data.as_deref(),
        )
        .await
        .map_err(|e| AppError::internal_error(format!("Gas estimation failed: {}", e)))?;

    success_response(serde_json::to_value(estimation)?)
}

/// GET /api/gas/price
/// 获取当前Gas价格
#[utoipa::path(
    get,
    path = "/api/gas/price",
    params(
        ("chain" = String, Query, description = "Chain symbol (ETH, BSC, POLYGON)")
    ),
    responses(
        (status = 200, description = "Current gas price", body = ApiResponse<serde_json::Value>)
    )
)]
pub async fn get_gas_price(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let chain = params
        .get("chain")
        .ok_or_else(|| AppError::bad_request("Missing chain parameter".to_string()))?;

    let service = GasEstimationServiceEnhanced::new();
    let gas_price = service
        .get_current_gas_price(chain)
        .await
        .map_err(|e| AppError::internal_error(format!("Failed to get gas price: {}", e)))?;

    let gwei = gas_price / 1_000_000_000;

    success_response(serde_json::json!({
        "chain": chain,
        "gas_price_wei": gas_price,
        "gas_price_gwei": gwei,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }))
}
