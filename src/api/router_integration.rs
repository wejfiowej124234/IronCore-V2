//! API路由集成
//!
//! 集成所有新增的非托管API端点

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    api::{fiat_onramp_non_custodial, multi_chain_api, wallet_unlock_api},
    app_state::AppState,
};

/// 创建非托管钱包相关的受保护路由（✅ 企业级标准 V1）
pub fn create_non_custodial_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        // ✅ V1: 钱包解锁/锁定 API
        .route(
            "/api/v1/wallets/unlock",
            post(wallet_unlock_api::unlock_wallet),
        )
        .route("/api/v1/wallets/lock", post(wallet_unlock_api::lock_wallet))
        .route(
            "/api/v1/wallets/:wallet_id/unlock-status",
            get(wallet_unlock_api::get_unlock_status),
        )
        // ✅ V1: 多链钱包API
        .route(
            "/api/v1/wallets/batch",
            post(multi_chain_api::create_multi_chain_wallets),
        )
        // ✅ V1: 法币充值API
        .route(
            "/api/v1/fiat/onramp/orders",
            post(fiat_onramp_non_custodial::create_onramp_order),
        )
        .with_state(state)
}

/// 创建公开路由（✅ 企业级标准 V1）
pub fn create_public_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/chains", get(multi_chain_api::list_chains))
        .route(
            "/api/v1/chains/by-curve",
            get(multi_chain_api::list_chains_by_curve),
        )
        .with_state(state)
}
