//! 统一速率限制中间件
//! 支持基于IP、用户ID、API Key的速率限制

use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::{api::middleware::auth::AuthInfo, app_state::AppState, error::AppError};

/// 速率限制配置
#[derive(Clone)]
pub struct RateLimitConfig {
    pub window_secs: u64,
    pub max_requests: i64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            window_secs: 60,
            max_requests: 1000, // ✅高限制(生产建议:100-200/分钟)
        }
    }
}

/// 速率限制中间件
///
/// 支持多种速率限制策略：
/// - 基于IP地址
/// - 基于用户ID
/// - 基于API Key
pub async fn rate_limit_middleware(
    State(st): State<Arc<AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let redis = st.redis.clone();
    // 获取速率限制配置（从环境变量或使用默认值）
    let config = get_rate_limit_config();

    // 确定速率限制键
    let rate_limit_key = determine_rate_limit_key(&req)?;

    // 检查速率限制
    let count = redis
        .rate_limit_incr(&rate_limit_key, Duration::from_secs(config.window_secs))
        .await
        .map_err(|e| AppError::bad_request(format!("Rate limit check failed: {}", e)))?;

    if count > config.max_requests {
        return Err(AppError {
            code: crate::error::AppErrorCode::RateLimit,
            message: format!(
                "Rate limit exceeded: {} requests per {} seconds",
                config.max_requests, config.window_secs
            ),
            status: StatusCode::TOO_MANY_REQUESTS,
            trace_id: None,
        });
    }

    // 添加速率限制头到响应
    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    // Header值解析应该总是成功，因为都是数字字符串
    // 使用unwrap_or_else提供更好的错误处理，如果失败说明有bug但不会panic
    headers.insert(
        "X-RateLimit-Limit",
        config
            .max_requests
            .to_string()
            .parse()
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(
        "X-RateLimit-Remaining",
        (config.max_requests - count)
            .max(0)
            .to_string()
            .parse()
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );
    headers.insert(
        "X-RateLimit-Reset",
        (chrono::Utc::now().timestamp() + config.window_secs as i64)
            .to_string()
            .parse()
            .unwrap_or_else(|_| HeaderValue::from_static("0")),
    );

    Ok(response)
}

/// 确定速率限制键
/// 优先级：用户ID > API Key > IP地址
fn determine_rate_limit_key(req: &Request) -> Result<String, AppError> {
    // 1. 尝试从认证信息中获取用户ID
    if let Some(auth_info) = req.extensions().get::<AuthInfo>() {
        return Ok(format!("rate_limit:user:{}", auth_info.user_id));
    }

    // 2. 尝试从API Key获取
    if let Some(api_key) = req.headers().get("X-API-Key") {
        if let Ok(key_str) = api_key.to_str() {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(key_str.as_bytes());
            let key_hash = faster_hex::hex_string(&hasher.finalize());
            return Ok(format!("rate_limit:api_key:{}", key_hash));
        }
    }

    // 3. 使用IP地址
    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .or_else(|| req.headers().get("X-Real-IP").and_then(|h| h.to_str().ok()))
        .unwrap_or("unknown");

    Ok(format!("rate_limit:ip:{}", ip))
}

/// 获取速率限制配置
fn get_rate_limit_config() -> RateLimitConfig {
    let window_secs = std::env::var("RATE_LIMIT_WINDOW_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let max_requests = std::env::var("RATE_LIMIT_MAX_REQUESTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    RateLimitConfig {
        window_secs,
        max_requests,
    }
}

/// Rate limit middleware return type
type RateLimitFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>;

/// 自定义速率限制中间件（可配置）
pub fn custom_rate_limit(
    config: RateLimitConfig,
) -> impl Fn(Arc<AppState>, Request, Next) -> RateLimitFuture {
    move |st: Arc<AppState>, req: Request, next: Next| {
        let config = config.clone();
        let redis = st.redis.clone();
        Box::pin(async move {
            let rate_limit_key = determine_rate_limit_key(&req)?;

            let count = redis
                .rate_limit_incr(&rate_limit_key, Duration::from_secs(config.window_secs))
                .await
                .map_err(|e| AppError::bad_request(format!("Rate limit check failed: {}", e)))?;

            if count > config.max_requests {
                return Err(AppError {
                    code: crate::error::AppErrorCode::RateLimit,
                    message: format!(
                        "Rate limit exceeded: {} requests per {} seconds",
                        config.max_requests, config.window_secs
                    ),
                    status: StatusCode::TOO_MANY_REQUESTS,
                    trace_id: None,
                });
            }

            let mut response = next.run(req).await;
            let headers = response.headers_mut();
            headers.insert(
                "X-RateLimit-Limit",
                config
                    .max_requests
                    .to_string()
                    .parse()
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            );
            headers.insert(
                "X-RateLimit-Remaining",
                (config.max_requests - count)
                    .max(0)
                    .to_string()
                    .parse()
                    .unwrap_or_else(|_| HeaderValue::from_static("0")),
            );

            Ok(response)
        })
    }
}
