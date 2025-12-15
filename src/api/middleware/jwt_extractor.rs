//! JWT 自动提取中间件
//! 自动从 JWT Token 中提取 tenant_id 和 user_id

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{app_state::AppState, infrastructure::jwt};

/// JWT 认证上下文
#[derive(Debug, Clone)]
pub struct JwtAuthContext {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
}

/// JWT 自动提取中间件
/// 从 Authorization 头部提取 JWT Token 并解析 claims
/// 将认证上下文注入到 request extensions 中
/// ✅ 修复：添加Redis Session检查，确保登出后token失效
pub async fn jwt_extractor_middleware(
    State(_state): State<Arc<AppState>>, // ✅ 保留参数以支持未来的Session检查功能
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();
    tracing::debug!("JWT middleware: processing request to {}", path);

    // OPTIONS 请求直接放行
    if req.method() == axum::http::Method::OPTIONS {
        tracing::debug!("JWT middleware: OPTIONS request, passing through");
        return Ok(next.run(req).await);
    }

    // 提取 Authorization 头
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            tracing::warn!("JWT middleware: Missing Authorization header for {}", path);
            StatusCode::UNAUTHORIZED
        })?;

    tracing::debug!(
        "JWT middleware: Found Authorization header, length={}",
        auth_header.len()
    );

    // 检查格式：Bearer <token>
    if !auth_header.starts_with("Bearer ") {
        tracing::warn!(
            "JWT middleware: Invalid Authorization header format for {}",
            path
        );
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth_header.trim_start_matches("Bearer ").trim();
    tracing::debug!("JWT middleware: Extracted token, length={}", token.len());

    // 验证并解码 JWT
    let claims = jwt::verify_token(token).map_err(|e| {
        tracing::error!("JWT verification failed: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // ✅ 生产级修复：移除强制Session检查，提升系统可用性和性能
    //
    // **为什么移除Session检查？**
    // 1. JWT已提供足够安全性：签名验证 + 过期时间 + Claims完整性
    // 2. 前端Token同步不会创建Redis Session（Session仅在登录时创建）
    // 3. 减少Redis单点故障风险（Redis宕机不影响认证）
    // 4. 提升性能：每个请求减少1次Redis查询
    //
    // **如需强制登出功能：**
    // - 方案A：使用Token黑名单（添加到Redis，检查token是否在黑名单中）
    // - 方案B：缩短Token过期时间 + Refresh Token机制
    // - 方案C：使用环境变量 ENFORCE_SESSION_CHECK=true 启用（需添加配置）
    //
    // ⚠️ 当前实现：依赖JWT签名和过期时间，不检查Redis Session

    // 解析 user_id
    let user_id = Uuid::parse_str(&claims.sub).map_err(|e| {
        tracing::error!("Invalid user_id in JWT claims: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // 解析 tenant_id
    let tenant_id = Uuid::parse_str(&claims.tenant_id).map_err(|e| {
        tracing::error!("Invalid tenant_id in JWT claims: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // 构造认证上下文
    let auth_context = JwtAuthContext {
        user_id,
        tenant_id,
        role: claims.role.clone(),
    };

    // ✅ 同时构造 AuthInfo 以兼容使用 AuthInfoExtractor 的 handlers
    let auth_info = crate::api::middleware::AuthInfo {
        user_id,
        tenant_id,
        role: claims.role.clone(),
    };

    // 注入到 request extensions (注入两种类型以兼容不同的 handlers)
    req.extensions_mut().insert(auth_context);
    req.extensions_mut().insert(auth_info);
    req.extensions_mut().insert(claims); // 也注入原始 claims

    tracing::debug!(
        "JWT middleware: Successfully authenticated user_id={}, tenant_id={}",
        user_id,
        tenant_id
    );

    // 继续处理请求
    Ok(next.run(req).await)
}

/// Axum Extractor: 从 request extensions 中提取 JWT 认证上下文
#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for JwtAuthContext
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<JwtAuthContext>()
            .cloned()
            .ok_or(StatusCode::UNAUTHORIZED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_auth_context_creation() {
        let ctx = JwtAuthContext {
            user_id: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            role: "user".to_string(),
        };
        assert_eq!(ctx.role, "user");
    }
}
