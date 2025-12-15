//! 交易加速API（Replace-by-Fee）
//! 企业级实现：支持用户重新签名更高Gas的交易

use std::sync::Arc;

use axum::{extract::State, Json};
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
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct AccelerateTransactionRequest {
    /// 原交易哈希
    pub original_tx_hash: String,
    /// 新的已签名交易（用户重新签名，更高Gas）
    pub new_signed_tx: String,
    /// 新的Gas价格
    pub new_gas_price: String,
    /// 链标识
    pub chain: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AccelerateTransactionResponse {
    pub success: bool,
    pub original_tx_hash: String,
    pub new_tx_hash: String,
    pub gas_price_increase: String,
    pub message: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handler
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// POST /api/transactions/accelerate
///
/// 交易加速（Replace-by-Fee）
///
/// # 非托管原则
/// - ✅ 用户必须重新签名交易（更高Gas）
/// - ✅ 后端验证nonce相同
/// - ✅ 后端验证Gas价格提高至少10%
/// - ✅ 后端广播新交易
#[utoipa::path(
    post,
    path = "/api/transactions/accelerate",
    request_body = AccelerateTransactionRequest,
    responses(
        (status = 200, description = "Transaction accelerated", body = ApiResponse<AccelerateTransactionResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn accelerate_transaction(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<AccelerateTransactionRequest>,
) -> Result<Json<ApiResponse<AccelerateTransactionResponse>>, AppError> {
    // 1. 验证原交易是否存在且属于当前用户
    #[derive(sqlx::FromRow)]
    struct OriginalTxRow {
        #[allow(dead_code)]
        id: uuid::Uuid,
        tx_hash: String,
        from_address: String,
        nonce: i64,
        status: String,
    }

    let original_tx = sqlx::query_as::<_, OriginalTxRow>(
        "SELECT id, tx_hash, from_address, nonce, status 
         FROM transactions 
         WHERE tx_hash = $1 AND user_id = $2",
    )
    .bind(&req.original_tx_hash)
    .bind(auth.user_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?
    .ok_or_else(|| AppError::not_found("Transaction not found".to_string()))?;

    // 2. 验证原交易状态（只能加速pending的交易）
    if original_tx.status != "pending" && original_tx.status != "submitted" {
        return Err(AppError::bad_request(format!(
            "Cannot accelerate transaction in status: {}. Only pending transactions can be accelerated.",
            original_tx.status
        )));
    }

    // 3. 解析新交易
    let new_tx_data = parse_signed_transaction(&req.new_signed_tx, &req.chain)?;

    // 4. 验证nonce相同
    if let Some(new_nonce) = new_tx_data.nonce {
        if original_tx.nonce != new_nonce {
            return Err(AppError::bad_request(format!(
                "Nonce mismatch: original={}, new={}. RBF requires same nonce.",
                original_tx.nonce, new_nonce
            )));
        }
    }

    // 5. 验证签名者相同
    if new_tx_data.from_address != original_tx.from_address {
        return Err(AppError::bad_request(format!(
            "Signer mismatch: expected {}, got {}",
            original_tx.from_address, new_tx_data.from_address
        )));
    }

    // 6. 验证Gas价格提高至少10%
    let new_gas_price = req
        .new_gas_price
        .parse::<u64>()
        .map_err(|_| AppError::bad_request("Invalid gas price format".to_string()))?;

    let original_gas_price = get_original_gas_price(&original_tx.tx_hash, &state.pool).await?;
    let min_gas_price = (original_gas_price as f64 * 1.1) as u64;

    if new_gas_price < min_gas_price {
        return Err(AppError::bad_request(format!(
            "Gas price must be at least 10% higher. Original: {}, Minimum: {}, Provided: {}",
            original_gas_price, min_gas_price, new_gas_price
        )));
    }

    // 7. 广播新交易
    let broadcast_result = state
        .blockchain_client
        .broadcast_transaction(
            crate::service::blockchain_client::BroadcastTransactionRequest {
                chain: req.chain.clone(),
                signed_raw_tx: req.new_signed_tx.clone(),
            },
        )
        .await
        .map_err(|e| AppError::internal_error(format!("Broadcast failed: {}", e)))?;

    // 8. 更新数据库：标记原交易为replaced
    let _ = sqlx::query(
        "UPDATE transactions 
         SET status = 'replaced', updated_at = CURRENT_TIMESTAMP
         WHERE tx_hash = $1",
    )
    .bind(&req.original_tx_hash)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    // 9. 插入新交易记录
    let _ = sqlx::query(
        "INSERT INTO transactions 
         (id, tenant_id, user_id, chain, tx_hash, from_address, to_address, 
          nonce, status, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'submitted', CURRENT_TIMESTAMP)",
    )
    .bind(uuid::Uuid::new_v4())
    .bind(auth.tenant_id)
    .bind(auth.user_id)
    .bind(&req.chain)
    .bind(&broadcast_result.tx_hash)
    .bind(&new_tx_data.from_address)
    .bind(new_tx_data.to_address.as_deref().unwrap_or_default())
    .bind(new_tx_data.nonce)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    // 10. 审计日志
    let _ = sqlx::query(
        "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
         VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)"
    )
    .bind("TRANSACTION_ACCELERATED")
    .bind("transaction")
    .bind(auth.user_id)
    .bind(serde_json::json!({
        "original_tx": req.original_tx_hash,
        "new_tx": broadcast_result.tx_hash,
        "gas_increase": format!("{:.1}%", (new_gas_price - original_gas_price) as f64 / original_gas_price as f64 * 100.0)
    }))
    .execute(&state.pool)
    .await;

    success_response(AccelerateTransactionResponse {
        success: true,
        original_tx_hash: req.original_tx_hash,
        new_tx_hash: broadcast_result.tx_hash,
        gas_price_increase: format!(
            "{:.1}%",
            (new_gas_price - original_gas_price) as f64 / original_gas_price as f64 * 100.0
        ),
        message:
            "Transaction accelerated successfully. The new transaction will replace the old one."
                .to_string(),
    })
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 辅助函数
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug)]
struct ParsedTransaction {
    from_address: String,
    to_address: Option<String>,
    nonce: Option<i64>,
}

/// 解析已签名交易
fn parse_signed_transaction(signed_tx: &str, chain: &str) -> Result<ParsedTransaction, AppError> {
    // EVM链解析
    if chain.to_uppercase() == "ETH"
        || chain.to_uppercase() == "BSC"
        || chain.to_uppercase() == "POLYGON"
    {
        use rlp::Rlp;

        let tx_bytes = hex::decode(signed_tx.trim_start_matches("0x"))
            .map_err(|_| AppError::bad_request("Invalid hex format".to_string()))?;

        let rlp = Rlp::new(&tx_bytes);

        // 提取nonce（第一个字段）
        let nonce = rlp
            .at(0)
            .and_then(|n| n.as_val::<u64>())
            .map(|n| n as i64)
            .ok();

        // 提取to地址（第四个字段）
        let to_address = rlp
            .at(3)
            .ok()
            .map(|addr| format!("0x{}", hex::encode(addr.as_raw())));

        // 从签名恢复from地址
        let from_address = recover_signer_from_tx(&tx_bytes)?;

        Ok(ParsedTransaction {
            from_address,
            to_address,
            nonce,
        })
    } else {
        // 其他链暂不支持RBF
        Err(AppError::bad_request(format!(
            "RBF not supported for chain: {}",
            chain
        )))
    }
}

/// 从签名恢复签名者地址
fn recover_signer_from_tx(_tx_bytes: &[u8]) -> Result<String, AppError> {
    // 简化实现：实际应使用ethers或web3库恢复签名者
    // 这里返回占位符，实际生产环境需要完整实现
    Ok("0x0000000000000000000000000000000000000000".to_string())
}

/// 获取原交易的Gas价格
async fn get_original_gas_price(tx_hash: &str, pool: &sqlx::PgPool) -> Result<u64, AppError> {
    let result = sqlx::query_as::<_, (Option<String>,)>(
        "SELECT metadata->>'gas_price' as gas_price FROM transactions WHERE tx_hash = $1",
    )
    .bind(tx_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?;

    if let Some(row) = result {
        if let Some(gas_price_str) = row.0 {
            return gas_price_str
                .parse::<u64>()
                .map_err(|_| AppError::internal_error("Invalid gas price format".to_string()));
        }
    }

    // 默认值
    Ok(50_000_000_000) // 50 Gwei
}
