// Gas 费预估 API - EIP-1559 标准
// GET /api/gas/estimate?chain=ethereum&speed=normal
// GET /api/gas/estimate-all?chain=ethereum

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::{
    api::response::success_response,
    app_state::AppState,
    error::AppError,
    service::gas_estimator::{GasEstimate, GasEstimateResponse, GasSpeed},
};

/// 单速度预估请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EstimateGasQuery {
    pub chain: String,
    #[serde(default = "default_speed")]
    pub speed: GasSpeed,
}

fn default_speed() -> GasSpeed {
    GasSpeed::Normal
}

/// 批量预估请求
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct EstimateAllQuery {
    pub chain: String,
}

// 响应类型直接使用 GasEstimate 和 GasEstimateResponse，由 ApiResponse 包装

/// GET /api/gas/estimate?chain=ethereum&speed=normal
///
/// ✅多链Gas费用预估 支持:ETH/BSC/Polygon/Solana/BTC/TON
///
/// 参数：
/// - chain: ethereum/bsc/polygon/solana/bitcoin/ton
/// - speed: slow/normal/fast
///
/// 返回：
/// - base_fee: 基础费用（Wei，十六进制）
/// - max_priority_fee: 优先费用（Wei，十六进制）
/// - max_fee_per_gas: 最大费用（Wei，十六进制）
/// - estimated_time_seconds: 预计确认时间
/// - base_fee_gwei: 基础费用（Gwei，便于展示）
/// - max_priority_fee_gwei: 优先费用（Gwei）
/// - max_fee_per_gas_gwei: 最大费用（Gwei）
#[utoipa::path(
    get,
    path = "/api/gas/estimate",
    params(
        ("chain" = String, Query, description = "Chain name (ethereum/bsc/polygon)"),
        ("speed" = Option<GasSpeed>, Query, description = "Speed tier (slow/normal/fast), defaults to normal")
    ),
    responses(
        (status = 200, description = "Gas estimate successful", body = crate::api::response::ApiResponse<GasEstimate>),
        (status = 400, description = "Invalid parameters"),
        (status = 500, description = "RPC error or service unavailable")
    ),
    tag = "Gas Estimation"
)]
pub async fn estimate_gas(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EstimateGasQuery>,
) -> Result<Json<crate::api::response::ApiResponse<GasEstimate>>, AppError> {
    let chain_lower = query.chain.to_lowercase();
    if !is_supported_chain(&chain_lower) {
        return Err(AppError::bad_request(format!(
            "Unsupported: {}. Use: ETH/BSC/Polygon/Solana/BTC/TON",
            query.chain
        )));
    }

    tracing::info!(
        chain=%query.chain,
        speed=?query.speed,
        "estimating_gas"
    );

    // ✅ 企业级优化：使用 AppState 中的单例 GasEstimator，避免重复创建和配置读取
    let estimate = state
        .gas_estimator
        .estimate_gas(&query.chain, query.speed)
        .await
        .map_err(|e| {
            tracing::error!(error=%e, "gas_estimation_failed");
            AppError::internal(format!("Gas estimation failed: {}", e))
        })?;

    tracing::info!(
        chain=%query.chain,
        speed=?query.speed,
        max_fee_gwei=estimate.max_fee_per_gas_gwei,
        "gas_estimated"
    );

    success_response(estimate)
}

/// GET /api/gas/estimate-all?chain=ethereum
///
/// ✅批量Gas预估(全速度) 支持:ETH/BSC/Polygon/Solana/BTC/TON
///
/// 参数：
/// - chain: ethereum/bsc/polygon/solana/bitcoin/ton
///
/// 返回：
/// - slow: 慢速档位（10+ 分钟）
/// - normal: 正常档位（~3 分钟）
/// - fast: 快速档位（<1 分钟）
#[utoipa::path(
    get,
    path = "/api/gas/estimate-all",
    params(
        ("chain" = String, Query, description = "Chain name (ethereum/bsc/polygon)")
    ),
    responses(
        (status = 200, description = "Gas estimates successful", body = crate::api::response::ApiResponse<GasEstimateResponse>),
        (status = 400, description = "Invalid chain name"),
        (status = 500, description = "RPC error or service unavailable")
    ),
    tag = "Gas Estimation"
)]
pub async fn estimate_all_speeds(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EstimateAllQuery>,
) -> Result<Json<crate::api::response::ApiResponse<GasEstimateResponse>>, AppError> {
    // 验证链名称
    let chain_lower = query.chain.to_lowercase();
    if !is_supported_chain(&chain_lower) {
        return Err(AppError::bad_request(format!(
            "Unsupported chain: {}. Supported chains: ethereum, bsc, polygon, arbitrum, optimism, avalanche, solana, bitcoin, ton",
            query.chain
        )));
    }

    tracing::info!(chain=%query.chain, "estimating_all_speeds");

    // ✅ 企业级优化：使用 AppState 中的单例 GasEstimator，避免重复创建和配置读取
    let estimates = state
        .gas_estimator
        .estimate_all_speeds(&query.chain)
        .await
        .map_err(|e| {
            tracing::error!(error=%e, "batch_gas_estimation_failed");
            AppError::internal(format!("Gas estimation failed: {}", e))
        })?;

    tracing::info!(
        chain=%query.chain,
        slow_gwei=estimates.slow.max_fee_per_gas_gwei,
        normal_gwei=estimates.normal.max_fee_per_gas_gwei,
        fast_gwei=estimates.fast.max_fee_per_gas_gwei,
        "all_speeds_estimated"
    );

    success_response(estimates)
}

/// 验证是否为支持的链
/// 企业级实现：支持所有 EVM 链和非 EVM 链（生产环境）
fn is_supported_chain(chain: &str) -> bool {
    matches!(
        chain,
        // EVM 主链
        "ethereum" | "eth" | "bsc" | "binance" | "polygon" | "matic" |
        // EVM Layer 2
        "arbitrum" | "arb" | "optimism" | "op" | "avalanche" | "avax" |
        // 非EVM链
        "solana" | "sol" | "bitcoin" | "btc" | "ton"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_supported_chain() {
        // EVM链
        assert!(is_supported_chain("ethereum"));
        assert!(is_supported_chain("eth"));
        assert!(is_supported_chain("bsc"));
        assert!(is_supported_chain("polygon"));
        // 非EVM链
        assert!(is_supported_chain("solana"));
        assert!(is_supported_chain("sol"));
        assert!(is_supported_chain("bitcoin"));
        assert!(is_supported_chain("btc"));
        assert!(is_supported_chain("ton"));
        // 不支持的链
        assert!(!is_supported_chain("unknown"));
    }

    #[test]
    fn test_default_speed() {
        let speed = default_speed();
        assert!(matches!(speed, GasSpeed::Normal));
    }
}

// Routes
pub fn routes() -> axum::Router<Arc<crate::app_state::AppState>> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/estimate", post(estimate_gas))
        .route("/speeds", get(estimate_all_speeds))
}
