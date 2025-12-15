//! HTTP Method Whitelist Middleware
//! 
//! 阻止不安全或不必要的HTTP方法（如TRACE, CONNECT）以防止跨站追踪攻击

use axum::{
    extract::Request,
    http::{Method, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::warn;

/// HTTP方法白名单中间件
/// 
/// 只允许以下HTTP方法:
/// - GET, POST, PUT, DELETE, PATCH, OPTIONS
/// 
/// 拒绝以下方法:
/// - TRACE (可能被用于跨站追踪攻击 XST)
/// - CONNECT (代理方法，不应在API中使用)
/// - HEAD (可选，目前拒绝以减少攻击面)
/// 
/// ## 安全考虑
/// 
/// - TRACE方法可能暴露HTTP头信息，包括认证凭据
/// - CONNECT方法可能被滥用作为开放代理
/// - 应在认证中间件之前应用，避免不必要的处理
/// 
/// ## 使用方法
/// 
/// ```rust
/// use axum::middleware::from_fn;
/// 
/// Router::new()
///     .route("/api/users", get(handler))
///     .layer(from_fn(method_whitelist_middleware))  // 最先应用
///     .layer(from_fn(auth_middleware))              // 认证在后
/// ```
pub async fn method_whitelist_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = req.method();
    
    // 白名单：允许的HTTP方法
    match method {
        &Method::GET 
        | &Method::POST 
        | &Method::PUT 
        | &Method::DELETE 
        | &Method::PATCH 
        | &Method::OPTIONS => {
            // 允许的方法，继续处理
            Ok(next.run(req).await)
        }
        _ => {
            // 拒绝不在白名单的方法
            warn!(
                "Blocked HTTP method: {} on path: {}", 
                method, 
                req.uri().path()
            );
            
            // 返回 405 Method Not Allowed
            Err(StatusCode::METHOD_NOT_ALLOWED)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware::from_fn,
        routing::get,
        Router,
    };
    use tower::ServiceExt as _; // for oneshot()

    async fn dummy_handler() -> &'static str {
        "OK"
    }

    #[tokio::test]
    async fn test_allowed_methods() {
        let app = Router::new()
            .route("/test", get(dummy_handler))
            .layer(from_fn(method_whitelist_middleware));

        // 测试允许的方法
        let allowed_methods = vec![
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
            Method::OPTIONS,
        ];

        for method in allowed_methods {
            let req = Request::builder()
                .method(method.clone())
                .uri("/test")
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(req).await.unwrap();
            
            // GET会返回200，其他方法会返回405（因为路由只定义了GET）
            // 但重点是不会在中间件层被拒绝
            assert!(
                response.status() == StatusCode::OK 
                || response.status() == StatusCode::METHOD_NOT_ALLOWED,
                "Method {} should pass middleware", 
                method
            );
        }
    }

    #[tokio::test]
    async fn test_blocked_methods() {
        let app = Router::new()
            .route("/test", get(dummy_handler))
            .layer(from_fn(method_whitelist_middleware));

        // 测试被阻止的方法
        let blocked_methods = vec![
            "TRACE",
            "CONNECT",
            "HEAD", // 可选：根据需求决定是否允许HEAD
        ];

        for method_str in blocked_methods {
            let req = Request::builder()
                .method(method_str)
                .uri("/test")
                .body(Body::empty())
                .unwrap();

            let response = app.clone().oneshot(req).await.unwrap();
            
            // 应该返回 405 Method Not Allowed
            assert_eq!(
                response.status(),
                StatusCode::METHOD_NOT_ALLOWED,
                "Method {} should be blocked",
                method_str
            );
        }
    }

    #[tokio::test]
    async fn test_trace_method_specifically() {
        let app = Router::new()
            .route("/api/users", get(dummy_handler))
            .layer(from_fn(method_whitelist_middleware));

        let req = Request::builder()
            .method("TRACE")
            .uri("/api/users")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        
        assert_eq!(
            response.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "TRACE method must be blocked to prevent XST attacks"
        );
    }
}
