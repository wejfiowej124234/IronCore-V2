//! 用户API - 获取用户信息和KYC状态

use axum::{extract::State, Json};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    api::{middleware::jwt_extractor::JwtAuthContext, response::success_response},
    app_state::AppState,
    error::AppError,
};

/// 用户KYC状态响应
#[derive(Debug, Serialize, Deserialize)]
pub struct UserKycStatusResponse {
    pub kyc_status: String,
    pub daily_limit: f64,
    pub monthly_limit: f64,
    pub daily_used: f64,
    pub monthly_used: f64,
}

/// 用户信息响应
#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub id: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub kyc_status: String,
    pub created_at: String,
}

/// GET /api/v1/users/kyc/status - 获取用户KYC状态和额度
pub async fn get_kyc_status(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
) -> Result<Json<crate::api::response::ApiResponse<UserKycStatusResponse>>, AppError> {
    tracing::info!("[UserAPI] get_kyc_status: user_id={}", auth.user_id);

    // 1. 从数据库查询用户的KYC状态
    #[derive(sqlx::FromRow)]
    struct UserKycRow {
        kyc_status: Option<String>,
    }
    
    let user = sqlx::query_as::<_, UserKycRow>(
        "SELECT kyc_status FROM users WHERE id = $1"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("[UserAPI] Failed to fetch user kyc_status: {}", e);
        AppError::database_error(e.to_string())
    })?;

    let kyc_status = user.kyc_status.unwrap_or_else(|| "unverified".to_string()).to_lowercase();
    
    // 2. 企业级实现：根据KYC等级返回不同的限额
    let (daily_limit, monthly_limit) = match kyc_status.as_str() {
        "unverified" => (0.0, 0.0),
        "basic" => (1000.0, 5000.0),
        "standard" => (10000.0, 50000.0),
        "premium" => (100000.0, 500000.0),
        _ => (0.0, 0.0),
    };

    // 3. 计算今日已用额度（UTC时间，考虑到订单表可能还没有数据）
    let today_start = chrono::Utc::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(chrono::Utc)
        .unwrap();
    #[derive(sqlx::FromRow)]
    struct TotalRow {
        total: Option<rust_decimal::Decimal>,
    }
    
    let daily_used_result = sqlx::query_as::<_, TotalRow>(
        r#"
        SELECT COALESCE(SUM(fiat_amount), 0) as total
        FROM fiat_onramp_orders 
        WHERE user_id = $1 
          AND created_at >= $2
          AND status IN ('completed', 'processing', 'pending')
        "#
    )
    .bind(auth.user_id)
    .bind(today_start)
    .fetch_one(&state.pool)
    .await;

    let daily_used = match daily_used_result {
        Ok(row) => row.total
            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0),
        Err(e) => {
            tracing::warn!("[UserAPI] Failed to query daily usage, defaulting to 0: {}", e);
            0.0
        }
    };

    // 4. 计算本月已用额度
    let now = chrono::Utc::now();
    let month_start = chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_local_timezone(chrono::Utc)
        .unwrap();
    
    let monthly_used_result = sqlx::query_as::<_, TotalRow>(
        r#"
        SELECT COALESCE(SUM(fiat_amount), 0) as total
        FROM fiat_onramp_orders 
        WHERE user_id = $1 
          AND created_at >= $2
          AND status IN ('completed', 'processing', 'pending')
        "#
    )
    .bind(auth.user_id)
    .bind(month_start)
    .fetch_one(&state.pool)
    .await;

    let monthly_used = match monthly_used_result {
        Ok(row) => row.total
            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0),
        Err(e) => {
            tracing::warn!("[UserAPI] Failed to query monthly usage, defaulting to 0: {}", e);
            0.0
        }
    };

    let response = UserKycStatusResponse {
        kyc_status: kyc_status.clone(),
        daily_limit,
        monthly_limit,
        daily_used,
        monthly_used,
    };

    success_response(response)
}

/// GET /api/v1/users/me - 获取当前用户信息
pub async fn get_user_info(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
) -> Result<Json<crate::api::response::ApiResponse<UserInfoResponse>>, AppError> {
    tracing::info!("[UserAPI] get_user_info: user_id={}", auth.user_id);

    // 从数据库查询用户信息
    #[derive(sqlx::FromRow)]
    struct UserInfoRow {
        id: uuid::Uuid,
        email: String,
        phone: Option<String>,
        kyc_status: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
    }
    
    let user = sqlx::query_as::<_, UserInfoRow>(
        "SELECT id, email, phone, kyc_status, created_at FROM users WHERE id = $1"
    )
    .bind(auth.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("[UserAPI] Failed to fetch user: {}", e);
        AppError::database_error(e.to_string())
    })?;

    let response = UserInfoResponse {
        id: user.id.to_string(),
        email: Some(user.email),
        phone: Some(user.phone.unwrap_or_default()),
        kyc_status: user.kyc_status.unwrap_or_else(|| "unverified".to_string()),
        created_at: user.created_at.to_string(),
    };

    success_response(response)
}
