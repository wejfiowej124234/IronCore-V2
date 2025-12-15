//! 法币充值和提现API
//! 企业级实现，禁止Mock数据
use std::{str::FromStr, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

use crate::{
    api::{
        middleware::jwt_extractor::JwtAuthContext,
        response::{convert_error, success_response},
    },
    app_state::AppState,
    error::AppError,
    service::fiat_service::FiatService,
};

/// GET /api/fiat/quote - 获取法币充值报价
#[derive(Debug, Deserialize)]
pub struct FiatQuoteQuery {
    pub amount: String,
    pub currency: String,
    /// 代币符号（例如: "USDT", "ETH"）
    /// 企业级标准：统一使用 token 字段，chain 信息从 token 推断
    pub token: String,
    pub payment_method: String,
    /// 可选：目标链（如果token是USDT等跨链代币，需要指定目标链）
    pub target_chain: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FiatQuoteResponse {
    pub fiat_amount: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub fee_amount: String,
    pub fee_percentage: f64,
    pub estimated_arrival: String,
    pub quote_expires_at: String,
    pub min_amount: String,
    pub max_amount: String,
    pub quote_id: String,
}

pub async fn get_fiat_quote(
    State(state): State<Arc<AppState>>,
    auth: Option<JwtAuthContext>,
    Query(query): Query<FiatQuoteQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<FiatQuoteResponse>>, AppError> {
    // 记录请求详情（支持匿名和已认证用户）
    let user_info = auth.as_ref().map(|a| format!("user_id={}", a.user_id)).unwrap_or_else(|| "anonymous".to_string());
    tracing::info!(
        "[FiatAPI] get_fiat_quote: {}, amount={}, currency={}, token={}, payment_method={}",
        user_info, query.amount, query.currency, query.token, query.payment_method
    );

    // ✅金额验证
    let amount = rust_decimal::Decimal::from_str(&query.amount)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid amount format".to_string()))?;

    if amount <= rust_decimal::Decimal::ZERO {
        return Err(convert_error(
            StatusCode::BAD_REQUEST,
            "Amount must be > 0".to_string(),
        ));
    }

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;

    // 使用默认租户ID和用户ID（用于匿名查询）
    let tenant_id = auth.as_ref().map(|a| a.tenant_id).unwrap_or_else(|| Uuid::nil());
    let user_id = auth.as_ref().map(|a| a.user_id).unwrap_or_else(|| Uuid::nil());

    let quote = fiat_service
        .get_onramp_quote(
            tenant_id,
            user_id,
            amount,
            &query.currency,
            &query.token,
            &query.payment_method,
            None,
            None,
        )
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            tracing::error!("[FiatAPI] get_fiat_quote failed: {}, error={}", user_info, error_msg);
            
            // 企业级错误映射：将业务错误映射为4xx，避免统一返回500
            if error_msg.contains("没有可用的支付服务商") || error_msg.contains("No available payment providers") {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "法币充值服务暂时不可用，请稍后重试".to_string())
            } else if error_msg.contains("没有支持您所在国家的支付服务商") || error_msg.contains("country") {
                convert_error(StatusCode::FORBIDDEN, "您所在地区暂不支持此服务".to_string())
            } else if error_msg.contains("无法获取报价") || error_msg.contains("Failed to fetch") {
                convert_error(StatusCode::BAD_GATEWAY, "获取报价失败，请重试".to_string())
            } else if error_msg.contains("金额") || error_msg.contains("amount") {
                convert_error(StatusCode::BAD_REQUEST, error_msg)
            } else {
                // 对于未知错误，返回503而不是500，表示服务暂时不可用
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "服务暂时不可用，请稍后重试".to_string())
            }
        })?;

    success_response(FiatQuoteResponse {
        fiat_amount: quote.fiat_amount.to_string(),
        crypto_amount: quote.crypto_amount.to_string(),
        exchange_rate: quote.exchange_rate.to_string(),
        fee_amount: quote.fee_amount.to_string(),
        fee_percentage: quote.fee_percentage.to_string().parse().unwrap_or(0.0),
        estimated_arrival: quote.estimated_arrival,
        quote_expires_at: quote.quote_expires_at.to_rfc3339(),
        min_amount: quote.min_amount.to_string(),
        max_amount: quote.max_amount.to_string(),
        quote_id: quote.quote_id,
    })
}

