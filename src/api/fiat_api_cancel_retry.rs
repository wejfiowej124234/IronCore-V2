//! 法币订单取消和重试API
//! 企业级实现，支持订单管理操作

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use serde::Serialize;

use crate::{
    api::{
        fiat_api::{FiatOrderResponse, WithdrawOrderResponse},
        middleware::jwt_extractor::JwtAuthContext,
        response::{convert_error, success_response},
    },
    app_state::AppState,
    error::AppError,
};

/// POST /api/fiat/order/:order_id/cancel - 取消充值订单
#[derive(Debug, Serialize)]
pub struct CancelOrderResponse {
    pub message: String,
}

pub async fn cancel_fiat_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(order_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<CancelOrderResponse>>, AppError> {
    let order_id = uuid::Uuid::parse_str(&order_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid order ID".to_string()))?;

    let fiat_service = crate::service::fiat_service::FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    fiat_service
        .cancel_order(auth.tenant_id, auth.user_id, order_id)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(CancelOrderResponse {
        message: "Order cancelled successfully".to_string(),
    })
}

/// POST /api/fiat/order/:order_id/retry - 重试充值订单
pub async fn retry_fiat_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(order_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<FiatOrderResponse>>, AppError> {
    let order_id = uuid::Uuid::parse_str(&order_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid order ID".to_string()))?;

    let fiat_service = crate::service::fiat_service::FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    let order = fiat_service
        .retry_order(auth.tenant_id, auth.user_id, order_id)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(FiatOrderResponse {
        order_id: order.id.to_string(),
        status: order.status.to_string(),
        payment_url: order.payment_url,
        fiat_amount: order.fiat_amount.to_string(),
        crypto_amount: order.crypto_amount.to_string(),
        exchange_rate: order.exchange_rate.to_string(),
        fee_amount: order.fee_amount.to_string(),
        estimated_arrival: Some("Instant".to_string()),
        created_at: order.created_at.to_rfc3339(),
        expires_at: order.order_expires_at.map(|d| d.to_rfc3339()),
    })
}

/// POST /api/fiat/offramp/order/:order_id/cancel - 取消提现订单
pub async fn cancel_offramp_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(order_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<CancelOrderResponse>>, AppError> {
    let order_id = uuid::Uuid::parse_str(&order_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid order ID".to_string()))?;

    let fiat_service = crate::service::fiat_service::FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    fiat_service
        .cancel_order(auth.tenant_id, auth.user_id, order_id)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(CancelOrderResponse {
        message: "Order cancelled successfully".to_string(),
    })
}

/// POST /api/fiat/offramp/order/:order_id/retry - 重试提现订单
pub async fn retry_offramp_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(order_id_str): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<WithdrawOrderResponse>>, AppError> {
    let order_id = uuid::Uuid::parse_str(&order_id_str)
        .map_err(|_| convert_error(StatusCode::BAD_REQUEST, "Invalid order ID".to_string()))?;

    let fiat_service = crate::service::fiat_service::FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    let order = fiat_service
        .retry_order(auth.tenant_id, auth.user_id, order_id)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(WithdrawOrderResponse {
        order_id: order.id.to_string(),
        status: order.status.to_string(),
        review_status: order.review_status,
        token_amount: order.crypto_amount.to_string(),
        token_symbol: order.crypto_token,
        stablecoin_amount: "0".to_string(),
        stablecoin_symbol: "USDT".to_string(),
        fiat_amount: order.fiat_amount.to_string(),
        fiat_currency: order.fiat_currency,
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
