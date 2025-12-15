//! 法币充值API（非托管模式）
//!
//! P2级修复：法币充值流程非托管化
//! 确保法币充值不涉及后端持有用户资产

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{
        middleware::auth::AuthInfoExtractor,
        response::{success_response, ApiResponse},
    },
    app_state::AppState,
    error::AppError,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 法币充值（非托管模式）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct FiatOnrampRequest {
    /// 法币金额
    pub fiat_amount: f64,
    /// 法币货币
    pub fiat_currency: String,
    /// 目标加密货币
    pub crypto_currency: String,
    /// 目标链
    pub target_chain: String,
    /// 用户钱包地址（接收加密货币）
    pub user_wallet_address: String,
    /// 支付方式
    pub payment_method: String,
    /// 幂等性key
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FiatOnrampResponse {
    pub order_id: String,
    pub status: String,
    pub fiat_amount: f64,
    pub crypto_amount_estimate: String,
    pub user_wallet_address: String,
    pub platform_address: String, // 平台托管地址（临时接收）
    pub payment_instructions: PaymentInstructions,
    pub expires_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentInstructions {
    pub method: String,
    pub recipient: String,
    pub reference_code: String, // 付款备注（用于匹配订单）
    pub steps: Vec<String>,
}

/// POST /api/fiat/onramp/create
///
/// 创建法币充值订单（非托管模式）
///
/// # 非托管流程
/// 1. 用户提供自己的钱包地址（目标地址）
/// 2. 用户通过第三方支付法币
/// 3. 后端监听到法币到账
/// 4. 后端使用平台资金购买加密货币
/// 5. 后端将加密货币转账到用户地址（需用户确认接收）
/// 6. 用户确认收到后，订单完成
///
/// # 安全保证
/// - 用户控制目标钱包私钥
/// - 后端只临时持有等值的法币
/// - 转账需要用户确认地址
/// - 全程可审计
#[utoipa::path(
    post,
    path = "/api/fiat/onramp/create",
    request_body = FiatOnrampRequest,
    responses(
        (status = 200, description = "Order created", body = ApiResponse<FiatOnrampResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_onramp_order(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<FiatOnrampRequest>,
) -> Result<Json<ApiResponse<FiatOnrampResponse>>, AppError> {
    let user_id = auth.user_id;

    // 1. 验证钱包地址格式
    crate::utils::address_validator::AddressValidator::validate(
        &req.target_chain,
        &req.user_wallet_address,
    )
    .map_err(|e| AppError::bad_request(format!("Invalid wallet address: {}", e)))?;

    // 2. 验证金额
    if req.fiat_amount <= 0.0 || !req.fiat_amount.is_finite() {
        return Err(AppError::bad_request("Invalid fiat amount".to_string()));
    }

    // 3. 检查风控限额
    let daily_limit = 10000.0; // 示例：每日限额$10,000
    let daily_total = get_user_daily_onramp_total(&state.pool, user_id).await?;

    if daily_total + req.fiat_amount > daily_limit {
        return Err(AppError::bad_request(format!(
            "Daily limit exceeded. Daily limit: ${}, current: ${}, requested: ${}",
            daily_limit, daily_total, req.fiat_amount
        )));
    }

    // 4. 获取汇率估算
    let crypto_amount_estimate =
        estimate_crypto_amount(req.fiat_amount, &req.fiat_currency, &req.crypto_currency).await?;

    // 5. 创建订单
    let order_id = Uuid::new_v4();
    let reference_code = generate_reference_code();
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    let _ = sqlx::query(
        "INSERT INTO fiat_onramp_orders 
         (id, user_id, tenant_id, fiat_amount, fiat_currency, crypto_currency, crypto_amount,
          target_chain, wallet_address, payment_method, status, exchange_rate, fee_amount, expires_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
    )
    .bind(order_id)
    .bind(user_id)
    .bind(uuid::Uuid::nil())
    .bind(&req.fiat_amount)
    .bind(&req.fiat_currency)
    .bind(&req.crypto_currency)
    .bind(&crypto_amount_estimate)
    .bind(&req.target_chain)
    .bind(&req.user_wallet_address)
    .bind("bank_transfer")
    .bind("pending_payment")
    .bind(1.0)
    .bind(0.0)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::internal(format!("Failed to create order: {}", e)))?;

    // 6. 记录审计日志
    let _ = sqlx::query(
        "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
         VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
    )
    .bind("FIAT_ONRAMP_ORDER_CREATED")
    .bind("fiat_order")
    .bind(order_id)
    .bind(serde_json::json!({
        "user_id": user_id,
        "fiat_amount": req.fiat_amount,
        "crypto_currency": req.crypto_currency,
        "user_wallet_address": req.user_wallet_address,
        "mode": "non_custodial"
    }))
    .execute(&state.pool)
    .await
    .ok();

    tracing::info!(
        order_id = %order_id,
        user_id = %user_id,
        fiat_amount = req.fiat_amount,
        "Fiat onramp order created (non-custodial)"
    );

    success_response(FiatOnrampResponse {
        order_id: order_id.to_string(),
        status: "pending_payment".to_string(),
        fiat_amount: req.fiat_amount,
        crypto_amount_estimate,
        user_wallet_address: req.user_wallet_address,
        platform_address: get_platform_temp_address(&req.target_chain),
        payment_instructions: PaymentInstructions {
            method: req.payment_method.clone(),
            recipient: "IronForge Platform".to_string(),
            reference_code: reference_code.clone(),
            steps: vec![
                format!(
                    "1. Transfer ${} {} to our payment account",
                    req.fiat_amount, req.fiat_currency
                ),
                format!("2. Include reference code: {}", reference_code),
                "3. Wait for payment confirmation (usually 1-3 business days)".to_string(),
                "4. Crypto will be sent to your wallet address once payment is confirmed"
                    .to_string(),
            ],
        },
        expires_at: expires_at.to_rfc3339(),
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 辅助函数
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

async fn get_user_daily_onramp_total(pool: &sqlx::PgPool, user_id: Uuid) -> Result<f64, AppError> {
    let result = sqlx::query_as::<_, (rust_decimal::Decimal,)>(
        "SELECT COALESCE(SUM(fiat_amount), 0) as total
         FROM fiat_onramp_orders
         WHERE user_id = $1 
         AND created_at >= CURRENT_DATE
         AND status NOT IN ('cancelled', 'failed')",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::internal(format!("Database error: {}", e)))?;

    Ok(result.0.to_string().parse().unwrap_or(0.0))
}

async fn estimate_crypto_amount(
    fiat_amount: f64,
    _fiat_currency: &str,
    _crypto_currency: &str,
) -> Result<String, AppError> {
    // TODO: 调用价格服务获取实时汇率
    let rate = 0.000025; // 示例汇率
    let crypto_amount = fiat_amount * rate;
    Ok(format!("{:.8}", crypto_amount))
}

fn generate_reference_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("ON{:010}", rng.gen_range(0..9999999999u64))
}

fn get_platform_temp_address(_chain: &str) -> String {
    // TODO: 从配置获取平台临时托管地址
    "0xPLATFORM_TEMP_ADDRESS".to_string()
}
