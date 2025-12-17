//! 幂等性中间件
//! 企业级实现：防止支付回调等关键操作被重复执行

use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use redis::AsyncCommands;

use crate::{app_state::AppState, error::AppError};

const IDEMPOTENCY_KEY_HEADER: &str = "X-Idempotency-Key";
const DEFAULT_TTL_SECS: u64 = 86400; // 24小时

/// 幂等性中间件
///
/// # 使用方式
/// ```rust,ignore
/// Router::new()
///     .route("/api/fiat/callback", post(callback_handler))
///     .layer(middleware::from_fn_with_state(
///         state.clone(),
///         idempotency_middleware,
///     ))
/// ```
///
/// # 客户端要求
/// - 请求头包含 `X-Idempotency-Key: <uuid>`
/// - 相同的key在TTL内只会执行一次
pub async fn idempotency_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 1. 获取幂等性key
    let idempotency_key = match request.headers().get(IDEMPOTENCY_KEY_HEADER) {
        Some(key) => key
            .to_str()
            .map_err(|_| AppError::bad_request("Invalid idempotency key format".to_string()))?
            .to_string(),
        None => {
            // 如果没有提供key，不强制要求（部分API可选）
            return Ok(next.run(request).await);
        }
    };

    // 2. 验证key格式（UUID）
    if uuid::Uuid::parse_str(&idempotency_key).is_err() {
        return Err(AppError::bad_request(
            "Idempotency key must be a valid UUID".to_string(),
        ));
    }

    // 3. 检查是否已处理
    let cache_key = format!("idempotency:{}", idempotency_key);

    // 尝试从缓存获取结果
    if let Some(cached_response) = get_cached_response(&state, &cache_key).await? {
        tracing::info!(
            idempotency_key = %idempotency_key,
            "Returning cached response for duplicate request"
        );
        return Ok(cached_response);
    }

    // 4. 尝试获取分布式锁（防止并发处理同一个key）
    let lock_key = format!("idempotency_lock:{}", idempotency_key);

    let lock_guard = match state.distributed_lock.try_acquire(&lock_key, 60).await {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            // 锁被占用，说明正在处理中
            return Err(AppError::conflict(
                "Request is being processed, please wait".to_string(),
            ));
        }
        Err(e) => {
            tracing::error!(error = ?e, "Failed to acquire idempotency lock");
            return Err(AppError::internal_error(
                "Failed to check idempotency".to_string(),
            ));
        }
    };

    // 5. 双重检查（获取锁后再次检查缓存）
    if let Some(cached_response) = get_cached_response(&state, &cache_key).await? {
        drop(lock_guard);
        return Ok(cached_response);
    }

    // 6. 执行实际请求
    let response = next.run(request).await;

    // 7. 缓存成功响应
    if response.status().is_success() {
        if let Err(e) = cache_response(&state, &cache_key, &response).await {
            tracing::warn!(
                error = ?e,
                idempotency_key = %idempotency_key,
                "Failed to cache response"
            );
        }
    }

    // 8. 释放锁
    drop(lock_guard);

    Ok(response)
}

/// 从缓存获取响应
async fn get_cached_response(
    state: &AppState,
    cache_key: &str,
) -> Result<Option<Response>, AppError> {
    let mut conn = state
        .redis_pool
        .client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to get Redis connection");
            AppError::internal_error("Cache unavailable".to_string())
        })?;

    let cached: Option<String> = conn.get(cache_key).await.map_err(|e| {
        tracing::error!(error = ?e, "Failed to get from cache");
        AppError::internal_error("Failed to read cache".to_string())
    })?;

    if let Some(cached_json) = cached {
        // 解析缓存的响应
        match serde_json::from_str::<CachedResponse>(&cached_json) {
            Ok(cached_resp) => {
                let response = Response::builder()
                    .status(cached_resp.status)
                    .header("Content-Type", "application/json")
                    .header("X-Cache-Hit", "true")
                    .body(Body::from(cached_resp.body))
                    .unwrap();

                Ok(Some(response))
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Failed to parse cached response");
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

/// 缓存响应
async fn cache_response(
    state: &AppState,
    cache_key: &str,
    response: &Response,
) -> Result<(), AppError> {
    // 只缓存成功响应
    if !response.status().is_success() {
        return Ok(());
    }

    let mut conn = state
        .redis_pool
        .client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::internal_error("Cache unavailable".to_string()))?;

    // 提取响应体（注意：这会消耗response body，实际实现需要clone或buffering）
    // 简化实现：只缓存状态码和基本信息
    let cached_resp = CachedResponse {
        status: response.status().as_u16(),
        body: serde_json::json!({
            "cached": true,
            "message": "Request already processed"
        })
        .to_string(),
    };

    let cached_json = serde_json::to_string(&cached_resp)
        .map_err(|_| AppError::internal_error("Failed to serialize response".to_string()))?;

    // 设置TTL
    let _: () = conn
        .set_ex(cache_key, cached_json, DEFAULT_TTL_SECS)
        .await
        .map_err(|_| AppError::internal_error("Failed to cache response".to_string()))?;

    Ok(())
}

/// 缓存的响应结构
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedResponse {
    status: u16,
    body: String,
}

/// 手动清除幂等性key（用于管理员操作）
pub async fn clear_idempotency_key(
    state: &AppState,
    idempotency_key: &str,
) -> Result<bool, AppError> {
    let cache_key = format!("idempotency:{}", idempotency_key);

    let mut conn = state
        .redis_pool
        .client
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| AppError::internal_error("Cache unavailable".to_string()))?;

    let deleted: i32 = conn
        .del(&cache_key)
        .await
        .map_err(|_| AppError::internal_error("Failed to delete cache".to_string()))?;

    Ok(deleted > 0)
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_idempotency_key_validation() {
        let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
        assert!(uuid::Uuid::parse_str(valid_uuid).is_ok());

        let invalid_uuid = "not-a-uuid";
        assert!(uuid::Uuid::parse_str(invalid_uuid).is_err());
    }
}