/// POST /api/fiat/order - 创建法币充值订单
#[derive(Debug, Deserialize)]
pub struct CreateFiatOrderRequest {
    pub amount: String,
    pub currency: String,
    pub token: String,
    pub payment_method: String,
    pub quote_id: String,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FiatOrderResponse {
    pub order_id: String,
    pub status: String,
    pub payment_url: Option<String>,
    pub fiat_amount: String,
    pub crypto_amount: String,
    pub exchange_rate: String,
    pub fee_amount: String,
    pub estimated_arrival: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

pub async fn create_fiat_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Json(req): Json<CreateFiatOrderRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<FiatOrderResponse>>, AppError> {
    // ✅金额验证
    let amount = rust_decimal::Decimal::from_str(&req.amount)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid amount format".to_string()))?;

    if amount <= rust_decimal::Decimal::ZERO {
        return Err(convert_error(
            StatusCode::BAD_REQUEST,
            "Amount must be > 0".to_string(),
        ));
    }

    // ✅钱包地址验证（如果提供）
    if let Some(ref addr) = req.wallet_address {
        if addr.trim().is_empty() {
            return Err(convert_error(
                StatusCode::BAD_REQUEST,
                "Invalid wallet address".to_string(),
            ));
        }
    }

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;

    let order = fiat_service
        .create_onramp_order(
            auth.tenant_id,
            auth.user_id,
            amount,
            &req.currency,
            &req.token,
            &req.payment_method,
            &req.quote_id,
            req.wallet_address.as_deref(),
            None,
            None,
        )
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            tracing::error!("[FiatAPI] create_fiat_order failed: user_id={}, error={}", auth.user_id, error_msg);
            
            // 企业级错误映射：将业务错误映射为4xx/5xx，避免统一返回500
            if error_msg.contains("没有可用的支付服务商") || error_msg.contains("No available payment providers") {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "法币充值服务暂时不可用，请稍后重试".to_string())
            } else if error_msg.contains("没有支持您所在国家的支付服务商") || error_msg.contains("country") {
                convert_error(StatusCode::FORBIDDEN, "您所在地区暂不支持此服务".to_string())
            } else if error_msg.contains("无法创建订单") || error_msg.contains("Failed to create") {
                convert_error(StatusCode::BAD_GATEWAY, "创建订单失败，请重试".to_string())
            } else if error_msg.contains("金额") || error_msg.contains("amount") || error_msg.contains("Invalid") {
                convert_error(StatusCode::BAD_REQUEST, error_msg)
            } else if error_msg.contains("Failed to fetch enabled providers from database") {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "服务暂时不可用，请稍后重试".to_string())
            } else {
                // 对于未知错误，返回503而不是500，表示服务暂时不可用
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "服务暂时不可用，请稍后重试".to_string())
            }
        })?;

    let response = FiatOrderResponse {
        order_id: order.id.to_string(),
        status: order.status.clone(),
        payment_url: order.payment_url.clone(),
        fiat_amount: order.fiat_amount.to_string(),
        crypto_amount: order.crypto_amount.to_string(),
        exchange_rate: order.exchange_rate.to_string(),
        fee_amount: order.fee_amount.to_string(),
        estimated_arrival: Some("Instant".to_string()),
        created_at: order.created_at.to_rfc3339(),
        expires_at: order.order_expires_at.map(|d| d.to_rfc3339()),
    };

    tracing::info!(
        "[FiatAPI] create_fiat_order response: order_id={}, status={}, payment_url={:?}",
        response.order_id,
        response.status,
        response.payment_url
    );

    success_response(response)
}

/// GET /api/fiat/offramp/quote - 获取法币提现报价
#[derive(Debug, Deserialize)]
pub struct OfframpQuoteQuery {
    pub token: String,
    pub amount: String,
    pub chain: String,
    pub fiat_currency: String,
    pub withdraw_method: String,
}

