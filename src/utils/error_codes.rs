//! 统一错误代码标准（R项修复）
//! 企业级实现：标准化错误代码，便于客户端处理

use serde::{Deserialize, Serialize};

/// 标准错误代码
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum ErrorCode {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 客户端错误（1xxx）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// 签名无效
    InvalidSignature = 1001,
    /// 钱包已锁定
    WalletLocked = 1002,
    /// 助记词无效
    InvalidMnemonic = 1003,
    /// 钱包密码错误
    InvalidWalletPassword = 1004,
    /// 交易格式错误
    InvalidTransactionFormat = 1005,
    /// 地址格式错误
    InvalidAddressFormat = 1006,

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 链上错误（2xxx）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// 余额不足
    InsufficientBalance = 2001,
    /// Gas价格过低
    GasPriceTooLow = 2002,
    /// Nonce过低
    NonceTooLow = 2003,
    /// 交易回滚
    TransactionReverted = 2004,
    /// Gas限制过低
    GasLimitTooLow = 2005,
    /// 合约执行失败
    ContractExecutionFailed = 2006,
    /// 代币不存在
    TokenNotFound = 2007,
    /// 代币余额不足
    InsufficientTokenBalance = 2008,

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 后端错误（3xxx）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// RPC不可用
    RpcUnavailable = 3001,
    /// 数据库错误
    DatabaseError = 3002,
    /// 速率限制
    RateLimitExceeded = 3003,
    /// 服务内部错误
    InternalServerError = 3004,
    /// 配置错误
    ConfigurationError = 3005,
    /// 外部服务不可用
    ExternalServiceUnavailable = 3006,

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 业务错误（4xxx）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// 风控拒绝
    RiskControlRejected = 4001,
    /// 钱包已存在
    WalletAlreadyExists = 4002,
    /// 订单未找到
    OrderNotFound = 4003,
    /// 订单状态不允许
    InvalidOrderStatus = 4004,
    /// 超出限额
    LimitExceeded = 4005,
    /// KYC未完成
    KycNotCompleted = 4006,
    /// 不支持的币种
    UnsupportedToken = 4007,
    /// 不支持的链
    UnsupportedChain = 4008,
    /// 滑点过大
    SlippageTooHigh = 4009,
    /// 跨链桥不可用
    BridgeUnavailable = 4010,

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 认证/授权错误（5xxx）
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    /// 未登录
    Unauthorized = 5001,
    /// Token过期
    TokenExpired = 5002,
    /// 权限不足
    Forbidden = 5003,
    /// 账户被锁定
    AccountLocked = 5004,
    /// 密码错误
    InvalidPassword = 5005,
    /// 邮箱已存在
    EmailAlreadyExists = 5006,
}

