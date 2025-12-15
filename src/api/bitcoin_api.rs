//! Bitcoin API - Bitcoin特定功能
//! 提供Bitcoin链的特殊功能，如费率估算

use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
pub struct BitcoinFeeEstimate {
    /// 快速确认费率 (sat/vB) - 1-2个区块
    pub fast_fee: u64,
    /// 中等速度费率 (sat/vB) - 3-6个区块
    pub medium_fee: u64,
    /// 慢速费率 (sat/vB) - 6+个区块
    pub slow_fee: u64,
    /// 最低费率 (sat/vB)
    pub minimum_fee: u64,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new().route("/fee-estimates", get(get_fee_estimates))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/v1/bitcoin/fee-estimates
///
/// 获取Bitcoin网络当前费率估算
#[utoipa::path(
    get,
    path = "/api/v1/bitcoin/fee-estimates",
    responses(
        (status = 200, description = "Fee estimates retrieved", body = crate::api::response::ApiResponse<BitcoinFeeEstimate>)
    ),
    tag = "Bitcoin"
)]
pub async fn get_fee_estimates(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<crate::api::response::ApiResponse<BitcoinFeeEstimate>>, AppError> {
    // TODO: 实际实现中，应该从Bitcoin节点或费率估算服务获取
    // 目前返回合理的默认值
    let estimate = BitcoinFeeEstimate {
        fast_fee: 50,      // ~10分钟确认
        medium_fee: 25,    // ~30分钟确认
        slow_fee: 10,      // ~1小时确认
        minimum_fee: 1,    // 最低中继费率
    };

    success_response(estimate)
}
