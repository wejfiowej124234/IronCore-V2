//! 简单交换API（同链交换，使用1inch等DEX聚合器）
//! 企业级实现，支持同链代币交换

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    api::{middleware::jwt_extractor::JwtAuthContext, response::success_response},
    app_state::AppState,
    error::AppError,
    repository::SwapTransactionRepository,
    service::{oneinch_service::OneInchService, token_service::TokenService, wallets},
};

/// GET /api/v1/swap/quote - 获取交换报价（简单交换，同链）
#[derive(Debug, Deserialize)]
pub struct SwapQuoteQuery {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub network: String,
}

#[derive(Debug, Serialize)]
pub struct SwapQuoteResponse {
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    /// ✅ 企业级实现：使用String传输，避免f64精度问题
    pub exchange_rate: String,
    /// ✅ 价格影响（百分比），使用String传输
    pub price_impact: String,
    pub gas_estimate: Option<String>,
    pub estimated_gas_usd: Option<String>,
    pub valid_for: Option<u32>,
}

/// 企业级标准：获取简单交换报价（同链交换，使用1inch等DEX聚合器）
pub async fn get_simple_swap_quote(
    State(st): State<Arc<AppState>>,
    Query(query): Query<SwapQuoteQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SwapQuoteResponse>>, AppError> {
    info!(
        "收到交换报价请求: {} {} -> {} on {}",
        query.amount, query.from, query.to, query.network
    );

    // ✅验证输入
    if query.from.trim().is_empty() || query.to.trim().is_empty() {
        return Err(AppError::bad_request("Token symbols required".to_string()));
    }

    let amount_f64 = query
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;

    if amount_f64 <= 0.0 || !amount_f64.is_finite() {
        return Err(AppError::bad_request(
            "Amount must be > 0 and finite".to_string(),
        ));
    }

    // ✅ 使用标准化的链标识符
    let chain_normalized =
        crate::utils::chain_normalizer::normalize_chain_identifier(&query.network)
            .map_err(|e| AppError::chain_not_supported(e.to_string()))?;

    let chain_id_i64 = crate::utils::chain_normalizer::get_chain_id(&chain_normalized)
        .map_err(|e| AppError::chain_not_supported(e.to_string()))?;
    let chain_id = chain_id_i64 as u64;

    // 创建代币服务（从数据库读取代币信息）
    let token_service = TokenService::new(st.pool.clone());

    // 获取代币地址（从数据库查询，替代硬编码）
    let from_token_addr = token_service
        .get_token_address(&query.from, chain_id)
        .await
        .map_err(|e| {
            error!("获取源代币地址失败: {:?}", e);
            AppError::internal("获取代币信息失败".to_string())
        })?
        .ok_or_else(|| AppError::bad_request(format!("不支持的源代币: {}", query.from)))?;

    let to_token_addr = token_service
        .get_token_address(&query.to, chain_id)
        .await
        .map_err(|e| {
            error!("获取目标代币地址失败: {:?}", e);
            AppError::internal("获取代币信息失败".to_string())
        })?
        .ok_or_else(|| AppError::bad_request(format!("不支持的目标代币: {}", query.to)))?;

    // 从数据库获取代币小数位数（替代硬编码）
    let from_decimals = token_service
        .get_token_decimals(&query.from, chain_id)
        .await
        .map_err(|e| {
            error!("获取代币小数位数失败: {:?}", e);
            AppError::internal("获取代币信息失败".to_string())
        })?
        .ok_or_else(|| {
            error!("代币 {} 在链 {} 上未找到或未启用", query.from, chain_id);
            AppError::bad_request(format!("代币 {} 不支持或未启用", query.from))
        })?;

    let amount_wei = (amount_f64 * 10f64.powi(from_decimals as i32)) as u128;
    let amount_str = amount_wei.to_string();

    // 创建1inch服务
    let oneinch_service = OneInchService::new();

    // ✅ 生产级别：必须配置API Key
    if oneinch_service.api_key.is_none() {
        error!("1inch API Key 未配置，无法提供交换服务");
        return Err(AppError::internal(
            "Swap功能暂时不可用，请联系管理员配置1inch API Key".to_string()
        ));
    }

    // 调用1inch API获取报价
    match oneinch_service
        .get_quote(chain_id, &from_token_addr, &to_token_addr, &amount_str)
        .await
    {
        Ok(quote) => {
            info!(
                "成功获取交换报价: {} {} → {} {} (汇率: {})",
                quote.from_amount,
                quote.from_token,
                quote.to_amount,
                quote.to_token,
                quote.exchange_rate
            );

            success_response(SwapQuoteResponse {
                from_token: quote.from_token,
                to_token: quote.to_token,
                from_amount: quote.from_amount,
                to_amount: quote.to_amount,
                exchange_rate: quote.exchange_rate.to_string(),
                price_impact: quote.price_impact.to_string(),
                gas_estimate: Some(quote.gas_estimate),
                estimated_gas_usd: Some(quote.estimated_gas_usd.to_string()),
                valid_for: Some(quote.valid_for),
            })
        }
        Err(e) => {
            error!("获取交换报价失败: {:?}", e);
            Err(AppError::rpc_error(format!("无法获取交换报价: {}", e)))
        }
    }
}

/// POST /api/v1/swap/execute - 执行简单交换（同链）
#[derive(Debug, Deserialize)]
pub struct SwapExecuteRequest {
    pub wallet_name: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
    pub network: String,
    pub slippage: f64,
    pub password: Option<String>,
    pub client_request_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SwapExecuteResponse {
    pub tx_id: String,
    pub status: String,
    pub from_amount: String,
    pub to_amount: String,
    /// ✅ 企业级实现：使用String传输汇率，避免精度问题
    pub actual_rate: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_used: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform_service_fee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_fee_collector: Option<String>,
    pub confirmations: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction: Option<SwapTransactionData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_approval: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SwapTransactionData {
    pub to: String,
    pub value: String,
    pub data: String,
    pub gas: Option<String>,
    pub gas_price: Option<String>,
}

pub async fn execute_simple_swap(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Json(req): Json<SwapExecuteRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SwapExecuteResponse>>, AppError> {
    info!(
        "收到交换执行请求: wallet={} 数量={} {} -> {} on {}",
        req.wallet_name, req.amount, req.from_token, req.to_token, req.network
    );

    // ✅验证输入
    if req.wallet_name.trim().is_empty() {
        return Err(AppError::bad_request("Wallet name required".to_string()));
    }

    let amount_f64 = req
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;

    if amount_f64 <= 0.0 || !amount_f64.is_finite() {
        return Err(AppError::bad_request(
            "Amount must be > 0 and finite".to_string(),
        ));
    }

    if !req.slippage.is_finite() || req.slippage < 0.0 || req.slippage > 50.0 {
        return Err(AppError::bad_request("Slippage: 0-50%".to_string()));
    }

    // ✅ 使用标准化的链标识符
    let chain_normalized = crate::utils::chain_normalizer::normalize_chain_identifier(&req.network)
        .map_err(|e| AppError::chain_not_supported(e.to_string()))?;

    let chain_id_i64 = crate::utils::chain_normalizer::get_chain_id(&chain_normalized)
        .map_err(|e| AppError::chain_not_supported(e.to_string()))?;
    let chain_id = chain_id_i64 as u64;

    // 企业级实现：获取钱包（支持UUID格式的wallet_id或wallet_name）
    let wallet = if let Ok(wallet_id) = Uuid::parse_str(&req.wallet_name) {
        let wallet_opt = wallets::get_wallet_by_id(&state.pool, wallet_id)
            .await
            .map_err(|e| AppError::internal(format!("获取钱包失败: {}", e)))?;

        let wallet = wallet_opt.ok_or_else(|| {
            AppError::wallet_not_found(format!("钱包 '{}' 不存在", req.wallet_name))
        })?;

        // ✅ 企业级实现：验证钱包所有权
        if wallet.tenant_id != auth.tenant_id || wallet.user_id != auth.user_id {
            error!(
                "钱包权限验证失败: wallet_id={}, wallet_tenant_id={}, wallet_user_id={}, auth_tenant_id={}, auth_user_id={}",
                wallet_id, wallet.tenant_id, wallet.user_id, auth.tenant_id, auth.user_id
            );
            return Err(AppError {
                code: crate::error::AppErrorCode::Forbidden,
                message: "无权访问此钱包".to_string(),
                status: axum::http::StatusCode::FORBIDDEN,
                trace_id: None,
            });
        }

        wallet
    } else {
        let user_wallets =
            wallets::list_wallets_by_user(&state.pool, auth.tenant_id, auth.user_id, 100, 0)
                .await
                .map_err(|e| AppError::internal(format!("获取钱包列表失败: {}", e)))?;

        user_wallets
            .into_iter()
            .find(|w| w.name.as_deref() == Some(&req.wallet_name))
            .ok_or_else(|| {
                AppError::wallet_not_found(format!("钱包 '{}' 不存在", req.wallet_name))
            })?
    };

    let token_service = TokenService::new(state.pool.clone());
    let from_token_addr = token_service
        .get_token_address(&req.from_token, chain_id)
        .await
        .map_err(|e| AppError::internal(format!("获取代币地址失败: {}", e)))?
        .ok_or_else(|| AppError::bad_request(format!("不支持的源代币: {}", req.from_token)))?;

    let to_token_addr = token_service
        .get_token_address(&req.to_token, chain_id)
        .await
        .map_err(|e| AppError::internal(format!("获取代币地址失败: {}", e)))?
        .ok_or_else(|| AppError::bad_request(format!("不支持的目标代币: {}", req.to_token)))?;

    let wallet_address = wallet.address.clone();
    let from_decimals = token_service
        .get_token_decimals(&req.from_token, chain_id)
        .await
        .map_err(|e| AppError::internal(format!("获取代币小数位数失败: {}", e)))?
        .ok_or_else(|| AppError::bad_request(format!("代币 {} 不支持或未启用", req.from_token)))?;

    let amount_wei = (amount_f64 * 10f64.powi(from_decimals as i32)) as u128;
    let amount_str = amount_wei.to_string();

    // ✅ 使用统一的Gas估算服务（预留，当前由1inch API提供）
    let _gas_estimation_service = crate::service::gas_estimation_service::GasEstimationService::new(
        state.rpc_selector.clone(),
        Some(state.redis.clone()),
    );

    let oneinch_service = OneInchService::new();
    let quote = oneinch_service
        .get_quote(chain_id, &from_token_addr, &to_token_addr, &amount_str)
        .await
        .map_err(|e| AppError::rpc_error(format!("无法获取交换报价: {}", e)))?;

    let tx_data = oneinch_service
        .get_swap_tx(
            chain_id,
            &from_token_addr,
            &to_token_addr,
            &amount_str,
            &wallet_address,
            req.slippage,
        )
        .await
        .map_err(|e| AppError::rpc_error(format!("无法获取交换交易数据: {}", e)))?;

    // 创建swap交易记录
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use crate::repository::swap_transaction::SwapTransaction;

    let swap_id = format!("swap_{}", Uuid::new_v4().to_string().replace("-", ""));
    let from_amount_decimal = Decimal::from_str(&req.amount)
        .map_err(|_| AppError::invalid_amount(format!("无效的交换数量: {}", req.amount)))?;

    let to_amount_decimal = Decimal::from_str(&quote.to_amount)
        .map_err(|_| AppError::internal("解析交换报价失败".to_string()))?;

    let slippage_decimal = Decimal::from_str(&req.slippage.to_string())
        .map_err(|_| AppError::bad_request(format!("无效的滑点值: {}", req.slippage)))?;

    let swap_tx = SwapTransaction {
        id: Uuid::new_v4(),
        tenant_id: auth.tenant_id,
        user_id: auth.user_id,
        wallet_id: Some(wallet.id),          // ✅ 修复：改为Option
        chain: Some(req.network.clone()),     // ✅ 使用network作为chain（包装成Option）
        network: req.network.clone(),
        from_token: req.from_token.clone(),
        to_token: req.to_token.clone(),
        from_amount: from_amount_decimal,
        to_amount: Some(to_amount_decimal),
        to_amount_min: None,                  // ✅ 新增
        slippage: Some(slippage_decimal),
        swap_id: swap_id.clone(),
        tx_hash: None,
        wallet_address: Some(wallet.address.clone()),  // ✅ 新增
        status: "pending".to_string(),
        fiat_order_id: None,                  // ✅ 新增
        gas_used: None,
        confirmations: 0,
        metadata: Some(serde_json::json!({
            "created_via": "api",
            "wallet_name": wallet.name.clone().unwrap_or_else(|| "unknown".to_string()),
            "wallet_address": wallet.address,
            "network": req.network,
        })),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let swap_repo = SwapTransactionRepository::new(state.pool.clone());
    swap_repo
        .create(&swap_tx)
        .await
        .map_err(|e| AppError::internal(format!("创建swap记录失败: {}", e)))
        .ok();

    success_response(SwapExecuteResponse {
        tx_id: swap_id,
        status: "pending".to_string(),
        from_amount: req.amount,
        to_amount: quote.to_amount,
        actual_rate: quote.exchange_rate.to_string(),
        gas_used: Some(quote.gas_estimate),
        gas_price: tx_data.gas_price.clone(),
        platform_service_fee: None,
        service_fee_collector: None,
        confirmations: 0,
        transaction: Some(SwapTransactionData {
            to: tx_data.to,
            value: tx_data.value,
            data: tx_data.data,
            gas: tx_data.gas,
            gas_price: tx_data.gas_price,
        }),
        needs_approval: None,
        router_address: None,
    })
}

/// PUT /api/v1/swap/:swap_id/status - 更新swap交易状态
#[derive(Debug, Deserialize)]
pub struct UpdateSwapStatusRequest {
    pub tx_hash: Option<String>,
    pub status: String,
    pub gas_used: Option<String>,
    pub confirmations: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UpdateSwapStatusResponse {
    pub swap_id: String,
    pub status: String,
    pub tx_hash: Option<String>,
}

pub async fn update_swap_status(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(swap_id): Path<String>,
    Json(req): Json<UpdateSwapStatusRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<UpdateSwapStatusResponse>>, AppError> {
    let valid_statuses = ["pending", "executing", "confirmed", "failed", "cancelled"];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::bad_request(format!("无效的状态: {}", req.status)));
    }

    let swap_repo = SwapTransactionRepository::new(state.pool.clone());
    let swap_tx = swap_repo
        .find_by_swap_id(&swap_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Swap交易 '{}' 不存在", swap_id)))?;

    if swap_tx.user_id != auth.user_id || swap_tx.tenant_id != auth.tenant_id {
        return Err(AppError {
            code: crate::error::AppErrorCode::Forbidden,
            message: "无权更新此swap交易".to_string(),
            status: axum::http::StatusCode::FORBIDDEN,
            trace_id: None,
        });
    }

    let to_amount = if req.status == "confirmed" {
        swap_tx.to_amount
    } else {
        None
    };
    let confirmations = req.confirmations.map(|c| c as i32);

    swap_repo
        .update_status(
            &swap_id,
            &req.status,
            req.tx_hash.as_deref(),
            to_amount,
            req.gas_used.as_deref(),
            confirmations,
        )
        .await
        .map_err(|e| AppError::internal(format!("更新swap状态失败: {}", e)))?;

    success_response(UpdateSwapStatusResponse {
        swap_id,
        status: req.status,
        tx_hash: req.tx_hash,
    })
}

/// GET /api/v1/swap/:swap_id - 获取swap交易状态
#[derive(Debug, Serialize)]
pub struct SwapStatusResponse {
    pub swap_id: String,
    pub status: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: Option<String>,
    pub network: String,
    pub tx_hash: Option<String>,
    pub gas_used: Option<String>,
    pub confirmations: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn get_swap_status(
    State(state): State<Arc<AppState>>,
    _auth: JwtAuthContext,
    Path(swap_id): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SwapStatusResponse>>, AppError> {
    let swap_repo = SwapTransactionRepository::new(state.pool.clone());
    let swap_tx = swap_repo
        .find_by_swap_id(&swap_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Swap交易 '{}' 不存在", swap_id)))?;

    success_response(SwapStatusResponse {
        swap_id: swap_tx.swap_id,
        status: swap_tx.status,
        from_token: swap_tx.from_token,
        to_token: swap_tx.to_token,
        from_amount: swap_tx.from_amount.to_string(),
        to_amount: swap_tx.to_amount.map(|d| d.to_string()),
        network: swap_tx.network,
        tx_hash: swap_tx.tx_hash,
        gas_used: swap_tx.gas_used,
        confirmations: swap_tx.confirmations as u32,
        created_at: swap_tx.created_at,
        updated_at: swap_tx.updated_at,
    })
}

// Routes
pub fn routes() -> axum::Router<Arc<crate::app_state::AppState>> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/quote", get(get_simple_swap_quote))
        .route("/execute", post(execute_simple_swap))
        .route("/:id", get(get_swap_status))
        .route("/:id/status", get(get_swap_status).put(update_swap_status))
}
