//! 限价单API
//! 企业级实现，支持限价单创建、查询、取消

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{middleware::jwt_extractor::JwtAuthContext, response::success_response},
    app_state::AppState,
    error::AppError,
};

/// POST /api/limit-order/create - 创建限价单
#[derive(Debug, Deserialize)]
pub struct CreateLimitOrderRequest {
    pub order_type: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
    pub limit_price: String,
    pub network: String,
    pub expiry_days: u32,
    pub wallet_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LimitOrderResponse {
    pub order_id: String,
    pub order_type: String,
    pub from_token: String,
    pub to_token: String,
    pub amount: String,
    pub limit_price: String,
    pub status: String,
    pub filled_amount: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub message: Option<String>,
}

pub async fn create_limit_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Json(req): Json<CreateLimitOrderRequest>,
) -> Result<axum::Json<crate::api::response::ApiResponse<LimitOrderResponse>>, AppError> {
    // ✅验证输入
    let amount = req
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;
    let limit_price = req
        .limit_price
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid price".to_string()))?;

    if amount <= 0.0 || !amount.is_finite() || limit_price <= 0.0 || !limit_price.is_finite() {
        return Err(AppError::bad_request(
            "Amount and price must be > 0 and finite".to_string(),
        ));
    }

    let order_id = Uuid::new_v4();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(req.expiry_days as i64);

    // ✅存储到数据库
    sqlx::query(r#"INSERT INTO public.limit_orders (id, user_id, tenant_id, order_type, from_token, to_token, amount, limit_price, network, wallet_id, status, expires_at, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'pending', $11, NOW())"#)
        .bind(order_id).bind(auth.user_id).bind(auth.tenant_id).bind(&req.order_type)
        .bind(&req.from_token).bind(&req.to_token).bind(rust_decimal::Decimal::from_f64_retain(amount).unwrap())
        .bind(rust_decimal::Decimal::from_f64_retain(limit_price).unwrap())
        .bind(&req.network).bind(req.wallet_id.as_ref().and_then(|s| Uuid::parse_str(s).ok()))
        .bind(expires_at).execute(&state.pool).await
        .map_err(|e| AppError::database_error(e.to_string()))?;

    success_response(LimitOrderResponse {
        order_id: order_id.to_string(),
        order_type: req.order_type,
        from_token: req.from_token,
        to_token: req.to_token,
        amount: req.amount,
        limit_price: req.limit_price,
        status: "pending".into(),
        filled_amount: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at: Some(expires_at.to_rfc3339()),
        message: Some("Limit order created".into()),
    })
}

/// GET /api/limit-order/list - 获取限价单列表
#[derive(Debug, Deserialize)]
pub struct LimitOrderListQuery {
    pub order_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LimitOrderListResponse {
    pub orders: Vec<LimitOrderResponse>,
    pub total_pages: u32,
    pub current_page: u32,
    pub total_count: u32,
}

pub async fn get_limit_order_list(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Query(query): Query<LimitOrderListQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<LimitOrderListResponse>>, AppError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);
    let offset = ((page - 1) * page_size) as i64;

    // ✅查询限价单
    let mut sql = "SELECT id, order_type, from_token, to_token, amount, limit_price, status, filled_amount, created_at, expires_at FROM public.limit_orders WHERE user_id = $1".to_string();
    if let Some(ref status) = query.status {
        sql.push_str(&format!(" AND status = '{}'", status));
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT $2 OFFSET $3");

    let orders: Vec<LimitOrderResponse> = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            rust_decimal::Decimal,
            rust_decimal::Decimal,
            String,
            Option<rust_decimal::Decimal>,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(&sql)
    .bind(auth.user_id)
    .bind(page_size as i64)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?
    .into_iter()
    .map(
        |(id, ot, ft, tt, amt, lp, st, fa, ca, ea)| LimitOrderResponse {
            order_id: id.to_string(),
            order_type: ot,
            from_token: ft,
            to_token: tt,
            amount: amt.to_string(),
            limit_price: lp.to_string(),
            status: st,
            filled_amount: fa.map(|f| f.to_string()),
            created_at: ca.to_rfc3339(),
            expires_at: ea.map(|e| e.to_rfc3339()),
            message: None,
        },
    )
    .collect();

    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM public.limit_orders WHERE user_id = $1")
            .bind(auth.user_id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);

    success_response(LimitOrderListResponse {
        orders,
        total_pages: ((total + page_size as i64 - 1) / page_size as i64) as u32,
        current_page: page,
        total_count: total as u32,
    })
}

/// GET /api/limit-order/:id - 获取限价单详情
pub async fn get_limit_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(id): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<LimitOrderResponse>>, AppError> {
    let order_id =
        Uuid::parse_str(&id).map_err(|_| AppError::bad_request("Invalid ID".to_string()))?;

    let order = sqlx::query_as::<_, (Uuid, String, String, String, rust_decimal::Decimal, rust_decimal::Decimal, String, Option<rust_decimal::Decimal>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, order_type, from_token, to_token, amount, limit_price, status, filled_amount, created_at, expires_at FROM public.limit_orders WHERE id = $1 AND user_id = $2"
    ).bind(order_id).bind(auth.user_id).fetch_optional(&state.pool).await
    .map_err(|e| AppError::database_error(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Order not found".to_string()))?;

    success_response(LimitOrderResponse {
        order_id: order.0.to_string(),
        order_type: order.1,
        from_token: order.2,
        to_token: order.3,
        amount: order.4.to_string(),
        limit_price: order.5.to_string(),
        status: order.6,
        filled_amount: order.7.map(|f| f.to_string()),
        created_at: order.8.to_rfc3339(),
        expires_at: order.9.map(|e| e.to_rfc3339()),
        message: None,
    })
}

/// POST /api/limit-order/:id/cancel - 取消限价单
pub async fn cancel_limit_order(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(id): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<LimitOrderResponse>>, AppError> {
    let order_id =
        Uuid::parse_str(&id).map_err(|_| AppError::bad_request("Invalid ID".to_string()))?;

    // ✅更新状态为cancelled
    let updated = sqlx::query("UPDATE public.limit_orders SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND user_id = $2 AND status = 'pending'")
        .bind(order_id).bind(auth.user_id).execute(&state.pool).await
        .map_err(|e| AppError::database_error(e.to_string()))?;

    if updated.rows_affected() == 0 {
        return Err(AppError::not_found(
            "Order not found or already filled/cancelled".to_string(),
        ));
    }

    // 查询更新后的订单
    let order = sqlx::query_as::<_, (Uuid, String, String, String, rust_decimal::Decimal, rust_decimal::Decimal, String, Option<rust_decimal::Decimal>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, order_type, from_token, to_token, amount, limit_price, status, filled_amount, created_at, expires_at FROM public.limit_orders WHERE id = $1"
    ).bind(order_id).fetch_one(&state.pool).await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    success_response(LimitOrderResponse {
        order_id: order.0.to_string(),
        order_type: order.1,
        from_token: order.2,
        to_token: order.3,
        amount: order.4.to_string(),
        limit_price: order.5.to_string(),
        status: order.6,
        filled_amount: order.7.map(|f| f.to_string()),
        created_at: order.8.to_rfc3339(),
        expires_at: order.9.map(|e| e.to_rfc3339()),
        message: Some("Order cancelled".into()),
    })
}
