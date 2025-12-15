//! Feature Flags API - 功能开关管理
//! 允许前端查询后端功能是否启用

use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use utoipa::ToSchema;

use crate::{
    api::response::success_response,
    app_state::AppState,
    error::AppError,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Request/Response Models
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FeatureFlags {
    /// 功能开关映射 (feature_name -> enabled)
    pub features: HashMap<String, bool>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_features))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/v1/features
///
/// 获取后端功能开关状态
#[utoipa::path(
    get,
    path = "/api/v1/features",
    responses(
        (status = 200, description = "Feature flags retrieved", body = crate::api::response::ApiResponse<FeatureFlags>)
    ),
    tag = "System"
)]
pub async fn get_features(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<crate::api::response::ApiResponse<FeatureFlags>>, AppError> {
    let mut features = HashMap::new();
    
    // 核心功能（当前都已启用）
    features.insert("multi_chain_wallets".to_string(), true);
    features.insert("evm_chains".to_string(), true);
    features.insert("bitcoin_support".to_string(), true);
    features.insert("token_detection".to_string(), true);
    features.insert("gas_estimation".to_string(), true);
    features.insert("fiat_onramp".to_string(), true);
    features.insert("swap".to_string(), true);
    features.insert("bridge".to_string(), true);
    
    // 高级功能（部分启用）
    features.insert("audit_logs".to_string(), true);
    features.insert("reconciliation".to_string(), true);
    features.insert("withdrawal_review".to_string(), true);
    features.insert("webhooks".to_string(), true);
    features.insert("country_restrictions".to_string(), true);
    
    // 未来功能（暂未启用）
    features.insert("solana_support".to_string(), false);
    features.insert("cosmos_support".to_string(), false);
    features.insert("staking".to_string(), false);
    features.insert("nft_support".to_string(), false);

    success_response(FeatureFlags { features })
}
