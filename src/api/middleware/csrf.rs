//! CSRF防护中间件
//! 提供CSRF Token验证和Origin检查

use std::{
    collections::HashMap,
    sync::{Arc, Arc as StdArc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

use crate::error::AppError;

/// CSRF Token存储（内存存储，生产环境应使用Redis）
type TokenStore = StdArc<Mutex<HashMap<String, TokenInfo>>>;

#[derive(Clone)]
struct TokenInfo {
    created_at: Instant,
    origin: String,
}

/// CSRF Token管理器
pub struct CsrfManager {
    tokens: TokenStore,
    token_ttl: Duration,
}

impl CsrfManager {
    /// 创建新的CSRF管理器
    pub fn new(token_ttl_secs: u64) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
            token_ttl: Duration::from_secs(token_ttl_secs),
        }
    }

    /// 生成CSRF Token
    pub fn generate_token(&self, origin: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", origin, Instant::now().elapsed().as_nanos()));
        let token = faster_hex::hex_string(&hasher.finalize());

        // Mutex锁不应该失败，如果失败说明有严重问题
        // 使用map_err返回错误而不是panic
        match self.tokens.lock() {
            Ok(mut tokens) => {
                tokens.insert(
                    token.clone(),
                    TokenInfo {
                        created_at: Instant::now(),
                        origin: origin.to_string(),
                    },
                );

                // 清理过期token（忽略错误，避免递归问题）
                drop(tokens);
                let _ = self.tokens.lock().map(|mut t| {
                    t.retain(|_, info| info.created_at.elapsed() <= self.token_ttl);
                });
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to acquire CSRF token store lock - this indicates a serious bug");
                // 即使锁失败，也返回token（降级处理）
            }
        }

        token
    }

    /// 验证CSRF Token
    pub fn verify_token(&self, token: &str, origin: &str) -> bool {
        let tokens = match self.tokens.lock() {
            Ok(tokens) => tokens,
            Err(e) => {
                tracing::error!(error = %e, "Failed to acquire CSRF token store lock");
                // 锁失败时返回false（拒绝验证）
                return false;
            }
        };

        if let Some(info) = tokens.get(token) {
            // 检查是否过期
            if info.created_at.elapsed() > self.token_ttl {
                drop(tokens);
                // 尝试移除过期token（忽略错误）
                let _ = self.tokens.lock().map(|mut t| {
                    t.remove(token);
                });
                return false;
            }

            // 检查Origin是否匹配
            if info.origin == origin {
                return true;
            }
        }

        false
    }

    /// 清理过期token
    #[allow(dead_code)]
    fn cleanup_expired(&self) {
        if let Ok(mut tokens) = self.tokens.lock() {
            tokens.retain(|_, info| info.created_at.elapsed() <= self.token_ttl);
        } else {
            tracing::warn!("Failed to acquire CSRF token store lock for cleanup");
        }
    }
}

