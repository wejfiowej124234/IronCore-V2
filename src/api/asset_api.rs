use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::response::{convert_error, success_response},
    app_state::AppState,
    error::AppError,
    infrastructure::jwt::Claims,
    service::{
        asset_service::{AssetResponse, AssetService, WalletAsset},
        cross_chain_bridge_service::{
            CrossChainBridgeService, CrossChainSwapRequest, CrossChainSwapResponse, SwapQuote,
        },
        price_service::PriceService,
    },
};

/// 获取价格查询参数
#[derive(Debug, Deserialize)]
pub struct PriceQuery {
    pub symbols: Option<String>, // 逗号分隔：eth,sol,btc
}

/// 价格响应
#[derive(Debug, Serialize)]
pub struct PricesResponse {
    pub prices: Vec<PriceData>,
    pub last_updated: String,
}

#[derive(Debug, Serialize)]
pub struct PriceData {
    pub symbol: String,
    pub price_usdt: f64,
    pub source: String,
}

/// 跨链兑换报价请求
#[derive(Debug, Deserialize)]
pub struct SwapQuoteRequest {
    pub source_chain: String,
    pub source_token: String,
    pub source_amount: f64,
    pub target_chain: String,
    pub target_token: String,
}

/// ========== API 处理器 ==========
/// GET /api/prices?symbols=eth,sol,btc
/// 获取加密货币价格
#[axum::debug_handler]
pub async fn get_prices(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PriceQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<PricesResponse>>, AppError> {
    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = PriceService::new(state.pool.clone(), redis_url);

    let symbols_str = query
        .symbols
        .unwrap_or_else(|| "ETH,SOL,BTC,BNB,MATIC,AVAX".to_string());
    let symbols: Vec<&str> = symbols_str.split(',').map(|s| s.trim()).collect();

    let price_map = price_service
        .get_prices(&symbols)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let prices: Vec<PriceData> = price_map
        .into_iter()
        .map(|(symbol, price)| PriceData {
            symbol,
            price_usdt: price,
            source: "coingecko".to_string(),
        })
        .collect();

    success_response(PricesResponse {
        prices,
        last_updated: chrono::Utc::now().to_rfc3339(),
    })
}

/// GET /api/wallets/assets
/// 获取用户总资产（所有钱包，USDT 统一展示）
#[axum::debug_handler]
pub async fn get_user_total_assets(
    State(state): State<Arc<AppState>>,
    axum::Extension(claims): axum::Extension<Claims>,
) -> Result<axum::Json<crate::api::response::ApiResponse<AssetResponse>>, AppError> {
    let user_id = claims
        .user_id()
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    let asset_service = AssetService::new(
        state.pool.clone(),
        price_service,
        state.blockchain_config.clone(),
    );

    let assets = asset_service
        .get_user_total_assets(user_id)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(assets)
}

/// GET /api/wallets/:id/assets
/// 获取单个钱包资产
#[axum::debug_handler]
pub async fn get_wallet_asset(
    State(state): State<Arc<AppState>>,
    Path(wallet_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<WalletAsset>>, AppError> {
    // ✅ID格式验证
    let wallet_uuid = Uuid::parse_str(&wallet_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid UUID format".to_string()))?;

    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    let asset_service = AssetService::new(
        state.pool.clone(),
        price_service,
        state.blockchain_config.clone(),
    );

    let asset = asset_service
        .get_wallet_asset(wallet_uuid)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                convert_error(StatusCode::NOT_FOUND, e.to_string())
            } else {
                convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    success_response(asset)
}

/// POST /api/swap/quote
/// 获取跨链兑换报价
#[axum::debug_handler]
/// 企业级标准：跨链兑换报价（推荐使用）
pub async fn get_swap_quote(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SwapQuoteRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SwapQuote>>, AppError> {
    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    // 企业级实现：跨链桥需要计算平台服务费和获取钱包地址
    use crate::repository::wallet_repository::PgWalletRepository;
    let wallet_repo = Arc::new(PgWalletRepository::new(state.pool.clone()));
    let bridge_service = CrossChainBridgeService::new(
        state.pool.clone(),
        price_service,
        state.cross_chain_config.clone(),
        state.fee_service.clone(), // 传入fee_service用于计算平台服务费
        wallet_repo,               // 传入wallet_repo用于获取钱包地址
    );

    let quote = bridge_service
        .get_swap_quote(
            &request.source_chain,
            &request.source_token,
            request.source_amount,
            &request.target_chain,
            &request.target_token,
        )
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(quote)
}

/// 废弃版本：POST /api/swap/quote（用于跨链兑换）
/// 企业级标准：此端点已移除，请使用 POST /api/swap/cross-chain-quote
#[allow(dead_code)]
#[deprecated(
    note = "This endpoint has been removed. Use POST /api/swap/cross-chain-quote instead."
)]
pub async fn get_swap_quote_deprecated(
    _state: State<Arc<AppState>>,
    _request: Json<SwapQuoteRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SwapQuote>>, AppError> {
    // 返回410 Gone错误
    Err(AppError {
        code: crate::error::AppErrorCode::BadRequest,
        message:
            "This endpoint has been removed. Please use POST /api/swap/cross-chain-quote instead."
                .to_string(),
        status: axum::http::StatusCode::GONE,
        trace_id: None,
    })
}

/// POST /api/swap/cross-chain
/// 执行跨链兑换
#[axum::debug_handler]
pub async fn execute_cross_chain_swap(
    State(state): State<Arc<AppState>>,
    axum::Extension(claims): axum::Extension<Claims>,
    Json(mut request): Json<CrossChainSwapRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<CrossChainSwapResponse>>, AppError> {
    let user_id = claims
        .user_id()
        .map_err(|e| AppError::unauthorized(format!("Invalid token: {}", e)))?;
    request.user_id = user_id;

    if request.source_amount <= 0.0 || !request.source_amount.is_finite() {
        return Err(AppError::bad_request("Invalid amount".to_string()));
    }
    if request
        .source_chain
        .eq_ignore_ascii_case(&request.target_chain)
    {
        return Err(AppError::bad_request(
            "Cannot swap to same chain".to_string(),
        ));
    }

    // 自动查找用户的源链钱包（如果未指定）
    if request.source_wallet_id == Uuid::nil() {
        use crate::repository::wallets;
        let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|e| {
            convert_error(StatusCode::BAD_REQUEST, format!("Invalid tenant_id: {}", e))
        })?;
        let user_wallets = wallets::list_by_user(&state.pool, tenant_id, user_id, 100, 0)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch wallets: {}", e)))?;

        // 找到第一个匹配源链的钱包
        if let Some(wallet) = user_wallets.first() {
            request.source_wallet_id = wallet.id;
        } else {
            return Err(AppError::bad_request(
                "No wallet found for source chain".to_string(),
            ));
        }
    }

    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    // 企业级实现：跨链桥需要计算平台服务费和获取钱包地址
    use crate::repository::wallet_repository::PgWalletRepository;
    let wallet_repo = Arc::new(PgWalletRepository::new(state.pool.clone()));
    let bridge_service = CrossChainBridgeService::new(
        state.pool.clone(),
        price_service,
        state.cross_chain_config.clone(),
        state.fee_service.clone(), // 传入fee_service用于计算平台服务费
        wallet_repo,               // 传入wallet_repo用于获取钱包地址
    );

    let response = bridge_service
        .execute_swap(request)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // 使用统一响应格式
    success_response(response)
}

/// GET /api/swap/:id
/// 查询跨链兑换状态
#[axum::debug_handler]
pub async fn get_swap_status(
    State(state): State<Arc<AppState>>,
    Path(swap_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<CrossChainSwapResponse>>, AppError> {
    let swap_uuid = Uuid::parse_str(&swap_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid swap ID".to_string()))?;

    let redis_url = std::env::var("REDIS_URL").ok();
    let price_service = Arc::new(PriceService::new(state.pool.clone(), redis_url));
    // 企业级实现：跨链桥需要计算平台服务费和获取钱包地址
    use crate::repository::wallet_repository::PgWalletRepository;
    let wallet_repo = Arc::new(PgWalletRepository::new(state.pool.clone()));
    let bridge_service = CrossChainBridgeService::new(
        state.pool.clone(),
        price_service,
        state.cross_chain_config.clone(),
        state.fee_service.clone(), // 传入fee_service用于计算平台服务费
        wallet_repo,               // 传入wallet_repo用于获取钱包地址
    );

    let status = bridge_service
        .get_swap_status(swap_uuid)
        .await
        .map_err(|e| {
            if e.to_string().contains("not found") {
                convert_error(StatusCode::NOT_FOUND, e.to_string())
            } else {
                convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            }
        })?;

    success_response(status)
}

// Routes
pub fn routes() -> axum::Router<Arc<crate::app_state::AppState>> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/prices", get(get_prices))
        .route("/total", get(get_user_total_assets))
        .route("/wallet/:wallet_id", get(get_wallet_asset))
        .route("/swap/quote", get(get_swap_quote))
        .route("/swap", post(execute_cross_chain_swap))
        .route("/swap/:id/status", get(get_swap_status))
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // 单元测试需要 mock AppState
}
