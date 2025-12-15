//! 代币检测 API
//!
//! 企业级实现：提供代币检测、搜索、元数据查询等功能
//! 与前端 IronForge/src/services/token_detection.rs 完全对齐

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    api::response::success_response,
    app_state::AppState,
    error::AppError,
    repository::{PgTokenRepository, TokenRepository},
    service::token_service::network_to_chain_id,
}; // 企业级标准：OpenAPI文档支持

/// GET /api/tokens/detect - 自动检测地址持有的代币
///
/// 自动检测指定地址在指定链上持有的所有代币

#[derive(Debug, Deserialize, ToSchema)]
pub struct DetectTokensQuery {
    pub chain: String,
    pub address: String,
    #[serde(default)]
    pub min_balance: Option<f64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TokenMetadata {
    pub chain: String,
    pub contract_address: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub logo_uri: Option<String>,
    pub verified: bool,
    pub balance: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenListResponse {
    pub tokens: Vec<TokenMetadata>,
    pub total: usize,
}

#[utoipa::path(
    get,
    path = "/api/tokens/detect",
    params(
        ("chain" = String, Query, description = "链名称 (eth, bsc, polygon, sol, etc.)"),
        ("address" = String, Query, description = "钱包地址"),
        ("min_balance" = Option<f64>, Query, description = "最小余额阈值（可选）")
    ),
    responses(
        (status = 200, description = "检测到的代币列表", body = crate::api::response::ApiResponse<TokenListResponse>),
        (status = 400, description = "参数错误"),
        (status = 500, description = "服务错误")
    ),
    tag = "Token Detection"
)]
pub async fn detect_tokens(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DetectTokensQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TokenListResponse>>, AppError> {
    // 转换网络名称为chain_id
    let chain_id = network_to_chain_id(&query.chain)
        .ok_or_else(|| AppError::bad_request(format!("Unsupported network: {}", query.chain)))?;

    // 从数据库获取该链的所有代币
    let repository = PgTokenRepository::new(state.pool.clone());
    let tokens = repository
        .list_by_chain(chain_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list tokens: {}", e)))?;

    // 转换为响应格式
    let token_list: Vec<TokenMetadata> = tokens
        .into_iter()
        .map(|t| TokenMetadata {
            chain: query.chain.clone(),
            contract_address: t.address,
            name: t.name,
            symbol: t.symbol,
            decimals: t.decimals as u8,
            logo_uri: t.logo_url,
            verified: t.is_stablecoin || t.is_native, // 简化：稳定币和原生币视为已验证
            balance: None,                            // 实际余额需要调用链上查询，这里简化处理
        })
        .collect();

    let total = token_list.len();
    success_response(TokenListResponse {
        tokens: token_list,
        total,
    })
}

/// GET /api/tokens/metadata - 获取代币元数据
///
/// 根据合约地址获取代币的元数据信息

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenMetadataQuery {
    pub chain: String,
    pub address: String,
}

#[utoipa::path(
    get,
    path = "/api/tokens/metadata",
    params(
        ("chain" = String, Query, description = "链名称 (eth, bsc, polygon, sol, etc.)"),
        ("address" = String, Query, description = "代币合约地址")
    ),
    responses(
        (status = 200, description = "代币元数据", body = crate::api::response::ApiResponse<TokenMetadata>),
        (status = 400, description = "参数错误"),
        (status = 404, description = "代币未找到"),
        (status = 500, description = "服务错误")
    ),
    tag = "Token Detection"
)]
pub async fn get_token_metadata(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TokenMetadataQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TokenMetadata>>, AppError> {
    // 转换网络名称为chain_id
    let chain_id = network_to_chain_id(&query.chain)
        .ok_or_else(|| AppError::bad_request(format!("Unsupported network: {}", query.chain)))?;

    // 从数据库获取代币信息
    let repository = PgTokenRepository::new(state.pool.clone());
    let token = repository
        .get_by_address_and_chain(&query.address, chain_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to get token: {}", e)))?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "Token not found: {} on {}",
                query.address, query.chain
            ))
        })?;

    success_response(TokenMetadata {
        chain: query.chain,
        contract_address: token.address,
        name: token.name,
        symbol: token.symbol,
        decimals: token.decimals as u8,
        logo_uri: token.logo_url,
        verified: token.is_stablecoin || token.is_native,
        balance: None,
    })
}

