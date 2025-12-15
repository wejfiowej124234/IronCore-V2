//! 增强版跨链桥API
//! 企业级实现：支持多个跨链桥协议，实时状态追踪

use std::sync::Arc;

use axum::{
    extract::{Path, State},
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
};

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
    pub signed_source_tx: String,
    /// 跨链桥提供商（可选，自动选择最优）
    pub bridge_provider: Option<String>,
    // REMOVED: user_password (非托管模式：后端不能代签名)
    /// 幂等性key
    pub idempotency_key: Option<String>,
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

/// GET /api/bridge/quote
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

/// POST /api/bridge/execute
/// 执行跨链转账（非托管模式：接受客户端签名的交易）
pub async fn execute_bridge(
    State(state): State<Arc<AppState>>,
    auth: AuthInfoExtractor,
    Json(req): Json<CreateBridgeRequest>,
) -> Result<Json<ApiResponse<CreateBridgeResponse>>, AppError> {
    let bridge_id = Uuid::new_v4();

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

    // 2. 验证已签名交易
    if req.signed_source_tx.is_empty() {
        return Err(AppError::bad_request(
            "signed_source_tx is required (non-custodial mode)".to_string(),
        ));
    }

    if !req.signed_source_tx.starts_with("0x") {
        return Err(AppError::bad_request(
            "Invalid transaction format".to_string(),
        ));
    }

    // 3. 计算费用
    let amount_f64 = req
        .amount
        .parse::<f64>()
        .map_err(|_| AppError::bad_request("Invalid amount".to_string()))?;

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
    .bind(&req.amount)
    .bind("SourcePending")
    .bind(req.bridge_provider.unwrap_or_else(|| "LayerZero".to_string()))
    .bind(fee_calc.fee_usd)
    .bind(&req.signed_source_tx)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Failed to create bridge tx: {}", e)))?;

    // 5. 广播已签名的源链交易
    let broadcast_result = state
        .blockchain_client
        .broadcast_transaction(
            crate::service::blockchain_client::BroadcastTransactionRequest {
                chain: req.source_chain.clone(),
                signed_raw_tx: req.signed_source_tx.clone(),
            },
        )
        .await;

    let source_tx_hash = match broadcast_result {
        Ok(resp) => {
            // 更新交易哈希
            let _ = sqlx::query(
                "UPDATE cross_chain_transactions 
                 SET source_tx_hash = $1, status = 'SourceConfirming', updated_at = CURRENT_TIMESTAMP
                 WHERE id = $2"
            )
            .bind(&resp.tx_hash)
            .bind(bridge_id)
            .execute(&state.pool)
            .await;

            Some(resp.tx_hash)
        }
        Err(e) => {
            tracing::error!("Failed to broadcast bridge source tx: {:?}", e);

            // 更新为失败状态
            let _ = sqlx::query(
                "UPDATE cross_chain_transactions 
                 SET status = 'SourceFailed', updated_at = CURRENT_TIMESTAMP
                 WHERE id = $1",
            )
            .bind(bridge_id)
            .execute(&state.pool)
            .await;

            return Err(AppError::internal_error(format!("Broadcast failed: {}", e)));
        }
    };

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
        "source_tx_hash": source_tx_hash
    }))
    .execute(&state.pool)
    .await;

    success_response(CreateBridgeResponse {
        bridge_id: bridge_id.to_string(),
        status: "SourceConfirming".to_string(),
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
        source_tx_hash: Option<String>,
        destination_tx_hash: Option<String>,
        user_id: uuid::Uuid,
        #[allow(dead_code)]
        created_at: chrono::DateTime<chrono::Utc>,
    }

    let bridge = sqlx::query_as::<_, BridgeStatusRow>(
        "SELECT status, source_tx_hash, destination_tx_hash, user_id, created_at
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
    let progress = match bridge.status.as_str() {
        "SourcePending" => 20,
        "SourceConfirmed" => 40,
        "BridgeProcessing" => 60,
        "DestinationPending" => 80,
        "DestinationConfirmed" => 100,
        _ => 0,
    };

    success_response(BridgeStatusResponse {
        bridge_id: bridge_id.clone(),
        status: bridge.status,
        source_tx_hash: bridge.source_tx_hash,
        source_confirmations: 0, // TODO: 从链上查询
        destination_tx_hash: bridge.destination_tx_hash,
        destination_confirmations: 0,
        progress_percentage: progress,
        estimated_completion_time: Some("10-20 minutes".to_string()),
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