#[derive(Debug, Serialize)]
pub struct OfframpQuoteResponse {
    pub token_amount: String,
    pub token_symbol: String,
    pub stablecoin_amount: String,
    pub stablecoin_symbol: String,
    pub fiat_amount: String,
    pub fiat_currency: String,
    pub exchange_rate_token_to_stable: String,
    pub exchange_rate_stable_to_fiat: String,
    pub fee_amount: String,
    pub fee_percentage: f64,
    pub swap_fee: String,       // 交换手续费（代币→稳定币）
    pub withdrawal_fee: String, // 提现手续费（稳定币→法币）
    pub estimated_arrival: String,
    pub quote_expires_at: String,
    pub min_amount: Option<String>,
    pub max_amount: Option<String>,
    pub quote_id: String,
}

pub async fn get_withdraw_quote(
    State(state): State<Arc<AppState>>,
    auth: Option<JwtAuthContext>,
    Query(query): Query<OfframpQuoteQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<OfframpQuoteResponse>>, AppError> {
    let amount = rust_decimal::Decimal::from_str(&query.amount)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid amount".to_string()))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;

    // 使用默认租户ID和用户ID（用于匿名查询）
    let tenant_id = auth.as_ref().map(|a| a.tenant_id).unwrap_or_else(|| Uuid::nil());
    let user_id = auth.as_ref().map(|a| a.user_id).unwrap_or_else(|| Uuid::nil());
    let user_info = auth.as_ref().map(|a| format!("user_id={}", a.user_id)).unwrap_or_else(|| "anonymous".to_string());

    let quote = fiat_service
        .get_offramp_quote(
            tenant_id,
            user_id,
            &query.token,
            amount,
            &query.chain,
            &query.fiat_currency,
            &query.withdraw_method,
        )
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            tracing::error!("[FiatAPI] get_offramp_quote failed: {}, error={}", user_info, error_msg);
            
            // 企业级错误映射
            if error_msg.contains("没有可用的支付服务商") || error_msg.contains("No available payment providers") {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "法币提现服务暂时不可用，请稍后重试".to_string())
            } else if error_msg.contains("没有支持您所在国家的支付服务商") || error_msg.contains("country") {
                convert_error(StatusCode::FORBIDDEN, "您所在地区暂不支持此服务".to_string())
            } else if error_msg.contains("无法获取报价") || error_msg.contains("Failed to fetch") {
                convert_error(StatusCode::BAD_GATEWAY, "获取报价失败，请重试".to_string())
            } else if error_msg.contains("金额") || error_msg.contains("amount") {
                convert_error(StatusCode::BAD_REQUEST, error_msg)
            } else {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "服务暂时不可用，请稍后重试".to_string())
            }
        })?;

    success_response(OfframpQuoteResponse {
        token_amount: quote.token_amount.to_string(),
        token_symbol: quote.token_symbol,
        stablecoin_amount: quote.stablecoin_amount.to_string(),
        stablecoin_symbol: quote.stablecoin_symbol,
        fiat_amount: quote.fiat_amount.to_string(),
        fiat_currency: quote.fiat_currency,
        exchange_rate_token_to_stable: quote.exchange_rate_token_to_stable.to_string(),
        exchange_rate_stable_to_fiat: quote.exchange_rate_stable_to_fiat.to_string(),
        fee_amount: quote.fee_amount.to_string(),
        fee_percentage: quote.fee_percentage.to_string().parse().unwrap_or(0.0),
        swap_fee: quote.swap_fee.to_string(),
        withdrawal_fee: quote.withdrawal_fee.to_string(),
        estimated_arrival: quote.estimated_arrival,
        quote_expires_at: quote.quote_expires_at.to_rfc3339(),
        min_amount: Some(quote.min_amount.to_string()),
        max_amount: Some(quote.max_amount.to_string()),
        quote_id: quote.quote_id,
    })
}

