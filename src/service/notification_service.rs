use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PublishNotificationInput {
    pub title: String,
    pub body: String,
    pub category: String,
    pub severity: Option<String>,
    pub scope: String, // global / user (future: segment)
    pub creator_role: String,
    pub user_ids: Option<Vec<Uuid>>, // when scope == user
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NotificationFeedItem {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub category: String,
    pub severity: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NotificationService {
    pool: PgPool,
}

impl NotificationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn publish(&self, input: PublishNotificationInput) -> Result<Uuid> {
        let severity = input.severity.clone().unwrap_or_else(|| "info".to_string());
        // Insert notification
        let rec = sqlx::query(
            r#"INSERT INTO notify.notifications (title, body, category, severity, scope, creator_role)
               VALUES ($1,$2,$3,$4,$5,$6) RETURNING id"#
        )
        .bind(&input.title)
        .bind(&input.body)
        .bind(&input.category)
        .bind(&severity)
        .bind(&input.scope)
        .bind(&input.creator_role)
        .fetch_one(&self.pool)
        .await?;
        let notif_id: Uuid = rec.get("id");

        // Determine target users
        let target_user_ids: Vec<Uuid> = match input.scope.as_str() {
            "global" => {
                // For skeleton: limit to first 500 users
                let rows = sqlx::query("SELECT id FROM users LIMIT 500")
                    .fetch_all(&self.pool)
                    .await?;
                rows.into_iter()
                    .filter_map(|r| r.try_get::<Uuid, _>("id").ok())
                    .collect()
            }
            "user" => input.user_ids.clone().unwrap_or_default(),
            _ => return Err(anyhow!("unsupported scope: {}", input.scope)),
        };

        // Bulk insert deliveries (simple loop; future optimize with COPY)
        for uid in target_user_ids {
            if let Err(e) = sqlx::query(
                r#"INSERT INTO notify.deliveries (notification_id, user_id, channel, status)
                    VALUES ($1,$2,'in_app','pending') ON CONFLICT DO NOTHING"#,
            )
            .bind(notif_id)
            .bind(uid)
            .execute(&self.pool)
            .await
            {
                tracing::warn!(
                    notification_id = %notif_id,
                    user_id = %uid,
                    error = %e,
                    "Failed to insert notification delivery - continuing with other users"
                );
            }
        }

        Ok(notif_id)
    }

    pub async fn feed(&self, user_id: Uuid, limit: i64) -> Result<Vec<NotificationFeedItem>> {
        let rows = sqlx::query(
            r#"SELECT n.id, n.title, n.body, n.category, n.severity, d.status, n.created_at
                FROM notify.notifications n
                JOIN notify.deliveries d ON n.id = d.notification_id
                WHERE d.user_id = $1 AND n.revoked = false
                ORDER BY n.created_at DESC
                LIMIT $2"#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for r in rows {
            items.push(NotificationFeedItem {
                id: r.get("id"),
                title: r.get("title"),
                body: r.get("body"),
                category: r.get("category"),
                severity: r.get("severity"),
                status: r.get("status"),
                created_at: r.get("created_at"),
            });
        }
        Ok(items)
    }
}
