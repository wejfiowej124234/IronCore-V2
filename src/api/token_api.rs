//! 代币信息 API
//! 企业级实现，从数据库读取代币信息

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    api::response::{convert_error, success_response},
    app_state::AppState,
    error::AppError,
    repository::{PgTokenRepository, TokenRepository},
    service::token_service::network_to_chain_id,
};

/// GET /api/tokens/list - 获取链上代币列表
#[derive(Debug, Deserialize)]
pub struct TokenListQuery {
    pub chain: String, // 网络名称，如 "ethereum", "bsc", "polygon"
}

// TokenListResponse 已移除，直接使用 Vec<TokenInfoResponse>

#[derive(Debug, Serialize)]
pub struct TokenInfoResponse {
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub is_native: bool,
    pub is_stablecoin: bool,
    pub logo_url: Option<String>,
    pub chain: String,
}

/// 获取代币列表✅
pub async fn get_token_list(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TokenListQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<Vec<TokenInfoResponse>>>, AppError> {
    info!("Token list request: chain={}", query.chain);

    let chain_id = network_to_chain_id(&query.chain).ok_or_else(|| {
        convert_error(
            StatusCode::BAD_REQUEST,
            format!("Unsupported network: {}", query.chain),
        )
    })?;

    // 从数据库获取代币列表
    let repository = PgTokenRepository::new(state.pool.clone());
    let tokens = repository.list_by_chain(chain_id).await.map_err(|e| {
        error!("获取代币列表失败: {:?}", e);
        convert_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "获取代币列表失败".to_string(),
        )
    })?;

    // 转换为响应格式
    let token_list: Vec<TokenInfoResponse> = tokens
        .into_iter()
        .map(|t| TokenInfoResponse {
            address: t.address,
            symbol: t.symbol,
            name: t.name,
            decimals: t.decimals as u8,
            is_native: t.is_native,
            is_stablecoin: t.is_stablecoin,
            logo_url: t.logo_url,
            chain: query.chain.clone(),
        })
        .collect();

    success_response(token_list)
}

/// GET /api/tokens/:address/info - 根据地址获取代币信息
#[derive(Debug, Deserialize)]
pub struct TokenInfoQuery {
    pub chain: String,
}

/// 根据地址获取代币信息
pub async fn get_token_info_by_address(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(address): axum::extract::Path<String>,
    Query(query): Query<TokenInfoQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<TokenInfoResponse>>, AppError> {
    info!(
        "收到代币信息请求: address={}, chain={}",
        address, query.chain
    );

    // 转换网络名称为chain_id
    let chain_id = network_to_chain_id(&query.chain).ok_or_else(|| {
        convert_error(
            StatusCode::BAD_REQUEST,
            format!("不支持的网络: {}", query.chain),
        )
    })?;

    // 从数据库获取代币信息
    let repository = PgTokenRepository::new(state.pool.clone());
    let token = repository
        .get_by_address_and_chain(&address, chain_id)
        .await
        .map_err(|e| {
            error!("获取代币信息失败: {:?}", e);
            convert_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "获取代币信息失败".to_string(),
            )
        })?
        .ok_or_else(|| {
            convert_error(
                StatusCode::NOT_FOUND,
                format!("未找到代币: {} on {}", address, query.chain),
            )
        })?;

    success_response(TokenInfoResponse {
        address: token.address,
        symbol: token.symbol,
        name: token.name,
        decimals: token.decimals as u8,
        is_native: token.is_native,
        is_stablecoin: token.is_stablecoin,
        logo_url: token.logo_url,
        chain: query.chain,
    })
}

/// GET /api/tokens/:symbol/address - 根据符号获取代币地址
#[derive(Debug, Deserialize)]
pub struct TokenAddressQuery {
    pub chain: String,
}

#[derive(Debug, Serialize)]
pub struct TokenAddressResponse {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
}

/// 根据符号获取代币地址
pub async fn get_token_address_by_symbol(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(symbol): axum::extract::Path<String>,
    Query(query): Query<TokenAddressQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<TokenAddressResponse>>, AppError> {
    info!("收到代币地址请求: symbol={}, chain={}", symbol, query.chain);

    // 转换网络名称为chain_id
    let chain_id = network_to_chain_id(&query.chain).ok_or_else(|| {
        convert_error(
            StatusCode::BAD_REQUEST,
            format!("不支持的网络: {}", query.chain),
        )
    })?;

    // 从数据库获取代币信息
    let repository = PgTokenRepository::new(state.pool.clone());
    let token = repository
        .get_by_symbol_and_chain(&symbol, chain_id)
        .await
        .map_err(|e| {
            error!("获取代币信息失败: {:?}", e);
            convert_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "获取代币信息失败".to_string(),
            )
        })?
        .ok_or_else(|| {
            convert_error(
                StatusCode::NOT_FOUND,
                format!("未找到代币: {} on {}", symbol, query.chain),
            )
        })?;

    success_response(TokenAddressResponse {
        address: token.address,
        symbol: token.symbol,
        decimals: token.decimals as u8,
    })
}