/// POST /api/fiat/offramp/order - 创建法币提现订单
#[derive(Debug, Deserialize)]
pub struct CreateWithdrawOrderRequest {
    pub token: String,
    pub amount: String,
    pub chain: String,
    pub fiat_currency: String,
    pub withdraw_method: String,
    pub recipient_info: serde_json::Value,
    pub quote_id: String,
}

#[derive(Debug, Serialize)]
pub struct WithdrawOrderResponse {
    pub order_id: String,
    pub status: String,
    pub review_status: Option<String>,
    pub token_amount: String,
    pub token_symbol: String,
    pub stablecoin_amount: String,
    pub stablecoin_symbol: String,
    pub fiat_amount: String,
    pub fiat_currency: String,
    pub fee_amount: String,
    pub estimated_arrival: String,
    pub swap_tx_hash: Option<String>,
    pub created_at: String,
    pub expires_at: String,
}

pub async fn create_withdraw_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Json(req): Json<CreateWithdrawOrderRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<WithdrawOrderResponse>>, AppError> {
    let amount = rust_decimal::Decimal::from_str(&req.amount)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid amount".to_string()))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;

    let order = fiat_service
        .create_offramp_order(
            auth.tenant_id,
            auth.user_id,
            &req.token,
            amount,
            &req.chain,
            &req.fiat_currency,
            &req.withdraw_method,
            req.recipient_info,
            &req.quote_id,
        )
        .await
        .map_err(|e| {
            let error_msg = e.to_string();
            tracing::error!("[FiatAPI] create_withdraw_order failed: user_id={}, error={}", auth.user_id, error_msg);
            
            // 企业级错误映射：将业务错误映射为4xx/5xx，避免统一返回500
            if error_msg.contains("没有可用的支付服务商") || error_msg.contains("No available payment providers") {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "法币提现服务暂时不可用，请稍后重试".to_string())
            } else if error_msg.contains("没有支持您所在国家的支付服务商") || error_msg.contains("country") {
                convert_error(StatusCode::FORBIDDEN, "您所在地区暂不支持此服务".to_string())
            } else if error_msg.contains("无法创建订单") || error_msg.contains("Failed to create") {
                convert_error(StatusCode::BAD_GATEWAY, "创建提现订单失败，请重试".to_string())
            } else if error_msg.contains("余额不足") || error_msg.contains("insufficient") {
                convert_error(StatusCode::BAD_REQUEST, "余额不足".to_string())
            } else if error_msg.contains("金额") || error_msg.contains("amount") || error_msg.contains("Invalid") {
                convert_error(StatusCode::BAD_REQUEST, error_msg)
            } else if error_msg.contains("Failed to fetch enabled providers from database") {
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "服务暂时不可用，请稍后重试".to_string())
            } else {
                // 对于未知错误，返回503而不是500，表示服务暂时不可用
                convert_error(StatusCode::SERVICE_UNAVAILABLE, "服务暂时不可用，请稍后重试".to_string())
            }
        })?;

    success_response(WithdrawOrderResponse {
        order_id: order.id.to_string(),
        status: order.status,
        review_status: order.review_status,
        token_amount: order.crypto_amount.to_string(),
        token_symbol: req.token,
        stablecoin_amount: "0".to_string(), // 从metadata提取
        stablecoin_symbol: "USDT".to_string(),
        fiat_amount: order.fiat_amount.to_string(),
        fiat_currency: req.fiat_currency,
        fee_amount: order.fee_amount.to_string(),
        estimated_arrival: "1-3 business days".to_string(),
        swap_tx_hash: order.swap_tx_hash,
        created_at: order.created_at.to_rfc3339(),
        expires_at: order
            .order_expires_at
            .map(|d| d.to_rfc3339())
            .unwrap_or_default(),
    })
}

