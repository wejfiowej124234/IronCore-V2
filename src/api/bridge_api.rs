//! 跨链桥 API
//!
//! 企业级实现：提供跨链桥接功能的标准API端点
//! 与前端 IronForge/src/services/bridge.rs 完全对齐

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema; // 企业级标准：OpenAPI文档支持
use uuid::Uuid;

use crate::{
    api::{middleware::jwt_extractor::JwtAuthContext, response::success_response},
    app_state::AppState,
    error::AppError,
    repository::wallet_repository::PgWalletRepository,
    service::{cross_chain_bridge_service::CrossChainBridgeService, price_service::PriceService},
};

/// GET /api/bridge/quote - 获取跨链桥报价
///
/// 获取跨链桥接的报价信息，包括目标金额、手续费、预估时间等

#[derive(Debug, Deserialize, ToSchema)]
pub struct BridgeQuoteQuery {
    pub from_chain: String,
    pub to_chain: String,
    pub token: String,
    pub amount: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeQuoteResponse {
    pub source_chain: String,
    pub target_chain: String,
    /// ✅ 企业级实现：金额使用String传输，避免精度损失
    pub source_amount: String,
    pub target_amount: String,
    pub exchange_rate: String,
    pub fee_usdt: String,
    pub total_fee_percentage: String,
    pub estimated_time_minutes: u32,
    pub recommended_protocol: String,
}

#[utoipa::path(
    get,
    path = "/api/bridge/quote",
    params(
        ("from_chain" = String, Query, description = "源链名称 (eth, bsc, polygon, sol, etc.)"),
        ("to_chain" = String, Query, description = "目标链名称 (eth, bsc, polygon, sol, etc.)"),
        ("token" = String, Query, description = "代币符号 (ETH, BNB, SOL, etc.)"),
        ("amount" = String, Query, description = "源链代币数量")
    ),
    responses(
        (status = 200, description = "跨链桥报价", body = crate::api::response::ApiResponse<BridgeQuoteResponse>),
        (status = 400, description = "参数错误"),
        (status = 500, description = "服务错误")
    ),
    tag = "Cross-Chain Bridge"
)]
pub async fn get_bridge_quote(
    State(state): State<Arc<AppState>>,
    _auth: JwtAuthContext,
    Query(query): Query<BridgeQuoteQuery>,
) -> Result<Json<crate::api::response::ApiResponse<BridgeQuoteResponse>>, AppError> {
    // 解析金额
    let source_amount = query
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount format".to_string()))?;

    if source_amount <= 0.0 {
        return Err(AppError::bad_request(
            "Amount must be greater than 0".to_string(),
        ));
    }

    // 创建服务实例
    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    let wallet_repo = Arc::new(PgWalletRepository::new(state.pool.clone()));
    let bridge_service = CrossChainBridgeService::new(
        state.pool.clone(),
        price_service,
        state.cross_chain_config.clone(),
        state.fee_service.clone(),
        wallet_repo,
    );

    // 获取报价
    let quote = bridge_service
        .get_swap_quote(
            &query.from_chain,
            &query.token,
            source_amount,
            &query.to_chain,
            &query.token, // 假设目标链使用相同代币符号
        )
        .await
        .map_err(|e| AppError::internal(format!("Failed to get bridge quote: {}", e)))?;

    success_response(BridgeQuoteResponse {
        source_chain: quote.source_chain,
        target_chain: quote.target_chain,
        source_amount: quote.source_amount.to_string(),
        target_amount: quote.target_amount.to_string(),
        exchange_rate: quote.exchange_rate.to_string(),
        fee_usdt: quote.fee_usdt.to_string(),
        total_fee_percentage: quote.total_fee_percentage.to_string(),
        estimated_time_minutes: quote.estimated_time_minutes,
        recommended_protocol: quote.recommended_protocol,
    })
}

/// POST /api/bridge/assets - 执行跨链桥接
///
/// 执行跨链资产桥接操作

