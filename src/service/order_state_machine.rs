//! 订单状态机模块
//!
//! 企业级实现：订单状态转换验证和审计
//! 解决问题：E.3 - 订单状态机不完整

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 订单状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
    Expired,
    Refunded,
}

impl OrderStatus {
    /// 从字符串解析状态
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(OrderStatus::Pending),
            "processing" => Ok(OrderStatus::Processing),
            "completed" => Ok(OrderStatus::Completed),
            "failed" => Ok(OrderStatus::Failed),
            "cancelled" => Ok(OrderStatus::Cancelled),
            "expired" => Ok(OrderStatus::Expired),
            "refunded" => Ok(OrderStatus::Refunded),
            _ => Err(anyhow::anyhow!("Invalid order status: {}", s)),
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderStatus::Pending => "pending",
            OrderStatus::Processing => "processing",
            OrderStatus::Completed => "completed",
            OrderStatus::Failed => "failed",
            OrderStatus::Cancelled => "cancelled",
            OrderStatus::Expired => "expired",
            OrderStatus::Refunded => "refunded",
        }
    }

    /// 判断是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderStatus::Completed
                | OrderStatus::Failed
                | OrderStatus::Cancelled
                | OrderStatus::Expired
                | OrderStatus::Refunded
        )
    }

    /// 判断是否可以取消
    pub fn can_cancel(&self) -> bool {
        matches!(self, OrderStatus::Pending | OrderStatus::Processing)
    }

    /// 判断是否可以重试
    pub fn can_retry(&self) -> bool {
        matches!(self, OrderStatus::Failed)
    }
}

/// 状态转换记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: OrderStatus,
    pub to: OrderStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reason: Option<String>,
}

/// 订单状态机
pub struct OrderStateMachine;

impl OrderStateMachine {
    /// 验证状态转换是否合法
    ///
    /// # 状态转换规则
    /// ```text
    /// Pending -> Processing  ✅
    /// Pending -> Cancelled   ✅
    /// Pending -> Expired     ✅
    /// Processing -> Completed ✅
    /// Processing -> Failed    ✅
    /// Completed -> Refunded   ✅ (特殊情况)
    ///
    /// 其他转换均不允许
    /// 终态不允许转换到其他状态（除了Completed->Refunded）
    /// ```
    pub fn validate_transition(from: OrderStatus, to: OrderStatus) -> Result<()> {
        // 相同状态：幂等性，允许
        if from == to {
            return Ok(());
        }

        // 定义合法的状态转换
        let valid = match from {
            OrderStatus::Pending => matches!(
                to,
                OrderStatus::Processing | OrderStatus::Cancelled | OrderStatus::Expired
            ),
            OrderStatus::Processing => matches!(to, OrderStatus::Completed | OrderStatus::Failed),
            OrderStatus::Completed => matches!(to, OrderStatus::Refunded),
            // 终态不允许转换（除了Completed->Refunded）
            OrderStatus::Failed
            | OrderStatus::Cancelled
            | OrderStatus::Expired
            | OrderStatus::Refunded => false,
        };

        if valid {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Invalid state transition: {} -> {}",
                from.as_str(),
                to.as_str()
            ))
        }
    }

    /// 获取状态的下一个可能状态列表
    pub fn get_next_states(current: OrderStatus) -> Vec<OrderStatus> {
        match current {
            OrderStatus::Pending => vec![
                OrderStatus::Processing,
                OrderStatus::Cancelled,
                OrderStatus::Expired,
            ],
            OrderStatus::Processing => vec![OrderStatus::Completed, OrderStatus::Failed],
            OrderStatus::Completed => vec![OrderStatus::Refunded],
            _ => vec![], // 其他终态无后续状态
        }
    }

    /// 创建状态转换记录
    pub fn create_transition(
        from: OrderStatus,
        to: OrderStatus,
        reason: Option<String>,
    ) -> StateTransition {
        StateTransition {
            from,
            to,
            timestamp: chrono::Utc::now(),
            reason,
        }
    }

    /// 检查操作是否允许
    pub fn can_perform_action(current: OrderStatus, action: &str) -> Result<()> {
        match action {
            "cancel" => {
                if current.can_cancel() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Cannot cancel order in status: {}",
                        current.as_str()
                    ))
                }
            }
            "retry" => {
                if current.can_retry() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Cannot retry order in status: {}",
                        current.as_str()
                    ))
                }
            }
            "refund" => {
                if current == OrderStatus::Completed {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Can only refund completed orders, current status: {}",
                        current.as_str()
                    ))
                }
            }
            _ => Err(anyhow::anyhow!("Unknown action: {}", action)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        // Pending -> Processing
        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Pending,
            OrderStatus::Processing
        )
        .is_ok());

        // Processing -> Completed
        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Processing,
            OrderStatus::Completed
        )
        .is_ok());

        // Pending -> Cancelled
        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Pending,
            OrderStatus::Cancelled
        )
        .is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        // Completed -> Pending (不允许)
        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Completed,
            OrderStatus::Pending
        )
        .is_err());

        // Failed -> Processing (不允许)
        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Failed,
            OrderStatus::Processing
        )
        .is_err());

        // Cancelled -> Completed (不允许)
        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Cancelled,
            OrderStatus::Completed
        )
        .is_err());
    }

    #[test]
    fn test_idempotent_transitions() {
        // 相同状态转换允许（幂等性）
        assert!(
            OrderStateMachine::validate_transition(OrderStatus::Pending, OrderStatus::Pending)
                .is_ok()
        );

        assert!(OrderStateMachine::validate_transition(
            OrderStatus::Completed,
            OrderStatus::Completed
        )
        .is_ok());
    }

    #[test]
    fn test_terminal_states() {
        assert!(OrderStatus::Completed.is_terminal());
        assert!(OrderStatus::Failed.is_terminal());
        assert!(OrderStatus::Cancelled.is_terminal());
        assert!(!OrderStatus::Pending.is_terminal());
        assert!(!OrderStatus::Processing.is_terminal());
    }

    #[test]
    fn test_action_permissions() {
        // Pending状态可以取消
        assert!(OrderStateMachine::can_perform_action(OrderStatus::Pending, "cancel").is_ok());

        // Completed状态不能取消
        assert!(OrderStateMachine::can_perform_action(OrderStatus::Completed, "cancel").is_err());

        // Failed状态可以重试
        assert!(OrderStateMachine::can_perform_action(OrderStatus::Failed, "retry").is_ok());

        // Pending状态不能重试
        assert!(OrderStateMachine::can_perform_action(OrderStatus::Pending, "retry").is_err());
    }

    #[test]
    fn test_get_next_states() {
        let next = OrderStateMachine::get_next_states(OrderStatus::Pending);
        assert_eq!(next.len(), 3);
        assert!(next.contains(&OrderStatus::Processing));
        assert!(next.contains(&OrderStatus::Cancelled));
        assert!(next.contains(&OrderStatus::Expired));

        let next = OrderStateMachine::get_next_states(OrderStatus::Completed);
        assert_eq!(next.len(), 1);
        assert!(next.contains(&OrderStatus::Refunded));

        let next = OrderStateMachine::get_next_states(OrderStatus::Failed);
        assert_eq!(next.len(), 0); // 终态无后续状态
    }
}
