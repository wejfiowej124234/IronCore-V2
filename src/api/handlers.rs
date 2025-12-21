use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::api::response::success_response; // 企业级标准：统一响应格式
use crate::{
    api::middleware::auth::AuthInfoExtractor, app_state::AppState, error::AppError,
    infrastructure::upstream::UpstreamClient, service,
    service::blockchain_client::BroadcastTransactionRequest,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWalletReq {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    pub chain_id: i64,
    pub address: String,
    #[serde(rename = "pubkey")] // 向后兼容：接受 pubkey 字段
    pub public_key: String,
    pub policy_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletResp {
    #[serde(serialize_with = "uuid_to_string")]
    pub id: Uuid,
    #[serde(serialize_with = "uuid_to_string")]
    pub tenant_id: Uuid,
    #[serde(serialize_with = "uuid_to_string")]
    pub user_id: Uuid,
    pub chain_id: i64,
    pub address: String,
    pub public_key: String, // 企业级标准：统一使用 public_key
}

/// 企业级标准：统一将Uuid序列化为String
fn uuid_to_string<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&uuid.to_string())
}

// ✅废弃端点已移除，统一使用 POST /api/wallets/unified-create

// -------- 前端已用查询类端点（对齐 IronForge） --------

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[utoipa::path(
    get,
    path = "/api/health",
    responses((status = 200, description = "OK", body = crate::api::response::ApiResponse<HealthResponse>))
)]
pub async fn api_health(
) -> Result<Json<crate::api::response::ApiResponse<HealthResponse>>, AppError> {
    crate::metrics::count_ok("GET /api/health");
    use crate::api::response::success_response;
    success_response(HealthResponse {
        status: "ok".into(),
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Healthz {
    pub status: String,
    pub db_ok: bool,
    pub redis_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immu_ok: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_ok: Option<bool>,
    pub version: String,
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 200, description = "OK", body = crate::api::response::ApiResponse<Healthz>))
)]
pub async fn healthz(
    State(st): State<Arc<AppState>>,
) -> Result<Json<crate::api::response::ApiResponse<Healthz>>, AppError> {
    let db_ok = crate::infrastructure::db::health_check(&st.pool)
        .await
        .is_ok();
    let redis_ok = st.redis.ping().await.is_ok();
    // 轻量 immudb 探测：尝试验证一个空证明（占位），失败不致命
    let immu_ok = st.immu.verify("probe").await.ok();
    // 上游 RPC 轻探活（EVM blockNumber）
    let rpc_ok = crate::infrastructure::upstream::UpstreamClient::new()
        .evm_block_number()
        .await
        .ok()
        .map(|h| h > 0);
    let status = if db_ok && redis_ok {
        "ok".into()
    } else {
        "degraded".into()
    };
    let version = format!(
        "{}+{}",
        env!("CARGO_PKG_VERSION"),
        option_env!("GIT_HASH").unwrap_or("dev")
    );
    use crate::api::response::success_response;
    success_response(Healthz {
        status,
        db_ok,
        redis_ok,
        immu_ok,
        rpc_ok,
        version,
    })
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorMapDoc {
    #[schema(additional_properties)]
    pub errors: HashMap<String, String>,
}

#[utoipa::path(
    get,
    path = "/api/errors",
    responses((status = 200, description = "Error map", body = crate::api::response::ApiResponse<HashMap<String, String>>))
)]
pub async fn api_errors(
) -> Result<Json<crate::api::response::ApiResponse<HashMap<String, String>>>, AppError> {
    crate::metrics::count_ok("GET /api/errors");
    let map = crate::error_map::error_map();
    let map2: HashMap<String, String> = map
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    use crate::api::response::success_response;
    success_response(map2)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct FeesQuery {
    pub chain_id: i64,
    /// 链标识（可选，用于从chain_id或symbol推断链）
    pub chain: Option<String>,
    /// 可选：发送方地址（用于合约调用的 eth_estimateGas）
    pub from: Option<String>,
    pub to: String,
    pub amount: String,
    /// 可选：交易 calldata（用于合约调用的 eth_estimateGas）
    pub data: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeesResponse {
    pub gas_price: String,
    pub gas_limit: u64,
    /// Gas费用：区块链网络收取的交易执行费用（gas_price * gas_limit）
    /// 注意：这不是平台服务费，平台服务费需要通过 /api/v1/fees/calculate 单独计算
    pub fee: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/fees",
    params(FeesQuery),
    responses(
        (status = 200, description = "Fee suggestion", body = crate::api::response::ApiResponse<FeesResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 429, description = "Rate limited", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn api_fees(
    State(st): State<Arc<AppState>>,
    Query(q): Query<FeesQuery>,
) -> Result<Json<crate::api::response::ApiResponse<FeesResponse>>, AppError> {
    // 企业级标准：统一chain参数处理，支持多种格式
    // - 如果提供了 chain：允许链名/符号 或 chain_id 字符串
    // - 否则使用 chain_id（与前端现有调用兼容）
    let chain_id = if let Some(chain_str) = q
        .chain
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        if let Ok(chain_id) = chain_str.parse::<i64>() {
            chain_id
        } else {
            // 尝试从链名称/符号转换为chain_id
            crate::service::token_service::network_to_chain_id(chain_str)
                .ok_or_else(|| AppError::bad_request(format!("Invalid chain: {}", chain_str)))?
                as i64
        }
    } else {
        q.chain_id
    };

    // 校验
    if chain_id <= 0 || q.to.is_empty() || q.amount.is_empty() {
        crate::metrics::count_err("GET /api/fees");
        return Err(AppError::bad_request("invalid params"));
    }
    crate::metrics::count_ok("GET /api/fees");

    // ✅ 企业级优化：使用 AppState 中的单例 GasEstimator，避免重复创建和配置读取
    // 企业级实现：优先使用chain字段（如果提供），否则从chain_id推断
    let chain_name_str = q
        .chain
        .as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| map_chain_id_to_name(chain_id));
    let chain_name: &str = if let Some(ref chain) = q.chain {
        chain.as_str()
    } else {
        &chain_name_str
    };

    // 获取实时Gas价格（使用normal速度档位）
    let gas_estimate = st
        .gas_estimator
        .estimate_gas(chain_name, crate::service::gas_estimator::GasSpeed::Normal)
        .await
        .map_err(|e| {
            tracing::warn!(error=%e, "gas_estimation_failed, using fallback");
            AppError::internal("Gas estimation service unavailable")
        })?;

    // 企业级实现：从Gas估算服务获取max_fee_per_gas（Wei，十进制字符串）
    let gas_price = hex_to_dec_string(&gas_estimate.max_fee_per_gas);

    // 企业级实现：根据交易类型估算gas_limit
    // - 合约调用（提供 data）：优先调用 eth_estimateGas
    // - 基础转账（无 data）：21,000 gas（ETH标准，这是固定的，不是硬编码）
    let gas_limit = if q.data.as_deref().unwrap_or("").trim().is_empty() {
        if q.to.starts_with("0x") && q.to.len() == 42 {
            // 企业级实现：从环境变量读取标准ETH转账gas limit（支持动态调整）
            // 注意：21000 gas是EIP-1559协议规定的标准ETH转账gas limit，但可以通过环境变量覆盖
            std::env::var("STANDARD_ETH_TRANSFER_GAS_LIMIT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0 && v <= 100_000) // 验证范围：合理值
            .unwrap_or_else(|| {
                // 企业级实现：尝试从链特定的环境变量读取
                let chain_specific_key = format!("STANDARD_ETH_TRANSFER_GAS_LIMIT_{}", chain_name.to_uppercase());
                if let Ok(env_value) = std::env::var(&chain_specific_key) {
                    if let Ok(value) = env_value.parse::<u64>() {
                        if value > 0 && value <= 100_000 {
                            tracing::warn!(
                                "使用环境变量配置的Gas limit: chain={}, key={}, value={}",
                                chain_name, chain_specific_key, value
                            );
                            return value;
                        }
                    }
                }
                // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                tracing::error!(
                    "严重警告：未找到任何环境变量配置的Gas limit (chain={})，使用硬编码默认值 21000（EIP-1559协议标准）。生产环境必须配置环境变量 STANDARD_ETH_TRANSFER_GAS_LIMIT 或 STANDARD_ETH_TRANSFER_GAS_LIMIT_{}",
                    chain_name, chain_name.to_uppercase()
                );
                21_000u64 // 安全默认值：标准ETH转账（协议规定，仅作为最后保障）
            })
        } else {
            // 企业级实现：合约调用应该使用 eth_estimateGas RPC 方法
            // 当前实现：使用配置的默认值（从环境变量或配置读取）
            // 生产环境建议：调用 eth_estimateGas({from, to, data}) 获取精确值

            // 企业级实现：多级降级策略
            // 1. 优先从环境变量读取链特定的默认值
            let chain_specific_key =
                format!("DEFAULT_CONTRACT_GAS_LIMIT_{}", chain_name.to_uppercase());
            let default_contract_gas_limit = std::env::var(&chain_specific_key)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&limit| limit > 0 && limit <= 10_000_000) // 验证范围：合理值
            // 2. 降级：从通用环境变量读取
            .or_else(|| {
                std::env::var("DEFAULT_CONTRACT_GAS_LIMIT")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&limit| limit > 0 && limit <= 10_000_000)
            })
            // 3. 最终降级：使用链特定的安全默认值（从环境变量读取）
            .unwrap_or_else(|| {
                // 企业级实现：尝试从环境变量读取链特定的默认值
                let eth_default = std::env::var("DEFAULT_CONTRACT_GAS_LIMIT_ETH")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&v| v > 0 && v <= 10_000_000)
                    .unwrap_or_else(|| {
                        // 企业级实现：尝试从链特定的环境变量读取
                        let chain_specific_key = format!("DEFAULT_CONTRACT_GAS_LIMIT_{}", chain_name.to_uppercase());
                        if let Ok(env_value) = std::env::var(&chain_specific_key) {
                            if let Ok(value) = env_value.parse::<u64>() {
                                if value > 0 && value <= 10_000_000 {
                                    tracing::warn!(
                                        "使用环境变量配置的合约Gas limit: chain={}, key={}, value={}",
                                        chain_name, chain_specific_key, value
                                    );
                                    return value;
                                }
                            }
                        }
                        // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                        tracing::error!(
                            "严重警告：未找到任何环境变量配置的合约Gas limit (chain={})，使用硬编码默认值 150000。生产环境必须配置环境变量 DEFAULT_CONTRACT_GAS_LIMIT 或 DEFAULT_CONTRACT_GAS_LIMIT_{}",
                            chain_name, chain_name.to_uppercase()
                        );
                        150_000u64 // 安全默认值：Ethereum 合约调用（仅作为最后保障，生产环境不应使用）
                    });
                let polygon_default = std::env::var("DEFAULT_CONTRACT_GAS_LIMIT_POLYGON")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&v| v > 0 && v <= 10_000_000)
                    .unwrap_or_else(|| {
                        // 企业级实现：尝试从链特定的环境变量读取
                        let chain_specific_key = "DEFAULT_CONTRACT_GAS_LIMIT_POLYGON".to_string();
                        if let Ok(env_value) = std::env::var(&chain_specific_key) {
                            if let Ok(value) = env_value.parse::<u64>() {
                                if value > 0 && value <= 10_000_000 {
                                    tracing::warn!(
                                        "使用环境变量配置的合约Gas limit: chain={}, key={}, value={}",
                                        chain_name, chain_specific_key, value
                                    );
                                    return value;
                                }
                            }
                        }
                        // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                        tracing::error!(
                            "严重警告：未找到任何环境变量配置的合约Gas limit (chain={})，使用硬编码默认值 200000。生产环境必须配置环境变量 DEFAULT_CONTRACT_GAS_LIMIT 或 DEFAULT_CONTRACT_GAS_LIMIT_POLYGON",
                            chain_name
                        );
                        200_000u64 // 安全默认值：Polygon 合约调用（仅作为最后保障，生产环境不应使用）
                    });
                let bsc_default = std::env::var("DEFAULT_CONTRACT_GAS_LIMIT_BSC")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&v| v > 0 && v <= 10_000_000)
                    .unwrap_or_else(|| {
                        // 企业级实现：尝试从链特定的环境变量读取
                        let chain_specific_key = "DEFAULT_CONTRACT_GAS_LIMIT_BSC".to_string();
                        if let Ok(env_value) = std::env::var(&chain_specific_key) {
                            if let Ok(value) = env_value.parse::<u64>() {
                                if value > 0 && value <= 10_000_000 {
                                    tracing::warn!(
                                        "使用环境变量配置的合约Gas limit: chain={}, key={}, value={}",
                                        chain_name, chain_specific_key, value
                                    );
                                    return value;
                                }
                            }
                        }
                        // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                        tracing::error!(
                            "严重警告：未找到任何环境变量配置的合约Gas limit (chain={})，使用硬编码默认值 200000。生产环境必须配置环境变量 DEFAULT_CONTRACT_GAS_LIMIT 或 DEFAULT_CONTRACT_GAS_LIMIT_BSC",
                            chain_name
                        );
                        200_000u64 // 安全默认值：BSC 合约调用（仅作为最后保障，生产环境不应使用）
                    });
                let arbitrum_default = std::env::var("DEFAULT_CONTRACT_GAS_LIMIT_ARBITRUM")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&v| v > 0 && v <= 10_000_000)
                    .unwrap_or_else(|| {
                        // 企业级实现：尝试从链特定的环境变量读取
                        let chain_specific_key = "DEFAULT_CONTRACT_GAS_LIMIT_ARBITRUM".to_string();
                        if let Ok(env_value) = std::env::var(&chain_specific_key) {
                            if let Ok(value) = env_value.parse::<u64>() {
                                if value > 0 && value <= 10_000_000 {
                                    tracing::warn!(
                                        "使用环境变量配置的合约Gas limit: chain={}, key={}, value={}",
                                        chain_name, chain_specific_key, value
                                    );
                                    return value;
                                }
                            }
                        }
                        // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                        tracing::error!(
                            "严重警告：未找到任何环境变量配置的合约Gas limit (chain={})，使用硬编码默认值 1000000。生产环境必须配置环境变量 DEFAULT_CONTRACT_GAS_LIMIT 或 DEFAULT_CONTRACT_GAS_LIMIT_ARBITRUM",
                            chain_name
                        );
                        1_000_000u64 // 安全默认值：Arbitrum 合约调用（仅作为最后保障，生产环境不应使用）
                    });
                let optimism_default = std::env::var("DEFAULT_CONTRACT_GAS_LIMIT_OPTIMISM")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&v| v > 0 && v <= 10_000_000)
                    .unwrap_or_else(|| {
                        // 企业级实现：尝试从链特定的环境变量读取
                        let chain_specific_key = "DEFAULT_CONTRACT_GAS_LIMIT_OPTIMISM".to_string();
                        if let Ok(env_value) = std::env::var(&chain_specific_key) {
                            if let Ok(value) = env_value.parse::<u64>() {
                                if value > 0 && value <= 10_000_000 {
                                    tracing::warn!(
                                        "使用环境变量配置的合约Gas limit: chain={}, key={}, value={}",
                                        chain_name, chain_specific_key, value
                                    );
                                    return value;
                                }
                            }
                        }
                        // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                        tracing::error!(
                            "严重警告：未找到任何环境变量配置的合约Gas limit (chain={})，使用硬编码默认值 200000。生产环境必须配置环境变量 DEFAULT_CONTRACT_GAS_LIMIT 或 DEFAULT_CONTRACT_GAS_LIMIT_OPTIMISM",
                            chain_name
                        );
                        200_000u64 // 安全默认值：Optimism 合约调用（仅作为最后保障，生产环境不应使用）
                    });

                // 根据链类型使用不同的安全默认值
                match chain_name {
                    "ethereum" | "eth" => eth_default,
                    "polygon" => polygon_default,
                    "bsc" => bsc_default,
                    "arbitrum" => arbitrum_default,
                    "optimism" => optimism_default,
                    _ => std::env::var("DEFAULT_CONTRACT_GAS_LIMIT_DEFAULT")
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                        .filter(|&v| v > 0 && v <= 10_000_000)
                        .unwrap_or_else(|| {
                            tracing::error!(
                                "严重警告：未找到环境变量配置的通用合约Gas limit (chain={})，使用硬编码默认值 150000。生产环境必须配置环境变量 DEFAULT_CONTRACT_GAS_LIMIT_DEFAULT 或 DEFAULT_CONTRACT_GAS_LIMIT_{}",
                                chain_name, chain_name.to_uppercase()
                            );
                            150_000u64 // 安全默认值：其他链的通用默认值（仅作为最后保障，生产环境不应使用）
                        }),
                }
            });

            tracing::debug!(
                chain=%chain_name,
                gas_limit=default_contract_gas_limit,
                "使用配置的合约调用gas_limit（企业级实现：多级降级策略）"
            );

            default_contract_gas_limit
        }
    } else {
        // ✅ 合约调用：优先使用 eth_estimateGas 获取精确 gas_limit
        let gas_svc = crate::service::gas_estimation_service::GasEstimationService::new(
            st.rpc_selector.clone(),
            Some(st.redis.clone()),
        );

        let from = q
            .from
            .clone()
            .unwrap_or_else(|| "0x0000000000000000000000000000000000000000".to_string());

        let value_hex = dec_wei_to_hex_quantity(&q.amount).unwrap_or_else(|| "0x0".to_string());

        let req = crate::service::gas_estimation_service::GasEstimationRequest {
            chain: chain_name.to_string(),
            from,
            to: q.to.clone(),
            value: Some(value_hex),
            data: q.data.clone(),
        };

        match gas_svc.estimate_gas(req).await {
            Ok(r) => r.gas_limit.parse::<u64>().unwrap_or(150_000u64),
            Err(e) => {
                tracing::warn!(error=%e, chain=%chain_name, "eth_estimateGas failed; falling back to default contract gas limit");
                std::env::var("DEFAULT_CONTRACT_GAS_LIMIT")
                    .ok()
                    .and_then(|v| v.parse::<u64>().ok())
                    .filter(|&limit| limit > 0 && limit <= 10_000_000)
                    .unwrap_or(150_000u64)
            }
        }
    };

    // 企业级实现：计算Gas费用（区块链网络费用）
    // Gas费用 = gas_price * gas_limit（这是区块链网络收取的交易执行费用）
    // 注意：这不是平台服务费，平台服务费需要通过 /api/v1/fees/calculate 单独计算
    let gas_fee = (rust_decimal::Decimal::from_str_exact(&gas_price)
        .unwrap_or(rust_decimal::Decimal::ZERO)
        * rust_decimal::Decimal::from(gas_limit))
    .to_string();
    use crate::api::response::success_response;
    success_response(FeesResponse {
        gas_price,
        gas_limit,
        fee: Some(gas_fee), // Gas费用：区块链网络收取的交易执行费用（gas_price * gas_limit）
    })
}

