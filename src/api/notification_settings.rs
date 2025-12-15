// 用户通知偏好设置 API
// 允许用户自定义接收哪些类型的通知、通知渠道、频率限制

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{middleware::auth::AuthInfoExtractor, response::success_response},
    app_state::AppState,
    error::AppError,
};

// ============ 通知类型枚举 ============

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    GasSpike,             // Gas 价格异常上涨
    NetworkCongestion,    // 网络拥堵
    NewFeature,           // 新功能发布
    SecurityAlert,        // 安全告警
    TransactionConfirmed, // 交易确认
    PriceAlert,           // 价格提醒
    SystemMaintenance,    // 系统维护
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannel {
    Email, // 邮件通知
    Push,  // 推送通知
    Sms,   // 短信通知
    InApp, // 应用内通知
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NotificationFrequency {
    Realtime, // 实时（每次触发都通知）
    Hourly,   // 每小时汇总
    Daily,    // 每日汇总
    Weekly,   // 每周汇总
}

// ============ 请求/响应结构 ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePreferenceReq {
    pub notification_type: NotificationType,
    pub channels: Vec<NotificationChannel>,
    pub frequency: NotificationFrequency,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdatePreferenceReq {
    pub channels: Option<Vec<NotificationChannel>>,
    pub frequency: Option<NotificationFrequency>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationPreferenceResp {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub channels: Vec<NotificationChannel>,
    pub frequency: NotificationFrequency,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationStatsResp {
    pub total_preferences: i64,
    pub enabled_count: i64,
    pub by_type: Vec<TypeCount>,
    pub by_channel: Vec<ChannelCount>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TypeCount {
    pub notification_type: NotificationType,
    pub count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChannelCount {
    pub channel: NotificationChannel,
    pub count: i64,
}

// ============ API 路由注册 ============

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/preferences", post(create_preference))
        .route("/preferences", get(list_preferences))
        .route("/preferences/:id", put(update_preference))
        .route("/preferences/:id", axum::routing::delete(delete_preference))
        .route("/preferences/stats", get(get_stats))
        .route("/preferences/reset-defaults", post(reset_to_defaults))
}

// ============ API 处理函数 ============

/// 创建通知偏好设置
#[utoipa::path(
    post,
    path = "/api/notifications/preferences",
    request_body = CreatePreferenceReq,
    responses(
        (status = 201, description = "Preference created", body = NotificationPreferenceResp),
        (status = 409, description = "Preference already exists"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_preference(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<CreatePreferenceReq>,
) -> Result<Json<crate::api::response::ApiResponse<NotificationPreferenceResp>>, AppError> {
    // 检查是否已存在该类型的偏好
    let notification_type_json = serde_json::to_string(&req.notification_type)
        .map_err(|e| AppError::internal(format!("Failed to serialize notification type: {}", e)))?;

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM notify.user_preferences WHERE user_id = $1 AND notification_type = $2)"
    )
    .bind(auth.user_id)
    .bind(notification_type_json)
    .fetch_one(&st.pool)
    .await?;

    if exists {
        return Err(AppError::conflict(
            "Notification preference already exists for this type",
        ));
    }

    let pref_id = Uuid::new_v4();
    let channels_json = serde_json::to_value(&req.channels)?;
    let frequency_str = serde_json::to_string(&req.frequency)?;
    let type_str = serde_json::to_string(&req.notification_type)?;

    sqlx::query(
        "INSERT INTO notify.user_preferences 
         (id, user_id, notification_type, channels, frequency, enabled)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(pref_id)
    .bind(auth.user_id)
    .bind(&type_str)
    .bind(&channels_json)
    .bind(&frequency_str)
    .bind(req.enabled)
    .execute(&st.pool)
    .await?;

    // 查询并返回创建的偏好
    let pref = get_preference_by_id(&st.pool, pref_id).await?;
    success_response(pref)
}

/// 查询用户的所有通知偏好
#[utoipa::path(
    get,
    path = "/api/notifications/preferences",
    responses(
        (status = 200, description = "Preferences list", body = Vec<NotificationPreferenceResp>),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_preferences(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
) -> Result<Json<crate::api::response::ApiResponse<Vec<NotificationPreferenceResp>>>, AppError> {
    let rows = sqlx::query(
        "SELECT id, user_id, notification_type, channels, frequency, enabled, created_at, updated_at
         FROM notify.user_preferences
         WHERE user_id = $1
         ORDER BY created_at DESC"
    )
    .bind(auth.user_id)
    .fetch_all(&st.pool)
    .await?;

    let mut prefs = Vec::new();
    for row in rows {
        let id: Uuid = row.get("id");
        let user_id: Uuid = row.get("user_id");
        let type_str: String = row.get("notification_type");
        let channels_json: sqlx::types::Json<Vec<NotificationChannel>> = row.get("channels");
        let frequency_str: String = row.get("frequency");
        let enabled: bool = row.get("enabled");
        let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
        let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

        let notification_type: NotificationType = serde_json::from_str(&type_str)?;
        let frequency: NotificationFrequency = serde_json::from_str(&frequency_str)?;

        prefs.push(NotificationPreferenceResp {
            id,
            user_id,
            notification_type,
            channels: channels_json.0,
            frequency,
            enabled,
            created_at: created_at.to_rfc3339(),
            updated_at: updated_at.to_rfc3339(),
        });
    }

    success_response(prefs)
}

/// 更新通知偏好设置
#[utoipa::path(
    put,
    path = "/api/notifications/preferences/{id}",
    request_body = UpdatePreferenceReq,
    responses(
        (status = 200, description = "Preference updated", body = NotificationPreferenceResp),
        (status = 404, description = "Preference not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_preference(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePreferenceReq>,
) -> Result<Json<crate::api::response::ApiResponse<NotificationPreferenceResp>>, AppError> {
    // 验证偏好属于当前用户
    let owner_check: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM notify.user_preferences WHERE id = $1 AND user_id = $2)",
    )
    .bind(id)
    .bind(auth.user_id)
    .fetch_one(&st.pool)
    .await?;

    if !owner_check {
        return Err(AppError::not_found("Notification preference not found"));
    }

    // 构建动态更新语句
    let mut updates = Vec::new();
    let mut param_count = 1;

    let mut query_str = "UPDATE notify.user_preferences SET ".to_string();

    if req.channels.is_some() {
        updates.push(format!("channels = ${}", param_count));
        param_count += 1;
    }
    if req.frequency.is_some() {
        updates.push(format!("frequency = ${}", param_count));
        param_count += 1;
    }
    if req.enabled.is_some() {
        updates.push(format!("enabled = ${}", param_count));
        param_count += 1;
    }

    if updates.is_empty() {
        return Err(AppError::bad_request("No fields to update"));
    }

    updates.push("updated_at = CURRENT_TIMESTAMP".to_string());
    query_str.push_str(&updates.join(", "));
    query_str.push_str(&format!(" WHERE id = ${}", param_count));

    let mut query = sqlx::query(&query_str);

    if let Some(channels) = &req.channels {
        let channels_json = serde_json::to_value(channels)?;
        query = query.bind(channels_json);
    }
    if let Some(frequency) = &req.frequency {
        let frequency_str = serde_json::to_string(frequency)?;
        query = query.bind(frequency_str);
    }
    if let Some(enabled) = req.enabled {
        query = query.bind(enabled);
    }

    query = query.bind(id);

    let result = query.execute(&st.pool).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Preference not found"));
    }

    let pref = get_preference_by_id(&st.pool, id).await?;
    success_response(pref)
}

/// 删除通知偏好设置
#[utoipa::path(
    delete,
    path = "/api/notifications/preferences/{id}",
    responses(
        (status = 204, description = "Preference deleted"),
        (status = 404, description = "Preference not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_preference(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM notify.user_preferences WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(auth.user_id)
        .execute(&st.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Preference not found"));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// 获取通知偏好统计
#[utoipa::path(
    get,
    path = "/api/notifications/preferences/stats",
    responses(
        (status = 200, description = "Statistics", body = NotificationStatsResp),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_stats(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
) -> Result<Json<crate::api::response::ApiResponse<NotificationStatsResp>>, AppError> {
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notify.user_preferences WHERE user_id = $1")
            .bind(auth.user_id)
            .fetch_one(&st.pool)
            .await?;

    let enabled: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notify.user_preferences WHERE user_id = $1 AND enabled = true",
    )
    .bind(auth.user_id)
    .fetch_one(&st.pool)
    .await?;

    // 按类型统计
    let type_counts: Vec<(String, i64)> = sqlx::query_as(
        "SELECT notification_type, COUNT(*) as count 
         FROM notify.user_preferences 
         WHERE user_id = $1 
         GROUP BY notification_type",
    )
    .bind(auth.user_id)
    .fetch_all(&st.pool)
    .await?;

    let by_type: Vec<TypeCount> = type_counts
        .into_iter()
        .map(|(type_str, count)| {
            let notification_type: NotificationType =
                serde_json::from_str(&type_str).unwrap_or(NotificationType::NewFeature);
            TypeCount {
                notification_type,
                count,
            }
        })
        .collect();

    // 按渠道统计（简化：假设每个偏好的所有渠道都计数）
    let by_channel = vec![
        ChannelCount {
            channel: NotificationChannel::Email,
            count: 0,
        },
        ChannelCount {
            channel: NotificationChannel::Push,
            count: 0,
        },
        ChannelCount {
            channel: NotificationChannel::InApp,
            count: 0,
        },
    ];

    success_response(NotificationStatsResp {
        total_preferences: total,
        enabled_count: enabled,
        by_type,
        by_channel,
    })
}

/// 重置为默认偏好设置
#[utoipa::path(
    post,
    path = "/api/notifications/preferences/reset-defaults",
    responses(
        (status = 200, description = "Defaults applied", body = Vec<NotificationPreferenceResp>),
    ),
    security(("bearer_auth" = []))
)]
pub async fn reset_to_defaults(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
) -> Result<Json<crate::api::response::ApiResponse<Vec<NotificationPreferenceResp>>>, AppError> {
    // 删除现有所有偏好
    sqlx::query("DELETE FROM notify.user_preferences WHERE user_id = $1")
        .bind(auth.user_id)
        .execute(&st.pool)
        .await?;

    // 创建默认偏好
    let defaults = vec![
        (
            NotificationType::SecurityAlert,
            vec![
                NotificationChannel::Email,
                NotificationChannel::Push,
                NotificationChannel::InApp,
            ],
            NotificationFrequency::Realtime,
            true,
        ),
        (
            NotificationType::TransactionConfirmed,
            vec![NotificationChannel::Push, NotificationChannel::InApp],
            NotificationFrequency::Realtime,
            true,
        ),
        (
            NotificationType::GasSpike,
            vec![NotificationChannel::InApp],
            NotificationFrequency::Hourly,
            true,
        ),
        (
            NotificationType::NewFeature,
            vec![NotificationChannel::Email],
            NotificationFrequency::Weekly,
            true,
        ),
        (
            NotificationType::SystemMaintenance,
            vec![NotificationChannel::Email, NotificationChannel::Push],
            NotificationFrequency::Realtime,
            true,
        ),
    ];

    let mut prefs = Vec::new();
    for (ntype, channels, frequency, enabled) in defaults {
        let pref_id = Uuid::new_v4();
        let channels_json = serde_json::to_value(&channels)?;
        let frequency_str = serde_json::to_string(&frequency)?;
        let type_str = serde_json::to_string(&ntype)?;

        sqlx::query(
            "INSERT INTO notify.user_preferences 
             (id, user_id, notification_type, channels, frequency, enabled)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(pref_id)
        .bind(auth.user_id)
        .bind(&type_str)
        .bind(&channels_json)
        .bind(&frequency_str)
        .bind(enabled)
        .execute(&st.pool)
        .await?;

        let pref = get_preference_by_id(&st.pool, pref_id).await?;
        prefs.push(pref);
    }

    success_response(prefs)
}

// ============ 辅助函数 ============

async fn get_preference_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<NotificationPreferenceResp, AppError> {
    let row = sqlx::query(
        "SELECT id, user_id, notification_type, channels, frequency, enabled, created_at, updated_at
         FROM notify.user_preferences
         WHERE id = $1"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    let user_id: Uuid = row.get("user_id");
    let type_str: String = row.get("notification_type");
    let channels_json: sqlx::types::Json<Vec<NotificationChannel>> = row.get("channels");
    let frequency_str: String = row.get("frequency");
    let enabled: bool = row.get("enabled");
    let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
    let updated_at: chrono::DateTime<chrono::Utc> = row.get("updated_at");

    let notification_type: NotificationType = serde_json::from_str(&type_str)?;
    let frequency: NotificationFrequency = serde_json::from_str(&frequency_str)?;

    Ok(NotificationPreferenceResp {
        id,
        user_id,
        notification_type,
        channels: channels_json.0,
        frequency,
        enabled,
        created_at: created_at.to_rfc3339(),
        updated_at: updated_at.to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_type_serialization() {
        let ntype = NotificationType::GasSpike;
        let json = serde_json::to_string(&ntype).unwrap();
        assert_eq!(json, "\"gas_spike\"");

        let parsed: NotificationType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, NotificationType::GasSpike);
    }

    #[test]
    fn test_notification_channel_serialization() {
        let channel = NotificationChannel::Email;
        let json = serde_json::to_string(&channel).unwrap();
        assert_eq!(json, "\"email\"");
    }

    #[test]
    fn test_notification_frequency_serialization() {
        let freq = NotificationFrequency::Daily;
        let json = serde_json::to_string(&freq).unwrap();
        assert_eq!(json, "\"daily\"");
    }
}