/// CSRF防护中间件
///
/// 注意：当前实现需要CSRF管理器通过请求扩展注入
/// 如果需要从AppState获取，可以使用State参数
pub async fn csrf_middleware(req: Request, next: Next) -> Result<Response, AppError> {
    let method = req.method();

    // 只对状态变更方法进行CSRF检查
    if !matches!(
        method,
        &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
    ) {
        return Ok(next.run(req).await);
    }

    let headers = req.headers();

    // 1. 检查Origin头
    let origin_str: Option<String> = headers
        .get("Origin")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("Referer")
                .and_then(|h| h.to_str().ok())
                .map(|r| {
                    // 从Referer提取origin
                    r.split('/').take(3).collect::<Vec<_>>().join("/")
                })
        });

    // 如果没有Origin头，可能是同源请求，允许通过
    let origin = match origin_str {
        Some(ref origin) => origin.as_str(),
        None => {
            // 允许同源请求（没有Origin头可能是同源）
            return Ok(next.run(req).await);
        }
    };

    // 2. 检查CSRF Token
    let csrf_token = headers
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            // 也支持从Cookie中读取
            headers
                .get("Cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|c| {
                    c.split(';')
                        .find(|s| s.trim().starts_with("csrf_token="))
                        .and_then(|s| s.split('=').nth(1))
                })
        });

    if let Some(token) = csrf_token {
        // 从请求扩展中获取CSRF管理器（需要在路由中注入）
        if let Some(csrf_manager) = req.extensions().get::<Arc<CsrfManager>>() {
            if csrf_manager.verify_token(token, origin) {
                return Ok(next.run(req).await);
            }
        }
    }

    // 3. 检查SameSite Cookie（如果使用Cookie认证）
    // 检查Cookie的SameSite属性
    if let Some(cookie_header) = headers.get("Cookie") {
        if let Ok(cookie_str) = cookie_header.to_str() {
            // 检查是否有SameSite=None的Cookie（需要CSRF保护）
            // 如果所有Cookie都是SameSite=Strict或SameSite=Lax，则CSRF风险较低
            let has_unsafe_cookie = cookie_str.contains("SameSite=None")
                || (!cookie_str.contains("SameSite=Strict")
                    && !cookie_str.contains("SameSite=Lax"));

            // 如果有不安全的Cookie，需要CSRF Token
            if has_unsafe_cookie && csrf_token.is_none() {
                return Err(AppError {
                    code: crate::error::AppErrorCode::BadRequest,
                    message: "CSRF token required for unsafe cookies".into(),
                    status: StatusCode::FORBIDDEN,
                    trace_id: None,
                });
            }
        }
    }

    // CSRF验证失败
    // 注意：如果没有配置CSRF管理器，则允许通过（降级处理）
    // 这样可以避免在没有配置CSRF时阻断所有请求
    if req.extensions().get::<Arc<CsrfManager>>().is_none() {
        tracing::warn!("CSRF middleware enabled but CsrfManager not found in request extensions, allowing request");
        return Ok(next.run(req).await);
    }

    Err(AppError {
        code: crate::error::AppErrorCode::BadRequest,
        message: "CSRF token validation failed".into(),
        status: StatusCode::FORBIDDEN,
        trace_id: None,
    })
}

/// CSRF防护中间件（从AppState获取CsrfManager）
/// 这是更推荐的实现方式
pub async fn csrf_middleware_with_state(
    State(st): State<Arc<crate::app_state::AppState>>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let method = req.method();

    // 只对状态变更方法进行CSRF检查
    if !matches!(
        method,
        &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
    ) {
        return Ok(next.run(req).await);
    }

    let headers = req.headers();

    // 1. 检查Origin头
    let origin_str: Option<String> = headers
        .get("Origin")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get("Referer")
                .and_then(|h| h.to_str().ok())
                .map(|r| r.split('/').take(3).collect::<Vec<_>>().join("/"))
        });

    // 如果没有Origin头，可能是同源请求，允许通过
    let origin = match origin_str.as_deref() {
        Some(origin) => origin,
        None => {
            return Ok(next.run(req).await);
        }
    };

    // 2. 检查CSRF Token
    let csrf_token = headers
        .get("X-CSRF-Token")
        .and_then(|h| h.to_str().ok())
        .or_else(|| {
            headers
                .get("Cookie")
                .and_then(|h| h.to_str().ok())
                .and_then(|c| {
                    c.split(';')
                        .find(|s| s.trim().starts_with("csrf_token="))
                        .and_then(|s| s.split('=').nth(1))
                })
        });

    if let Some(token) = csrf_token {
        if st.csrf.verify_token(token, origin) {
            return Ok(next.run(req).await);
        }
    }

    // CSRF验证失败
    Err(AppError {
        code: crate::error::AppErrorCode::BadRequest,
        message: "CSRF token validation failed".into(),
        status: StatusCode::FORBIDDEN,
        trace_id: None,
    })
}

/// 生成CSRF Token的辅助函数
pub fn generate_csrf_token(csrf_manager: &Arc<CsrfManager>, origin: &str) -> String {
    csrf_manager.generate_token(origin)
}
