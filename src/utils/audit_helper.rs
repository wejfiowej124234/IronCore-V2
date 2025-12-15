//! 审计日志辅助函数
//! 提供统一的审计日志写入接口

use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::infrastructure::audit::ImmuCtx;

/// 写入审计事件
///
/// # Arguments
/// * `immu` - Immudb客户端
/// * `event` - 事件名称（如 "wallet.create"）
/// * `tenant_id` - 租户ID
/// * `actor` - 操作者ID（通常是user_id）
/// * `resource` - 资源ID（如wallet_id, tx_id等）
/// * `payload` - 事件负载（JSON对象）
///
/// # Returns
/// 返回Result，失败时返回错误（但不阻断主流程）
pub async fn write_audit_event(
    immu: &Arc<ImmuCtx>,
    event: &str,
    tenant_id: Uuid,
    actor: Uuid,
    resource: Uuid,
    payload: serde_json::Value,
) -> Result<()> {
    // 序列化payload
    let payload_str = payload.to_string();

    // 计算payload的SHA256哈希
    let mut hasher = Sha256::new();
    hasher.update(payload_str.as_bytes());
    let payload_hash = faster_hex::hex_string(&hasher.finalize());

    // 创建审计事件
    let audit_event = crate::infrastructure::audit::AuditEvent {
        event: event.into(),
        tenant_id: tenant_id.to_string(),
        actor: actor.to_string(),
        resource: resource.to_string(),
        payload_hash,
        ts: Utc::now().to_rfc3339(),
    };

    // 写入审计日志（最佳努力，不阻断主流程）
    immu.write_event(&audit_event).await?;

    Ok(())
}

/// 写入审计事件（简化版本，使用字符串作为resource）
pub async fn write_audit_event_str(
    immu: &Arc<ImmuCtx>,
    event: &str,
    tenant_id: Uuid,
    actor: Uuid,
    resource: &str,
    payload: serde_json::Value,
) -> Result<()> {
    let payload_str = payload.to_string();
    let mut hasher = Sha256::new();
    hasher.update(payload_str.as_bytes());
    let payload_hash = faster_hex::hex_string(&hasher.finalize());

    let audit_event = crate::infrastructure::audit::AuditEvent {
        event: event.into(),
        tenant_id: tenant_id.to_string(),
        actor: actor.to_string(),
        resource: resource.into(),
        payload_hash,
        ts: Utc::now().to_rfc3339(),
    };

    immu.write_event(&audit_event).await?;
    Ok(())
}

/// 写入审计事件（异步，不等待结果）
/// 用于不需要等待审计日志写入完成的场景
pub fn write_audit_event_async(
    immu: Arc<ImmuCtx>,
    event: String,
    tenant_id: Uuid,
    actor: Uuid,
    resource: Uuid,
    payload: serde_json::Value,
) {
    tokio::spawn(async move {
        if let Err(e) = write_audit_event(&immu, &event, tenant_id, actor, resource, payload).await
        {
            tracing::warn!("Failed to write audit event {}: {}", event, e);
            // 记录审计日志失败metrics（使用静态字符串）
            crate::metrics::count_err("audit.write.failed");
        } else {
            // 记录审计日志成功metrics（使用静态字符串）
            crate::metrics::count_ok("audit.write.success");
        }
    });
}

/// 写入审计事件（使用字符串actor，异步）
/// 用于actor不是UUID的场景（如"system"）
pub fn write_audit_event_async_str_actor(
    immu: Arc<ImmuCtx>,
    event: String,
    tenant_id: Uuid,
    actor: String,
    resource: Uuid,
    payload: serde_json::Value,
) {
    tokio::spawn(async move {
        let actor_uuid = Uuid::parse_str(&actor).unwrap_or_else(|_| Uuid::nil());
        if let Err(e) = write_audit_event_str(
            &immu,
            &event,
            tenant_id,
            actor_uuid,
            &resource.to_string(),
            payload,
        )
        .await
        {
            tracing::warn!("Failed to write audit event {}: {}", event, e);
            // 记录审计日志失败metrics（使用静态字符串）
            crate::metrics::count_err("audit.write.failed");
        } else {
            // 记录审计日志成功metrics（使用静态字符串）
            crate::metrics::count_ok("audit.write.success");
        }
    });
}
