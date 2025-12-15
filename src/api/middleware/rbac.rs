//! 基于角色的权限控制（RBAC）中间件
//! 企业级实现：检查用户角色是否有权限访问特定资源
//!
//! 角色定义：
//! - admin: 管理员，拥有所有权限
//! - operator: 操作员，可以执行操作但无法修改系统配置
//! - viewer: 查看者，只能查看数据
//!
//! 权限检查函数：
//! - require_role(): 要求特定角色
//! - require_any_role(): 要求角色在允许列表中
//! - require_admin(): 要求管理员角色
//! - require_operator_or_admin(): 要求操作员或管理员角色

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

use crate::{api::middleware::auth::AuthInfo, error::AppError};

/// 角色权限定义
pub mod roles {
    pub const ADMIN: &str = "admin";
    pub const OPERATOR: &str = "operator";
    pub const VIEWER: &str = "viewer";
}

/// 权限检查：要求特定角色
pub fn require_role(auth_info: &AuthInfo, required_role: &str) -> Result<(), AppError> {
    if auth_info.role != required_role {
        return Err(AppError {
            code: crate::error::AppErrorCode::Forbidden,
            message: format!("Required role: {}", required_role),
            status: StatusCode::FORBIDDEN,
            trace_id: None,
        });
    }
    Ok(())
}

/// 权限检查：要求角色在允许列表中
pub fn require_any_role(auth_info: &AuthInfo, allowed_roles: &[&str]) -> Result<(), AppError> {
    if !allowed_roles.contains(&auth_info.role.as_str()) {
        return Err(AppError {
            code: crate::error::AppErrorCode::Forbidden,
            message: format!("Required one of roles: {:?}", allowed_roles),
            status: StatusCode::FORBIDDEN,
            trace_id: None,
        });
    }
    Ok(())
}

/// 权限检查：要求管理员角色
pub fn require_admin(auth_info: &AuthInfo) -> Result<(), AppError> {
    require_role(auth_info, roles::ADMIN)
}

/// 权限检查：要求操作员或管理员角色
pub fn require_operator_or_admin(auth_info: &AuthInfo) -> Result<(), AppError> {
    require_any_role(auth_info, &[roles::ADMIN, roles::OPERATOR])
}

/// RBAC中间件：检查用户角色
pub async fn rbac_middleware(
    req: Request,
    next: Next,
    required_role: Option<&str>,
    allowed_roles: Option<&[&str]>,
) -> Result<Response, AppError> {
    // 从请求扩展中获取认证信息
    let auth_info = req.extensions().get::<AuthInfo>().ok_or_else(|| AppError {
        code: crate::error::AppErrorCode::Unauthorized,
        message: "Not authenticated".into(),
        status: StatusCode::UNAUTHORIZED,
        trace_id: None,
    })?;

    // 检查权限
    if let Some(role) = required_role {
        require_role(auth_info, role)?;
    } else if let Some(roles) = allowed_roles {
        require_any_role(auth_info, roles)?;
    }

    Ok(next.run(req).await)
}
