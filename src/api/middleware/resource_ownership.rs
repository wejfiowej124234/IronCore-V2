// 资源所有权验证中间件

use std::sync::Arc;

use axum::{extract::State, middleware::Next, response::Response};
use uuid::Uuid;

use crate::{app_state::AppState, error::AppError};

/// 验证钱包所有权
pub async fn verify_wallet_ownership(
    State(state): State<Arc<AppState>>,
    wallet_id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<(), AppError> {
    let wallet = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>(
        "SELECT id, user_id, tenant_id FROM wallets WHERE id = $1",
    )
    .bind(wallet_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database_error(format!("Database error: {}", e)))?;

    let (_, wallet_user_id, wallet_tenant_id) =
        wallet.ok_or_else(|| AppError::wallet_not_found("Wallet not found"))?;

    if wallet_user_id != user_id || wallet_tenant_id != tenant_id {
        return Err(AppError::permission_denied(
            "Not authorized to access this wallet",
        ));
    }

    Ok(())
}

/// 验证资源所有权（通用）
pub async fn verify_resource_ownership<T>(
    State(state): State<Arc<AppState>>,
    table_name: &str,
    resource_id: Uuid,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<(), AppError>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
{
    let query = format!(
        "SELECT user_id, tenant_id FROM {} WHERE id = $1",
        table_name
    );
    let resource: Option<(Uuid, Uuid)> = sqlx::query_as(&query)
        .bind(resource_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| AppError::database_error(format!("Database error: {}", e)))?;

    let (resource_user_id, resource_tenant_id) =
        resource.ok_or_else(|| AppError::resource_not_found("Resource not found"))?;

    if resource_user_id != user_id || resource_tenant_id != tenant_id {
        return Err(AppError::permission_denied(
            "Not authorized to access this resource",
        ));
    }

    Ok(())
}

/// 资源所有权验证中间件（示例）
pub async fn resource_ownership_middleware(
    State(_state): State<Arc<AppState>>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, AppError> {
    // 从请求中提取用户信息（假设已通过认证中间件）
    // 这里可以根据实际需求实现具体的验证逻辑
    let response = next.run(req).await;
    Ok(response)
}
