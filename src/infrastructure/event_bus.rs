// Event Bus 框架
// 异步事件发布/订阅系统，支持事件持久化和重试

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

// ============ 事件类型定义 ============

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum DomainEvent {
    TransactionConfirmed {
        tx_id: Uuid,
        tx_hash: String,
        chain_type: String,
        user_id: Uuid,
    },
    FeeCollectorRotated {
        old_address: String,
        new_address: String,
        chain_type: String,
        rotated_at: String,
    },
    GasSpikeDetected {
        chain_type: String,
        current_gwei: f64,
        threshold_gwei: f64,
        detected_at: String,
    },
    WalletCreated {
        wallet_id: Uuid,
        user_id: Uuid,
        chain_type: String,
        address: String,
    },
    AdminOperationPerformed {
        operator_id: Uuid,
        operation: String,
        target: String,
        timestamp: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub event: DomainEvent,
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub retry_count: u32,
}

// ============ Event Handler Trait ============

#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &DomainEvent) -> Result<()>;
    fn event_types(&self) -> Vec<&'static str>;
}

// ============ Event Bus 接口 ============

#[async_trait]
pub trait EventBus: Send + Sync {
    /// 发布事件
    async fn publish(&self, event: DomainEvent) -> Result<()>;

    /// 订阅事件
    async fn subscribe(&self, handler: Arc<dyn EventHandler>);

    /// 获取事件历史
    async fn get_event_history(&self, limit: i64, offset: i64) -> Result<Vec<EventEnvelope>>;
}

// ============ 内存 Event Bus 实现（支持持久化） ============

pub struct InMemoryEventBus {
    handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>>,
    pool: Option<PgPool>,
    sender: mpsc::UnboundedSender<EventEnvelope>,
}

impl InMemoryEventBus {
    pub fn new(pool: Option<PgPool>) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<EventEnvelope>();
        let handlers: Arc<RwLock<Vec<Arc<dyn EventHandler>>>> = Arc::new(RwLock::new(Vec::new()));

        let handlers_clone = handlers.clone();

        // 后台任务：处理事件分发
        tokio::spawn(async move {
            while let Some(envelope) = receiver.recv().await {
                let handlers_read = handlers_clone.read().await;

                for handler in handlers_read.iter() {
                    let event_type = match &envelope.event {
                        DomainEvent::TransactionConfirmed { .. } => "TransactionConfirmed",
                        DomainEvent::FeeCollectorRotated { .. } => "FeeCollectorRotated",
                        DomainEvent::GasSpikeDetected { .. } => "GasSpikeDetected",
                        DomainEvent::WalletCreated { .. } => "WalletCreated",
                        DomainEvent::AdminOperationPerformed { .. } => "AdminOperationPerformed",
                    };

                    if handler.event_types().contains(&event_type) {
                        if let Err(e) = handler.handle(&envelope.event).await {
                            tracing::error!("Event handler error: {:?}, event: {:?}", e, envelope);
                            // 重试逻辑可以在这里实现
                        }
                    }
                }
            }
        });

        Self {
            handlers,
            pool,
            sender,
        }
    }

    /// 持久化事件到数据库
    async fn persist_event(&self, envelope: &EventEnvelope) -> Result<()> {
        if let Some(pool) = &self.pool {
            let event_json = serde_json::to_value(&envelope.event)?;

            sqlx::query(
                "INSERT INTO events.domain_events (id, event_type, event_data, published_at, retry_count)
                 VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(envelope.event_id)
            .bind(event_type_str(&envelope.event))
            .bind(event_json)
            .bind(envelope.published_at)
            .bind(envelope.retry_count as i32)
            .execute(pool)
            .await?;
        }
        Ok(())
    }
}

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<()> {
        let envelope = EventEnvelope {
            event_id: Uuid::new_v4(),
            event: event.clone(),
            published_at: chrono::Utc::now(),
            retry_count: 0,
        };

        // 持久化事件
        self.persist_event(&envelope).await?;

        // 发送到处理队列
        self.sender
            .send(envelope)
            .map_err(|e| anyhow::anyhow!("Failed to send event: {}", e))?;

        Ok(())
    }

    async fn subscribe(&self, handler: Arc<dyn EventHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }

    async fn get_event_history(&self, limit: i64, offset: i64) -> Result<Vec<EventEnvelope>> {
        if let Some(pool) = &self.pool {
            let rows = sqlx::query_as::<
                _,
                (
                    Uuid,
                    String,
                    sqlx::types::JsonValue,
                    chrono::DateTime<chrono::Utc>,
                    i32,
                ),
            >(
                "SELECT id, event_type, event_data, published_at, retry_count
                 FROM events.domain_events
                 ORDER BY published_at DESC
                 LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;

            let mut envelopes = Vec::new();
            for (id, _event_type, event_data, published_at, retry_count) in rows {
                let event: DomainEvent = serde_json::from_value(event_data)?;
                envelopes.push(EventEnvelope {
                    event_id: id,
                    event,
                    published_at,
                    retry_count: retry_count as u32,
                });
            }

            Ok(envelopes)
        } else {
            Ok(vec![])
        }
    }
}

