//! 认证 API - 用户注册、登录、登出等
//!
//! 符合模块化架构设计

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use crate::{
    api::handlers::{
        get_login_history, get_me, login, logout, refresh_token, register, reset_password,
        set_password,
    },
    app_state::AppState,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 认证 API 路由
///
/// ## 公开端点（不需要认证）
/// - POST /register - 用户注册
/// - POST /login - 用户登录
/// - POST /refresh - 刷新访问令牌
///
/// ## 受保护端点（需要认证）
/// - POST /logout - 用户登出
/// - GET /me - 获取当前用户信息
/// - POST /set-password - 设置密码
/// - POST /reset-password - 重置密码
/// - GET /login-history - 登录历史记录
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        // 公开路由（不需要认证）
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        // 需要认证的路由（由全局认证中间件处理）
        .route("/logout", post(logout))
        .route("/me", get(get_me))
        .route("/set-password", post(set_password))
        .route("/reset-password", post(reset_password))
        .route("/login-history", get(get_login_history))
}
