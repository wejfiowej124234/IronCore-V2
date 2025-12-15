//! 法币订单列表API
//! 企业级实现，支持订单列表查询

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::{
        fiat_api::{FiatOrderStatusResponse, WithdrawOrderStatusResponse},
        middleware::jwt_extractor::JwtAuthContext,
        response::{convert_error, success_response},
    },
    app_state::AppState,
    error::AppError,
};

/// GET /api/fiat/orders - 获取充值订单列表
#[derive(Debug, Deserialize)]
pub struct FiatOrdersQuery {
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct FiatOrdersResponse {
    pub orders: Vec<FiatOrderStatusResponse>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

pub async fn get_fiat_orders(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Query(query): Query<FiatOrdersQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<FiatOrdersResponse>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let fiat_service = crate::service::fiat_service::FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    let (orders, total) = fiat_service
        .list_orders(
            auth.tenant_id,
            auth.user_id,
            Some("onramp"),
            query.status.as_deref(),
            page,
            page_size,
        )
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let order_responses: Vec<FiatOrderStatusResponse> = orders
        .into_iter()
        .map(|order| FiatOrderStatusResponse {
            order_id: order.id.to_string(),
            status: order.status.to_string(),
            fiat_amount: order.fiat_amount.to_string(),
            crypto_amount: order.crypto_amount.to_string(),
            exchange_rate: Some(order.exchange_rate.to_string()),
            fee_amount: Some(order.fee_amount.to_string()),
            payment_url: order.payment_url,
            tx_hash: None,
            created_at: order.created_at.to_rfc3339(),
            updated_at: order.updated_at.to_rfc3339(),
            completed_at: order.completed_at.map(|d| d.to_rfc3339()),
            error_message: None,
        })
        .collect();

    let total_pages = (total as f64 / page_size as f64).ceil() as u32;

    success_response(FiatOrdersResponse {
        orders: order_responses,
        total,
        page,
        page_size,
        total_pages,
    })
}

/// GET /api/fiat/offramp/orders - 获取提现订单列表
#[derive(Debug, Deserialize)]
pub struct OfframpOrdersQuery {
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct OfframpOrdersResponse {
    pub orders: Vec<WithdrawOrderStatusResponse>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

pub async fn get_offramp_orders(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Query(query): Query<OfframpOrdersQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<OfframpOrdersResponse>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let fiat_service = crate::service::fiat_service::FiatService::new(
        state.pool.clone(),
        state.price_service.clone(),
        std::env::var("ONRAMPER_API_KEY").ok(),
        std::env::var("TRANSFI_API_KEY").ok(),
        std::env::var("TRANSFI_SECRET").ok(),
    )?;
    let (orders, total) = fiat_service
        .list_orders(
            auth.tenant_id,
            auth.user_id,
            Some("offramp"),
            query.status.as_deref(),
            page,
            page_size,
        )
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let order_responses: Vec<WithdrawOrderStatusResponse> = orders
        .into_iter()
        .map(|order| WithdrawOrderStatusResponse {
            order_id: order.id.to_string(),
            status: order.status.to_string(),
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
        .collect();

    let total_pages = (total as f64 / page_size as f64).ceil() as u32;

    success_response(OfframpOrdersResponse {
        orders: order_responses,
        total,
        page,
        page_size,
        total_pages,
    })
}
