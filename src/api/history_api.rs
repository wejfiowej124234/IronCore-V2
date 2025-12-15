//! 交易历史API
//! 企业级实现，支持交换、充值、提现历史查询

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    api::{
        middleware::jwt_extractor::JwtAuthContext,
        response::{convert_error, success_response},
    },
    app_state::AppState,
    error::AppError,
    repository::SwapTransactionRepository,
};

/// GET /api/swap/history - 获取交易历史列表（企业级实现）
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    pub tx_type: Option<String>,
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HistoryItem {
    pub id: String,
    pub tx_type: String,
    pub status: String,
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    /// 平台服务费：钱包服务商收取的服务费用（与Gas费用完全独立）
    /// 注意：这不是Gas费用，Gas费用在gas_fee字段中
    pub fee_amount: Option<String>,
    /// Gas费用：区块链网络收取的交易执行费用（gas_used * gas_price）
    /// 注意：这是区块链网络费用，与平台服务费完全独立！
    pub gas_fee: Option<String>,
    pub tx_hash: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub fiat_order_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub transactions: Vec<HistoryItem>,
    pub total: u32,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

pub async fn get_transaction_history(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Query(query): Query<HistoryQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<HistoryResponse>>, AppError> {
    info!(
        "获取swap交易历史: user_id={}, tenant_id={}",
        auth.user_id, auth.tenant_id
    );

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20).min(100); // 限制最大100
    let offset = (page - 1) * page_size;

    // 从数据库查询swap交易历史
    let swap_repo = SwapTransactionRepository::new(state.pool.clone());

    let swap_transactions = swap_repo
        .list_by_user(
            auth.tenant_id,
            auth.user_id,
            query.status.as_deref(),
            page_size as i64,
            offset as i64,
        )
        .await
        .map_err(|e| {
            error!("查询swap交易历史失败: {:?}", e);
            convert_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询交易历史失败".to_string(),
            )
        })?;

    let total = swap_repo
        .count_by_user(auth.tenant_id, auth.user_id, query.status.as_deref())
        .await
        .map_err(|e| {
            error!("统计swap交易数量失败: {:?}", e);
            convert_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "统计交易数量失败".to_string(),
            )
        })? as u32;

    let transactions: Vec<HistoryItem> = swap_transactions
        .into_iter()
        .map(|tx| {
            HistoryItem {
                id: tx.swap_id.clone(),
                tx_type: "swap".to_string(),
                status: tx.status.clone(),
                from_token: tx.from_token.clone(),
                to_token: tx.to_token.clone(),
                from_amount: tx.from_amount.to_string(),
                to_amount: tx.to_amount.map(|a| a.to_string()).unwrap_or_default(),
                // 平台服务费：钱包服务商收取的服务费用（与Gas费用完全独立）
                // 注意：当前swap_transactions表没有存储服务费，如果需要可以从metadata中提取
                // 平台服务费存储在fee_audit表的platform_fee字段中
                fee_amount: None, // 平台服务费（如果有，从fee_audit表查询）
                // Gas费用：区块链网络收取的交易执行费用（gas_used * gas_price）
                // 注意：这是区块链网络费用，与平台服务费完全独立！
                gas_fee: tx.gas_used.clone(),
                tx_hash: tx.tx_hash.clone(),
                created_at: tx.created_at.to_rfc3339(),
                completed_at: if tx.status == "confirmed" || tx.status == "failed" {
                    Some(tx.updated_at.to_rfc3339())
                } else {
                    None
                },
                fiat_order_id: None,
                metadata: tx.metadata.clone(),
            }
        })
        .collect();

    let total_pages = (total as f64 / page_size as f64).ceil() as u32;

    success_response(HistoryResponse {
        transactions,
        total,
        page,
        page_size,
        total_pages,
    })
}

/// GET /api/swap/history/:id - 获取交易详情（企业级实现）
pub async fn get_transaction_detail(
    State(state): State<Arc<AppState>>,
    auth: JwtAuthContext,
    Path(id): Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<HistoryItem>>, AppError> {
    info!("获取swap交易详情: swap_id={}, user_id={}", id, auth.user_id);

    // 从数据库查询swap交易详情
    let swap_repo = SwapTransactionRepository::new(state.pool.clone());

    let swap_tx = swap_repo
        .find_by_swap_id(&id)
        .await
        .map_err(|e| {
            error!("查询swap交易详情失败: {:?}", e);
            convert_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "查询交易详情失败".to_string(),
            )
        })?
        .ok_or_else(|| convert_error(StatusCode::NOT_FOUND, "交易不存在".to_string()))?;

    // 验证权限：只能查看自己的交易
    if swap_tx.user_id != auth.user_id || swap_tx.tenant_id != auth.tenant_id {
        return Err(convert_error(
            StatusCode::FORBIDDEN,
            "无权查看此交易".to_string(),
        ));
    }

    let history_item = HistoryItem {
        id: swap_tx.swap_id.clone(),
        tx_type: "swap".to_string(),
        status: swap_tx.status.clone(),
        from_token: swap_tx.from_token.clone(),
        to_token: swap_tx.to_token.clone(),
        from_amount: swap_tx.from_amount.to_string(),
        to_amount: swap_tx.to_amount.map(|a| a.to_string()).unwrap_or_default(),
        fee_amount: None,
        gas_fee: swap_tx.gas_used.clone(),
        tx_hash: swap_tx.tx_hash.clone(),
        created_at: swap_tx.created_at.to_rfc3339(),
        completed_at: if swap_tx.status == "confirmed" || swap_tx.status == "failed" {
            Some(swap_tx.updated_at.to_rfc3339())
        } else {
            None
        },
        fiat_order_id: None,
        metadata: swap_tx.metadata.clone(),
    };

    success_response(history_item)
}