// ============ 示例 Event Handler 实现 ============

pub struct TransactionConfirmedHandler;

#[async_trait]
impl EventHandler for TransactionConfirmedHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<()> {
        if let DomainEvent::TransactionConfirmed {
            tx_id,
            tx_hash,
            chain_type,
            user_id,
        } = event
        {
            tracing::info!(
                "Transaction confirmed: tx_id={}, tx_hash={}, chain={}, user={}",
                tx_id,
                tx_hash,
                chain_type,
                user_id
            );

            // 这里可以：
            // 1. 发送通知给用户
            // 2. 更新钱包余额
            // 3. 记录到审计日志
        }
        Ok(())
    }

    fn event_types(&self) -> Vec<&'static str> {
        vec!["TransactionConfirmed"]
    }
}

pub struct GasSpikeHandler;

#[async_trait]
impl EventHandler for GasSpikeHandler {
    async fn handle(&self, event: &DomainEvent) -> Result<()> {
        if let DomainEvent::GasSpikeDetected {
            chain_type,
            current_gwei,
            threshold_gwei,
            detected_at,
        } = event
        {
            tracing::warn!(
                "Gas spike detected on {}: current={} gwei, threshold={} gwei, time={}",
                chain_type,
                current_gwei,
                threshold_gwei,
                detected_at
            );

            // 这里可以：
            // 1. 发送告警通知
            // 2. 调整 RPC 选择策略
            // 3. 触发费用优化
        }
        Ok(())
    }

    fn event_types(&self) -> Vec<&'static str> {
        vec!["GasSpikeDetected"]
    }
}

// ============ 辅助函数 ============

fn event_type_str(event: &DomainEvent) -> &'static str {
    match event {
        DomainEvent::TransactionConfirmed { .. } => "TransactionConfirmed",
        DomainEvent::FeeCollectorRotated { .. } => "FeeCollectorRotated",
        DomainEvent::GasSpikeDetected { .. } => "GasSpikeDetected",
        DomainEvent::WalletCreated { .. } => "WalletCreated",
        DomainEvent::AdminOperationPerformed { .. } => "AdminOperationPerformed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = InMemoryEventBus::new(None);

        let handler = Arc::new(TransactionConfirmedHandler);
        bus.subscribe(handler).await;

        let event = DomainEvent::TransactionConfirmed {
            tx_id: Uuid::new_v4(),
            tx_hash: "0x123456".to_string(),
            chain_type: "ethereum".to_string(),
            user_id: Uuid::new_v4(),
        };

        let result = bus.publish(event).await;
        assert!(result.is_ok());

        // 等待异步处理
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[tokio::test]
    async fn test_multiple_handlers() {
        let bus = InMemoryEventBus::new(None);

        bus.subscribe(Arc::new(TransactionConfirmedHandler)).await;
        bus.subscribe(Arc::new(GasSpikeHandler)).await;

        let gas_event = DomainEvent::GasSpikeDetected {
            chain_type: "ethereum".to_string(),
            current_gwei: 150.0,
            threshold_gwei: 100.0,
            detected_at: chrono::Utc::now().to_rfc3339(),
        };

        let result = bus.publish(gas_event).await;
        assert!(result.is_ok());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    #[test]
    fn test_event_serialization() {
        let event = DomainEvent::WalletCreated {
            wallet_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            chain_type: "bitcoin".to_string(),
            address: "bc1q...".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("WalletCreated"));

        let parsed: DomainEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, event);
    }
}
