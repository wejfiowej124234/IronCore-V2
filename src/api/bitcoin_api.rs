//! Bitcoin API - Bitcoin特定功能
//! 提供Bitcoin链的特殊功能，如费率估算

use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use tracing::warn;

use crate::{api::response::success_response, app_state::AppState, error::AppError};

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
    // 生产级：从公开费率服务获取（默认 mempool.space，可用环境变量覆盖）
    // API: https://mempool.space/docs/api/rest#get-recommended-fees
    #[derive(Debug, Deserialize)]
    struct MempoolRecommendedFees {
        #[serde(rename = "fastestFee")]
        fastest_fee: u64,
        #[serde(rename = "halfHourFee")]
        half_hour_fee: u64,
        #[serde(rename = "hourFee")]
        hour_fee: u64,
        #[serde(rename = "economyFee")]
        economy_fee: Option<u64>,
        #[serde(rename = "minimumFee")]
        minimum_fee: u64,
    }

    let url = std::env::var("BTC_FEE_ESTIMATOR_URL")
        .unwrap_or_else(|_| "https://mempool.space/api/v1/fees/recommended".to_string());

    let estimate = match reqwest::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .and_then(|resp| resp.error_for_status())
    {
        Ok(resp) => match resp.json::<MempoolRecommendedFees>().await {
            Ok(fees) => BitcoinFeeEstimate {
                fast_fee: fees.fastest_fee,
                medium_fee: fees.half_hour_fee,
                slow_fee: fees.hour_fee,
                minimum_fee: fees.minimum_fee.max(fees.economy_fee.unwrap_or(1)),
            },
            Err(e) => {
                warn!("Failed to parse BTC fee estimates from {}: {:?}", url, e);
                BitcoinFeeEstimate {
                    fast_fee: 50,
                    medium_fee: 25,
                    slow_fee: 10,
                    minimum_fee: 1,
                }
            }
        },
        Err(e) => {
            warn!("Failed to fetch BTC fee estimates from {}: {:?}", url, e);
            BitcoinFeeEstimate {
                fast_fee: 50,
                medium_fee: 25,
                slow_fee: 10,
                minimum_fee: 1,
            }
        }
    };

    success_response(estimate)
}
