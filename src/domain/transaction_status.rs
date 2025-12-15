//! 统一交易状态定义
//! 企业级实现：所有交易表使用统一的状态枚举

use std::fmt;

use serde::{Deserialize, Serialize};

/// 企业级统一交易状态机
/// ✅ 适用于所有链和所有交易类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// 交易已创建，等待签名
    Created,

    /// 交易已签名，等待广播
    Signed,

    /// 交易已广播到节点，等待上链
    Pending,

    /// 交易已上链，正在确认中（区块确认数 < 最低要求）
    Executing,

    /// 交易已确认（区块确认数 >= 最低要求）
    Confirmed,

    /// 交易失败（链上执行失败或revert）
    Failed,

    /// 交易超时（长时间未确认）
    Timeout,

    /// 交易被替换（被更高 gas 的交易替换）
    Replaced,

    /// 交易已取消
    Cancelled,
}

impl TransactionStatus {
    /// 获取状态描述
    pub fn description(&self) -> &'static str {
        match self {
            Self::Created => "交易已创建",
            Self::Signed => "交易已签名",
            Self::Pending => "交易待确认",
            Self::Executing => "交易确认中",
            Self::Confirmed => "交易已确认",
            Self::Failed => "交易失败",
            Self::Timeout => "交易超时",
            Self::Replaced => "交易已替换",
            Self::Cancelled => "交易已取消",
        }
    }

    /// 是否为最终状态（不可再转换）
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            Self::Confirmed | Self::Failed | Self::Timeout | Self::Replaced | Self::Cancelled
        )
    }

    /// 验证状态转换合法性
    /// ✅ 企业级：强制状态机约束
    pub fn can_transition_to(&self, target: &Self) -> bool {
        use TransactionStatus::*;

        match (self, target) {
            // Created → Signed | Cancelled
            (Created, Signed) | (Created, Cancelled) => true,

            // Signed → Pending | Cancelled
            (Signed, Pending) | (Signed, Cancelled) => true,

            // Pending → Executing | Failed | Timeout | Cancelled
            (Pending, Executing)
            | (Pending, Failed)
            | (Pending, Timeout)
            | (Pending, Cancelled) => true,

            // Executing → Confirmed | Failed | Timeout | Replaced
            (Executing, Confirmed)
            | (Executing, Failed)
            | (Executing, Timeout)
            | (Executing, Replaced) => true,

            // 最终状态不可转换
            _ if self.is_final() => false,

            // 其他转换非法
            _ => false,
        }
    }

    /// 从字符串解析（兼容旧数据）
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "created" => Self::Created,
            "signed" => Self::Signed,
            "pending" => Self::Pending,
            "executing" => Self::Executing,
            "confirmed" | "success" | "completed" => Self::Confirmed,
            "failed" | "error" => Self::Failed,
            "timeout" | "expired" => Self::Timeout,
            "replaced" | "superseded" => Self::Replaced,
            "cancelled" | "canceled" => Self::Cancelled,
            _ => Self::Pending, // 默认值
        }
    }

    /// 转换为数据库字符串
    pub fn to_db_string(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Signed => "signed",
            Self::Pending => "pending",
            Self::Executing => "executing",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Timeout => "timeout",
            Self::Replaced => "replaced",
            Self::Cancelled => "cancelled",
        }
    }

    /// 转换为前端显示字符串（国际化key）
    pub fn to_i18n_key(&self) -> &'static str {
        match self {
            Self::Created => "transaction.status.created",
            Self::Signed => "transaction.status.signed",
            Self::Pending => "transaction.status.pending",
            Self::Executing => "transaction.status.executing",
            Self::Confirmed => "transaction.status.confirmed",
            Self::Failed => "transaction.status.failed",
            Self::Timeout => "transaction.status.timeout",
            Self::Replaced => "transaction.status.replaced",
            Self::Cancelled => "transaction.status.cancelled",
        }
    }
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_db_string())
    }
}

impl PartialOrd for TransactionStatus {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TransactionStatus {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use TransactionStatus::*;

        let order = |s: &TransactionStatus| -> u8 {
            match s {
                Created => 0,
                Signed => 1,
                Pending => 2,
                Executing => 3,
                Confirmed => 4,
                Failed => 4,
                Timeout => 4,
                Replaced => 4,
                Cancelled => 4,
            }
        };

        order(self).cmp(&order(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transitions() {
        use TransactionStatus::*;

        // 合法转换
        assert!(Created.can_transition_to(&Signed));
        assert!(Signed.can_transition_to(&Pending));
        assert!(Pending.can_transition_to(&Executing));
        assert!(Executing.can_transition_to(&Confirmed));

        // 非法转换
        assert!(!Created.can_transition_to(&Confirmed));
        assert!(!Pending.can_transition_to(&Signed));

        // 最终状态不可转换
        assert!(!Confirmed.can_transition_to(&Pending));
        assert!(!Failed.can_transition_to(&Executing));
    }

    #[test]
    fn test_is_final() {
        use TransactionStatus::*;

        assert!(!Created.is_final());
        assert!(!Pending.is_final());
        assert!(Confirmed.is_final());
        assert!(Failed.is_final());
        assert!(Timeout.is_final());
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            TransactionStatus::from_str("pending"),
            TransactionStatus::Pending
        );
        assert_eq!(
            TransactionStatus::from_str("confirmed"),
            TransactionStatus::Confirmed
        );
        assert_eq!(
            TransactionStatus::from_str("success"),
            TransactionStatus::Confirmed
        );
    }

    #[test]
    fn test_to_db_string() {
        assert_eq!(TransactionStatus::Pending.to_db_string(), "pending");
        assert_eq!(TransactionStatus::Confirmed.to_db_string(), "confirmed");
    }
}