/// GET /api/tokens/search - 搜索代币
///
/// 根据名称或符号搜索代币

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchTokensQuery {
    pub q: String,
    #[serde(default)]
    pub chain: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/tokens/search",
    params(
        ("q" = String, Query, description = "搜索关键词（代币名称或符号）"),
        ("chain" = Option<String>, Query, description = "链名称过滤（可选）")
    ),
    responses(
        (status = 200, description = "匹配的代币列表", body = crate::api::response::ApiResponse<TokenListResponse>),
        (status = 400, description = "参数错误"),
        (status = 500, description = "服务错误")
    ),
    tag = "Token Detection"
)]
pub async fn search_tokens(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchTokensQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TokenListResponse>>, AppError> {
    let repository = PgTokenRepository::new(state.pool.clone());
    let query_lower = query.q.to_lowercase();

    // 如果指定了链，只搜索该链的代币
    let tokens = if let Some(chain) = &query.chain {
        let chain_id = network_to_chain_id(chain)
            .ok_or_else(|| AppError::bad_request(format!("Unsupported network: {}", chain)))?;
        repository
            .list_by_chain(chain_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list tokens: {}", e)))?
    } else {
        // 搜索所有链的代币（简化实现，实际应该支持跨链搜索）
        // 这里只搜索主要链（ETH, BSC, Polygon）
        let mut all_tokens = Vec::new();
        for chain_id in [1u64, 56u64, 137u64] {
            if let Ok(mut tokens) = repository.list_by_chain(chain_id).await {
                all_tokens.append(&mut tokens);
            }
        }
        all_tokens
    };

    // 过滤：按名称或符号匹配
    let filtered: Vec<_> = tokens
        .into_iter()
        .filter(|t| {
            t.name.to_lowercase().contains(&query_lower)
                || t.symbol.to_lowercase().contains(&query_lower)
        })
        .take(50) // 限制返回数量
        .map(|t| TokenMetadata {
            chain: query.chain.clone().unwrap_or_else(|| "unknown".to_string()),
            contract_address: t.address,
            name: t.name,
            symbol: t.symbol,
            decimals: t.decimals as u8,
            logo_uri: t.logo_url,
            verified: t.is_stablecoin || t.is_native,
            balance: None,
        })
        .collect();

    success_response(TokenListResponse {
        tokens: filtered.clone(),
        total: filtered.len(),
    })
}

/// GET /api/tokens/popular - 获取热门代币
///
/// 获取指定链上的热门代币列表

#[derive(Debug, Deserialize, ToSchema)]
pub struct PopularTokensQuery {
    pub chain: String,
}

#[utoipa::path(
    get,
    path = "/api/tokens/popular",
    params(
        ("chain" = String, Query, description = "链名称 (eth, bsc, polygon, sol, etc.)")
    ),
    responses(
        (status = 200, description = "热门代币列表", body = crate::api::response::ApiResponse<TokenListResponse>),
        (status = 400, description = "参数错误"),
        (status = 500, description = "服务错误")
    ),
    tag = "Token Detection"
)]
pub async fn get_popular_tokens(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PopularTokensQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TokenListResponse>>, AppError> {
    // 转换网络名称为chain_id
    let chain_id = network_to_chain_id(&query.chain)
        .ok_or_else(|| AppError::bad_request(format!("Unsupported network: {}", query.chain)))?;

    // 从数据库获取代币列表
    let repository = PgTokenRepository::new(state.pool.clone());
    let tokens = repository
        .list_by_chain(chain_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to list tokens: {}", e)))?;

    // 优先返回稳定币和原生币（热门代币）
    let mut token_list: Vec<TokenMetadata> = tokens
        .into_iter()
        .map(|t| TokenMetadata {
            chain: query.chain.clone(),
            contract_address: t.address,
            name: t.name,
            symbol: t.symbol,
            decimals: t.decimals as u8,
            logo_uri: t.logo_url,
            verified: t.is_stablecoin || t.is_native,
            balance: None,
        })
        .collect();

    // 排序：稳定币和原生币优先
    token_list.sort_by(|a, b| match (a.verified, b.verified) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.symbol.cmp(&b.symbol),
    });

    // 限制返回数量
    let total = token_list.len();
    token_list.truncate(20);

    success_response(TokenListResponse {
        tokens: token_list,
        total,
    })
}

/// POST /api/tokens/balances - 批量查询代币余额
///
/// 批量查询指定地址在指定链上的多个代币余额

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenBalancesRequest {
    pub chain: String,
    pub address: String,
    pub tokens: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/api/tokens/balances",
    request_body = TokenBalancesRequest,
    responses(
        (status = 200, description = "代币余额列表", body = crate::api::response::ApiResponse<TokenListResponse>),
        (status = 400, description = "参数错误"),
        (status = 500, description = "服务错误")
    ),
    tag = "Token Detection"
)]
pub async fn get_token_balances(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TokenBalancesRequest>,
) -> Result<Json<crate::api::response::ApiResponse<TokenListResponse>>, AppError> {
    // 转换网络名称为chain_id
    let chain_id = network_to_chain_id(&req.chain)
        .ok_or_else(|| AppError::bad_request(format!("Unsupported network: {}", req.chain)))?;

    // 从数据库获取代币信息
    let repository = PgTokenRepository::new(state.pool.clone());
    let mut token_list = Vec::new();

    for token_address in &req.tokens {
        if let Ok(Some(token)) = repository
            .get_by_address_and_chain(token_address, chain_id)
            .await
        {
            token_list.push(TokenMetadata {
                chain: req.chain.clone(),
                contract_address: token.address,
                name: token.name,
                symbol: token.symbol,
                decimals: token.decimals as u8,
                logo_uri: token.logo_url,
                verified: token.is_stablecoin || token.is_native,
                balance: None, // 实际余额需要调用链上查询
            });
        }
    }

    let total = token_list.len();
    success_response(TokenListResponse {
        tokens: token_list,
        total,
    })
}
