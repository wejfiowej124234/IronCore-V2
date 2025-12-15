use std::sync::Arc;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::{
        middleware::{auth::AuthInfoExtractor, rbac::require_admin},
        response::success_response,
    },
    app_state::AppState,
    error::AppError,
    service::notification_service::{NotificationService, PublishNotificationInput},
};

#[derive(Debug, Deserialize)]
pub struct PublishReq {
    pub title: String,
    pub body: String,
    pub category: String,
    pub severity: Option<String>,
    pub scope: String,               // global / user
    pub user_ids: Option<Vec<Uuid>>, // when scope == user
}

#[derive(Debug, Serialize)]
pub struct PublishResp {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct FeedResp {
    pub items: Vec<crate::service::notification_service::NotificationFeedItem>,
}

pub async fn publish_notification(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<PublishReq>,
) -> Result<Json<crate::api::response::ApiResponse<PublishResp>>, AppError> {
    // 简单角色校验（需具有 admin 权限）
    require_admin(&auth)?;

    let service = NotificationService::new(state.pool.clone());
    let id = service
        .publish(PublishNotificationInput {
            title: req.title,
            body: req.body,
            category: req.category,
            severity: req.severity,
            scope: req.scope,
            creator_role: auth.role.clone(),
            user_ids: req.user_ids,
        })
        .await
        .map_err(|e| AppError::internal(format!("publish failed: {}", e)))?;

    success_response(PublishResp { id })
}

pub async fn get_feed(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
) -> Result<Json<crate::api::response::ApiResponse<FeedResp>>, AppError> {
    let service = NotificationService::new(state.pool.clone());
    let items = service
        .feed(auth.user_id, 50)
        .await
        .map_err(|e| AppError::internal(format!("feed query failed: {}", e)))?;
    success_response(FeedResp { items })
}

/// ✅ 企业级标准 V1：通知路由
pub fn create_notification_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v1/notifications/publish", post(publish_notification))
        .route("/api/v1/notifications/feed", get(get_feed))
}

// Alias for consistency
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/publish", post(publish_notification))
        .route("/feed", get(get_feed))
}