/// GET /api/fiat/offramp/order/:order_id - 查询提现订单状态
#[derive(Debug, Serialize)]
pub struct WithdrawOrderStatusResponse {
    pub order_id: String,
    pub status: String,
    pub token_amount: String,
    pub token_symbol: String,
    pub stablecoin_amount: String,
    pub stablecoin_symbol: String,
    pub fiat_amount: String,
    pub fiat_currency: String,
    pub fee_amount: String,
    pub swap_tx_hash: Option<String>,
    pub withdrawal_tx_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

pub async fn get_withdraw_order_status(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(order_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<WithdrawOrderStatusResponse>>, AppError> {
    let order_id = Uuid::parse_str(&order_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid order ID".to_string()))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    let order = fiat_service.get_order_status(order_id).await.map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("not found") {
            convert_error(StatusCode::NOT_FOUND, "订单不存在".to_string())
        } else {
            tracing::error!("[FiatAPI] get_order_status failed: order_id={}, error={}", order_id, error_msg);
            convert_error(StatusCode::SERVICE_UNAVAILABLE, "查询订单状态失败，请稍后重试".to_string())
        }
    })?;

    // 验证订单属于当前用户
    if order.user_id != auth.user_id || order.tenant_id != auth.tenant_id {
        return Err(convert_error(
            StatusCode::FORBIDDEN,
            "Order not found".to_string(),
        ));
    }

    success_response(WithdrawOrderStatusResponse {
        order_id: order.id.to_string(),
        status: order.status,
        token_amount: order.crypto_amount.to_string(),
        token_symbol: order.crypto_token,
        stablecoin_amount: "0".to_string(), // 从metadata提取
        stablecoin_symbol: "USDT".to_string(),
        fiat_amount: order.fiat_amount.to_string(),
        fiat_currency: order.fiat_currency,
        fee_amount: order.fee_amount.to_string(),
        swap_tx_hash: order.swap_tx_hash,
        withdrawal_tx_hash: order.withdrawal_tx_hash,
        created_at: order.created_at.to_rfc3339(),
        updated_at: order.updated_at.to_rfc3339(),
        completed_at: order.completed_at.map(|d| d.to_rfc3339()),
        error_message: None,
    })
}

/// GET /api/fiat/order/:order_id - 查询充值订单状态
pub async fn get_fiat_order_status(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(order_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<FiatOrderStatusResponse>>, AppError> {
    let order_id = Uuid::parse_str(&order_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid order ID".to_string()))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    let order = fiat_service.get_order_status(order_id).await.map_err(|e| {
        let error_msg = e.to_string();
        if error_msg.contains("not found") {
            convert_error(StatusCode::NOT_FOUND, "订单不存在".to_string())
        } else {
            tracing::error!("[FiatAPI] get_order_status failed: order_id={}, error={}", order_id, error_msg);
            convert_error(StatusCode::SERVICE_UNAVAILABLE, "查询订单状态失败，请稍后重试".to_string())
        }
    })?;

    // 验证订单属于当前用户
    if order.user_id != auth.user_id || order.tenant_id != auth.tenant_id {
        return Err(convert_error(
            StatusCode::FORBIDDEN,
            "Order not found".to_string(),
        ));
    }

    // 只有充值订单才使用这个端点
    if order.order_type != "onramp" {
        return Err(convert_error(
            StatusCode::BAD_REQUEST,
            "This endpoint is for onramp orders only".to_string(),
        ));
    }

    success_response(FiatOrderStatusResponse {
        order_id: order.id.to_string(),
        status: order.status,
        fiat_amount: order.fiat_amount.to_string(),
        crypto_amount: order.crypto_amount.to_string(),
        exchange_rate: Some(order.exchange_rate.to_string()),
        fee_amount: Some(order.fee_amount.to_string()),
        payment_url: order.payment_url,
        tx_hash: None, // 充值订单通常没有tx_hash
        created_at: order.created_at.to_rfc3339(),
        updated_at: order.updated_at.to_rfc3339(),
        completed_at: order.completed_at.map(|d| d.to_rfc3339()),
        error_message: None,
    })
}

#[derive(Debug, Serialize)]
pub struct FiatOrderStatusResponse {
    pub order_id: String,
    pub status: String,
    pub fiat_amount: String,
    pub crypto_amount: String,
    pub exchange_rate: Option<String>,
    pub fee_amount: Option<String>,
    pub payment_url: Option<String>,
    pub tx_hash: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
}

/// GET /api/v1/fiat/onramp/orders - 列出充值订单
#[derive(Debug, Deserialize)]
pub struct ListOrdersQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct OrderListItem {
    pub order_id: String,
    pub status: String,
    pub fiat_amount: String,
    pub fiat_currency: String,
    pub crypto_amount: String,
    pub crypto_token: String,
    pub payment_method: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderListItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

pub async fn list_onramp_orders(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Query(query): Query<ListOrdersQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<OrderListResponse>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let mut sql = String::from(
        "SELECT id, status, fiat_amount, fiat_currency, crypto_amount, crypto_token, 
                payment_method, created_at, updated_at, COUNT(*) OVER() as total
         FROM fiat.orders 
         WHERE user_id = $1 AND tenant_id = $2 AND order_type = 'onramp'"
    );

    let mut param_count = 2;
    if query.status.is_some() {
        param_count += 1;
        sql.push_str(&format!(" AND status = ${}", param_count));
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT $");
    sql.push_str(&(param_count + 1).to_string());
    sql.push_str(" OFFSET $");
    sql.push_str(&(param_count + 2).to_string());

    let mut query_builder = sqlx::query(&sql)
        .bind(auth.user_id)
        .bind(auth.tenant_id);

    if let Some(ref status) = query.status {
        query_builder = query_builder.bind(status);
    }

    let rows = query_builder
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("[FiatAPI] list_onramp_orders failed: error={}", e);
            convert_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch orders".to_string())
        })?;

    let total = if !rows.is_empty() {
        rows[0].try_get::<i64, _>("total").unwrap_or(0)
    } else {
        0
    };

    let orders: Vec<OrderListItem> = rows
        .into_iter()
        .map(|row| OrderListItem {
            order_id: row.get::<Uuid, _>("id").to_string(),
            status: row.get("status"),
            fiat_amount: row.get::<rust_decimal::Decimal, _>("fiat_amount").to_string(),
            fiat_currency: row.get("fiat_currency"),
            crypto_amount: row.get::<rust_decimal::Decimal, _>("crypto_amount").to_string(),
            crypto_token: row.get("crypto_token"),
            payment_method: row.get("payment_method"),
            created_at: row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            updated_at: row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at").to_rfc3339(),
        })
        .collect();

    success_response(OrderListResponse {
        orders,
        total,
        page,
        page_size,
    })
}

/// GET /api/v1/fiat/offramp/orders - 列出提现订单
pub async fn list_offramp_orders(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Query(query): Query<ListOrdersQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<OrderListResponse>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let mut sql = String::from(
        "SELECT id, status, fiat_amount, fiat_currency, crypto_amount, crypto_token, 
                payment_method, created_at, updated_at, COUNT(*) OVER() as total
         FROM fiat.orders 
         WHERE user_id = $1 AND tenant_id = $2 AND order_type = 'offramp'"
    );

    let mut param_count = 2;
    if query.status.is_some() {
        param_count += 1;
        sql.push_str(&format!(" AND status = ${}", param_count));
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT $");
    sql.push_str(&(param_count + 1).to_string());
    sql.push_str(" OFFSET $");
    sql.push_str(&(param_count + 2).to_string());

    let mut query_builder = sqlx::query(&sql)
        .bind(auth.user_id)
        .bind(auth.tenant_id);

    if let Some(ref status) = query.status {
        query_builder = query_builder.bind(status);
    }

    let rows = query_builder
        .bind(page_size)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("[FiatAPI] list_offramp_orders failed: error={}", e);
            convert_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch orders".to_string())
        })?;

    let total = if !rows.is_empty() {
        rows[0].try_get::<i64, _>("total").unwrap_or(0)
    } else {
        0
    };

    let orders: Vec<OrderListItem> = rows
        .into_iter()
        .map(|row| OrderListItem {
            order_id: row.get::<Uuid, _>("id").to_string(),
            status: row.get("status"),
            fiat_amount: row.get::<rust_decimal::Decimal, _>("fiat_amount").to_string(),
            fiat_currency: row.get("fiat_currency"),
            crypto_amount: row.get::<rust_decimal::Decimal, _>("crypto_amount").to_string(),
            crypto_token: row.get("crypto_token"),
            payment_method: row.get("payment_method"),
            created_at: row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            updated_at: row.get::<chrono::DateTime<chrono::Utc>, _>("updated_at").to_rfc3339(),
        })
        .collect();

    success_response(OrderListResponse {
        orders,
        total,
        page,
        page_size,
    })
}

/// Webhook处理器 - Onramper
#[derive(Debug, Serialize, Deserialize)]
pub struct OnramperWebhookPayload {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub status: String, // completed, failed, processing
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    pub amount: Option<String>,
    pub currency: Option<String>,
}

pub async fn onramper_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<crate::api::response::ApiResponse<()>>, AppError> {
    tracing::info!("[FiatAPI] Onramper webhook received");

    // 1. 验证签名
    let webhook_secret = std::env::var("ONRAMPER_WEBHOOK_SECRET")
        .unwrap_or_else(|_| "test_onramper_webhook_secret".to_string());
    
    if let Err(e) = crate::security::webhook_verifier::verify_onramper_signature(
        &headers,
        &body,
        &webhook_secret,
    ) {
        tracing::error!("[FiatAPI] Onramper signature verification failed: {}", e);
        return Err(convert_error(
            StatusCode::UNAUTHORIZED,
            "Invalid webhook signature".to_string(),
        ));
    }

    // 2. 解析payload
    let payload: OnramperWebhookPayload = serde_json::from_str(&body)
        .map_err(|e| convert_error(
            StatusCode::BAD_REQUEST,
            format!("Invalid webhook payload: {}", e),
        ))?;

    // 3. 映射状态
    let order_status = match payload.status.as_str() {
        "completed" => crate::service::fiat_service::FiatOrderStatus::Completed,
        "failed" => crate::service::fiat_service::FiatOrderStatus::Failed,
        "processing" => crate::service::fiat_service::FiatOrderStatus::Processing,
        _ => crate::service::fiat_service::FiatOrderStatus::Pending,
    };

    // 4. 更新订单
    let order_id = Uuid::from_str(&payload.order_id)
        .map_err(|_| convert_error(
            StatusCode::BAD_REQUEST,
            "Invalid order ID format".to_string(),
        ))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    fiat_service
        .update_order_status(
            order_id,
            order_status,
            payload.tx_hash.clone(),
            Some(serde_json::to_value(&payload).unwrap()),
        )
        .await
        .map_err(|e| convert_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update order: {}", e),
        ))?;