impl ErrorCode {
    /// 获取错误消息（英文）
    pub fn message_en(&self) -> &'static str {
        match self {
            // 客户端错误
            ErrorCode::InvalidSignature => "Invalid transaction signature",
            ErrorCode::WalletLocked => "Wallet is locked. Please unlock first",
            ErrorCode::InvalidMnemonic => "Invalid mnemonic phrase",
            ErrorCode::InvalidWalletPassword => "Incorrect wallet password",
            ErrorCode::InvalidTransactionFormat => "Invalid transaction format",
            ErrorCode::InvalidAddressFormat => "Invalid address format",

            // 链上错误
            ErrorCode::InsufficientBalance => "Insufficient balance",
            ErrorCode::GasPriceTooLow => "Gas price too low",
            ErrorCode::NonceTooLow => "Transaction nonce too low",
            ErrorCode::TransactionReverted => "Transaction reverted",
            ErrorCode::GasLimitTooLow => "Gas limit too low",
            ErrorCode::ContractExecutionFailed => "Smart contract execution failed",
            ErrorCode::TokenNotFound => "Token not found",
            ErrorCode::InsufficientTokenBalance => "Insufficient token balance",

            // 后端错误
            ErrorCode::RpcUnavailable => "Blockchain RPC unavailable",
            ErrorCode::DatabaseError => "Database error",
            ErrorCode::RateLimitExceeded => "Rate limit exceeded",
            ErrorCode::InternalServerError => "Internal server error",
            ErrorCode::ConfigurationError => "Configuration error",
            ErrorCode::ExternalServiceUnavailable => "External service unavailable",

            // 业务错误
            ErrorCode::RiskControlRejected => "Transaction rejected by risk control",
            ErrorCode::WalletAlreadyExists => "Wallet already exists",
            ErrorCode::OrderNotFound => "Order not found",
            ErrorCode::InvalidOrderStatus => "Invalid order status",
            ErrorCode::LimitExceeded => "Transaction limit exceeded",
            ErrorCode::KycNotCompleted => "KYC verification required",
            ErrorCode::UnsupportedToken => "Token not supported",
            ErrorCode::UnsupportedChain => "Chain not supported",
            ErrorCode::SlippageTooHigh => "Slippage exceeds tolerance",
            ErrorCode::BridgeUnavailable => "Cross-chain bridge unavailable",

            // 认证错误
            ErrorCode::Unauthorized => "Authentication required",
            ErrorCode::TokenExpired => "Token expired",
            ErrorCode::Forbidden => "Access forbidden",
            ErrorCode::AccountLocked => "Account locked due to multiple failed attempts",
            ErrorCode::InvalidPassword => "Invalid password",
            ErrorCode::EmailAlreadyExists => "Email already registered",
        }
    }

    /// 获取错误消息（中文）
    pub fn message_zh(&self) -> &'static str {
        match self {
            // 客户端错误
            ErrorCode::InvalidSignature => "交易签名无效",
            ErrorCode::WalletLocked => "钱包已锁定，请先解锁",
            ErrorCode::InvalidMnemonic => "助记词无效",
            ErrorCode::InvalidWalletPassword => "钱包密码错误",
            ErrorCode::InvalidTransactionFormat => "交易格式错误",
            ErrorCode::InvalidAddressFormat => "地址格式错误",

            // 链上错误
            ErrorCode::InsufficientBalance => "余额不足",
            ErrorCode::GasPriceTooLow => "Gas价格过低",
            ErrorCode::NonceTooLow => "交易序号过低",
            ErrorCode::TransactionReverted => "交易被回滚",
            ErrorCode::GasLimitTooLow => "Gas限制过低",
            ErrorCode::ContractExecutionFailed => "智能合约执行失败",
            ErrorCode::TokenNotFound => "代币不存在",
            ErrorCode::InsufficientTokenBalance => "代币余额不足",

            // 后端错误
            ErrorCode::RpcUnavailable => "区块链节点不可用",
            ErrorCode::DatabaseError => "数据库错误",
            ErrorCode::RateLimitExceeded => "请求过于频繁",
            ErrorCode::InternalServerError => "服务器内部错误",
            ErrorCode::ConfigurationError => "配置错误",
            ErrorCode::ExternalServiceUnavailable => "外部服务不可用",

            // 业务错误
            ErrorCode::RiskControlRejected => "交易被风控拒绝",
            ErrorCode::WalletAlreadyExists => "钱包已存在",
            ErrorCode::OrderNotFound => "订单不存在",
            ErrorCode::InvalidOrderStatus => "订单状态不允许此操作",
            ErrorCode::LimitExceeded => "超出交易限额",
            ErrorCode::KycNotCompleted => "需要完成实名认证",
            ErrorCode::UnsupportedToken => "不支持的代币",
            ErrorCode::UnsupportedChain => "不支持的链",
            ErrorCode::SlippageTooHigh => "滑点超出容忍范围",
            ErrorCode::BridgeUnavailable => "跨链桥不可用",

            // 认证错误
            ErrorCode::Unauthorized => "需要登录",
            ErrorCode::TokenExpired => "登录已过期",
            ErrorCode::Forbidden => "权限不足",
            ErrorCode::AccountLocked => "账户已被锁定",
            ErrorCode::InvalidPassword => "密码错误",
            ErrorCode::EmailAlreadyExists => "邮箱已被注册",
        }
    }

    /// 获取错误消息（多语言）
    pub fn message(&self, lang: &str) -> &'static str {
        match lang {
            "zh" | "zh-CN" | "zh-TW" => self.message_zh(),
            _ => self.message_en(),
        }
    }

    /// 获取恢复建议（英文）
    pub fn recovery_hint_en(&self) -> Option<&'static str> {
        match self {
            ErrorCode::GasPriceTooLow => Some("Try increasing gas price by at least 10%"),
            ErrorCode::InsufficientBalance => Some("Please add funds to your wallet"),
            ErrorCode::NonceTooLow => Some("Transaction nonce conflict. Please refresh and retry"),
            ErrorCode::WalletLocked => Some("Please unlock your wallet with wallet password"),
            ErrorCode::InvalidWalletPassword => {
                Some("Please check your wallet password and try again")
            }
            ErrorCode::SlippageTooHigh => Some("Increase slippage tolerance or try again later"),
            _ => None,
        }
    }

    /// 获取恢复建议（中文）
    pub fn recovery_hint_zh(&self) -> Option<&'static str> {
        match self {
            ErrorCode::GasPriceTooLow => Some("请将Gas价格提高至少10%"),
            ErrorCode::InsufficientBalance => Some("请充值到您的钱包"),
            ErrorCode::NonceTooLow => Some("交易序号冲突，请刷新后重试"),
            ErrorCode::WalletLocked => Some("请使用钱包密码解锁"),
            ErrorCode::InvalidWalletPassword => Some("请检查钱包密码后重试"),
            ErrorCode::SlippageTooHigh => Some("请提高滑点容忍度或稍后重试"),
            _ => None,
        }
    }

    /// 是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ErrorCode::RpcUnavailable
                | ErrorCode::ExternalServiceUnavailable
                | ErrorCode::RateLimitExceeded
                | ErrorCode::InternalServerError
        )
    }

    /// HTTP状态码
    pub fn http_status(&self) -> u16 {
        match self {
            // 400 Bad Request
            ErrorCode::InvalidSignature
            | ErrorCode::InvalidMnemonic
            | ErrorCode::InvalidTransactionFormat
            | ErrorCode::InvalidAddressFormat
            | ErrorCode::GasPriceTooLow
            | ErrorCode::NonceTooLow
            | ErrorCode::GasLimitTooLow
            | ErrorCode::UnsupportedToken
            | ErrorCode::UnsupportedChain => 400,

            // 401 Unauthorized
            ErrorCode::Unauthorized | ErrorCode::TokenExpired | ErrorCode::InvalidPassword => 401,

            // 403 Forbidden
            ErrorCode::Forbidden
            | ErrorCode::AccountLocked
            | ErrorCode::RiskControlRejected
            | ErrorCode::KycNotCompleted => 403,

            // 404 Not Found
            ErrorCode::OrderNotFound | ErrorCode::TokenNotFound => 404,

            // 409 Conflict
            ErrorCode::WalletAlreadyExists | ErrorCode::EmailAlreadyExists => 409,

            // 422 Unprocessable Entity
            ErrorCode::WalletLocked
            | ErrorCode::InvalidWalletPassword
            | ErrorCode::InvalidOrderStatus
            | ErrorCode::SlippageTooHigh => 422,

            // 429 Too Many Requests
            ErrorCode::RateLimitExceeded => 429,

            // 402 Payment Required
            ErrorCode::InsufficientBalance
            | ErrorCode::InsufficientTokenBalance
            | ErrorCode::LimitExceeded => 402,

            // 503 Service Unavailable
            ErrorCode::RpcUnavailable
            | ErrorCode::BridgeUnavailable
            | ErrorCode::ExternalServiceUnavailable => 503,

            // 500 Internal Server Error
            ErrorCode::DatabaseError
            | ErrorCode::InternalServerError
            | ErrorCode::ConfigurationError
            | ErrorCode::TransactionReverted
            | ErrorCode::ContractExecutionFailed => 500,
        }
    }
}

