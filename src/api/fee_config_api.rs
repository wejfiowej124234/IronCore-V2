//! 费率配置管理API
//! 企业级实现：管理员配置所有费率，前端统一调用

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    api::{
        middleware::auth::AuthInfoExtractor,
        response::{success_response, ApiResponse},
    },
    app_state::AppState,
    error::AppError,
    service::unified_fee_config_service::{FeeConfig, FeeType, UnifiedFeeConfigService},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct CalculateFeeRequest {
    /// 费率类型
    pub fee_type: String, // "SwapServiceFee", "BridgeFee", etc.
    /// 金额（USD）
    pub amount_usd: f64,
    /// 链标识（可选）
    pub chain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CalculateFeeResponse {
    pub fee_type: String,
    pub amount_usd: f64,
    pub fee_usd: f64,
    pub rate_percentage: f64,
    pub net_amount_usd: f64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFeeConfigRequest {
    pub fee_type: String,
    pub chain: Option<String>,
    pub rate_percentage: f64,
    pub min_fee_usd: Option<f64>,
    pub max_fee_usd: Option<f64>,
    pub fixed_fee_usd: Option<f64>,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListFeeConfigsResponse {
    pub total: usize,
    pub configs: Vec<FeeConfigDto>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeeConfigDto {
    pub fee_type: String,
    pub chain: Option<String>,
    pub rate_percentage: f64,
    pub min_fee_usd: Option<f64>,
    pub max_fee_usd: Option<f64>,
    pub fixed_fee_usd: Option<f64>,
    pub enabled: bool,
    pub description: String,
    pub updated_at: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/calculate", post(calculate_fee))
        .route("/list", get(list_fee_configs))
        .route("/update", post(update_fee_config))
        .route("/initialize", post(initialize_defaults))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// POST /api/fees/calculate
///
/// 计算费用（前端调用）
#[utoipa::path(
    post,
    path = "/api/fees/calculate",
    request_body = CalculateFeeRequest,
    responses(
        (status = 200, description = "Fee calculated", body = CalculateFeeResponse)
    )
)]
pub async fn calculate_fee(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CalculateFeeRequest>,
) -> Result<Json<ApiResponse<CalculateFeeResponse>>, AppError> {
    let service = UnifiedFeeConfigService::new(state.pool.clone());

    let fee_type = parse_fee_type(&req.fee_type)?;

    let result = service
        .calculate_fee(fee_type, req.amount_usd, req.chain.as_deref())
        .await
        .map_err(|e| AppError::internal_error(format!("Failed to calculate fee: {}", e)))?;

    success_response(CalculateFeeResponse {
        fee_type: format!("{:?}", result.fee_type),
        amount_usd: result.amount_usd,
        fee_usd: result.fee_usd,
        rate_percentage: result.rate_percentage,
        net_amount_usd: result.net_amount_usd,
    })
}

/// GET /api/fees/list
///
/// 列出所有费率配置
#[utoipa::path(
    get,
    path = "/api/fees/list",
    responses(
        (status = 200, description = "Fee configs listed", body = ListFeeConfigsResponse)
    )
)]
pub async fn list_fee_configs(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<ListFeeConfigsResponse>>, AppError> {
    let service = UnifiedFeeConfigService::new(state.pool.clone());

    let configs = service
        .list_all_configs()
        .await
        .map_err(|e| AppError::internal_error(format!("Failed to list configs: {}", e)))?;

    let dtos: Vec<FeeConfigDto> = configs
        .into_iter()
        .map(|c| FeeConfigDto {
            fee_type: format!("{:?}", c.fee_type),
            chain: c.chain,
            rate_percentage: c.rate_percentage,
            min_fee_usd: c.min_fee_usd,
            max_fee_usd: c.max_fee_usd,
            fixed_fee_usd: c.fixed_fee_usd,
            enabled: c.enabled,
            description: c.description,
            updated_at: c.updated_at.to_rfc3339(),
        })
        .collect();

    success_response(ListFeeConfigsResponse {
        total: dtos.len(),
        configs: dtos,
    })
}

/// POST /api/fees/update
///
/// 更新费率配置（管理员）
#[utoipa::path(
    post,
    path = "/api/fees/update",
    request_body = UpdateFeeConfigRequest,
    responses(
        (status = 200, description = "Fee config updated")
    )
)]
pub async fn update_fee_config(
    State(state): State<Arc<AppState>>,
    _auth: AuthInfoExtractor, // TODO: 验证管理员权限
    Json(req): Json<UpdateFeeConfigRequest>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = UnifiedFeeConfigService::new(state.pool.clone());

    let fee_type = parse_fee_type(&req.fee_type)?;

    let config = FeeConfig {
        fee_type,
        chain: req.chain,
        rate_percentage: req.rate_percentage,
        min_fee_usd: req.min_fee_usd,
        max_fee_usd: req.max_fee_usd,
        fixed_fee_usd: req.fixed_fee_usd,
        enabled: req.enabled,
        description: req.description,
        updated_at: chrono::Utc::now(),
    };

    service
        .update_fee_config(&config)
        .await
        .map_err(|e| AppError::internal_error(format!("Failed to update config: {}", e)))?;

    success_response(())
}

/// POST /api/fees/initialize
///
/// 初始化默认费率（首次部署）
#[utoipa::path(
    post,
    path = "/api/fees/initialize",
    responses(
        (status = 200, description = "Defaults initialized")
    )
)]
pub async fn initialize_defaults(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let service = UnifiedFeeConfigService::new(state.pool.clone());

    service
        .initialize_defaults()
        .await
        .map_err(|e| AppError::internal_error(format!("Failed to initialize: {}", e)))?;

    success_response(())
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Helper Functions
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn parse_fee_type(s: &str) -> Result<FeeType, AppError> {
    match s {
        "SwapServiceFee" => Ok(FeeType::SwapServiceFee),
        "GasFee" => Ok(FeeType::GasFee),
        "BridgeFee" => Ok(FeeType::BridgeFee),
        "WithdrawalFee" => Ok(FeeType::WithdrawalFee),
        "FiatDepositFee" => Ok(FeeType::FiatDepositFee),
        "FiatWithdrawalFee" => Ok(FeeType::FiatWithdrawalFee),
        _ => Err(AppError::bad_request(format!("Invalid fee type: {}", s))),
    }
}