    tracing::info!("[FiatAPI] ✅ Onramper webhook processed: order_id={}", order_id);
    success_response(())
}

/// Webhook处理器 - TransFi
#[derive(Debug, Serialize, Deserialize)]
pub struct TransFiWebhookPayload {
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub status: String, // SUCCESS, FAILED, PROCESSING
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<String>,
    pub amount: Option<String>,
}

pub async fn transfi_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<crate::api::response::ApiResponse<()>>, AppError> {
    tracing::info!("[FiatAPI] TransFi webhook received");

    // 1. 验证签名
    let webhook_secret = std::env::var("TRANSFI_WEBHOOK_SECRET")
        .unwrap_or_else(|_| "test_transfi_webhook_secret".to_string());
    
    if let Err(e) = crate::security::webhook_verifier::verify_transfi_signature(
        &headers,
        &body,
        "POST",
        "/api/v1/fiat/webhook/transfi",
        &webhook_secret,
    ) {
        tracing::error!("[FiatAPI] TransFi signature verification failed: {}", e);
        return Err(convert_error(
            StatusCode::UNAUTHORIZED,
            "Invalid webhook signature".to_string(),
        ));
    }

    // 2. 解析payload
    let payload: TransFiWebhookPayload = serde_json::from_str(&body)
        .map_err(|e| convert_error(
            StatusCode::BAD_REQUEST,
            format!("Invalid webhook payload: {}", e),
        ))?;

    // 3. 映射状态
    let order_status = match payload.status.as_str() {
        "SUCCESS" => crate::service::fiat_service::FiatOrderStatus::Completed,
        "FAILED" => crate::service::fiat_service::FiatOrderStatus::Failed,
        "PROCESSING" => crate::service::fiat_service::FiatOrderStatus::Processing,
        _ => crate::service::fiat_service::FiatOrderStatus::Pending,
    };

    // 4. 更新订单
    let order_id = Uuid::from_str(&payload.order_id)
        .map_err(|_| convert_error(
            StatusCode::BAD_REQUEST,
            "Invalid order ID format".to_string(),
        ))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    fiat_service
        .update_order_status(
            order_id,
            order_status,
            payload.transaction_hash.clone(),
            Some(serde_json::to_value(&payload).unwrap()),
        )
        .await
        .map_err(|e| convert_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update order: {}", e),
        ))?;

    tracing::info!("[FiatAPI] ✅ TransFi webhook processed: order_id={}", order_id);
    success_response(())
}