/// 错误响应体
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// 错误代码
    pub code: u32,
    /// 错误消息（技术）
    pub message: String,
    /// 用户友好消息
    pub user_message: String,
    /// 恢复建议
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_hint: Option<String>,
    /// 是否可重试
    pub retryable: bool,
    /// 详细信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// 追踪ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl ErrorResponse {
    /// 创建错误响应
    pub fn new(code: ErrorCode, details: Option<serde_json::Value>, lang: &str) -> Self {
        Self {
            code: code as u32,
            message: code.message_en().to_string(),
            user_message: code.message(lang).to_string(),
            recovery_hint: match lang {
                "zh" | "zh-CN" | "zh-TW" => code.recovery_hint_zh().map(|s| s.to_string()),
                _ => code.recovery_hint_en().map(|s| s.to_string()),
            },
            retryable: code.is_retryable(),
            details,
            trace_id: None,
        }
    }

    /// 设置追踪ID
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_values() {
        assert_eq!(ErrorCode::InvalidSignature as u32, 1001);
        assert_eq!(ErrorCode::InsufficientBalance as u32, 2001);
        assert_eq!(ErrorCode::RpcUnavailable as u32, 3001);
        assert_eq!(ErrorCode::RiskControlRejected as u32, 4001);
        assert_eq!(ErrorCode::Unauthorized as u32, 5001);
    }

    #[test]
    fn test_http_status_mapping() {
        assert_eq!(ErrorCode::InvalidSignature.http_status(), 400);
        assert_eq!(ErrorCode::Unauthorized.http_status(), 401);
        assert_eq!(ErrorCode::Forbidden.http_status(), 403);
        assert_eq!(ErrorCode::OrderNotFound.http_status(), 404);
        assert_eq!(ErrorCode::WalletAlreadyExists.http_status(), 409);
        assert_eq!(ErrorCode::RateLimitExceeded.http_status(), 429);
        assert_eq!(ErrorCode::InsufficientBalance.http_status(), 402);
        assert_eq!(ErrorCode::RpcUnavailable.http_status(), 503);
    }

    #[test]
    fn test_multilingual_messages() {
        assert_eq!(
            ErrorCode::InsufficientBalance.message("en"),
            "Insufficient balance"
        );
        assert_eq!(ErrorCode::InsufficientBalance.message("zh"), "余额不足");
    }

    #[test]
    fn test_recovery_hints() {
        assert_eq!(
            ErrorCode::GasPriceTooLow.recovery_hint_en(),
            Some("Try increasing gas price by at least 10%")
        );
        assert_eq!(
            ErrorCode::GasPriceTooLow.recovery_hint_zh(),
            Some("请将Gas价格提高至少10%")
        );
    }

    #[test]
    fn test_retryable() {
        assert!(ErrorCode::RpcUnavailable.is_retryable());
        assert!(ErrorCode::RateLimitExceeded.is_retryable());
        assert!(!ErrorCode::InvalidSignature.is_retryable());
        assert!(!ErrorCode::InsufficientBalance.is_retryable());
    }
}
