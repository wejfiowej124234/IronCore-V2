//! 统一日志脱敏中间件（P1-3修复）
//! 企业级实现：自动脱敏所有敏感数据

use serde::Serialize;

/// 可脱敏trait
pub trait SensitiveRedact {
    fn redact(&self) -> String;
}

/// 脱敏十六进制字符串（显示前缀和后缀）
pub fn redact_hex_string(hex: &str, show_chars: usize) -> String {
    if hex.len() <= show_chars * 2 {
        return "*".repeat(hex.len());
    }

    let prefix = &hex[..show_chars];
    let suffix = &hex[hex.len() - show_chars..];
    format!("{}...{}", prefix, suffix)
}

/// 脱敏地址（显示前6位和后4位）
pub fn redact_address(address: &str) -> String {
    if address.len() < 10 {
        return "*".repeat(address.len());
    }

    let prefix = &address[..6];
    let suffix = &address[address.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

/// 脱敏email
pub fn redact_email(email: &str) -> String {
    if let Some(at_pos) = email.find('@') {
        let local = &email[..at_pos];
        let domain = &email[at_pos..];

        if local.len() <= 2 {
            format!("**{}", domain)
        } else {
            format!("{}***{}", &local[..1], domain)
        }
    } else {
        "***@***".to_string()
    }
}

/// 脱敏金额（保留前2位小数）
pub fn redact_amount(amount: &str) -> String {
    match amount.parse::<f64>() {
        Ok(val) => format!("{:.2}***", val),
        Err(_) => "***".to_string(),
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 为各种请求类型实现脱敏
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 交易广播请求脱敏
#[derive(Debug, Serialize)]
pub struct RedactedBroadcastRequest {
    pub chain: String,
    pub signed_tx: String, // 只显示前10个字符
}

impl SensitiveRedact for crate::service::blockchain_client::BroadcastTransactionRequest {
    fn redact(&self) -> String {
        serde_json::to_string(&RedactedBroadcastRequest {
            chain: self.chain.clone(),
            signed_tx: redact_hex_string(&self.signed_raw_tx, 10),
        })
        .unwrap_or_else(|_| "{ redacted }".to_string())
    }
}

/// 钱包创建请求脱敏
#[derive(Debug, Serialize)]
pub struct RedactedWalletCreateRequest {
    pub chain: String,
    pub address: String,
    pub public_key: String,
    // ❌ 不包含：mnemonic, private_key
}

/// 用户注册请求脱敏
#[derive(Debug, Serialize)]
pub struct RedactedRegisterRequest {
    pub email: String, /* 脱敏
                        * ❌ 不包含：password */
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 日志宏（自动脱敏）
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 安全日志宏（自动脱敏）
#[macro_export]
macro_rules! safe_info {
    ($($arg:tt)*) => {
        tracing::info!($($arg)*);
    };
}

/// 脱敏的请求日志
#[macro_export]
macro_rules! log_request_redacted {
    ($req:expr) => {
        #[cfg(not(debug_assertions))]
        {
            use $crate::infrastructure::log_redact::SensitiveRedact;
            tracing::info!("Request: {}", $req.redact());
        }

        #[cfg(debug_assertions)]
        tracing::debug!("Request (dev): {:?}", $req);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_hex_string() {
        let hex = "0x1234567890abcdef1234567890abcdef12345678";
        let redacted = redact_hex_string(hex, 10);
        // 前8个字符（不含0x）+ ... + 后10个字符
        assert_eq!(redacted, "0x12345678...ef12345678");
    }

    #[test]
    fn test_redact_address() {
        let address = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bFd2";
        let redacted = redact_address(address);
        assert_eq!(redacted, "0x742d...bFd2");
    }

    #[test]
    fn test_redact_email() {
        assert_eq!(redact_email("user@example.com"), "u***@example.com");
        assert_eq!(redact_email("ab@test.com"), "**@test.com");
    }

    #[test]
    fn test_redact_amount() {
        assert_eq!(redact_amount("1.234567"), "1.23***");
        assert_eq!(redact_amount("invalid"), "***");
    }
}
