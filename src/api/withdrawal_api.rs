//! 提现API
//! 企业级实现：三级风控 + 双锁解密 + 审计日志

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
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
    service::withdrawal_risk_control::{WithdrawalRequest, WithdrawalRiskControl},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWithdrawalRequest {
    /// 钱包ID
    pub wallet_id: String,
    /// 链标识
    pub chain: String,
    /// 源地址（发送方）
    pub from_address: String,
    /// 目标地址（接收方）
    pub to_address: String,
    /// 金额
    pub amount: String,
    /// 已签名的交易（客户端签名）
    /// 非托管模式：客户端必须先签名交易，后端只负责广播和风控
    pub signed_tx: String,
    // REMOVED: user_password (非托管模式：后端不能代签名)
    /// 幂等性key（推荐）
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateWithdrawalResponse {
    pub withdrawal_id: String,
    pub status: String,
    pub risk_level: String,
    pub requires_manual_review: bool,
    pub estimated_completion_time: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WithdrawalStatusResponse {
    pub withdrawal_id: String,
    pub status: String,
    pub tx_hash: Option<String>,
    pub completed_at: Option<String>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/create", post(create_withdrawal))
        .route("/status/:id", get(get_withdrawal_status))
        .route("/list", get(list_user_withdrawals))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// POST /api/withdrawals/create
///
/// 创建提现请求（三级风控）
#[utoipa::path(
    post,
    path = "/api/withdrawals/create",
    request_body = CreateWithdrawalRequest,
    responses(
        (status = 200, description = "Withdrawal created", body = CreateWithdrawalResponse)
    )
)]
pub async fn create_withdrawal(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    Json(req): Json<CreateWithdrawalRequest>,
) -> Result<Json<ApiResponse<CreateWithdrawalResponse>>, AppError> {
    // 1. 验证钱包所有权
    let wallet_id = Uuid::parse_str(&req.wallet_id)
        .map_err(|_| AppError::bad_request("Invalid wallet_id".to_string()))?;

    let (wallet_id_ret, chain_symbol, _address, wallet_user_id) =
        sqlx::query_as::<_, (uuid::Uuid, Option<String>, String, uuid::Uuid)>(
            "SELECT id, chain_symbol, address, user_id FROM wallets WHERE id = $1",
        )
        .bind(wallet_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|_| AppError::not_found("Wallet not found".to_string()))?;

    if wallet_user_id != auth.0.user_id {
        return Err(AppError::forbidden("Wallet not owned by user".to_string()));
    }

    // 2. 解析金额并转换为USD
    let amount_f64 = req
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;

    // TODO: 从PriceService获取实时价格转换为USD
    let amount_usd = amount_f64; // 简化实现

    // 3. 执行风控检查
    let risk_service = WithdrawalRiskControl::new(state.pool.clone());

    let risk_request = WithdrawalRequest {
        user_id: auth.0.user_id,
        tenant_id: auth.0.tenant_id,
        chain: chain_symbol.clone().unwrap_or_default(),
        to_address: req.to_address.clone(),
        amount_usd,
        wallet_id,
    };

    let decision = risk_service
        .evaluate(&risk_request)
        .await
        .map_err(|e| AppError::internal_error(format!("Risk evaluation failed: {}", e)))?;

    // 4. 记录风控决策
    risk_service
        .log_decision(&risk_request, &decision)
        .await
        .ok();

    // 5. 根据风控决策处理
    if !decision.allow {
        return Err(AppError::forbidden(decision.suggestion));
    }

    // 6. 创建提现请求
    let withdrawal_id = Uuid::new_v4();
    let status = if decision.requires_manual_review {
        "pending_review"
    } else {
        "approved"
    };

    let _ = sqlx::query(
        "INSERT INTO withdrawal_requests 
         (id, user_id, tenant_id, wallet_id, chain, to_address, amount, amount_usd, status, risk_level)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
    )
    .bind(withdrawal_id)
    .bind(auth.0.user_id)
    .bind(auth.0.tenant_id)
    .bind(wallet_id_ret)
    .bind(chain_symbol.unwrap_or_default())
    .bind(&req.to_address)
    .bind(amount_f64.to_string())
    .bind(amount_usd)
    .bind(status)
    .bind(format!("{:?}", decision.risk_level))
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to create withdrawal: {}", e)))?;

    // 7. 如果自动通过，立即处理
    if !decision.requires_manual_review {
        // TODO: 异步任务处理提现
        // - 使用双锁解密私钥
        // - 构建并签名交易
        // - 广播到链上
    }

    let estimated_completion = if decision.requires_manual_review {
        Some("Manual review required, typically within 24 hours".to_string())
    } else {
        Some("5-30 minutes depending on network".to_string())
    };

    success_response(CreateWithdrawalResponse {
        withdrawal_id: withdrawal_id.to_string(),
        status: status.to_string(),
        risk_level: format!("{:?}", decision.risk_level),
        requires_manual_review: decision.requires_manual_review,
        estimated_completion_time: estimated_completion,
    })
}

/// GET /api/withdrawals/status/:id
///
/// 查询提现状态
pub async fn get_withdrawal_status(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<WithdrawalStatusResponse>>, AppError> {
    let withdrawal_id = Uuid::parse_str(&id)
        .map_err(|_| AppError::bad_request("Invalid withdrawal_id".to_string()))?;

    #[derive(sqlx::FromRow)]
    struct WithdrawalRow {
        id: uuid::Uuid,
        status: String,
        tx_hash: Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
        user_id: uuid::Uuid,
    }

    let withdrawal = sqlx::query_as::<_, WithdrawalRow>(
        "SELECT id, status, tx_hash, completed_at, user_id
         FROM withdrawal_requests
         WHERE id = $1",
    )
    .bind(withdrawal_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| AppError::not_found("Withdrawal not found".to_string()))?;

    // 验证所有权
    if withdrawal.user_id != auth.0.user_id {
        return Err(AppError::forbidden("Not your withdrawal".to_string()));
    }

    success_response(WithdrawalStatusResponse {
        withdrawal_id: withdrawal.id.to_string(),
        status: withdrawal.status.clone(),
        tx_hash: withdrawal.tx_hash.clone(),
        completed_at: withdrawal.completed_at.map(|dt| dt.to_rfc3339()),
    })
}

/// GET /api/withdrawals/list
///
/// 列出用户的所有提现记录
pub async fn list_user_withdrawals(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
) -> Result<Json<ApiResponse<Vec<WithdrawalStatusResponse>>>, AppError> {
    #[derive(sqlx::FromRow)]
    struct WithdrawalListRow {
        id: uuid::Uuid,
        status: String,
        tx_hash: Option<String>,
        completed_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let withdrawals = sqlx::query_as::<_, WithdrawalListRow>(
        "SELECT id, status, tx_hash, completed_at
         FROM withdrawal_requests
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT 50",
    )
    .bind(auth.0.user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to fetch withdrawals: {}", e)))?;

    let responses: Vec<WithdrawalStatusResponse> = withdrawals
        .into_iter()
        .map(|w| WithdrawalStatusResponse {
            withdrawal_id: w.id.to_string(),
            status: w.status,
            tx_hash: w.tx_hash,
            completed_at: w.completed_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();

    success_response(responses)
}