/// Webhook处理器 - Alchemy Pay
#[derive(Debug, Serialize, Deserialize)]
pub struct AlchemyPayWebhookPayload {
    #[serde(rename = "orderNo")]
    pub order_no: String,
    pub status: String, // COMPLETED, FAILED, PENDING
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
    pub crypto: Option<String>,
    pub amount: Option<String>,
}

pub async fn alchemypay_webhook(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<crate::api::response::ApiResponse<()>>, AppError> {
    tracing::info!("[FiatAPI] Alchemy Pay webhook received");

    // 1. 验证签名
    let webhook_secret = std::env::var("ALCHEMYPAY_WEBHOOK_SECRET")
        .unwrap_or_else(|_| "test_alchemy_webhook_secret".to_string());
    
    if let Err(e) = crate::security::webhook_verifier::verify_alchemypay_signature(
        &headers,
        &body,
        &webhook_secret,
    ) {
        tracing::error!("[FiatAPI] Alchemy Pay signature verification failed: {}", e);
        return Err(convert_error(
            StatusCode::UNAUTHORIZED,
            "Invalid webhook signature".to_string(),
        ));
    }

    // 2. 解析payload
    let payload: AlchemyPayWebhookPayload = serde_json::from_str(&body)
        .map_err(|e| convert_error(
            StatusCode::BAD_REQUEST,
            format!("Invalid webhook payload: {}", e),
        ))?;

    // 3. 映射状态
    let order_status = match payload.status.as_str() {
        "COMPLETED" => crate::service::fiat_service::FiatOrderStatus::Completed,
        "FAILED" => crate::service::fiat_service::FiatOrderStatus::Failed,
        "PENDING" => crate::service::fiat_service::FiatOrderStatus::Processing,
        _ => crate::service::fiat_service::FiatOrderStatus::Pending,
    };

    // 4. 更新订单
    let order_id = Uuid::from_str(&payload.order_no)
        .map_err(|_| convert_error(
            StatusCode::BAD_REQUEST,
            "Invalid order ID format".to_string(),
        ))?;

    let fiat_service = FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    fiat_service
        .update_order_status(
            order_id,
            order_status,
            payload.tx_hash.clone(),
            Some(serde_json::to_value(&payload).unwrap()),
        )
        .await
        .map_err(|e| convert_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update order: {}", e),
        ))?;

    tracing::info!("[FiatAPI] ✅ Alchemy Pay webhook processed: order_id={}", order_id);
    success_response(())
}

// Routes
pub fn routes() -> axum::Router<Arc<crate::app_state::AppState>> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/deposit/quote", get(get_fiat_quote))
        .route("/deposit/orders", post(create_fiat_order))
        .route("/deposit/orders/:id", get(get_fiat_order_status))
        .route("/withdraw/quote", get(get_withdraw_quote))
        .route("/withdraw/orders", post(create_withdraw_order))
        .route("/withdraw/orders/:id", get(get_withdraw_order_status))
        // Webhook endpoints (不需要JWT认证)
        .route("/webhook/onramper", post(onramper_webhook))
        .route("/webhook/transfi", post(transfi_webhook))
        .route("/webhook/alchemypay", post(alchemypay_webhook))
}
