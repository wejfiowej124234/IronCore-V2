//! 钱包解锁验证API（B项双锁机制增强）
//! 企业级实现：后端验证客户端已解锁钱包

use std::sync::Arc;

use axum::{extract::State, Json};
use chrono::{DateTime, Duration, Utc};
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
// 钱包解锁令牌（Wallet Unlock Token）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 解锁令牌请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUnlockTokenRequest {
    /// 钱包ID
    pub wallet_id: String,
    /// 解锁证明（客户端本地生成的随机字符串）
    pub unlock_proof: String,
    /// 解锁时间戳
    pub unlocked_at: i64,
}

/// 解锁令牌响应
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateUnlockTokenResponse {
    pub token: String,
    pub expires_at: String,
    pub wallet_id: String,
}

/// 验证解锁令牌请求
#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyUnlockTokenRequest {
    pub wallet_id: String,
    pub unlock_token: String,
}

/// 验证解锁令牌响应
#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyUnlockTokenResponse {
    pub valid: bool,
    pub wallet_id: String,
    pub remaining_seconds: Option<i64>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// API Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// POST /api/wallets/unlock-token/create
///
/// 创建钱包解锁令牌
///
/// # 双锁机制强化
/// - 锁1（账户锁）：JWT Token验证
/// - 锁2（钱包锁）：Unlock Token验证
///
/// # 流程
/// 1. 客户端使用钱包密码解锁钱包（本地）
/// 2. 客户端生成unlock_proof（随机字符串）
/// 3. 客户端请求后端创建unlock_token
/// 4. 后端返回有效期15分钟的token
/// 5. 客户端发送交易时携带unlock_token
/// 6. 后端验证unlock_token有效性
#[utoipa::path(
    post,
    path = "/api/wallets/unlock-token/create",
    request_body = CreateUnlockTokenRequest,
    responses(
        (status = 200, description = "Token created", body = ApiResponse<CreateUnlockTokenResponse>),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_unlock_token(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<CreateUnlockTokenRequest>,
) -> Result<Json<ApiResponse<CreateUnlockTokenResponse>>, AppError> {
    // 验证unlock_proof长度（防止暴力破解）
    if req.unlock_proof.len() < 32 {
        return Err(AppError::bad_request(
            "unlock_proof too short (min 32 chars)".to_string(),
        ));
    }

    // 生成unlock_token（服务端随机字符串）
    let unlock_token = generate_secure_token();
    let expires_at = Utc::now() + Duration::minutes(15);

    // 存储到数据库（或Redis）
    let _ = sqlx::query(
        "INSERT INTO wallet_unlock_tokens 
         (user_id, wallet_id, unlock_token, unlock_proof, expires_at, created_at)
         VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
         ON CONFLICT (user_id, wallet_id) 
         DO UPDATE SET unlock_token = $3, unlock_proof = $4, expires_at = $5, created_at = CURRENT_TIMESTAMP"
    )
    .bind(auth.user_id)
    .bind(&req.wallet_id)
    .bind(&unlock_token)
    .bind(&req.unlock_proof)
    .bind(expires_at)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    success_response(CreateUnlockTokenResponse {
        token: unlock_token,
        expires_at: expires_at.to_rfc3339(),
        wallet_id: req.wallet_id,
    })
}

/// POST /api/wallets/unlock-token/verify
///
/// 验证钱包解锁令牌
///
/// # 使用场景
/// 所有需要签名的操作（转账、跨链、提现等）都应验证unlock_token
#[utoipa::path(
    post,
    path = "/api/wallets/unlock-token/verify",
    request_body = VerifyUnlockTokenRequest,
    responses(
        (status = 200, description = "Token verified", body = ApiResponse<VerifyUnlockTokenResponse>),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(("bearer_auth" = []))
)]
pub async fn verify_unlock_token(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<VerifyUnlockTokenRequest>,
) -> Result<Json<ApiResponse<VerifyUnlockTokenResponse>>, AppError> {
    let result = sqlx::query_as::<_, (String, DateTime<Utc>)>(
        "SELECT unlock_token, expires_at FROM wallet_unlock_tokens
         WHERE user_id = $1 AND wallet_id = $2",
    )
    .bind(auth.user_id)
    .bind(&req.wallet_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    if let Some(row) = result {
        let now = Utc::now();
        let expires_at = row.1;

        let valid = row.0 == req.unlock_token && now < expires_at;
        let remaining_seconds = if valid {
            Some((expires_at - now).num_seconds())
        } else {
            None
        };

        success_response(VerifyUnlockTokenResponse {
            valid,
            wallet_id: req.wallet_id,
            remaining_seconds,
        })
    } else {
        success_response(VerifyUnlockTokenResponse {
            valid: false,
            wallet_id: req.wallet_id,
            remaining_seconds: None,
        })
    }
}

/// POST /api/wallets/unlock-token/refresh
///
/// 刷新解锁令牌（重置过期时间）
pub async fn refresh_unlock_token(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<VerifyUnlockTokenRequest>,
) -> Result<Json<ApiResponse<CreateUnlockTokenResponse>>, AppError> {
    let expires_at = Utc::now() + Duration::minutes(15);

    let _ = sqlx::query(
        "UPDATE wallet_unlock_tokens 
         SET expires_at = $1, updated_at = CURRENT_TIMESTAMP
         WHERE user_id = $2 AND wallet_id = $3 AND unlock_token = $4",
    )
    .bind(expires_at)
    .bind(auth.user_id)
    .bind(&req.wallet_id)
    .bind(&req.unlock_token)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    success_response(CreateUnlockTokenResponse {
        token: req.unlock_token,
        expires_at: expires_at.to_rfc3339(),
        wallet_id: req.wallet_id,
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 辅助函数
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 生成安全的随机令牌
fn generate_secure_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(random_bytes)
}