/// GET /api/v1/tokens/:token_address/balance - 获取单个代币余额
#[derive(Debug, Deserialize)]
pub struct TokenBalanceQuery {
    pub address: String, // 钱包地址
    pub chain: String,   // 链名称
}

#[derive(Debug, Serialize)]
pub struct TokenBalanceResponse {
    pub token: TokenInfoResponse,
    pub balance_raw: String,
    pub balance_formatted: f64,
}

/// 获取代币余额 ✅
pub async fn get_token_balance(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(token_address): axum::extract::Path<String>,
    Query(query): Query<TokenBalanceQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<TokenBalanceResponse>>, AppError> {
    info!(
        "收到代币余额请求: token={}, wallet={}, chain={}",
        token_address, query.address, query.chain
    );

    // 统一链标识符（支持 eth/ETH/1/Ethereum 等）
    let chain_cfg = crate::utils::chain_normalizer::get_chain_config(&query.chain).map_err(|e| {
        convert_error(
            StatusCode::BAD_REQUEST,
            format!("Unsupported network: {} ({})", query.chain, e),
        )
    })?;
    let chain_id: u64 = u64::try_from(chain_cfg.chain_id).map_err(|_| {
        convert_error(
            StatusCode::BAD_REQUEST,
            format!("Unsupported chain_id for token lookup: {}", chain_cfg.chain_id),
        )
    })?;
    let canonical_chain = chain_cfg.canonical_name;

    // 代币余额接口目前只支持EVM链（ERC-20 / 原生币）
    if !crate::utils::chain_normalizer::is_evm_chain(canonical_chain) {
        return Err(convert_error(
            StatusCode::BAD_REQUEST,
            format!("Token balance only supports EVM chains, got: {}", query.chain),
        ));
    }

    // 校验钱包地址格式
    let wallet_address = crate::infrastructure::rpc_validator::validate_address(&query.address)
        .map_err(|e| {
            convert_error(
                StatusCode::BAD_REQUEST,
                format!("Invalid wallet address: {} ({})", query.address, e),
            )
        })?;

    // 从数据库获取代币信息
    let repository = PgTokenRepository::new(state.pool.clone());
    let token = repository
        .get_by_address_and_chain(&token_address, chain_id)
        .await
        .map_err(|e| {
            error!("Failed to query token: {}", e);
            convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?
        .ok_or_else(|| {
            convert_error(
                StatusCode::NOT_FOUND,
                format!("未找到代币: {} on {}", token_address, query.chain),
            )
        })?;

    // 调用链上RPC获取真实余额（非托管：后端只做查询，不触碰私钥）
    let raw_balance_u128 = if token.is_native {
        state
            .blockchain_client
            .get_native_balance(canonical_chain, &wallet_address)
            .await
            .map_err(|e| {
                error!(
                    "Native balance RPC failed: chain={}, wallet={}, err={:?}",
                    canonical_chain, wallet_address, e
                );
                convert_error(
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to fetch on-chain native balance: {e}"),
                )
            })?
    } else {
        // 校验合约地址格式（优先使用DB里的地址，避免路径大小写/别名问题）
        let contract_address = crate::infrastructure::rpc_validator::validate_address(&token.address)
            .map_err(|e| {
                convert_error(
                    StatusCode::BAD_REQUEST,
                    format!("Invalid token contract address: {} ({})", token.address, e),
                )
            })?;

        state
            .blockchain_client
            .get_erc20_balance(canonical_chain, &contract_address, &wallet_address)
            .await
            .map_err(|e| {
                error!(
                    "ERC20 balance RPC failed: chain={}, token={}, wallet={}, err={:?}",
                    canonical_chain, contract_address, wallet_address, e
                );
                convert_error(
                    StatusCode::BAD_GATEWAY,
                    format!("Failed to fetch on-chain token balance: {e}"),
                )
            })?
    };

    // 计算格式化余额（balance / 10^decimals）
    let decimals_u32 = u32::from((token.decimals as i32).max(0) as u8);
    let divisor = 10u128.checked_pow(decimals_u32).unwrap_or(1);
    let balance_formatted = (raw_balance_u128 as f64) / (divisor as f64);

    success_response(TokenBalanceResponse {
        token: TokenInfoResponse {
            address: token.address.clone(),
            symbol: token.symbol.clone(),
            name: token.name.clone(),
            decimals: token.decimals as u8,
            is_native: token.is_native,
            is_stablecoin: token.is_stablecoin,
            logo_url: token.logo_url.clone(),
            chain: canonical_chain.to_string(),
        },
        balance_raw: raw_balance_u128.to_string(),
        balance_formatted,
    })
}

// Routes
pub fn routes() -> axum::Router<Arc<crate::app_state::AppState>> {
    use axum::routing::get;

    axum::Router::new()
        .route("/list", get(get_token_list))
        .route("/by-address", get(get_token_info_by_address))
        .route("/by-symbol", get(get_token_address_by_symbol))
}