#[derive(Debug, Deserialize, ToSchema)]
pub struct BridgeAssetsRequest {
    pub from_wallet: String,
    pub from_chain: String,
    pub to_chain: String,
    pub token: String,
    pub amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_request_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeAssetsResponse {
    #[serde(alias = "bridge_id", alias = "swap_id")]
    pub swap_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge_tx_id: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_chain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_chain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_target_amount: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_usdt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_time_minutes: Option<u32>,
}

pub async fn bridge_assets(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Json(req): Json<BridgeAssetsRequest>,
) -> Result<Json<crate::api::response::ApiResponse<BridgeAssetsResponse>>, AppError> {
    // 解析金额
    let source_amount = req
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount format".to_string()))?;

    if source_amount <= 0.0 {
        return Err(AppError::bad_request(
            "Amount must be greater than 0".to_string(),
        ));
    }

    // 获取用户ID
    let user_id = auth.user_id;

    // 创建服务实例
    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    let wallet_repo = Arc::new(PgWalletRepository::new(state.pool.clone()));
    let bridge_service = CrossChainBridgeService::new(
        state.pool.clone(),
        price_service,
        state.cross_chain_config.clone(),
        state.fee_service.clone(),
        wallet_repo,
    );

    // 构建跨链兑换请求
    use crate::service::cross_chain_bridge_service::CrossChainSwapRequest;
    let swap_request = CrossChainSwapRequest {
        user_id,
        source_chain: req.from_chain.clone(),
        source_token: req.token.clone(),
        source_amount,
        source_wallet_id: Uuid::parse_str(&req.from_wallet)
            .map_err(|_| AppError::bad_request("Invalid wallet ID format".to_string()))?,
        target_chain: req.to_chain.clone(),
        target_token: req.token.clone(), // 假设目标链使用相同代币符号
        target_wallet_id: None,
    };

    // 执行跨链兑换
    let response = bridge_service
        .execute_swap(swap_request)
        .await
        .map_err(|e| AppError::internal(format!("Failed to bridge assets: {}", e)))?;

    success_response(BridgeAssetsResponse {
        swap_id: response.swap_id,
        bridge_tx_id: None,
        status: response.status,
        target_chain: Some(req.to_chain),
        amount: Some(response.source_amount.to_string()),
        from_chain: Some(req.from_chain),
        token: Some(req.token),
        estimated_target_amount: Some(response.estimated_target_amount.to_string()),
        exchange_rate: Some(response.exchange_rate.to_string()),
        fee_usdt: Some(response.fee_usdt.to_string()),
        estimated_time_minutes: Some(response.estimated_time_minutes),
    })
}

/// GET /api/bridge/:bridge_id - 查询跨链桥状态✅
pub async fn get_bridge_status(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    axum::extract::Path(bridge_id): axum::extract::Path<String>,
) -> Result<Json<crate::api::response::ApiResponse<serde_json::Value>>, AppError> {
    let id = uuid::Uuid::parse_str(&bridge_id)
        .map_err(|_| AppError::bad_request("Invalid ID".to_string()))?;

    // ✅使用sqlx::query_as替代query!宏(避免编译时检查依赖)
    #[derive(sqlx::FromRow)]
    struct BridgeRecord {
        id: uuid::Uuid,
        status: String,
        from_chain: String,
        to_chain: String,
        token: String,
        amount: rust_decimal::Decimal,
        tx_hash_source: Option<String>,
        tx_hash_target: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: chrono::DateTime<chrono::Utc>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
        error_message: Option<String>,
    }

    let record = sqlx::query_as::<_, BridgeRecord>(
        "SELECT id, status, from_chain, to_chain, token, amount, tx_hash_source, tx_hash_target, created_at, updated_at, completed_at, error_message FROM public.bridge_transactions WHERE id = $1 AND user_id = $2"
    ).bind(id).bind(auth.user_id)
        .fetch_optional(&state.pool).await.map_err(|e| AppError::database_error(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Bridge not found".to_string()))?;
    success_response(serde_json::json!({
        "bridge_id": record.id, "status": record.status, "from_chain": record.from_chain,
        "to_chain": record.to_chain, "token": record.token, "amount": record.amount.to_string(),
        "tx_hash_source": record.tx_hash_source, "tx_hash_target": record.tx_hash_target,
        "created_at": record.created_at.to_rfc3339(), "updated_at": record.updated_at.to_rfc3339(),
        "completed_at": record.completed_at.map(|t| t.to_rfc3339()), "error_message": record.error_message
    }))
}

// Routes
pub fn routes() -> axum::Router<Arc<crate::app_state::AppState>> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/quote", post(get_bridge_quote)) // ✅ 前端使用POST
        .route("/assets", post(bridge_assets)) // ✅ 前端路径
        .route("/transfer", post(bridge_assets)) // 兼容别名
        .route("/:id", get(get_bridge_status)) // ✅ 前端路径
        .route("/:id/status", get(get_bridge_status)) // 兼容别名
}