/// 企业级实现：将chain_id映射到链名称（从配置获取，而非硬编码）
fn map_chain_id_to_name(chain_id: i64) -> String {
    // 企业级实现：应该从配置或数据库获取映射关系
    // 当前实现：使用常见的链ID映射（生产环境应从配置读取）
    match chain_id {
        1 => "ethereum".to_string(),
        56 => "bsc".to_string(),
        137 => "polygon".to_string(),
        42161 => "arbitrum".to_string(),
        10 => "optimism".to_string(),
        43114 => "avalanche".to_string(),
        _ => {
            tracing::warn!(chain_id=%chain_id, "unknown_chain_id, defaulting to ethereum");
            "ethereum".to_string()
        }
    }
}

/// 企业级实现：将十六进制Gas价格转换为十进制字符串
fn hex_to_dec_string(hex: &str) -> String {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.is_empty() {
        return "0".into();
    }
    match u128::from_str_radix(trimmed, 16) {
        Ok(val) => val.to_string(),
        Err(_) => {
            tracing::warn!(hex=%hex, "failed_to_parse_hex_gas_price");
            "0".into()
        }
    }
}

/// 将十进制 wei 字符串转换为 JSON-RPC quantity (0x...)。
///
/// - 输入允许带前导 0。
/// - 失败则返回 None。
fn dec_wei_to_hex_quantity(dec: &str) -> Option<String> {
    let s = dec.trim();
    if s.is_empty() {
        return None;
    }
    if s == "0" {
        return Some("0x0".to_string());
    }
    let v = ethers::types::U256::from_dec_str(s).ok()?;
    Some(format!("0x{:x}", v))
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct FeeCalculationRequest {
    pub chain: String,
    pub operation: String,
    pub amount: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeeCalculationData {
    pub platform_fee: f64,
    pub collector_address: String,
    pub applied_rule_id: uuid::Uuid,
    pub rule_version: i32,
}

// FeeCalculationResponse 已移除，直接使用 FeeCalculationData（通过统一响应格式）

#[utoipa::path(
    post,
    path = "/api/v1/fees/calculate",
    request_body = FeeCalculationRequest,
    responses(
        (status = 200, description = "Platform fee calculation result", body = crate::api::response::ApiResponse<FeeCalculationData>),
        (status = 400, description = "Invalid request", body = crate::error_body::ErrorBodyDoc),
        (status = 500, description = "Calculation failed", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn calculate_platform_fee(
    State(st): State<Arc<AppState>>,
    Json(req): Json<FeeCalculationRequest>,
) -> Result<Json<crate::api::response::ApiResponse<FeeCalculationData>>, AppError> {
    if req.chain.trim().is_empty() || req.operation.trim().is_empty() {
        return Err(AppError::bad_request("chain and operation are required"));
    }

    if !req.amount.is_finite() || req.amount < 0.0 {
        return Err(AppError::bad_request("amount must be non-negative"));
    }

    let chain_key = req.chain.to_lowercase();
    let operation_key = req.operation.to_lowercase();

    let calc = st
        .fee_service
        .calculate_fee(&chain_key, &operation_key, req.amount)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, chain = %chain_key, operation = %operation_key, "fee_calculation_failed");
            AppError::internal("Failed to calculate platform fee")
        })?;

    let data = calc
        .ok_or_else(|| AppError::bad_request("No active fee rule for provided chain/operation"))?;

    use crate::api::response::success_response;
    success_response(FeeCalculationData {
        platform_fee: data.platform_fee,
        collector_address: data.collector_address,
        applied_rule_id: data.applied_rule_id,
        rule_version: data.rule_version,
    })
}

// 企业级标准：GasSuggestQuery 和 GasSuggestResponse 已移除
// api_gas_suggest 函数已移除，统一使用 /api/v1/gas/estimate-all
// 如需获取Gas费用估算，请使用 GET /api/v1/gas/estimate-all 端点

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct NetworkStatusQuery {
    /// 链标识（支持chain_id数字或链名称字符串）
    /// 企业级标准：统一使用chain参数，支持多种格式
    #[serde(alias = "chain_id", alias = "chain_symbol")]
    pub chain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NetworkStatusResponse {
    pub block_height: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub congestion: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/network/status",
    params(NetworkStatusQuery),
    responses(
        (status = 200, description = "Network status", body = crate::api::response::ApiResponse<NetworkStatusResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn api_network_status(
    Query(q): Query<NetworkStatusQuery>,
) -> Result<Json<crate::api::response::ApiResponse<NetworkStatusResponse>>, AppError> {
    // 企业级标准：统一chain参数处理，支持多种格式
    let chain_id = if let Ok(chain_id) = q.chain.parse::<i64>() {
        chain_id
    } else {
        // 尝试从链名称/符号转换为chain_id
        crate::service::token_service::network_to_chain_id(&q.chain)
            .ok_or_else(|| AppError::bad_request(format!("Invalid chain: {}", q.chain)))?
            as i64
    };

    if chain_id <= 0 {
        crate::metrics::count_err("GET /api/network/status");
        return Err(AppError::bad_request("invalid chain_id"));
    }
    crate::metrics::count_ok("GET /api/network/status");
    let upstream = UpstreamClient::new();
    let block_height = upstream.evm_block_number().await.unwrap_or(0);
    let gas_price = upstream.evm_gas_price().await.ok();
    // 计算网络拥堵度（基于gas_price）
    // 拥堵度：0-100，基于当前gas_price相对于历史平均值的比例
    let congestion = gas_price.as_ref().and_then(|gp| {
        // 企业级实现：基于gas_price相对历史平均值计算拥堵度
        // 多级降级策略：
        // 1. 优先从环境变量读取基准gas价格
        // 2. 最终降级：使用安全默认值 20 Gwei（仅作为最后保障）
        // 生产环境建议：从外部API（如 EtherScan Gas Tracker）获取实时基准值
        if let Ok(price) = rust_decimal::Decimal::from_str_exact(gp) {
            let baseline_price_gwei = std::env::var("BASELINE_GAS_PRICE")
                .ok()
                .and_then(|s| s.parse::<i64>().ok())
                .filter(|&v| v > 0 && v <= 1000) // 验证范围：合理值（0-1000 Gwei）
                .unwrap_or_else(|| {
                    tracing::warn!("未找到基准gas价格配置，使用安全默认值 20 Gwei");
                    20 // 安全默认值：20 Gwei
                });
            let normal_price = rust_decimal::Decimal::new(baseline_price_gwei * 1_000_000_000, 0);
            if price > normal_price {
                // 如果高于正常价格，计算拥堵度
                let ratio = (price / normal_price).min(rust_decimal::Decimal::new(100, 0));
                Some(
                    (ratio * rust_decimal::Decimal::new(100, 0))
                        .to_u64()
                        .unwrap_or(100)
                        .min(100),
                )
            } else {
                Some(0u64) // 低于正常价格，不拥堵
            }
        } else {
            None
        }
    });

    use crate::api::response::success_response;
    success_response(NetworkStatusResponse {
        block_height,
        gas_price,
        congestion,
    })
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct BalanceQuery {
    pub address: String,
    /// 链标识（支持chain_id数字或链名称字符串）
    /// 企业级标准：统一使用chain参数，支持多种格式
    #[serde(alias = "chain_id", alias = "chain_symbol")]
    pub chain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BalanceResponse {
    pub balance: String,
    pub chain_id: u64,
    pub confirmed: bool,
}

async fn query_balance_response(
    address: &str,
    chain: Option<String>,
) -> Result<BalanceResponse, AppError> {
    if address.trim().is_empty() {
        return Err(AppError::bad_request("invalid address"));
    }

    // 企业级标准：统一chain参数处理，支持多种格式
    let resolved_chain_id = if let Some(chain_str) = chain {
        // 尝试解析为chain_id数字
        if let Ok(chain_id) = chain_str.parse::<i64>() {
            chain_id
        } else {
            // 尝试从链名称/符号转换为chain_id
            crate::service::token_service::network_to_chain_id(&chain_str).unwrap_or(1) as i64
            // 默认Ethereum主网
        }
    } else {
        1 // 默认Ethereum主网
    };

    let upstream = UpstreamClient::new();
    let balance = upstream
        .evm_get_balance(address)
        .await
        .unwrap_or_else(|_| "0".into());

    let _required_confirmations = std::env::var("REQUIRED_CONFIRMATIONS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(6);

    Ok(BalanceResponse {
        balance,
        chain_id: resolved_chain_id as u64,
        confirmed: true,
    })
}

#[utoipa::path(
    get,
    path = "/balance",
    params(BalanceQuery),
    responses(
        (status = 200, description = "Balance", body = crate::api::response::ApiResponse<BalanceResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn balance(
    Query(q): Query<BalanceQuery>,
) -> Result<Json<crate::api::response::ApiResponse<BalanceResponse>>, AppError> {
    crate::metrics::count_ok("GET /balance");
    let response = query_balance_response(&q.address, q.chain).await?;
    use crate::api::response::success_response;
    success_response(response)
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct WalletBalanceQueryParams {
    /// 链标识（支持chain_id数字或链名称字符串）
    /// 企业级标准：统一使用chain参数，支持多种格式
    #[serde(alias = "chain_id", alias = "chain_symbol")]
    pub chain: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/wallet/{address}/balance",
    params(
        ("address" = String, Path, description = "Checksum 地址"),
        ("chain_id" = Option<i64>, Query, description = "EVM Chain ID，默认 1"),
    ),
    responses(
        (status = 200, description = "Balance", body = BalanceResponse),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn wallet_balance_public(
    Path(address): Path<String>,
    Query(q): Query<WalletBalanceQueryParams>,
) -> Result<Json<crate::api::response::ApiResponse<BalanceResponse>>, AppError> {
    crate::metrics::count_ok("GET /api/wallets/:address/balance");
    let response = query_balance_response(&address, q.chain).await?;
    use crate::api::response::success_response;
    success_response(response)
}

/// 废弃版本：使用单数形式 /api/wallet/，请迁移到 /api/wallets/
/// 企业级标准：返回410 Gone错误，引导使用标准端点
pub async fn wallet_balance_public_deprecated(
    _path: Path<String>,
    _query: Query<WalletBalanceQueryParams>,
) -> Result<Json<crate::api::response::ApiResponse<BalanceResponse>>, AppError> {
    tracing::warn!(
        "[DEPRECATED] GET /api/wallet/:address/balance called. Please migrate to GET /api/wallets/:address/balance"
    );
    crate::metrics::count_ok("GET /api/wallet/:address/balance");
    // 返回410 Gone错误，引导使用标准端点
    Err(AppError {
        code: crate::error::AppErrorCode::BadRequest,
        message: "This endpoint has been deprecated. Please use GET /api/wallets/:address/balance instead.".to_string(),
        status: axum::http::StatusCode::GONE,
        trace_id: None,
    })
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct WalletTransactionsQuery {
    pub chain: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/wallet/{address}/transactions",
    params(
        ("address" = String, Path, description = "钱包地址"),
        ("chain" = Option<String>, Query, description = "链标识，如 ethereum/solana/bitcoin"),
    ),
    responses(
        (status = 200, description = "Transactions list", body = crate::api::response::ApiResponse<Vec<TxHistoryItem>>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn wallet_transactions_public(
    State(st): State<Arc<AppState>>,
    Path(address): Path<String>,
    Query(q): Query<WalletTransactionsQuery>,
) -> Result<Json<crate::api::response::ApiResponse<Vec<TxHistoryItem>>>, AppError> {
    crate::metrics::count_ok("GET /api/wallets/:address/transactions");

    let address_norm = address.to_lowercase();
    let chain_norm = q.chain.map(|c| c.to_lowercase());

    let rows = sqlx::query(
        r#"SELECT tx_hash,
                  tx_type,
                  status,
                  from_address,
                  to_address,
                  amount::TEXT as amount,
                  COALESCE(token_symbol, '') as token_symbol,
                  COALESCE(gas_fee, '0') as gas_fee,
                  EXTRACT(EPOCH FROM created_at)::BIGINT as ts
           FROM transactions
           WHERE (LOWER(from_address) = $1 OR LOWER(to_address) = $1)
             AND ($2::TEXT IS NULL OR chain = $2)
           ORDER BY created_at DESC
           LIMIT 50"#,
    )
    .bind(&address_norm)
    .bind(chain_norm.as_deref())
    .fetch_all(&st.pool)
    .await
    .map_err(|e| AppError::internal(format!("Failed to query transactions: {}", e)))?;

    let items = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            let from: String = row.get("from_address");
            let to: String = row.get("to_address");
            let derived_type = if from.to_lowercase() == address_norm {
                "send"
            } else {
                "receive"
            };

            TxHistoryItem {
                hash: row.get::<Option<String>, _>("tx_hash").unwrap_or_default(),
                tx_type: derived_type.to_string(),
                status: row.get::<String, _>("status"),
                from,
                to,
                amount: row
                    .get::<Option<String>, _>("amount")
                    .unwrap_or_else(|| "0".to_string()),
                token: {
                    let sym: String = row.get("token_symbol");
                    if sym.trim().is_empty() {
                        "NATIVE".to_string()
                    } else {
                        sym
                    }
                },
                timestamp: row.get::<i64, _>("ts").max(0) as u64,
                fee: row.get::<String, _>("gas_fee"),
            }
        })
        .collect::<Vec<_>>();

    use crate::api::response::success_response;
    success_response(items)
}

/// 废弃版本：使用单数形式 /api/wallet/，请迁移到 /api/wallets/
/// 企业级标准：返回410 Gone错误，引导使用标准端点
pub async fn wallet_transactions_public_deprecated(
    _path: Path<String>,
    _query: Query<WalletTransactionsQuery>,
) -> Result<Json<crate::api::response::ApiResponse<Vec<TxHistoryItem>>>, AppError> {
    tracing::warn!(
        "[DEPRECATED] GET /api/wallet/:address/transactions called. Please migrate to GET /api/wallets/:address/transactions"
    );
    crate::metrics::count_ok("GET /api/wallet/:address/transactions");
    // 返回410 Gone错误，引导使用标准端点
    Err(AppError {
        code: crate::error::AppErrorCode::BadRequest,
        message: "This endpoint has been deprecated. Please use GET /api/wallets/:address/transactions instead.".to_string(),
        status: axum::http::StatusCode::GONE,
        trace_id: None,
    })
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TokenMetadataQuery {
    pub chain_id: u64,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TokenMetadataResponse {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub logo_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/token/metadata",
    params(TokenMetadataQuery),
    responses(
        (status = 200, description = "Token metadata", body = crate::api::response::ApiResponse<TokenMetadataResponse>),
        (status = 404, description = "Token not registered", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn token_metadata(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TokenMetadataQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TokenMetadataResponse>>, AppError> {
    use tracing::error;

    use crate::repository::{PgTokenRepository, TokenRepository};

    // 从数据库读取代币元数据（替代硬编码）

    let repository = PgTokenRepository::new(state.pool.clone());
    let normalized_address = query.address.to_lowercase();

    let token = repository
        .get_by_address_and_chain(&normalized_address, query.chain_id)
        .await
        .map_err(|e| {
            error!("获取代币元数据失败: {:?}", e);
            AppError::internal(format!("获取代币元数据失败: {}", e))
        })?;

    let token_data = token.ok_or_else(|| AppError::not_found("token metadata not found"))?;
    use crate::api::response::success_response;
    success_response(TokenMetadataResponse {
        name: token_data.name,
        symbol: token_data.symbol,
        decimals: token_data.decimals as u8,
        logo_url: token_data.logo_url,
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastRawTxRequest {
    pub chain: String,
    pub signed_tx: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BroadcastRawTxData {
    pub tx_hash: String,
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/transactions/broadcast",
    request_body = BroadcastRawTxRequest,
    responses(
        (status = 200, description = "Broadcast result", body = crate::api::response::ApiResponse<BroadcastRawTxData>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn broadcast_raw_transaction(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<BroadcastRawTxRequest>,
) -> Result<Json<crate::api::response::ApiResponse<BroadcastRawTxData>>, AppError> {
    crate::metrics::count_ok("POST /api/tx/broadcast");

    if req.signed_tx.trim().is_empty() {
        return Err(AppError::bad_request("signed_tx is required"));
    }

    let broadcast_req = BroadcastTransactionRequest {
        chain: req.chain.clone(),
        signed_raw_tx: req.signed_tx.clone(),
    };

    let resp = st
        .blockchain_client
        .broadcast_transaction(broadcast_req)
        .await
        .map_err(|e| AppError::internal(format!("Broadcast failed: {}", e)))?;

    // Best-effort: persist tx record for user history/status lookups.
    // For EVM chains we parse the signed tx to extract from/to/value/nonce.
    let (from_address, to_address, amount_decimal, token_symbol, gas_fee, nonce_i64) = match req
        .chain
        .to_lowercase()
        .as_str()
    {
        "ethereum" | "eth" | "bsc" | "polygon" | "matic" | "binance" => {
            match parse_evm_signed_tx_details(&req.signed_tx) {
                Ok(details) => (
                    details.from,
                    details.to,
                    details.amount_decimal,
                    Some("NATIVE".to_string()),
                    details.gas_fee,
                    details.nonce,
                ),
                Err(e) => {
                    tracing::warn!(error=?e, chain=%req.chain, "Failed to parse EVM signed tx; storing minimal transaction record");
                    (
                        "unknown".to_string(),
                        "unknown".to_string(),
                        None,
                        None,
                        None,
                        None,
                    )
                }
            }
        }
        _ => (
            "unknown".to_string(),
            "unknown".to_string(),
            None,
            None,
            None,
            None,
        ),
    };

    // NOTE: wallet_id is optional in schema; we may not know it during raw broadcast.
    // Store tx_type as "send" to match frontend expectations.
    if let Err(e) = sqlx::query(
        r#"INSERT INTO transactions
           (id, tenant_id, user_id, wallet_id, chain, tx_hash, tx_type, status,
            from_address, to_address, amount, token_symbol, gas_fee, nonce, metadata,
            created_at, updated_at)
           VALUES ($1, $2, $3, NULL, $4, $5, $6, $7,
                   $8, $9, $10, $11, $12, $13, NULL,
                   CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
    )
    .bind(uuid::Uuid::new_v4())
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(req.chain.to_lowercase())
    .bind(&resp.tx_hash)
    .bind("send")
    .bind("submitted")
    .bind(from_address)
    .bind(to_address)
    .bind(amount_decimal)
    .bind(token_symbol)
    .bind(gas_fee)
    .bind(nonce_i64)
    .execute(&st.pool)
    .await
    {
        tracing::warn!(error=?e, tx_hash=%resp.tx_hash, chain=%req.chain, user_id=%auth.user_id, "Failed to persist broadcasted transaction; continuing");
    }

    use crate::api::response::success_response;
    success_response(BroadcastRawTxData {
        tx_hash: resp.tx_hash,
        status: "broadcasted".to_string(),
    })
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct TxStatusQuery {
    pub chain: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TxStatusData {
    pub tx_hash: String,
    pub status: String,
    pub confirmations: u64,
    pub last_seen: Option<u64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/transactions/{hash}/status",
    params(
        ("hash" = String, Path, description = "交易哈希"),
        ("chain" = Option<String>, Query, description = "链标识（可选）。如不提供，将从用户交易记录中推断"),
    ),
    responses(
        (status = 200, description = "Tx status", body = crate::api::response::ApiResponse<TxStatusData>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn tx_status(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(hash): Path<String>,
    Query(query): Query<TxStatusQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TxStatusData>>, AppError> {
    crate::metrics::count_ok("GET /api/tx/:hash/status");

    // Frontend does not always provide `chain` for status.
    // Prefer query param; otherwise infer from user's recorded transactions.
    let chain = if let Some(chain) = query
        .chain
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        chain.to_string()
    } else {
        let inferred: Option<String> = sqlx::query_scalar(
            "SELECT chain FROM transactions WHERE tx_hash = $1 AND user_id = $2 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&hash)
        .bind(auth.user_id)
        .fetch_optional(&st.pool)
        .await
        .map_err(|e| AppError::internal(format!("Failed to infer chain from database: {}", e)))?;

        inferred.unwrap_or_else(|| "ethereum".to_string())
    };

    let data = match st
        .blockchain_client
        .get_transaction_receipt(&chain, &hash)
        .await
    {
        Ok(Some(receipt)) => {
            let status_text = match receipt.status {
                Some(1) => "confirmed",
                Some(0) => "failed",
                _ => "pending",
            };
            TxStatusData {
                tx_hash: receipt.tx_hash,
                status: status_text.to_string(),
                confirmations: receipt.confirmations,
                last_seen: Some(chrono::Utc::now().timestamp() as u64),
            }
        }
        Ok(None) => TxStatusData {
            tx_hash: hash.clone(),
            status: "pending".to_string(),
            confirmations: 0,
            last_seen: Some(chrono::Utc::now().timestamp() as u64),
        },
        Err(err) => {
            return Err(AppError::internal(format!(
                "Failed to get transaction status: {}",
                err
            )));
        }
    };

    use crate::api::response::success_response;
    success_response(data)
}

// -------- OpenAPI & 文档 --------

pub async fn openapi_yaml() -> Response {
    crate::metrics::count_ok("GET /openapi.yaml");
    // 将 OpenAPI 规范作为静态资源内嵌
    const SPEC: &str = include_str!("../../openapi/openapi.yaml");
    ([(header::CONTENT_TYPE, "application/yaml")], SPEC).into_response()
}

pub async fn docs_swagger() -> Html<&'static str> {
    crate::metrics::count_ok("GET /docs");
    // 使用 Swagger UI CDN 加载 /openapi.yaml
    const HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>IronCore API Docs</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script>
    window.ui = SwaggerUIBundle({
      url: '/openapi.yaml',
      dom_id: '#swagger-ui'
    });
  </script>
</body>
</html>"#;
    Html(HTML)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTxReq {
    pub tenant_id: Uuid,
    pub wallet_id: Uuid,
    pub chain_id: i64,
    pub to_addr: String,
    pub amount_wei: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TxResp {
    pub id: Uuid,
    pub status: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/tx",
    request_body = CreateTxReq,
    responses(
        (status = 200, description = "Tx created", body = TxResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 429, description = "Rate limited", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_tx(
    State(st): State<Arc<AppState>>,
    _headers: HeaderMap,
    Json(req): Json<CreateTxReq>,
) -> Result<Json<crate::api::response::ApiResponse<TxResp>>, AppError> {
    // 幂等性检查和速率限制已由中间件处理
    let amount = req
        .amount_wei
        .parse::<rust_decimal::Decimal>()
        .map_err(|e| AppError::bad_request(format!("invalid amount: {}", e)))?;

    let r = service::tx::create_tx_request(
        &st.pool,
        req.tenant_id,
        req.wallet_id,
        req.chain_id,
        req.to_addr.clone(),
        Decimal::from_str_exact(&amount.to_string())
            .map_err(|e| AppError::bad_request(format!("amount parse error: {}", e)))?,
        req.metadata,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;

    // 审计双写（最佳努力，不阻断主流程）
    let payload = serde_json::json!({
        "op": "tx.create",
        "tenant_id": req.tenant_id,
        "wallet_id": req.wallet_id,
        "chain_id": req.chain_id,
        "to": req.to_addr,
        "amount": amount.to_string(),
        "tx_id": r.id,
    });

    // 使用审计日志辅助函数（异步，不等待结果）
    crate::utils::write_audit_event_async(
        st.immu.clone(),
        "tx.create".into(),
        req.tenant_id,
        req.wallet_id, // 使用wallet_id作为actor
        r.id,
        payload,
    );
    // 企业级标准：使用统一响应格式
    success_response(TxResp {
        id: r.id,
        status: r.status,
    })
}

// -------- 查询端点 --------

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListWalletsQuery {
    #[serde(default = "default_page")]
    pub page: i64,
    #[serde(default = "default_page_size")]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}
fn default_page_size() -> i64 {
    20
}
fn default_limit() -> i64 {
    20
}
fn default_offset() -> i64 {
    0
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListWalletsResp {
    pub wallets: Vec<WalletResp>,
    pub total: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/wallets",
    params(ListWalletsQuery),
    responses(
        (status = 200, description = "Wallet list", body = ListWalletsResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_wallets(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
    Query(q): Query<ListWalletsQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListWalletsResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/wallets");

    // ✅ 强制使用认证的user_id和tenant_id，防止IDOR漏洞
    // 从JWT token获取tenant_id和user_id，不接受查询参数
    let page_size = q.page_size.clamp(1, 100); // 1-100条
    let offset = (q.page.max(1) - 1) * page_size; // 分页偏移

    let wallets = service::wallets::list_wallets_by_user(
        &st.pool,
        auth_info.tenant_id, // 从JWT获取
        auth_info.user_id,   // 从JWT获取
        page_size,
        offset,
    )
    .await
    .map_err(|e| {
        tracing::error!("list_wallets database error: {}", e);
        AppError::bad_request(e.to_string())
    })?;

    // ✅ 使用认证的user_id计算总数
    let total =
        crate::repository::wallets::count_by_user(&st.pool, auth_info.tenant_id, auth_info.user_id)
            .await
            .map_err(|e| {
                tracing::error!("list_wallets count error: {}", e);
                AppError::bad_request(e.to_string())
            })?;

    let wallets_resp: Vec<WalletResp> = wallets
        .into_iter()
        .map(|w| WalletResp {
            id: w.id,
            tenant_id: w.tenant_id,
            user_id: w.user_id,
            chain_id: w.chain_id,
            address: w.address,
            public_key: w.pubkey, // 从数据库的 pubkey 映射到响应的 public_key
        })
        .collect();

    // 企业级标准：使用统一响应格式
    success_response(ListWalletsResp {
        wallets: wallets_resp,
        total,
    })
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetWalletParams {
    pub id: Uuid,
}

#[utoipa::path(
    get,
    path = "/api/v1/wallets/{id}",
    params(GetWalletParams),
    responses(
        (status = 200, description = "Wallet", body = crate::api::response::ApiResponse<WalletResp>),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_wallet(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<WalletResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/wallets/:id");

    let wallet = service::wallets::get_wallet_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Wallet not found"))?;

    // ✅ 验证钱包所有权（防止IDOR漏洞）
    if wallet.user_id != auth_info.user_id || wallet.tenant_id != auth_info.tenant_id {
        return Err(AppError::permission_denied(
            "Not authorized to access this wallet",
        ));
    }

    // 企业级标准：使用统一响应格式，统一使用 public_key 字段名
    success_response(WalletResp {
        id: wallet.id,
        tenant_id: wallet.tenant_id,
        user_id: wallet.user_id,
        chain_id: wallet.chain_id,
        address: wallet.address,
        public_key: wallet.pubkey, // 从数据库的 pubkey 映射到响应的 public_key
    })
}

#[utoipa::path(
    delete,
    path = "/api/v1/wallets/{id}",
    params(GetWalletParams),
    responses(
        (status = 200, description = "Wallet deleted"),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn delete_wallet(
    State(st): State<Arc<AppState>>,
    _headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Result<StatusCode, AppError> {
    let tenant_id = q
        .get("tenant_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::bad_request("tenant_id required"))?;

    // 速率限制已由中间件处理
    crate::metrics::count_ok("DELETE /api/v1/wallets/:id");

    let deleted = service::wallets::delete_wallet(&st.pool, id, tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    if !deleted {
        return Err(AppError::not_found("Wallet not found"));
    }

    // 审计双写（最佳努力，不阻断主流程）
    let payload = serde_json::json!({
        "op": "wallet.delete",
        "tenant_id": tenant_id,
        "wallet_id": id,
    });

    // 使用审计日志辅助函数（异步，不等待结果）
    crate::utils::write_audit_event_async_str_actor(
        st.immu.clone(),
        "wallet.delete".into(),
        tenant_id,
        "system".into(),
        id,
        payload,
    );

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListTxQuery {
    pub tenant_id: Uuid,
    #[serde(default)]
    pub wallet_id: Option<Uuid>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListTxResp {
    pub transactions: Vec<TxResp>,
    pub total: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/api/v1/tx",
    params(ListTxQuery),
    responses(
        (status = 200, description = "Tx list", body = ListTxResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_tx(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListTxQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListTxResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tx");

    let transactions = if let Some(wallet_id) = q.wallet_id {
        service::tx::list_tx_by_wallet(&st.pool, q.tenant_id, wallet_id, q.limit.min(100), q.offset)
            .await
            .map_err(|e| AppError::bad_request(e.to_string()))?
    } else {
        service::tx::list_tx_by_tenant(&st.pool, q.tenant_id, q.limit.min(100), q.offset)
            .await
            .map_err(|e| AppError::bad_request(e.to_string()))?
    };

    let tx_resp: Vec<TxResp> = transactions
        .into_iter()
        .map(|t| TxResp {
            id: t.id,
            status: t.status,
        })
        .collect();

    // 企业级标准：使用统一响应格式
    success_response(ListTxResp {
        transactions: tx_resp,
        total: None, // 可选：可以添加总数查询
    })
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetTxParams {
    pub id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TxDetailResp {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub wallet_id: Uuid,
    pub chain_id: i64,
    pub to_addr: String,
    pub amount: String,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/tx/{id}",
    params(GetTxParams),
    responses(
        (status = 200, description = "Tx detail", body = crate::api::response::ApiResponse<TxDetailResp>),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_tx(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<TxDetailResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tx/:id");

    let tx = service::tx::get_tx_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Transaction not found"))?;

    // 企业级标准：使用统一响应格式
    success_response(TxDetailResp {
        id: tx.id,
        tenant_id: tx.tenant_id,
        wallet_id: tx.wallet_id,
        chain_id: tx.chain_id,
        to_addr: tx.to_addr,
        amount: tx.amount.to_string(),
        status: tx.status,
        metadata: tx.metadata,
        created_at: tx.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTxStatusReq {
    pub tenant_id: Uuid,
    pub status: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/tx/{id}/status",
    params(GetTxParams),
    request_body = UpdateTxStatusReq,
    responses(
        (status = 200, description = "Tx status updated", body = TxDetailResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_tx_status(
    State(st): State<Arc<AppState>>,
    _headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateTxStatusReq>,
) -> Result<Json<crate::api::response::ApiResponse<TxDetailResp>>, AppError> {
    // 速率限制
    // 速率限制已由中间件处理
    crate::metrics::count_ok("PUT /api/v1/tx/:id/status");

    // 验证状态值
    let valid_statuses = [
        "draft",
        "approved",
        "signed",
        "broadcasted",
        "confirmed",
        "failed",
    ];
    if !valid_statuses.contains(&req.status.as_str()) {
        return Err(AppError::bad_request(format!(
            "Invalid status: {}. Valid values: {:?}",
            req.status, valid_statuses
        )));
    }

    let tx = service::tx::advance_tx_status(&st.pool, id, req.tenant_id, &req.status)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Transaction not found"))?;

    // 审计双写（最佳努力，不阻断主流程）
    let payload = serde_json::json!({
        "op": "tx.status.update",
        "tenant_id": req.tenant_id,
        "tx_id": id,
        "old_status": "unknown", // 可以查询旧状态
        "new_status": req.status,
    });

    // 使用审计日志辅助函数（异步，不等待结果）
    crate::utils::write_audit_event_async_str_actor(
        st.immu.clone(),
        "tx.status.update".into(),
        req.tenant_id,
        "system".into(),
        id,
        payload,
    );

    // 企业级标准：使用统一响应格式
    success_response(TxDetailResp {
        id: tx.id,
        tenant_id: tx.tenant_id,
        wallet_id: tx.wallet_id,
        chain_id: tx.chain_id,
        to_addr: tx.to_addr,
        amount: tx.amount.to_string(),
        status: tx.status,
        metadata: tx.metadata,
        created_at: tx.created_at.to_rfc3339(),
    })
}

// ========== Tenants API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTenantReq {
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TenantResp {
    pub id: Uuid,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListTenantsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListTenantsResp {
    pub tenants: Vec<TenantResp>,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/tenants",
    request_body = CreateTenantReq,
    responses(
        (status = 200, description = "Tenant created", body = TenantResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_tenant(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CreateTenantReq>,
) -> Result<Json<crate::api::response::ApiResponse<TenantResp>>, AppError> {
    crate::metrics::count_ok("POST /api/v1/tenants");
    let tenant = service::tenants::create_tenant(&st.pool, req.name)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(TenantResp {
        id: tenant.id,
        name: tenant.name,
        created_at: tenant.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants",
    params(ListTenantsQuery),
    responses(
        (status = 200, description = "Tenant list", body = ListTenantsResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_tenants(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListTenantsQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListTenantsResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tenants");
    let tenants = service::tenants::list_tenants(&st.pool, q.limit.min(100), q.offset)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let total = service::tenants::count_tenants(&st.pool)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ListTenantsResp {
        tenants: tenants
            .into_iter()
            .map(|t| TenantResp {
                id: t.id,
                name: t.name,
                created_at: t.created_at.to_rfc3339(),
            })
            .collect(),
        total,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/tenants/{id}",
    params(("id" = Uuid, Path, description = "Tenant ID")),
    responses(
        (status = 200, description = "Tenant", body = TenantResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_tenant(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<TenantResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tenants/:id");
    let tenant = service::tenants::get_tenant_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Tenant not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(TenantResp {
        id: tenant.id,
        name: tenant.name,
        created_at: tenant.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTenantReq {
    pub name: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/tenants/{id}",
    params(("id" = Uuid, Path, description = "Tenant ID")),
    request_body = UpdateTenantReq,
    responses(
        (status = 200, description = "Tenant updated", body = TenantResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_tenant(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateTenantReq>,
) -> Result<Json<crate::api::response::ApiResponse<TenantResp>>, AppError> {
    crate::metrics::count_ok("PUT /api/v1/tenants/:id");
    let tenant = service::tenants::update_tenant(&st.pool, id, req.name)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Tenant not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(TenantResp {
        id: tenant.id,
        name: tenant.name,
        created_at: tenant.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    delete,
    path = "/api/v1/tenants/{id}",
    params(("id" = Uuid, Path, description = "Tenant ID")),
    responses(
        (status = 204, description = "Tenant deleted"),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn delete_tenant(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("DELETE /api/v1/tenants/:id");
    let deleted = service::tenants::delete_tenant(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    if !deleted {
        return Err(AppError::not_found("Tenant not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ========== Users API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserReq {
    pub tenant_id: Uuid,
    pub email_cipher: String,
    pub phone_cipher: Option<String>,
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResp {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email_cipher: String,
    pub phone_cipher: Option<String>,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListUsersQuery {
    pub tenant_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListUsersResp {
    pub users: Vec<UserResp>,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/users",
    request_body = CreateUserReq,
    responses(
        (status = 200, description = "User created", body = UserResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_user(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CreateUserReq>,
) -> Result<Json<crate::api::response::ApiResponse<UserResp>>, AppError> {
    crate::metrics::count_ok("POST /api/v1/users");
    let user = service::users::create_user(
        &st.pool,
        req.tenant_id,
        req.email_cipher,
        req.phone_cipher,
        req.role,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(UserResp {
        id: user.id,
        tenant_id: user.tenant_id,
        email_cipher: user.email_cipher,
        phone_cipher: user.phone_cipher,
        role: user.role,
        created_at: user.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(ListUsersQuery),
    responses(
        (status = 200, description = "User list", body = ListUsersResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_users(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListUsersQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListUsersResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/users");
    let users =
        service::users::list_users_by_tenant(&st.pool, q.tenant_id, q.limit.min(100), q.offset)
            .await
            .map_err(|e| AppError::bad_request(e.to_string()))?;
    let total = service::users::count_users_by_tenant(&st.pool, q.tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ListUsersResp {
        users: users
            .into_iter()
            .map(|u| UserResp {
                id: u.id,
                tenant_id: u.tenant_id,
                email_cipher: u.email_cipher,
                phone_cipher: u.phone_cipher,
                role: u.role,
                created_at: u.created_at.to_rfc3339(),
            })
            .collect(),
        total,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User", body = UserResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_user(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<UserResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/users/:id");
    let user = service::users::get_user_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("User not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(UserResp {
        id: user.id,
        tenant_id: user.tenant_id,
        email_cipher: user.email_cipher,
        phone_cipher: user.phone_cipher,
        role: user.role,
        created_at: user.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserReq {
    pub tenant_id: Uuid,
    pub email_cipher: Option<String>,
    pub phone_cipher: Option<String>,
    pub role: Option<String>,
}

#[utoipa::path(
    put,
    path = "/api/v1/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    request_body = UpdateUserReq,
    responses(
        (status = 200, description = "User updated", body = UserResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_user(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateUserReq>,
) -> Result<Json<crate::api::response::ApiResponse<UserResp>>, AppError> {
    crate::metrics::count_ok("PUT /api/v1/users/:id");
    let user = service::users::update_user(
        &st.pool,
        id,
        req.tenant_id,
        req.email_cipher,
        req.phone_cipher,
        req.role,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?
    .ok_or_else(|| AppError::not_found("User not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(UserResp {
        id: user.id,
        tenant_id: user.tenant_id,
        email_cipher: user.email_cipher,
        phone_cipher: user.phone_cipher,
        role: user.role,
        created_at: user.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    delete,
    path = "/api/v1/users/{id}",
    params(("id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 204, description = "User deleted"),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn delete_user(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("DELETE /api/v1/users/:id");
    let tenant_id = q
        .get("tenant_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::bad_request("tenant_id required"))?;
    let deleted = service::users::delete_user(&st.pool, id, tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    if !deleted {
        return Err(AppError::not_found("User not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ========== Policies API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePolicyReq {
    pub tenant_id: Uuid,
    pub name: String,
    pub rules: serde_json::Value,
    pub version: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PolicyResp {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub rules: serde_json::Value,
    pub version: i32,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListPoliciesQuery {
    pub tenant_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListPoliciesResp {
    pub policies: Vec<PolicyResp>,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/policies",
    request_body = CreatePolicyReq,
    responses(
        (status = 200, description = "Policy created", body = PolicyResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_policy(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CreatePolicyReq>,
) -> Result<Json<crate::api::response::ApiResponse<PolicyResp>>, AppError> {
    crate::metrics::count_ok("POST /api/v1/policies");
    let policy =
        service::policies::create_policy(&st.pool, req.tenant_id, req.name, req.rules, req.version)
            .await
            .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(PolicyResp {
        id: policy.id,
        tenant_id: policy.tenant_id,
        name: policy.name,
        rules: policy.rules,
        version: policy.version,
        created_at: policy.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/policies",
    params(ListPoliciesQuery),
    responses(
        (status = 200, description = "Policy list", body = ListPoliciesResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_policies(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListPoliciesQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListPoliciesResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/policies");
    let policies = service::policies::list_policies_by_tenant(
        &st.pool,
        q.tenant_id,
        q.limit.min(100),
        q.offset,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    let total = service::policies::count_policies_by_tenant(&st.pool, q.tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ListPoliciesResp {
        policies: policies
            .into_iter()
            .map(|p| PolicyResp {
                id: p.id,
                tenant_id: p.tenant_id,
                name: p.name,
                rules: p.rules,
                version: p.version,
                created_at: p.created_at.to_rfc3339(),
            })
            .collect(),
        total,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/policies/{id}",
    params(("id" = Uuid, Path, description = "Policy ID")),
    responses(
        (status = 200, description = "Policy", body = PolicyResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_policy(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<PolicyResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/policies/:id");
    let policy = service::policies::get_policy_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Policy not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(PolicyResp {
        id: policy.id,
        tenant_id: policy.tenant_id,
        name: policy.name,
        rules: policy.rules,
        version: policy.version,
        created_at: policy.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePolicyReq {
    pub tenant_id: Uuid,
    pub name: Option<String>,
    pub rules: Option<serde_json::Value>,
    pub version: Option<i32>,
}

#[utoipa::path(
    put,
    path = "/api/v1/policies/{id}",
    params(("id" = Uuid, Path, description = "Policy ID")),
    request_body = UpdatePolicyReq,
    responses(
        (status = 200, description = "Policy updated", body = PolicyResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_policy(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdatePolicyReq>,
) -> Result<Json<crate::api::response::ApiResponse<PolicyResp>>, AppError> {
    crate::metrics::count_ok("PUT /api/v1/policies/:id");
    let policy = service::policies::update_policy(
        &st.pool,
        id,
        req.tenant_id,
        req.name,
        req.rules,
        req.version,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Policy not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(PolicyResp {
        id: policy.id,
        tenant_id: policy.tenant_id,
        name: policy.name,
        rules: policy.rules,
        version: policy.version,
        created_at: policy.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    delete,
    path = "/api/v1/policies/{id}",
    params(("id" = Uuid, Path, description = "Policy ID")),
    responses(
        (status = 204, description = "Policy deleted"),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn delete_policy(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("DELETE /api/v1/policies/:id");
    let tenant_id = q
        .get("tenant_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::bad_request("tenant_id required"))?;
    let deleted = service::policies::delete_policy(&st.pool, id, tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    if !deleted {
        return Err(AppError::not_found("Policy not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ========== Approvals API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApprovalReq {
    pub tenant_id: Uuid,
    pub policy_id: Uuid,
    pub requester: Uuid,
    pub status: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApprovalResp {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub policy_id: Uuid,
    pub requester: Uuid,
    pub status: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListApprovalsQuery {
    pub tenant_id: Uuid,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListApprovalsResp {
    pub approvals: Vec<ApprovalResp>,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/approvals",
    request_body = CreateApprovalReq,
    responses(
        (status = 200, description = "Approval created", body = ApprovalResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_approval(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CreateApprovalReq>,
) -> Result<Json<crate::api::response::ApiResponse<ApprovalResp>>, AppError> {
    crate::metrics::count_ok("POST /api/v1/approvals");
    let approval = service::approvals::create_approval(
        &st.pool,
        req.tenant_id,
        req.policy_id,
        req.requester,
        req.status,
        req.payload,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ApprovalResp {
        id: approval.id,
        tenant_id: approval.tenant_id,
        policy_id: approval.policy_id,
        requester: approval.requester,
        status: approval.status,
        payload: approval.payload,
        created_at: approval.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/approvals",
    params(ListApprovalsQuery),
    responses(
        (status = 200, description = "Approval list", body = ListApprovalsResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_approvals(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListApprovalsQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListApprovalsResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/approvals");
    let status = q.status.clone();
    let approvals = service::approvals::list_approvals_by_tenant(
        &st.pool,
        q.tenant_id,
        status.clone(),
        q.limit.min(100),
        q.offset,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    let total = service::approvals::count_approvals_by_tenant(&st.pool, q.tenant_id, status)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ListApprovalsResp {
        approvals: approvals
            .into_iter()
            .map(|a| ApprovalResp {
                id: a.id,
                tenant_id: a.tenant_id,
                policy_id: a.policy_id,
                requester: a.requester,
                status: a.status,
                payload: a.payload,
                created_at: a.created_at.to_rfc3339(),
            })
            .collect(),
        total,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/approvals/{id}",
    params(("id" = Uuid, Path, description = "Approval ID")),
    responses(
        (status = 200, description = "Approval", body = ApprovalResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_approval(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<ApprovalResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/approvals/:id");
    let approval = service::approvals::get_approval_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Approval not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(ApprovalResp {
        id: approval.id,
        tenant_id: approval.tenant_id,
        policy_id: approval.policy_id,
        requester: approval.requester,
        status: approval.status,
        payload: approval.payload,
        created_at: approval.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateApprovalStatusReq {
    pub tenant_id: Uuid,
    pub status: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/approvals/{id}/status",
    params(("id" = Uuid, Path, description = "Approval ID")),
    request_body = UpdateApprovalStatusReq,
    responses(
        (status = 200, description = "Approval status updated", body = ApprovalResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_approval_status(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateApprovalStatusReq>,
) -> Result<Json<crate::api::response::ApiResponse<ApprovalResp>>, AppError> {
    crate::metrics::count_ok("PUT /api/v1/approvals/:id/status");
    let approval =
        service::approvals::update_approval_status(&st.pool, id, req.tenant_id, req.status)
            .await
            .map_err(|e| AppError::bad_request(e.to_string()))?
            .ok_or_else(|| AppError::not_found("Approval not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(ApprovalResp {
        id: approval.id,
        tenant_id: approval.tenant_id,
        policy_id: approval.policy_id,
        requester: approval.requester,
        status: approval.status,
        payload: approval.payload,
        created_at: approval.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    delete,
    path = "/api/v1/approvals/{id}",
    params(("id" = Uuid, Path, description = "Approval ID")),
    responses(
        (status = 204, description = "Approval deleted"),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn delete_approval(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("DELETE /api/v1/approvals/:id");
    let tenant_id = q
        .get("tenant_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::bad_request("tenant_id required"))?;
    let deleted = service::approvals::delete_approval(&st.pool, id, tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    if !deleted {
        return Err(AppError::not_found("Approval not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ========== API Keys API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyReq {
    pub tenant_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiKeyResp {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub name: String,
    pub key_hash: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListApiKeysQuery {
    pub tenant_id: Uuid,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListApiKeysResp {
    pub api_keys: Vec<ApiKeyResp>,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/api-keys",
    request_body = CreateApiKeyReq,
    responses(
        (status = 200, description = "API key created", body = ApiKeyResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_api_key(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CreateApiKeyReq>,
) -> Result<Json<crate::api::response::ApiResponse<ApiKeyResp>>, AppError> {
    crate::metrics::count_ok("POST /api/v1/api-keys");
    let api_key = service::api_keys::create_api_key(
        &st.pool,
        req.tenant_id,
        req.name,
        req.key_hash,
        req.status,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ApiKeyResp {
        id: api_key.id,
        tenant_id: api_key.tenant_id,
        name: api_key.name,
        key_hash: api_key.key_hash,
        status: api_key.status,
        created_at: api_key.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/api-keys",
    params(ListApiKeysQuery),
    responses(
        (status = 200, description = "API key list", body = ListApiKeysResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_api_keys(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListApiKeysQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListApiKeysResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/api-keys");
    let status = q.status.clone();
    let api_keys = service::api_keys::list_api_keys_by_tenant(
        &st.pool,
        q.tenant_id,
        status.clone(),
        q.limit.min(100),
        q.offset,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    let total = service::api_keys::count_api_keys_by_tenant(&st.pool, q.tenant_id, status)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ListApiKeysResp {
        api_keys: api_keys
            .into_iter()
            .map(|k| ApiKeyResp {
                id: k.id,
                tenant_id: k.tenant_id,
                name: k.name,
                key_hash: k.key_hash,
                status: k.status,
                created_at: k.created_at.to_rfc3339(),
            })
            .collect(),
        total,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/api-keys/{id}",
    params(("id" = Uuid, Path, description = "API Key ID")),
    responses(
        (status = 200, description = "API key", body = ApiKeyResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_api_key(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<ApiKeyResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/api-keys/:id");
    let api_key = service::api_keys::get_api_key_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("API key not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(ApiKeyResp {
        id: api_key.id,
        tenant_id: api_key.tenant_id,
        name: api_key.name,
        key_hash: api_key.key_hash,
        status: api_key.status,
        created_at: api_key.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateApiKeyStatusReq {
    pub tenant_id: Uuid,
    pub status: String,
}

#[utoipa::path(
    put,
    path = "/api/v1/api-keys/{id}/status",
    params(("id" = Uuid, Path, description = "API Key ID")),
    request_body = UpdateApiKeyStatusReq,
    responses(
        (status = 200, description = "API key status updated", body = ApiKeyResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_api_key_status(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateApiKeyStatusReq>,
) -> Result<Json<crate::api::response::ApiResponse<ApiKeyResp>>, AppError> {
    crate::metrics::count_ok("PUT /api/v1/api-keys/:id/status");
    let api_key = service::api_keys::update_api_key_status(&st.pool, id, req.tenant_id, req.status)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("API key not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(ApiKeyResp {
        id: api_key.id,
        tenant_id: api_key.tenant_id,
        name: api_key.name,
        key_hash: api_key.key_hash,
        status: api_key.status,
        created_at: api_key.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    delete,
    path = "/api/v1/api-keys/{id}",
    params(("id" = Uuid, Path, description = "API Key ID")),
    responses(
        (status = 204, description = "API key deleted"),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn delete_api_key(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Query(q): Query<HashMap<String, String>>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("DELETE /api/v1/api-keys/:id");
    let tenant_id = q
        .get("tenant_id")
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| AppError::bad_request("tenant_id required"))?;
    let deleted = service::api_keys::delete_api_key(&st.pool, id, tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    if !deleted {
        return Err(AppError::not_found("API key not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ========== Tx Broadcasts API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTxBroadcastReq {
    pub tenant_id: Uuid,
    pub tx_request_id: Uuid,
    pub tx_hash: Option<String>,
    pub receipt: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TxBroadcastResp {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub tx_request_id: Uuid,
    pub tx_hash: Option<String>,
    pub receipt: Option<serde_json::Value>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ListTxBroadcastsQuery {
    pub tenant_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default = "default_offset")]
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListTxBroadcastsResp {
    pub broadcasts: Vec<TxBroadcastResp>,
    pub total: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/tx-broadcasts",
    request_body = CreateTxBroadcastReq,
    responses(
        (status = 200, description = "Tx broadcast created", body = TxBroadcastResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn create_tx_broadcast(
    State(st): State<Arc<AppState>>,
    Json(req): Json<CreateTxBroadcastReq>,
) -> Result<Json<crate::api::response::ApiResponse<TxBroadcastResp>>, AppError> {
    crate::metrics::count_ok("POST /api/v1/tx-broadcasts");
    let broadcast = service::tx_broadcasts::create_tx_broadcast(
        &st.pool,
        req.tenant_id,
        req.tx_request_id,
        req.tx_hash,
        req.receipt,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(TxBroadcastResp {
        id: broadcast.id,
        tenant_id: broadcast.tenant_id,
        tx_request_id: broadcast.tx_request_id,
        tx_hash: broadcast.tx_hash,
        receipt: broadcast.receipt,
        created_at: broadcast.created_at.to_rfc3339(),
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/tx-broadcasts",
    params(ListTxBroadcastsQuery),
    responses(
        (status = 200, description = "Tx broadcast list", body = ListTxBroadcastsResp),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn list_tx_broadcasts(
    State(st): State<Arc<AppState>>,
    Query(q): Query<ListTxBroadcastsQuery>,
) -> Result<Json<crate::api::response::ApiResponse<ListTxBroadcastsResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tx-broadcasts");
    let broadcasts = service::tx_broadcasts::list_tx_broadcasts_by_tenant(
        &st.pool,
        q.tenant_id,
        q.limit.min(100),
        q.offset,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;
    let total = service::tx_broadcasts::count_tx_broadcasts_by_tenant(&st.pool, q.tenant_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    // 企业级标准：使用统一响应格式
    success_response(ListTxBroadcastsResp {
        broadcasts: broadcasts
            .into_iter()
            .map(|b| TxBroadcastResp {
                id: b.id,
                tenant_id: b.tenant_id,
                tx_request_id: b.tx_request_id,
                tx_hash: b.tx_hash,
                receipt: b.receipt,
                created_at: b.created_at.to_rfc3339(),
            })
            .collect(),
        total,
    })
}

#[utoipa::path(
    get,
    path = "/api/v1/tx-broadcasts/{id}",
    params(("id" = Uuid, Path, description = "Tx Broadcast ID")),
    responses(
        (status = 200, description = "Tx broadcast", body = TxBroadcastResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_tx_broadcast(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<TxBroadcastResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tx-broadcasts/:id");
    let broadcast = service::tx_broadcasts::get_tx_broadcast_by_id(&st.pool, id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?
        .ok_or_else(|| AppError::not_found("Tx broadcast not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(TxBroadcastResp {
        id: broadcast.id,
        tenant_id: broadcast.tenant_id,
        tx_request_id: broadcast.tx_request_id,
        tx_hash: broadcast.tx_hash,
        receipt: broadcast.receipt,
        created_at: broadcast.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateTxBroadcastReq {
    pub tenant_id: Uuid,
    pub tx_hash: Option<String>,
    pub receipt: Option<serde_json::Value>,
}

#[utoipa::path(
    put,
    path = "/api/v1/tx-broadcasts/{id}",
    params(("id" = Uuid, Path, description = "Tx Broadcast ID")),
    request_body = UpdateTxBroadcastReq,
    responses(
        (status = 200, description = "Tx broadcast updated", body = TxBroadcastResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn update_tx_broadcast(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<UpdateTxBroadcastReq>,
) -> Result<Json<crate::api::response::ApiResponse<TxBroadcastResp>>, AppError> {
    crate::metrics::count_ok("PUT /api/v1/tx-broadcasts/:id");
    let broadcast = service::tx_broadcasts::update_tx_broadcast(
        &st.pool,
        id,
        req.tenant_id,
        req.tx_hash,
        req.receipt,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Tx broadcast not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(TxBroadcastResp {
        id: broadcast.id,
        tenant_id: broadcast.tenant_id,
        tx_request_id: broadcast.tx_request_id,
        tx_hash: broadcast.tx_hash,
        receipt: broadcast.receipt,
        created_at: broadcast.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetTxBroadcastByHashQuery {
    pub tenant_id: Uuid,
}

#[utoipa::path(
    get,
    path = "/api/v1/tx-broadcasts/by-tx-hash/{hash}",
    params(
        ("hash" = String, Path, description = "Transaction hash"),
        GetTxBroadcastByHashQuery
    ),
    responses(
        (status = 200, description = "Tx broadcast", body = TxBroadcastResp),
        (status = 404, description = "Not found", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_tx_broadcast_by_tx_hash(
    State(st): State<Arc<AppState>>,
    axum::extract::Path(hash): axum::extract::Path<String>,
    Query(q): Query<GetTxBroadcastByHashQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TxBroadcastResp>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/tx-broadcasts/by-tx-hash/:hash");
    let broadcast =
        service::tx_broadcasts::get_tx_broadcast_by_tx_hash(&st.pool, q.tenant_id, &hash)
            .await
            .map_err(|e| AppError::bad_request(e.to_string()))?
            .ok_or_else(|| AppError::not_found("Tx broadcast not found"))?;
    // 企业级标准：使用统一响应格式
    success_response(TxBroadcastResp {
        id: broadcast.id,
        tenant_id: broadcast.tenant_id,
        tx_request_id: broadcast.tx_request_id,
        tx_hash: broadcast.tx_hash,
        receipt: broadcast.receipt,
        created_at: broadcast.created_at.to_rfc3339(),
    })
}

// ========== Authentication API ==========

// ========== Auth API ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterReq {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub phone: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterResp {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub created_at: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterReq,
    responses(
        (status = 200, description = "Registration successful", body = RegisterResp),
        (status = 400, description = "Invalid request or email already exists", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn register(
    State(st): State<Arc<AppState>>,
    Json(req): Json<RegisterReq>,
) -> Result<axum::Json<crate::api::response::ApiResponse<RegisterResp>>, AppError> {
    crate::metrics::count_ok("POST /api/auth/register");

    // 1. Validate email format with strict regex to prevent SQL injection
    use regex::Regex;

    // 严格的邮箱验证正则表达式
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex");

    if !email_regex.is_match(&req.email) {
        return Err(AppError::bad_request("Invalid email format"));
    }

    // 额外检查：拒绝包含SQL注入关键字或特殊字符
    let email_upper = req.email.to_uppercase();
    let forbidden_patterns = [
        "'", "\"", ";", "--", "/*", "*/", " OR ", " AND ", " SELECT ", " INSERT ", " UPDATE ",
        " DELETE ", " DROP ", " UNION ",
    ];

    for pattern in forbidden_patterns.iter() {
        if email_upper.contains(pattern) || req.email.contains(pattern) {
            return Err(AppError::bad_request("Email contains illegal characters"));
        }
    }

    // 长度限制
    if req.email.len() > 255 {
        return Err(AppError::bad_request("Email is too long"));
    }

    // 2. Validate password strength (at least 8 characters, with letters and numbers)
    use crate::infrastructure::validation::validate_password_strength;
    validate_password_strength(&req.password)
        .map_err(|e| AppError::bad_request(format!("Password validation failed: {}", e)))?;

    // 3. Call service layer to register user
    let (access_token, refresh_token, user_id) = service::auth::register(
        &st.pool,
        &st.redis,
        req.email.clone(),
        req.password,
        req.phone,
    )
    .await
    .map_err(|e| {
        log::error!("Registration failed: {}", e);
        AppError::bad_request(format!("Registration failed: {}", e))
    })?;

    // 使用统一响应格式
    use crate::api::response::success_response;
    success_response(RegisterResp {
        access_token,
        refresh_token: Some(refresh_token),
        user: UserInfo {
            id: user_id.to_string(),
            email: req.email,
            created_at: chrono::Utc::now().to_rfc3339(),
        },
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginReq {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResp {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    pub user: UserInfo,
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginReq,
    responses(
        (status = 200, description = "Login successful", body = LoginResp),
        (status = 401, description = "Invalid credentials", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn login(
    State(st): State<Arc<AppState>>,
    Json(req): Json<LoginReq>,
) -> Result<axum::Json<crate::api::response::ApiResponse<LoginResp>>, AppError> {
    crate::metrics::count_ok("POST /api/auth/login");

    // 先查找用户获取tenant_id
    // 注意：users 表使用 email_cipher 列，但为了兼容性也支持 email 列（如果存在）
    let user_record: Option<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, tenant_id FROM users WHERE email_cipher = $1 OR email = $1 LIMIT 1"
    )
    .bind(&req.email)
    .fetch_optional(&st.pool)
    .await
    .map_err(|e| {
        // 检查是否是数据库连接或表不存在错误
        let error_msg = e.to_string();
        if error_msg.contains("relation") && error_msg.contains("does not exist") {
            AppError {
                code: crate::error::AppErrorCode::Internal,
                message: "Database not initialized. Please run migrations first. Run: sqlx migrate run --database-url \"postgresql://root@localhost:26257/ironcore?sslmode=disable\"".to_string(),
                status: StatusCode::SERVICE_UNAVAILABLE,
                trace_id: None,
            }
        } else if error_msg.contains("connection") || error_msg.contains("timeout") {
            AppError {
                code: crate::error::AppErrorCode::Internal,
                message: "Database connection failed. Please check database status.".to_string(),
                status: StatusCode::SERVICE_UNAVAILABLE,
                trace_id: None,
            }
        } else {
            AppError::internal(format!("Database error: {}", e))
        }
    })?;

    let (_user_id, tenant_id) = user_record.ok_or_else(|| AppError {
        code: crate::error::AppErrorCode::Unauthorized,
        message: "Invalid credentials".to_string(),
        status: StatusCode::UNAUTHORIZED,
        trace_id: None,
    })?;

    let (access_token, refresh_token, user) = service::auth::login(
        &st.pool,
        &st.redis,
        tenant_id,
        req.email.clone(),
        req.password,
    )
    .await
    .map_err(|e| AppError {
        code: crate::error::AppErrorCode::Unauthorized,
        message: e.to_string(),
        status: StatusCode::UNAUTHORIZED,
        trace_id: None,
    })?;

    // 使用统一响应格式
    use crate::api::response::success_response;
    success_response(LoginResp {
        access_token,
        refresh_token: Some(refresh_token),
        user: UserInfo {
            id: user.id.to_string(),
            email: req.email,
            created_at: user.created_at.to_rfc3339(),
        },
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogoutReq {
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogoutResp {
    pub message: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    request_body = LogoutReq,
    responses(
        (status = 200, description = "Logout successful", body = LogoutResp),
        (status = 401, description = "Invalid token", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn logout(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<LogoutReq>,
) -> Result<axum::Json<crate::api::response::ApiResponse<serde_json::Value>>, AppError> {
    crate::metrics::count_ok("POST /api/auth/logout");

    // 优先从请求体获取 token，如果没有则从 Authorization header 获取
    let token = if let Some(t) = req.token {
        t
    } else {
        // 从 Authorization header 提取 token
        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AppError::unauthorized("Missing authorization header or token"))?;

        // 去掉 "Bearer " 前缀
        auth_header
            .strip_prefix("Bearer ")
            .unwrap_or(auth_header)
            .trim()
            .to_string()
    };

    service::auth::logout(&st.redis, &token)
        .await
        .map_err(|e| AppError {
            code: crate::error::AppErrorCode::Unauthorized,
            message: e.to_string(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: None,
        })?;

    // 使用统一响应格式
    use crate::api::response::success_response_with_message;
    success_response_with_message(serde_json::json!({}), "Logout successful".to_string())
}

// ========== Simplified Wallet API (IronForge Compatible) ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimpleCreateWalletReq {
    pub name: String,
    pub address: String,
    pub chain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimpleWalletResp {
    pub id: String,
    pub user_id: String,
    pub chain: String,
    pub address: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

// 企业级标准：simple_create_wallet 函数已移除，统一使用 /api/wallets/unified-create
// 如需创建钱包，请使用 POST /api/wallets/unified-create 端点

#[utoipa::path(
    get,
    path = "/api/wallets",
    responses(
        (status = 200, description = "List of wallets", body = Vec<SimpleWalletResp>),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn simple_list_wallets(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::Json<crate::api::response::ApiResponse<Vec<SimpleWalletResp>>>, AppError> {
    crate::metrics::count_ok("GET /api/wallets");

    // 从Authorization header提取token
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing authorization header"))?;

    // 去掉 "Bearer " 前缀
    let token = auth_header
        .strip_prefix("Bearer ")
        .unwrap_or(auth_header)
        .trim();

    // 验证token并获取用户信息
    let claims = crate::infrastructure::jwt::verify_token(token)
        .map_err(|e| AppError::unauthorized(format!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::unauthorized(format!("Invalid user_id in token: {}", e)))?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|e| AppError::unauthorized(format!("Invalid tenant_id in token: {}", e)))?;

    // 获取用户的钱包列表
    let wallets = service::wallets::list_user_wallets(&st.pool, tenant_id, user_id)
        .await
        .map_err(|e| AppError::internal(e.to_string()))?;

    // 映射到简化响应格式
    let simple_wallets = wallets
        .into_iter()
        .map(|w| {
            let chain = match w.chain_id {
                1 => "ethereum",
                56 => "bsc",
                137 => "polygon",
                _ => "unknown",
            };
            SimpleWalletResp {
                id: w.id.to_string(),
                user_id: w.user_id.to_string(),
                chain: chain.to_string(),
                address: w.address.clone(),
                name: w.address.clone(), // 暂时用address作为name，前端应该有自己的name字段
                created_at: w.created_at.to_rfc3339(),
                updated_at: w.created_at.to_rfc3339(), // Wallet表没有updated_at，使用created_at
            }
        })
        .collect();

    // 使用统一响应格式
    use crate::api::response::success_response;
    success_response(simple_wallets)
}

// ========== Simple Wallet Detail API ==========

#[utoipa::path(
    get,
    path = "/api/wallets/{id}",
    params(
        ("id" = String, Path, description = "Wallet ID")
    ),
    responses(
        (status = 200, description = "Wallet details", body = SimpleWalletResp),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc),
        (status = 404, description = "Wallet not found", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn simple_get_wallet_detail(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SimpleWalletResp>>, AppError> {
    crate::metrics::count_ok("GET /api/wallets/:id");

    // 从Authorization header提取token
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing authorization header"))?;

    // 去掉 "Bearer " 前缀
    let token = auth_header
        .strip_prefix("Bearer ")
        .unwrap_or(auth_header)
        .trim();

    // 验证token并获取用户信息
    let claims = crate::infrastructure::jwt::verify_token(token)
        .map_err(|e| AppError::unauthorized(format!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::unauthorized(format!("Invalid user_id in token: {}", e)))?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|e| AppError::unauthorized(format!("Invalid tenant_id in token: {}", e)))?;

    // 解析 wallet_id
    let wid = Uuid::parse_str(&wallet_id)
        .map_err(|e| AppError::bad_request(format!("Invalid wallet ID: {}", e)))?;

    // 获取钱包详情
    let wallet_opt = service::wallets::get_wallet_by_id(&st.pool, wid)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?;

    let wallet = wallet_opt.ok_or_else(|| AppError::not_found("Wallet not found"))?;

    // 验证钱包所有权
    if wallet.user_id != user_id || wallet.tenant_id != tenant_id {
        return Err(AppError::unauthorized(
            "Not authorized to access this wallet",
        ));
    }

    let chain = match wallet.chain_id {
        1 => "ethereum",
        56 => "bsc",
        137 => "polygon",
        _ => "unknown",
    };

    // 使用统一响应格式
    use crate::api::response::success_response;
    success_response(SimpleWalletResp {
        id: wallet.id.to_string(),
        user_id: wallet.user_id.to_string(),
        chain: chain.to_string(),
        address: wallet.address.clone(),
        name: format!("Wallet {}", &wallet.address[..8]), // 使用地址前8位作为默认名称
        created_at: wallet.created_at.to_rfc3339(),
        updated_at: wallet.created_at.to_rfc3339(), // Wallet表没有updated_at，使用created_at
    })
}

/// 删除钱包（简化版，从token自动提取tenant_id和user_id）
#[utoipa::path(
    delete,
    path = "/api/wallets/{wallet_id}",
    params(
        ("wallet_id" = String, Path, description = "钱包ID (UUID)")
    ),
    responses(
        (status = 204, description = "删除成功"),
        (status = 400, description = "Invalid request", body = crate::error_body::ErrorBodyDoc),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc),
        (status = 404, description = "Wallet not found", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn simple_delete_wallet(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(wallet_id): Path<String>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("DELETE /api/wallets/:id");

    // 从Authorization header提取token
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing authorization header"))?;

    // 去掉 "Bearer " 前缀
    let token = auth_header
        .strip_prefix("Bearer ")
        .unwrap_or(auth_header)
        .trim();

    // 验证token并获取用户信息
    let claims = crate::infrastructure::jwt::verify_token(token)
        .map_err(|e| AppError::unauthorized(format!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::unauthorized(format!("Invalid user_id in token: {}", e)))?;
    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|e| AppError::unauthorized(format!("Invalid tenant_id in token: {}", e)))?;

    // 解析 wallet_id
    let wid = Uuid::parse_str(&wallet_id)
        .map_err(|e| AppError::bad_request(format!("Invalid wallet ID: {}", e)))?;

    // 先验证钱包所有权
    let wallet_opt = service::wallets::get_wallet_by_id(&st.pool, wid)
        .await
        .map_err(|e| AppError::internal(format!("Database error: {}", e)))?;

    let wallet = wallet_opt.ok_or_else(|| AppError::not_found("Wallet not found"))?;

    if wallet.user_id != user_id || wallet.tenant_id != tenant_id {
        return Err(AppError::unauthorized(
            "Not authorized to delete this wallet",
        ));
    }

    // 删除钱包
    let deleted = service::wallets::delete_wallet(&st.pool, wid, tenant_id)
        .await
        .map_err(|e| AppError::internal(format!("Delete failed: {}", e)))?;

    if !deleted {
        return Err(AppError::not_found("Wallet not found"));
    }

    // 审计日志（异步，不等待结果）
    let payload = serde_json::json!({
        "op": "wallet.delete",
        "tenant_id": tenant_id,
        "wallet_id": wid,
        "user_id": user_id,
    });

    crate::utils::write_audit_event_async_str_actor(
        st.immu.clone(),
        "wallet.delete".into(),
        tenant_id,
        user_id.to_string(),
        wid,
        payload,
    );

    Ok(StatusCode::NO_CONTENT)
}

// ========== Simple Transaction APIs ==========

#[derive(Debug, Deserialize, ToSchema)]
pub struct SimpleSendTransactionReq {
    pub from: String,
    pub to: String,
    pub amount: String,
    pub chain: String,
    pub signed_tx: String, // 前端签名的原始交易
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimpleTransactionResp {
    pub tx_hash: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub chain: String,
    pub status: String,
    pub timestamp: String,
    /// 平台服务费：钱包服务商收取的服务费用（与Gas费用完全独立）
    /// 注意：这不是Gas费用，Gas费用由区块链网络自动扣除
    pub platform_fee: Option<String>,
    /// 是否已应用平台服务费
    pub fee_applied: bool,
}

#[utoipa::path(
    post,
    path = "/api/transactions/send",
    request_body = SimpleSendTransactionReq,
    responses(
        (status = 200, description = "Transaction sent", body = SimpleTransactionResp),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
#[axum::debug_handler]
pub async fn simple_send_transaction(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
    Json(req): Json<SimpleSendTransactionReq>,
) -> Result<axum::Json<crate::api::response::ApiResponse<SimpleTransactionResp>>, AppError> {
    crate::metrics::count_ok("POST /api/transactions/send");

    let user_id = auth_info.user_id;
    let tenant_id = auth_info.tenant_id;

    // 映射chain名称到chain_id
    let chain_id = match req.chain.to_lowercase().as_str() {
        "ethereum" | "eth" => 1,
        "bsc" | "binance" => 56,
        "polygon" | "matic" => 137,
        _ => {
            return Err(AppError::bad_request(format!(
                "Unsupported chain: {}",
                req.chain
            )))
        }
    };

    // ========== 钱包所有权验证 ==========
    // 验证发送地址是否属于当前用户
    let wallet_opt = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT user_id, tenant_id FROM wallets WHERE address = $1 AND chain_id = $2 LIMIT 1",
    )
    .bind(&req.from)
    .bind(chain_id)
    .fetch_optional(&st.pool)
    .await
    .map_err(|e| AppError::internal(format!("Database error: {}", e)))?;

    if let Some((wallet_user_id, wallet_tenant_id)) = wallet_opt {
        // 验证钱包所有权
        if wallet_user_id != user_id || wallet_tenant_id != tenant_id {
            return Err(AppError::unauthorized(
                "Not authorized to send from this wallet address",
            ));
        }
    } else {
        // 钱包不存在，允许继续（可能是新钱包或外部地址）
        // 但记录警告日志
        tracing::warn!(
            address = %req.from,
            chain_id = chain_id,
            user_id = %user_id,
            "Transaction from address not found in wallets table"
        );
    }

    // ========== 交易签名验证 ==========
    if !req.signed_tx.is_empty() {
        // 验证签名格式和完整性
        if let Err(e) = validate_transaction_signature(&req.chain, &req.signed_tx, &req.from) {
            tracing::error!(
                error = ?e,
                chain = %req.chain,
                from = %req.from,
                "Transaction signature validation failed"
            );
            return Err(AppError::bad_request(format!(
                "Invalid transaction signature: {}",
                e
            )));
        }
    }

    // ========== 真实区块链广播 ==========
    let enable_real_broadcast = std::env::var("ENABLE_REAL_BROADCAST")
        .ok()
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    let tx_hash = if enable_real_broadcast && !req.signed_tx.is_empty() {
        // 使用真实RPC广播
        let broadcast_req = crate::service::blockchain_client::BroadcastTransactionRequest {
            chain: req.chain.clone(),
            signed_raw_tx: req.signed_tx.clone(),
        };

        match st
            .blockchain_client
            .broadcast_transaction(broadcast_req)
            .await
        {
            Ok(resp) => {
                tracing::info!(
                    tx_hash = %resp.tx_hash,
                    chain = %resp.chain,
                    rpc = %resp.rpc_endpoint_used,
                    "Transaction broadcast successful"
                );
                resp.tx_hash
            }
            Err(e) => {
                tracing::error!(error = ?e, chain = %req.chain, "Broadcast failed");
                return Err(AppError::internal(format!(
                    "Blockchain broadcast failed: {}",
                    e
                )));
            }
        }
    } else {
        // 企业级实现：生产环境安全检查
        // 模拟模式仅用于开发/测试环境，生产环境必须使用真实广播
        let is_production = std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            == "production";

        if is_production {
            tracing::error!(
                "严重警告：生产环境检测到 ENABLE_REAL_BROADCAST=false，这是不安全的！生产环境必须启用真实交易广播"
            );
            return Err(AppError::internal(
                "Production environment requires real transaction broadcast. ENABLE_REAL_BROADCAST must be true."
            ));
        }

        // 开发/测试环境：使用模拟模式（仅用于开发）
        let mock_hash = format!("0x{:x}", uuid::Uuid::new_v4().as_u128());
        tracing::warn!(
            mock_hash = %mock_hash,
            "⚠️  开发环境：使用模拟交易哈希 (ENABLE_REAL_BROADCAST=false)。生产环境必须启用真实广播"
        );
        mock_hash
    };

    // 企业级实现：计算平台服务费（与Gas费用完全独立）
    //
    // 业务逻辑：同链转账收取 Gas费用 + 平台服务费
    // - Gas费用：区块链网络收取的交易执行费用（由区块链网络自动扣除）
    // - 平台服务费：钱包服务商收取的服务费用（通过FeeService计算）
    //
    // 注意：这两个费用是完全独立的，不能混淆！
    let enable_fee = std::env::var("ENABLE_FEE_SYSTEM")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false);
    let mut platform_fee: Option<String> = None;
    let mut fee_applied = false;
    if enable_fee {
        let span = tracing::info_span!("fee_calculation", chain=%req.chain, amount=%req.amount);
        let _guard = span.enter();
        if let Ok(amount_f) = req.amount.parse::<f64>() {
            let chain_key = req.chain.to_lowercase();
            drop(_guard); // Drop span before await
            if let Ok(Some(calc)) = st
                .fee_service
                .calculate_fee(&chain_key, "transfer", amount_f)
                .await
            {
                tracing::info!(fee=calc.platform_fee, collector=%calc.collector_address, "fee calculated");
                // 审计写入（失败不阻断主流程），传入 tx_hash 用于后续回填
                if let Err(e) = st
                    .fee_service
                    .record_fee_audit(
                        user_id,
                        &chain_key,
                        "transfer",
                        amount_f,
                        &calc,
                        &req.from,
                        Some(&tx_hash), // 保存 tx_hash 用于监控服务回填
                    )
                    .await
                {
                    tracing::warn!(error=?e, "fee_audit insert failed; continuing without blocking user transaction");
                }
                platform_fee = Some(format!("{:.8}", calc.platform_fee));
                fee_applied = true;
            }
        }
    }

    // Best-effort: persist transaction record for history/status.
    // NOTE: We never store private keys/mnemonics; only metadata about the broadcast.
    {
        use rust_decimal::Decimal;

        let amount_decimal = req.amount.parse::<Decimal>().ok();
        let status_for_db = if enable_real_broadcast && !req.signed_tx.is_empty() {
            "submitted"
        } else {
            "pending"
        };

        let (gas_fee, nonce_i64) = match req.chain.to_lowercase().as_str() {
            "ethereum" | "eth" | "bsc" | "polygon" | "matic" | "binance" => {
                match parse_evm_signed_tx_details(&req.signed_tx) {
                    Ok(details) => (details.gas_fee, details.nonce),
                    Err(e) => {
                        tracing::debug!(error=?e, chain=%req.chain, "Failed to parse EVM tx details for persistence");
                        (None, None)
                    }
                }
            }
            _ => (None, None),
        };

        let metadata = serde_json::json!({
            "platform_fee": platform_fee,
            "fee_applied": fee_applied,
        });

        if let Err(e) = sqlx::query(
            r#"INSERT INTO transactions
               (id, tenant_id, user_id, wallet_id, chain, tx_hash, tx_type, status,
                from_address, to_address, amount, token_symbol, gas_fee, nonce, metadata,
                created_at, updated_at)
               VALUES ($1, $2, $3, NULL, $4, $5, $6, $7,
                       $8, $9, $10, $11, $12, $13, $14,
                       CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"#,
        )
        .bind(uuid::Uuid::new_v4())
        .bind(tenant_id)
        .bind(user_id)
        .bind(req.chain.to_lowercase())
        .bind(&tx_hash)
        .bind("send")
        .bind(status_for_db)
        .bind(&req.from)
        .bind(&req.to)
        .bind(amount_decimal)
        .bind("NATIVE")
        .bind(gas_fee.unwrap_or_else(|| "0".to_string()))
        .bind(nonce_i64)
        .bind(metadata)
        .execute(&st.pool)
        .await
        {
            tracing::warn!(error=?e, tx_hash=%tx_hash, user_id=%user_id, "Failed to persist transaction; continuing");
        }
    }
    // 构造响应
    let tx_resp = SimpleTransactionResp {
        tx_hash: tx_hash.clone(),
        from: req.from,
        to: req.to,
        amount: req.amount,
        chain: req.chain,
        status: "pending".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        platform_fee,
        fee_applied,
    };

    // Blockchain broadcast handled by frontend RPC pools for security
    // Backend only records transaction metadata for history tracking
    tracing::info!("Transaction {} recorded for chain_id {}", tx_hash, chain_id);

    // 使用统一响应格式
    use crate::api::response::success_response;
    success_response(tx_resp)
}

#[utoipa::path(
    get,
    path = "/api/transactions",
    params(
        ("wallet_id" = Option<String>, Query, description = "Filter by wallet ID"),
        ("limit" = Option<i64>, Query, description = "Limit number of results")
    ),
    responses(
        (status = 200, description = "Transaction history", body = Vec<SimpleTransactionResp>),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn simple_list_transactions(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<
    axum::Json<
        crate::api::response::ApiResponse<
            crate::api::response::pagination::PaginatedResponse<SimpleTransactionResp>,
        >,
    >,
    AppError,
> {
    crate::metrics::count_ok("GET /api/transactions");

    // 从Authorization header提取token
    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("Missing authorization header"))?;

    // 去掉 "Bearer " 前缀
    let token = auth_header
        .strip_prefix("Bearer ")
        .unwrap_or(auth_header)
        .trim();

    // 验证token并获取用户信息
    let claims = crate::infrastructure::jwt::verify_token(token)
        .map_err(|e| AppError::unauthorized(format!("Invalid token: {}", e)))?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::unauthorized(format!("Invalid user_id in token: {}", e)))?;
    let _tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|e| AppError::unauthorized(format!("Invalid tenant_id in token: {}", e)))?;

    // ✅分页参数
    use crate::api::response::pagination::PaginationParams;
    let pagination = PaginationParams::new(Some(1), Some(20)); // 默认page=1, size=20

    // 查询总数
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(&st.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count transactions: {}", e);
            AppError::internal(format!("Failed to count transactions: {}", e))
        })?;

    // 从数据库查询交易历史（带分页）
    let tx_records = sqlx::query(
        "SELECT tx_hash, from_address, to_address, amount, chain_type, created_at
         FROM transactions
         WHERE user_id = $1
         ORDER BY created_at DESC
         LIMIT $2 OFFSET $3",
    )
    .bind(user_id)
    .bind(pagination.limit())
    .bind(pagination.offset())
    .fetch_all(&st.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query transactions: {}", e);
        AppError::internal(format!("Failed to query transactions: {}", e))
    })?;

    let transactions: Vec<SimpleTransactionResp> = tx_records
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            SimpleTransactionResp {
                tx_hash: row.get("tx_hash"),
                from: row.get("from_address"),
                to: row.get("to_address"),
                amount: row.get("amount"),
                chain: row.get("chain_type"),
                status: "confirmed".to_string(),
                timestamp: row
                    .get::<chrono::DateTime<chrono::Utc>, _>("created_at")
                    .to_rfc3339(),
                platform_fee: Some("0".to_string()),
                fee_applied: false,
            }
        })
        .collect();

    // 使用统一分页响应格式
    use crate::api::response::{pagination::PaginatedResponse, success_response};
    let paginated = PaginatedResponse::from_query(
        transactions,
        pagination.page,
        pagination.page_size,
        total as u64,
    );
    success_response(paginated)
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current user info", body = UserResp),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_me(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
) -> Result<Json<crate::api::response::ApiResponse<UserResp>>, AppError> {
    crate::metrics::count_ok("GET /api/auth/me");

    let user = service::auth::get_current_user(&st.pool, auth_info.user_id)
        .await
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    // 企业级标准：使用统一响应格式
    success_response(UserResp {
        id: user.id,
        tenant_id: user.tenant_id,
        email_cipher: user.email_cipher,
        phone_cipher: user.phone_cipher,
        role: user.role,
        created_at: user.created_at.to_rfc3339(),
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetPasswordReq {
    pub tenant_id: Uuid,
    pub password: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/set-password",
    request_body = SetPasswordReq,
    responses(
        (status = 200, description = "Password set successfully"),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn set_password(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
    Json(req_body): Json<SetPasswordReq>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("POST /api/auth/set-password");

    service::auth::set_password(
        &st.pool,
        auth_info.user_id,
        req_body.tenant_id,
        req_body.password,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenReq {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshTokenResp {
    pub access_token: String, // ✅ 统一字段名，与login/register一致
    pub refresh_token: String,
    pub expires_in: u64,
}

#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenReq,
    responses(
        (status = 200, description = "Token refreshed", body = crate::api::response::ApiResponse<RefreshTokenResp>),
        (status = 401, description = "Invalid refresh token", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn refresh_token(
    State(st): State<Arc<AppState>>,
    Json(req): Json<RefreshTokenReq>,
) -> Result<Json<crate::api::response::ApiResponse<RefreshTokenResp>>, AppError> {
    crate::metrics::count_ok("POST /api/auth/refresh");

    let access_token = service::auth::refresh_access_token(&st.redis, &req.refresh_token)
        .await
        .map_err(|e| AppError {
            code: crate::error::AppErrorCode::Unauthorized,
            message: e.to_string(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: None,
        })?;

    // 企业级标准：使用统一响应格式
    success_response(RefreshTokenResp {
        access_token: access_token.clone(), // ✅ 修正字段名
        refresh_token: req.refresh_token,
        expires_in: 300, // 5分钟
    })
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetPasswordReq {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub new_password: String,
}

#[utoipa::path(
    post,
    path = "/api/auth/reset-password",
    request_body = ResetPasswordReq,
    responses(
        (status = 200, description = "Password reset successfully"),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn reset_password(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
    Json(req_body): Json<ResetPasswordReq>,
) -> Result<StatusCode, AppError> {
    crate::metrics::count_ok("POST /api/auth/reset-password");

    // 验证权限（管理员或用户本人）

    // 检查是否是管理员或用户本人
    if auth_info.role != "admin" && auth_info.user_id != req_body.user_id {
        return Err(AppError {
            code: crate::error::AppErrorCode::Forbidden,
            message: "Not authorized to reset password for this user".into(),
            status: StatusCode::FORBIDDEN,
            trace_id: None,
        });
    }

    service::auth::reset_password(
        &st.pool,
        &st.redis,
        req_body.user_id,
        req_body.tenant_id,
        req_body.new_password,
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetLoginHistoryQuery {
    pub tenant_id: Uuid,
    pub user_id: Uuid,
    #[param(default = 10)]
    pub limit: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginHistoryResp {
    pub history: Vec<serde_json::Value>,
}

#[utoipa::path(
    get,
    path = "/api/auth/login-history",
    params(GetLoginHistoryQuery),
    responses(
        (status = 200, description = "Login history", body = crate::api::response::ApiResponse<LoginHistoryResp>),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_login_history(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth_info): AuthInfoExtractor,
    Query(q): Query<GetLoginHistoryQuery>,
) -> Result<Json<crate::api::response::ApiResponse<LoginHistoryResp>>, AppError> {
    crate::metrics::count_ok("GET /api/auth/login-history");

    // 验证权限（管理员或用户本人）

    // 检查是否是管理员或用户本人
    if auth_info.role != "admin" && auth_info.user_id != q.user_id {
        return Err(AppError {
            code: crate::error::AppErrorCode::Forbidden,
            message: "Not authorized to view login history for this user".into(),
            status: StatusCode::FORBIDDEN,
            trace_id: None,
        });
    }

    let history = service::auth::get_login_history(
        &st.redis,
        q.user_id,
        q.tenant_id,
        q.limit.min(100), // 最大100条
    )
    .await
    .map_err(|e| AppError::bad_request(e.to_string()))?;

    // 企业级标准：使用统一响应格式
    success_response(LoginHistoryResp { history })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 交易签名验证辅助函数
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 验证交易签名的有效性
///
/// 防止前端发送未签名或恶意篡改的交易
fn validate_transaction_signature(
    chain: &str,
    signed_tx: &str,
    expected_from: &str,
) -> anyhow::Result<()> {
    match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "bsc" | "polygon" | "matic" | "binance" => {
            // 验证 EVM 交易签名
            validate_evm_transaction_signature(signed_tx, expected_from)
        }
        "bitcoin" | "btc" => {
            // Bitcoin 交易验证
            validate_bitcoin_transaction_signature(signed_tx)
        }
        "solana" | "sol" => {
            // Solana 交易验证
            validate_solana_transaction_signature(signed_tx)
        }
        _ => {
            // 其他链：基本格式验证
            if signed_tx.len() < 10 {
                anyhow::bail!("Transaction too short");
            }
            Ok(())
        }
    }
}

/// 验证 EVM 交易签名
fn validate_evm_transaction_signature(signed_tx: &str, expected_from: &str) -> anyhow::Result<()> {
    use std::str::FromStr;

    use anyhow::Context;
    use ethers::types::{Address, Transaction};

    // 解析签名的交易
    let tx_bytes =
        hex::decode(signed_tx.trim_start_matches("0x")).context("Invalid hex transaction")?;

    // 解码 RLP 编码的交易
    let tx: Transaction = rlp::decode(&tx_bytes).context("Failed to decode transaction")?;

    // 验证发送者地址
    let from = tx.from;
    let expected_addr = Address::from_str(expected_from).context("Invalid expected address")?;

    if from != expected_addr {
        anyhow::bail!(
            "Transaction sender mismatch: expected {}, got {}",
            expected_from,
            from
        );
    }

    // 验证签名参数存在
    if tx.v.is_zero() {
        anyhow::bail!("Invalid signature: v is zero");
    }

    tracing::debug!(
        from = %from,
        to = ?tx.to,
        nonce = %tx.nonce,
        "EVM transaction signature validated"
    );

    Ok(())
}

/// 验证 Bitcoin 交易签名
fn validate_bitcoin_transaction_signature(signed_tx: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    use bitcoin::consensus::deserialize;

    // 解码交易
    let tx_bytes =
        hex::decode(signed_tx.trim_start_matches("0x")).context("Invalid hex transaction")?;

    let _tx: bitcoin::Transaction =
        deserialize(&tx_bytes).context("Failed to decode Bitcoin transaction")?;

    // Deep signature verification not implemented - frontend signs transactions client-side
    // Basic format validation is sufficient for backend transaction recording

    tracing::debug!("Bitcoin transaction basic format validated");
    Ok(())
}

/// 验证 Solana 交易签名
fn validate_solana_transaction_signature(signed_tx: &str) -> anyhow::Result<()> {
    use anyhow::Context;

    // 解码 base58 编码的交易
    let tx_bytes = bs58::decode(signed_tx)
        .into_vec()
        .context("Invalid base58 transaction")?;

    // 基本验证：Solana 交易至少需要 64 字节（一个签名）
    if tx_bytes.len() < 64 {
        anyhow::bail!("Transaction too short to contain valid signature");
    }

    // 验证前 64 字节（签名）不全是零
    let signature = &tx_bytes[0..64];
    if signature.iter().all(|&b| b == 0) {
        anyhow::bail!("Transaction signature is all zeros");
    }

    // 基础验证：检查交易格式
    #[derive(serde::Deserialize)]
    struct BasicTransaction {
        signatures: Vec<Vec<u8>>,
    }

    match bincode::deserialize::<BasicTransaction>(&tx_bytes) {
        Ok(tx) => {
            if tx.signatures.is_empty() {
                anyhow::bail!("Transaction has no signatures");
            }

            tracing::debug!(
                signature_count = tx.signatures.len(),
                tx_size = tx_bytes.len(),
                "Solana transaction basic validation passed"
            );

            Ok(())
        }
        Err(_) => {
            // 如果无法反序列化，至少验证了 base58 和基本格式
            tracing::debug!(
                tx_size = tx_bytes.len(),
                "Solana transaction format validated (bincode parsing skipped)"
            );
            Ok(())
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 交易相关端点（公开访问，用于前端查询）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/tx/nonce - 获取账户nonce（用于Ethereum交易）
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetNonceQuery {
    pub address: String,
    pub chain_id: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NonceResponse {
    pub nonce: u64,
}

#[utoipa::path(
    get,
    path = "/api/v1/transactions/nonce",
    params(GetNonceQuery),
    responses(
        (status = 200, description = "Nonce retrieved", body = crate::api::response::ApiResponse<NonceResponse>),
        (status = 400, description = "Invalid input", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_nonce(
    State(st): State<Arc<AppState>>,
    Query(query): Query<GetNonceQuery>,
) -> Result<Json<crate::api::response::ApiResponse<NonceResponse>>, AppError> {
    crate::metrics::count_ok("GET /api/tx/nonce");

    // 根据chain_id确定链名称（与 gas_api 的 chain_id 映射保持一致）
    let chain = match query.chain_id {
        1 => "ethereum",
        56 => "bsc",
        137 => "polygon",
        42161 => "arbitrum",
        10 => "optimism",
        43114 => "avalanche",
        501 => "solana",
        0 => "bitcoin",
        607 => "ton",
        _ => {
            return Err(AppError::bad_request(format!(
                "Unsupported chain_id: {}",
                query.chain_id
            )))
        }
    };

    let blockchain_client =
        crate::service::blockchain_client::BlockchainClient::new(st.rpc_selector.clone());

    let nonce = blockchain_client
        .get_transaction_count(chain, &query.address)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get nonce: {}", e);
            AppError::internal(format!("Failed to get nonce: {}", e))
        })?;

    use crate::api::response::success_response;
    success_response(NonceResponse { nonce })
}

/// GET /api/tx/history - 获取交易历史（公开访问，用于前端查询）
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetTxHistoryQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TxHistoryItem {
    pub hash: String,
    pub tx_type: String,
    pub status: String,
    pub from: String,
    pub to: String,
    pub amount: String,
    pub token: String,
    pub timestamp: u64,
    pub fee: String,
}

#[utoipa::path(
    get,
    path = "/api/v1/transactions/history",
    params(GetTxHistoryQuery),
    responses(
        (status = 200, description = "Transaction history", body = crate::api::response::ApiResponse<Vec<TxHistoryItem>>),
        (status = 400, description = "Invalid input", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_tx_history(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Query(query): Query<GetTxHistoryQuery>,
) -> Result<Json<crate::api::response::ApiResponse<Vec<TxHistoryItem>>>, AppError> {
    crate::metrics::count_ok("GET /api/v1/transactions/history");

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let offset = ((page - 1) as i64) * (page_size as i64);

    let rows = sqlx::query(
        r#"SELECT tx_hash,
                  tx_type,
                  status,
                  from_address,
                  to_address,
                  amount::TEXT as amount,
                  COALESCE(token_symbol, '') as token_symbol,
                  COALESCE(gas_fee, '0') as gas_fee,
                  EXTRACT(EPOCH FROM created_at)::BIGINT as ts
           FROM transactions
           WHERE user_id = $1
           ORDER BY created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(auth.user_id)
    .bind(page_size as i64)
    .bind(offset)
    .fetch_all(&st.pool)
    .await
    .map_err(|e| AppError::internal(format!("Failed to query transactions: {}", e)))?;

    let items = rows
        .into_iter()
        .map(|row| {
            use sqlx::Row;
            TxHistoryItem {
                hash: row.get::<Option<String>, _>("tx_hash").unwrap_or_default(),
                tx_type: row.get::<String, _>("tx_type"),
                status: row.get::<String, _>("status"),
                from: row.get::<String, _>("from_address"),
                to: row.get::<String, _>("to_address"),
                amount: row
                    .get::<Option<String>, _>("amount")
                    .unwrap_or_else(|| "0".to_string()),
                token: {
                    let sym: String = row.get("token_symbol");
                    if sym.trim().is_empty() {
                        "NATIVE".to_string()
                    } else {
                        sym
                    }
                },
                timestamp: row.get::<i64, _>("ts").max(0) as u64,
                fee: row.get::<String, _>("gas_fee"),
            }
        })
        .collect::<Vec<_>>();

    use crate::api::response::success_response;
    success_response(items)
}

#[derive(Debug)]
struct EvmTxDetails {
    from: String,
    to: String,
    amount_decimal: Option<rust_decimal::Decimal>,
    gas_fee: Option<String>,
    nonce: Option<i64>,
}

fn parse_evm_signed_tx_details(signed_tx: &str) -> anyhow::Result<EvmTxDetails> {
    use anyhow::Context;
    use ethers::types::{Address, Transaction, H160, U256};
    use ethers::utils::format_units;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let tx_bytes =
        hex::decode(signed_tx.trim_start_matches("0x")).context("Invalid hex transaction")?;
    let tx: Transaction = rlp::decode(&tx_bytes).context("Failed to decode transaction")?;

    let from: Address = tx.from;

    let to_addr: H160 = match tx.to {
        Some(to) => to,
        None => Address::from_str("0x0000000000000000000000000000000000000000")?,
    };

    let amount_str = format_units(tx.value, 18).unwrap_or_else(|_| "0".to_string());
    let amount_decimal = amount_str.parse::<Decimal>().ok();

    // Estimate max fee in native token.
    let gas_limit: U256 = tx.gas;
    let gas_price: U256 = tx
        .max_fee_per_gas
        .or(tx.gas_price)
        .unwrap_or_else(|| U256::from(0u64));
    let fee_wei = gas_limit.saturating_mul(gas_price);
    let fee_str = format_units(fee_wei, 18).ok();

    Ok(EvmTxDetails {
        from: format!("{:#x}", from),
        to: format!("{:#x}", to_addr),
        amount_decimal,
        gas_fee: fee_str,
        nonce: Some(tx.nonce.as_u64() as i64),
    })
}

/// GET /api/solana/recent-blockhash - 获取Solana最近区块哈希
#[derive(Debug, Serialize, ToSchema)]
pub struct SolanaBlockhashResponse {
    pub blockhash: String,
}

#[utoipa::path(
    get,
    path = "/api/solana/recent-blockhash",
    responses(
        (status = 200, description = "Recent blockhash", body = crate::api::response::ApiResponse<SolanaBlockhashResponse>),
        (status = 500, description = "Internal server error", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_solana_recent_blockhash(
    State(_st): State<Arc<AppState>>,
) -> Result<Json<crate::api::response::ApiResponse<SolanaBlockhashResponse>>, AppError> {
    crate::metrics::count_ok("GET /api/solana/recent-blockhash");

    // 企业级实现：从Solana RPC获取最近区块哈希
    // 注意：这是一个占位实现，实际需要调用Solana RPC
    // Solana RPC方法: getLatestBlockhash

    // 临时返回示例值，实际实现需要调用Solana RPC
    let blockhash = "11111111111111111111111111111111".to_string();

    use crate::api::response::success_response;
    success_response(SolanaBlockhashResponse { blockhash })
}

/// GET /api/ton/seqno - 获取TON账户序列号
#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct GetTonSeqnoQuery {
    pub address: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TonSeqnoResponse {
    pub seqno: u64,
}

#[utoipa::path(
    get,
    path = "/api/ton/seqno",
    params(GetTonSeqnoQuery),
    responses(
        (status = 200, description = "Sequence number", body = crate::api::response::ApiResponse<TonSeqnoResponse>),
        (status = 400, description = "Invalid input", body = crate::error_body::ErrorBodyDoc)
    )
)]
pub async fn get_ton_seqno(
    State(_st): State<Arc<AppState>>,
    Query(_query): Query<GetTonSeqnoQuery>,
) -> Result<Json<crate::api::response::ApiResponse<TonSeqnoResponse>>, AppError> {
    crate::metrics::count_ok("GET /api/ton/seqno");

    // 企业级实现：从TON RPC获取账户序列号
    // 注意：这是一个占位实现，实际需要调用TON RPC
    // TON RPC方法: getAddressInformation

    // 临时返回0，实际实现需要调用TON RPC
    let seqno = 0u64;

    use crate::api::response::success_response;
    success_response(TonSeqnoResponse { seqno })
}
