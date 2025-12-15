//! Nonce管理API（F项客户端接口）
//! 企业级实现：为客户端提供准确的nonce值

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    routing::get,
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
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize)]
pub struct GetNonceQuery {
    pub chain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GetNonceResponse {
    pub address: String,
    pub chain: String,
    pub nonce: u64,
    pub pending_nonce: u64, // 包含pending交易的nonce
    pub next_nonce: u64,    // 建议使用的nonce
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// API Handler
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/wallets/:address/nonce
///
/// 获取地址的下一个nonce值
///
/// # 企业级实现
/// - 查询链上confirmed nonce
/// - 查询本地pending nonce
/// - 返回max(chain_nonce, pending_nonce) + 1
#[utoipa::path(
    get,
    path = "/api/wallets/{address}/nonce",
    params(
        ("address" = String, Path, description = "Wallet address"),
        ("chain" = String, Query, description = "Chain identifier (eth, bsc, polygon)")
    ),
    responses(
        (status = 200, description = "Nonce retrieved", body = ApiResponse<GetNonceResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_nonce(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(_auth): AuthInfoExtractor,
    Path(address): Path<String>,
    Query(query): Query<GetNonceQuery>,
) -> Result<Json<ApiResponse<GetNonceResponse>>, AppError> {
    // 验证地址格式
    crate::utils::address_validator::AddressValidator::validate(&query.chain, &address)
        .map_err(|e| AppError::bad_request(format!("Invalid address: {}", e)))?;

    // 1. 查询链上nonce（使用nonce_manager服务）
    let nonce_manager = crate::service::nonce_manager::NonceManager::new(
        state.pool.clone(),
        state.distributed_lock.clone(),
    );
    let chain_nonce = nonce_manager
        .get_next_nonce(&query.chain, &address, &state.blockchain_client)
        .await
        .map_err(|e| AppError::internal_error(format!("Failed to get nonce: {}", e)))?;

    // 2. 查询本地pending nonce
    let pending_nonce = get_pending_nonce(&address, &query.chain, &state.pool).await?;

    // 3. 计算下一个nonce
    let next_nonce = std::cmp::max(chain_nonce, pending_nonce) + 1;

    tracing::debug!(
        "Nonce for {} on {}: chain={}, pending={}, next={}",
        address,
        query.chain,
        chain_nonce,
        pending_nonce,
        next_nonce
    );

    success_response(GetNonceResponse {
        address,
        chain: query.chain,
        nonce: chain_nonce,
        pending_nonce,
        next_nonce,
    })
}

/// 获取pending nonce（本地数据库）
async fn get_pending_nonce(
    address: &str,
    chain: &str,
    pool: &sqlx::PgPool,
) -> Result<u64, AppError> {
    let result = sqlx::query_as::<_, (Option<i64>,)>(
        "SELECT MAX(nonce) as max_nonce
         FROM transactions
         WHERE from_address = $1
           AND chain = $2
           AND status IN ('pending', 'submitted')",
    )
    .bind(address)
    .bind(chain.to_uppercase())
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    Ok(result.and_then(|r| r.0).unwrap_or(0) as u64)
}

/// 路由配置
pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/:address/nonce", get(get_nonce))
}
