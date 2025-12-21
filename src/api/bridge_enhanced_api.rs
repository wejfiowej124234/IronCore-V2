//! 增强版跨链桥API
//! 企业级实现：支持多个跨链桥协议，实时状态追踪

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
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
// Bridge History
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct BridgeHistoryQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeHistoryItem {
    pub bridge_id: String,
    pub status: String,
    pub source_chain: String,
    pub source_address: String,
    pub destination_chain: String,
    pub destination_address: String,
    pub token_symbol: String,
    pub amount: String,
    pub bridge_provider: Option<String>,
    pub fee_paid_usd: Option<f64>,
    pub source_tx_hash: Option<String>,
    pub destination_tx_hash: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeHistoryResponse {
    pub bridges: Vec<BridgeHistoryItem>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateBridgeRequest {
    /// 源链
    pub source_chain: String,
    /// 源地址（用户钱包）
    pub source_address: String,
    /// 目标链
    pub destination_chain: String,
    /// 目标地址（接收地址）
    pub destination_address: String,
    /// Token符号
    pub token_symbol: String,
    /// 金额
    pub amount: String,
    /// 已签名的源链交易（客户端签名）
    /// 非托管模式：客户端必须先签名交易，后端只负责广播
    ///
    /// 兼容字段：如果提供了 `route_steps`，则可省略。
    #[serde(default)]
    pub signed_source_tx: Option<String>,

    /// 可选：route-based 执行步骤（Phase A：approve + swap）。
    /// 客户端负责签名，后端负责按顺序广播并追踪。
    #[serde(default)]
    pub route_steps: Vec<SignedRouteStep>,
    /// 跨链桥提供商（可选，自动选择最优）
    pub bridge_provider: Option<String>,
    // REMOVED: user_password (非托管模式：后端不能代签名)
    /// 幂等性key
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SignedRouteStep {
    /// "approve" | "swap" | ...
    pub kind: String,
    /// step 所在链（Phase A: 与 source_chain 相同）
    pub chain: String,
    /// Hex encoded signed transaction (0x...)
    pub signed_tx: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateBridgeResponse {
    pub bridge_id: String,
    pub status: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub amount: String,
    pub estimated_arrival_time: String,
    pub fee_info: BridgeFeeInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeFeeInfo {
    pub bridge_fee_usd: f64,
    pub source_gas_fee_usd: f64,
    pub destination_gas_fee_usd: f64,
    pub total_fee_usd: f64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeStatusResponse {
    pub bridge_id: String,
    pub status: String,
    pub source_tx_hash: Option<String>,
    pub source_confirmations: u32,
    pub destination_tx_hash: Option<String>,
    pub destination_confirmations: u32,
    pub progress_percentage: u8,
    pub estimated_completion_time: Option<String>,

    /// Phase A: route-based execution hashes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approve_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swap_tx_hash: Option<String>,
    /// Optional: per-step hashes (for UI/debug)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_step_hashes: Option<Vec<RouteStepHash>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RouteStepHash {
    pub kind: String,
    pub tx_hash: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BridgeQuoteResponse {
    pub source_chain: String,
    pub destination_chain: String,
    pub token_symbol: String,
    pub amount: String,
    pub estimated_receive_amount: String,
    pub fee_breakdown: BridgeFeeInfo,
    pub estimated_time_minutes: u32,
    pub recommended_provider: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/quote", get(get_bridge_quote))
        .route("/execute", post(execute_bridge))
        .route("/:bridge_id", get(get_bridge_status))
        .route("/:bridge_id/cancel", post(cancel_bridge))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/v1/bridge/quote
/// 获取跨链桥报价（企业级：比较多个桥选最优）
pub async fn get_bridge_quote(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<BridgeQuoteResponse>>, AppError> {
    let source_chain = params
        .get("source_chain")
        .ok_or_else(|| AppError::bad_request("Missing source_chain".to_string()))?;

    let destination_chain = params
        .get("destination_chain")
        .ok_or_else(|| AppError::bad_request("Missing destination_chain".to_string()))?;

    let token_symbol = params
        .get("token_symbol")
        .ok_or_else(|| AppError::bad_request("Missing token_symbol".to_string()))?;

    let amount = params
        .get("amount")
        .ok_or_else(|| AppError::bad_request("Missing amount".to_string()))?;

    // 使用统一费率服务计算跨链桥费用
    let fee_service = crate::service::unified_fee_config_service::UnifiedFeeConfigService::new(
        state.pool.clone(),
    );

    let amount_f64 = amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;

    let fee_calc = fee_service
        .calculate_fee(
            crate::service::unified_fee_config_service::FeeType::BridgeFee,
            amount_f64,
            Some(source_chain),
        )
        .await
        .map_err(|e| AppError::internal_error(format!("Fee calculation failed: {}", e)))?;

    // 估算Gas费用（源链和目标链）
    let source_gas_fee = 5.0; // TODO: 实际估算
    let dest_gas_fee = 3.0;

    success_response(BridgeQuoteResponse {
        source_chain: source_chain.clone(),
        destination_chain: destination_chain.clone(),
        token_symbol: token_symbol.clone(),
        amount: amount.clone(),
        estimated_receive_amount: format!("{:.6}", amount_f64 - fee_calc.fee_usd),
        fee_breakdown: BridgeFeeInfo {
            bridge_fee_usd: fee_calc.fee_usd,
            source_gas_fee_usd: source_gas_fee,
            destination_gas_fee_usd: dest_gas_fee,
            total_fee_usd: fee_calc.fee_usd + source_gas_fee + dest_gas_fee,
        },
        estimated_time_minutes: 15,
        recommended_provider: "LayerZero".to_string(),
    })
}

/// POST /api/v1/bridge/execute
/// 执行跨链转账（非托管模式：接受客户端签名的交易）
pub async fn execute_bridge(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    Json(req): Json<CreateBridgeRequest>,
) -> Result<Json<ApiResponse<CreateBridgeResponse>>, AppError> {
    let bridge_id = Uuid::new_v4();

    // Default response status (may change to Failed if broadcast fails)
    let mut response_status = "SourcePending".to_string();

    // 1. 验证链和地址
    crate::utils::address_validator::AddressValidator::validate(
        &req.source_chain,
        &req.source_address,
    )
    .map_err(|e| AppError::bad_request(format!("Invalid source address: {}", e)))?;

    crate::utils::address_validator::AddressValidator::validate(
        &req.destination_chain,
        &req.destination_address,
    )
    .map_err(|e| AppError::bad_request(format!("Invalid destination address: {}", e)))?;

    // 2. 验证已签名交易（单笔 or route steps）
    let has_steps = !req.route_steps.is_empty();
    let has_single = req
        .signed_source_tx
        .as_deref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);

    if !has_steps && !has_single {
        return Err(AppError::bad_request(
            "Either signed_source_tx or route_steps is required (non-custodial mode)".to_string(),
        ));
    }

    // 基础 hex 校验 helper
    let validate_signed_hex = |signed: &str| -> Result<(), AppError> {
        if !signed.starts_with("0x") {
            return Err(AppError::bad_request(
                "Invalid transaction format".to_string(),
            ));
        }
        let hex_part = signed.strip_prefix("0x").unwrap_or(signed);
        let raw_bytes = hex::decode(hex_part)
            .map_err(|_| AppError::bad_request("Invalid transaction hex".to_string()))?;
        if raw_bytes.len() < 10 {
            return Err(AppError::bad_request(
                "Invalid raw transaction: too short".to_string(),
            ));
        }
        Ok(())
    };

    if has_steps {
        for step in &req.route_steps {
            if step.signed_tx.trim().is_empty() {
                return Err(AppError::bad_request(
                    "route_steps contains empty signed_tx".to_string(),
                ));
            }
            // Phase A: enforce chain match to reduce footguns
            if !step.chain.eq_ignore_ascii_case(&req.source_chain) {
                return Err(AppError::bad_request(format!(
                    "route_steps.chain must match source_chain (got {}, expected {})",
                    step.chain, req.source_chain
                )));
            }
            validate_signed_hex(step.signed_tx.trim())?;
        }
    }

    if has_single {
        validate_signed_hex(req.signed_source_tx.as_deref().unwrap().trim())?;
    }

    // 3. 计算费用
    // DB 里 amount/fee_paid 是 DECIMAL；费用计算使用 f64
    let amount_decimal: Decimal = req
        .amount
        .parse()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;

    let amount_f64 = amount_decimal
        .to_f64()
        .ok_or_else(|| AppError::bad_request("Invalid amount".to_string()))?;

    let fee_service = crate::service::unified_fee_config_service::UnifiedFeeConfigService::new(
        state.pool.clone(),
    );
    let fee_calc = fee_service
        .calculate_fee(
            crate::service::unified_fee_config_service::FeeType::BridgeFee,
            amount_f64,
            Some(&req.source_chain),
        )
        .await
        .map_err(|e| AppError::internal_error(format!("Fee calculation failed: {}", e)))?;

    let fee_paid_decimal = Decimal::from_f64_retain(fee_calc.fee_usd).unwrap_or(Decimal::ZERO);

    // 4. 创建跨链交易记录
    let _ = sqlx::query(
        "INSERT INTO cross_chain_transactions
         (id, user_id, tenant_id, source_chain, source_address, destination_chain,
          destination_address, token_symbol, amount, status, bridge_provider, fee_paid, source_tx_hash, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, CURRENT_TIMESTAMP)"
    )
    .bind(bridge_id)
    .bind(auth.0.user_id)
    .bind(auth.0.tenant_id)
    .bind(&req.source_chain)
    .bind(&req.source_address)
    .bind(&req.destination_chain)
    .bind(&req.destination_address)
    .bind(&req.token_symbol)
    .bind(amount_decimal)
    .bind("SourcePending")
    .bind(req.bridge_provider.unwrap_or_else(|| "LayerZero".to_string()))
    .bind(fee_paid_decimal)
    // ✅ 不持久化用户签名交易原文（避免将敏感raw_tx写入DB）
    .bind(Option::<String>::None)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to create bridge tx: {}", e)))?;

    // 5. 广播已签名的源链交易（单笔 or steps）
    let mut source_tx_hash: Option<String> = None;
    let mut approve_tx_hash: Option<String> = None;
    let mut swap_tx_hash: Option<String> = None;
    let mut route_step_hashes: Vec<RouteStepHash> = Vec::new();

    if has_steps {
        let mut last_ok: Option<String> = None;
        let mut swap_hash: Option<String> = None;

        for step in &req.route_steps {
            let broadcast_result = state
                .blockchain_client
                .broadcast_transaction(
                    crate::service::blockchain_client::BroadcastTransactionRequest {
                        chain: req.source_chain.clone(),
                        signed_raw_tx: step.signed_tx.clone(),
                    },
                )
                .await;

            match broadcast_result {
                Ok(resp) => {
                    last_ok = Some(resp.tx_hash.clone());
                    if step.kind.eq_ignore_ascii_case("swap") {
                        swap_hash = Some(resp.tx_hash.clone());
                    }

                    if step.kind.eq_ignore_ascii_case("approve") {
                        approve_tx_hash = Some(resp.tx_hash.clone());
                    }
                    if step.kind.eq_ignore_ascii_case("swap") {
                        swap_tx_hash = Some(resp.tx_hash.clone());
                    }

                    route_step_hashes.push(RouteStepHash {
                        kind: step.kind.clone(),
                        tx_hash: resp.tx_hash.clone(),
                    });
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    let msg_lc = msg.to_lowercase();
                    if msg_lc.contains("invalid raw transaction") || msg_lc.contains("too short") {
                        return Err(AppError::bad_request(format!("Broadcast failed: {}", msg)));
                    }
                    tracing::error!(
                        "Failed to broadcast bridge route step: kind={} err={:?}",
                        step.kind,
                        e
                    );

                    response_status = "Failed".to_string();
                    let _ = sqlx::query(
                        "UPDATE cross_chain_transactions
                          SET status = 'Failed', updated_at = CURRENT_TIMESTAMP
                         WHERE id = $1",
                    )
                    .bind(bridge_id)
                    .execute(&state.pool)
                    .await;

                    // Best-effort: store last successful tx hash for debugging
                    source_tx_hash = last_ok.clone();
                    if let Some(ref h) = source_tx_hash {
                        let _ = sqlx::query(
                            "UPDATE cross_chain_transactions
                             SET source_tx_hash = $1, updated_at = CURRENT_TIMESTAMP
                             WHERE id = $2",
                        )
                        .bind(h)
                        .bind(bridge_id)
                        .execute(&state.pool)
                        .await;
                    }

                    break;
                }
            }
        }

        // Prefer swap tx hash as primary source_tx_hash
        if response_status != "Failed" {
            source_tx_hash = swap_hash.or(last_ok);
            if let Some(ref h) = source_tx_hash {
                let _ = sqlx::query(
                    "UPDATE cross_chain_transactions
                     SET source_tx_hash = $1, status = 'SourcePending', updated_at = CURRENT_TIMESTAMP
                     WHERE id = $2",
                )
                .bind(h)
                .bind(bridge_id)
                .execute(&state.pool)
                .await;
            }

            // Persist step hashes into metadata (best-effort)
            let metadata = serde_json::json!({
                "route": {
                    "approve_tx_hash": approve_tx_hash,
                    "swap_tx_hash": swap_tx_hash,
                    "step_hashes": route_step_hashes,
                }
            });

            let _ = sqlx::query(
                "UPDATE cross_chain_transactions
                 SET metadata = COALESCE(metadata, '{}'::jsonb) || $1::jsonb,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = $2",
            )
            .bind(metadata)
            .bind(bridge_id)
            .execute(&state.pool)
            .await;
        }
    } else {
        let signed = req.signed_source_tx.clone().unwrap_or_default();
        let broadcast_result = state
            .blockchain_client
            .broadcast_transaction(
                crate::service::blockchain_client::BroadcastTransactionRequest {
                    chain: req.source_chain.clone(),
                    signed_raw_tx: signed,
                },
            )
            .await;

        source_tx_hash = match broadcast_result {
            Ok(resp) => {
                // 更新交易哈希
                let _ = sqlx::query(
                    "UPDATE cross_chain_transactions
                     SET source_tx_hash = $1, status = 'SourcePending', updated_at = CURRENT_TIMESTAMP
                     WHERE id = $2",
                )
                .bind(&resp.tx_hash)
                .bind(bridge_id)
                .execute(&state.pool)
                .await;

                Some(resp.tx_hash)
            }
            Err(e) => {
                let msg = format!("{}", e);
                let msg_lc = msg.to_lowercase();
                if msg_lc.contains("invalid raw transaction") || msg_lc.contains("too short") {
                    return Err(AppError::bad_request(format!("Broadcast failed: {}", msg)));
                }

                tracing::error!("Failed to broadcast bridge source tx: {:?}", e);

                // Even if broadcast fails, the bridge request itself was created and persisted.
                // Return bridge_id with a failed status so clients can show it in history/status UI.
                response_status = "Failed".to_string();

                // 更新为失败状态
                let _ = sqlx::query(
                    "UPDATE cross_chain_transactions
                      SET status = 'Failed', updated_at = CURRENT_TIMESTAMP
                     WHERE id = $1",
                )
                .bind(bridge_id)
                .execute(&state.pool)
                .await;

                None
            }
        };
    }

    // 6. 记录审计日志
    let _ = sqlx::query(
        "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
         VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
    )
    .bind("BRIDGE_TX_CREATED")
    .bind("bridge")
    .bind(bridge_id)
    .bind(serde_json::json!({
    "source_chain": req.source_chain,
    "destination_chain": req.destination_chain,
        "amount": req.amount,
        "source_tx_hash": source_tx_hash,
        "route_steps": req.route_steps
    }))
    .execute(&state.pool)
    .await;

    success_response(CreateBridgeResponse {
        bridge_id: bridge_id.to_string(),
        status: response_status,
        source_chain: req.source_chain,
        destination_chain: req.destination_chain,
        amount: req.amount,
        estimated_arrival_time: "15-30 minutes".to_string(),
        fee_info: BridgeFeeInfo {
            bridge_fee_usd: fee_calc.fee_usd,
            source_gas_fee_usd: 5.0,
            destination_gas_fee_usd: 3.0,
            total_fee_usd: fee_calc.fee_usd + 8.0,
        },
    })
}

/// GET /api/bridge/:bridge_id
/// 查询跨链状态
pub async fn get_bridge_status(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    Path(bridge_id): Path<String>,
) -> Result<Json<ApiResponse<BridgeStatusResponse>>, AppError> {
    let bridge_uuid = Uuid::parse_str(&bridge_id)
        .map_err(|_| AppError::bad_request("Invalid bridge_id".to_string()))?;

    #[derive(sqlx::FromRow)]
    struct BridgeStatusRow {
        status: String,
        source_chain: String,
        source_tx_hash: Option<String>,
        destination_chain: String,
        destination_tx_hash: Option<String>,
        source_confirmations: Option<i64>,
        destination_confirmations: Option<i64>,
        metadata: Option<serde_json::Value>,
        user_id: uuid::Uuid,
        #[allow(dead_code)]
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let bridge = sqlx::query_as::<_, BridgeStatusRow>(
        "SELECT status, source_chain, source_tx_hash,
                destination_chain, destination_tx_hash,
                source_confirmations, destination_confirmations,
                metadata, user_id, created_at
         FROM cross_chain_transactions
         WHERE id = $1",
    )
    .bind(bridge_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| AppError::not_found("Bridge transaction not found".to_string()))?;

    // 验证所有权
    if bridge.user_id != auth.0.user_id {
        return Err(AppError::forbidden(
            "Not your bridge transaction".to_string(),
        ));
    }

    // 计算进度百分比
    let base_progress = match bridge.status.as_str() {
        "SourcePending" => 20,
        "SourceConfirmed" => 40,
        "BridgeProcessing" => 60,
        "DestinationPending" => 80,
        "DestinationConfirmed" => 100,
        _ => 0,
    };

    // ✅ 尝试用链上receipt刷新确认数（失败则降级到DB中的最新值）
    let mut source_confirmations = bridge.source_confirmations.unwrap_or(0).max(0) as u32;
    if let Some(ref tx_hash) = bridge.source_tx_hash {
        if let Ok(Some(receipt)) = state
            .blockchain_client
            .get_transaction_receipt(&bridge.source_chain, tx_hash)
            .await
        {
            // 注意：receipt.confirmations 为 u64
            source_confirmations = receipt.confirmations as u32;
            let _ = sqlx::query(
                "UPDATE cross_chain_transactions
                 SET source_confirmations = $1, updated_at = CURRENT_TIMESTAMP
                 WHERE id = $2",
            )
            .bind(receipt.confirmations as i64)
            .bind(bridge_uuid)
            .execute(&state.pool)
            .await;
        }
    }

    let mut destination_confirmations = bridge.destination_confirmations.unwrap_or(0).max(0) as u32;
    if let Some(ref tx_hash) = bridge.destination_tx_hash {
        if let Ok(Some(receipt)) = state
            .blockchain_client
            .get_transaction_receipt(&bridge.destination_chain, tx_hash)
            .await
        {
            destination_confirmations = receipt.confirmations as u32;
            let _ = sqlx::query(
                "UPDATE cross_chain_transactions
                 SET destination_confirmations = $1, updated_at = CURRENT_TIMESTAMP
                 WHERE id = $2",
            )
            .bind(receipt.confirmations as i64)
            .bind(bridge_uuid)
            .execute(&state.pool)
            .await;
        }
    }

    let (approve_tx_hash, swap_tx_hash, route_step_hashes) = bridge
        .metadata
        .as_ref()
        .and_then(|m| m.get("route"))
        .map(|route| {
            let approve = route
                .get("approve_tx_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let swap = route
                .get("swap_tx_hash")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let steps = route
                .get("step_hashes")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|it| {
                            let kind = it.get("kind")?.as_str()?.to_string();
                            let tx_hash = it.get("tx_hash")?.as_str()?.to_string();
                            Some(RouteStepHash { kind, tx_hash })
                        })
                        .collect::<Vec<_>>()
                });
            (approve, swap, steps)
        })
        .unwrap_or((None, None, None));

    // ✅ Phase A: step-aware progress based on receipts/confirmations.
    // Best-effort: if RPC calls fail, fall back to base_progress.
    let mut progress_percentage = base_progress;
    if approve_tx_hash.is_some() || swap_tx_hash.is_some() {
        let mut approve_confirmations: u32 = 0;
        let mut swap_confirmations: u32 = 0;

        if let Some(ref approve_hash) = approve_tx_hash {
            // Avoid double RPC calls if approve == source_tx_hash
            if bridge.source_tx_hash.as_deref() == Some(approve_hash.as_str()) {
                approve_confirmations = source_confirmations;
            } else if let Ok(Some(receipt)) = state
                .blockchain_client
                .get_transaction_receipt(&bridge.source_chain, approve_hash)
                .await
            {
                approve_confirmations = receipt.confirmations as u32;
            }
        }

        if let Some(ref swap_hash) = swap_tx_hash {
            // Avoid double RPC calls if swap == source_tx_hash
            if bridge.source_tx_hash.as_deref() == Some(swap_hash.as_str()) {
                swap_confirmations = source_confirmations;
            } else if let Ok(Some(receipt)) = state
                .blockchain_client
                .get_transaction_receipt(&bridge.source_chain, swap_hash)
                .await
            {
                swap_confirmations = receipt.confirmations as u32;
            }
        }

        // Progress model (simple + production-friendly):
        // - route exists: 0→approve sent/confirmed→swap sent/confirmed→destination tx seen/confirmed.
        let mut p: u8 = 0;
        if approve_tx_hash.is_some() {
            p = p.max(10);
        }
        if approve_confirmations > 0 {
            p = p.max(30);
        }
        if swap_tx_hash.is_some() {
            p = p.max(40);
        }
        if swap_confirmations > 0 {
            p = p.max(70);
        }
        if bridge.destination_tx_hash.is_some() {
            p = p.max(80);
        }
        if destination_confirmations > 0 || bridge.status == "DestinationConfirmed" {
            p = 100;
        }

        // For failures, still report best-effort step-based progress.
        progress_percentage = p;
    }

    success_response(BridgeStatusResponse {
        bridge_id: bridge_id.clone(),
        status: bridge.status,
        source_tx_hash: bridge.source_tx_hash,
        source_confirmations,
        destination_tx_hash: bridge.destination_tx_hash,
        destination_confirmations,
        progress_percentage,
        estimated_completion_time: Some("10-20 minutes".to_string()),

        approve_tx_hash,
        swap_tx_hash,
        route_step_hashes,
    })
}

/// GET /api/v1/bridge/history
///
/// 返回当前用户跨链桥历史（分页）
pub async fn get_bridge_history(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    axum::extract::Query(q): axum::extract::Query<BridgeHistoryQuery>,
) -> Result<Json<ApiResponse<BridgeHistoryResponse>>, AppError> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(1)
         FROM cross_chain_transactions
         WHERE tenant_id = $1 AND user_id = $2",
    )
    .bind(auth.0.tenant_id)
    .bind(auth.0.user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to count bridge history: {}", e)))?;

    #[derive(sqlx::FromRow)]
    struct BridgeRow {
        id: uuid::Uuid,
        status: String,
        source_chain: String,
        source_address: String,
        destination_chain: String,
        destination_address: String,
        token_symbol: String,
        amount: String,
        bridge_provider: Option<String>,
        fee_paid: Option<f64>,
        source_tx_hash: Option<String>,
        destination_tx_hash: Option<String>,
        created_at: chrono::DateTime<chrono::Utc>,
        updated_at: Option<chrono::DateTime<chrono::Utc>>,
    }

    let rows = sqlx::query_as::<_, BridgeRow>(
        "SELECT
            id,
            status,
            source_chain,
            source_address,
            destination_chain,
            destination_address,
            token_symbol,
            amount::TEXT as amount,
            bridge_provider,
            fee_paid::FLOAT8 as fee_paid,
            source_tx_hash,
            destination_tx_hash,
            created_at,
            updated_at
         FROM cross_chain_transactions
         WHERE tenant_id = $1 AND user_id = $2
         ORDER BY created_at DESC
         LIMIT $3 OFFSET $4",
    )
    .bind(auth.0.tenant_id)
    .bind(auth.0.user_id)
    .bind(page_size)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to query bridge history: {}", e)))?;

    let bridges = rows
        .into_iter()
        .map(|r| BridgeHistoryItem {
            bridge_id: r.id.to_string(),
            status: r.status,
            source_chain: r.source_chain,
            source_address: r.source_address,
            destination_chain: r.destination_chain,
            destination_address: r.destination_address,
            token_symbol: r.token_symbol,
            amount: r.amount,
            bridge_provider: r.bridge_provider,
            fee_paid_usd: r.fee_paid,
            source_tx_hash: r.source_tx_hash,
            destination_tx_hash: r.destination_tx_hash,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.map(|t| t.to_rfc3339()),
        })
        .collect();

    success_response(BridgeHistoryResponse {
        bridges,
        total,
        page,
        page_size,
    })
}

/// POST /api/bridge/:bridge_id/cancel
/// 取消跨链交易（仅源链未确认时可用）
pub async fn cancel_bridge(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    Path(bridge_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let bridge_uuid = Uuid::parse_str(&bridge_id)
        .map_err(|_| AppError::bad_request("Invalid bridge_id".to_string()))?;

    #[derive(sqlx::FromRow)]
    struct BridgeCancelCheckRow {
        status: String,
        user_id: uuid::Uuid,
    }

    let bridge = sqlx::query_as::<_, BridgeCancelCheckRow>(
        "SELECT status, user_id FROM cross_chain_transactions WHERE id = $1",
    )
    .bind(bridge_uuid)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| AppError::not_found("Bridge transaction not found".to_string()))?;

    if bridge.user_id != auth.0.user_id {
        return Err(AppError::forbidden(
            "Not your bridge transaction".to_string(),
        ));
    }

    if bridge.status != "SourcePending" {
        return Err(AppError::bad_request(
            "Cannot cancel: transaction already processing".to_string(),
        ));
    }

    // 更新状态为已取消
    let _ = sqlx::query(
        "UPDATE cross_chain_transactions SET status = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind("Cancelled")
    .bind(bridge_uuid)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to cancel: {}", e)))?;

    success_response(())
}
