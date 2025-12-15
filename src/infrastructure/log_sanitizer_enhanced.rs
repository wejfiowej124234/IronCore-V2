//! 增强型日志脱敏器
//!
//! P3级修复：完善日志脱敏系统
//! 确保所有敏感数据在日志中被自动脱敏

use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 敏感字段模式
static SENSITIVE_PATTERNS: OnceLock<Vec<SensitivePattern>> = OnceLock::new();

/// 敏感模式定义
#[derive(Debug, Clone)]
struct SensitivePattern {
    field_name: Regex,
    redaction_strategy: RedactionStrategy,
}

#[derive(Debug, Clone, PartialEq)]
enum RedactionStrategy {
    FullRedact,                 // 完全替换为 "***"
    PartialShow(usize),         // 显示前N个字符
    PrefixSuffix(usize, usize), // 显示前缀和后缀
    #[allow(dead_code)]
    HashRedact, // 替换为哈希值
}

/// 日志脱敏器
pub struct LogSanitizer;

impl LogSanitizer {
    /// 初始化敏感模式
    fn init_patterns() -> Vec<SensitivePattern> {
        vec![
            // 私钥相关（绝对禁止记录）
            SensitivePattern {
                field_name: Regex::new(r"(?i)(private[_-]?key|priv[_-]?key|secret[_-]?key)")
                    .unwrap(),
                redaction_strategy: RedactionStrategy::FullRedact,
            },
            // 助记词（绝对禁止记录）
            SensitivePattern {
                field_name: Regex::new(r"(?i)(mnemonic|seed[_-]?phrase|recovery[_-]?phrase)")
                    .unwrap(),
                redaction_strategy: RedactionStrategy::FullRedact,
            },
            // 密码（绝对禁止记录）
            SensitivePattern {
                field_name: Regex::new(r"(?i)(password|passwd|pwd)").unwrap(),
                redaction_strategy: RedactionStrategy::FullRedact,
            },
            // 签名数据（显示前后缀）
            SensitivePattern {
                field_name: Regex::new(r"(?i)(signature|signed[_-]?tx|signed[_-]?data)").unwrap(),
                redaction_strategy: RedactionStrategy::PrefixSuffix(6, 4),
            },
            // Token（显示前缀）
            SensitivePattern {
                field_name: Regex::new(r"(?i)(token|access[_-]?token|auth[_-]?token)").unwrap(),
                redaction_strategy: RedactionStrategy::PartialShow(8),
            },
            // Email（脱敏本地部分）
            SensitivePattern {
                field_name: Regex::new(r"(?i)(email|e[_-]?mail)").unwrap(),
                redaction_strategy: RedactionStrategy::PartialShow(3),
            },
            // 加密数据
            SensitivePattern {
                field_name: Regex::new(r"(?i)(encrypted[_-]?data|cipher[_-]?text)").unwrap(),
                redaction_strategy: RedactionStrategy::PrefixSuffix(4, 4),
            },
        ]
    }

    /// 获取敏感模式列表
    fn patterns() -> &'static Vec<SensitivePattern> {
        SENSITIVE_PATTERNS.get_or_init(Self::init_patterns)
    }

    /// 脱敏JSON对象
    pub fn sanitize_json(value: &Value) -> Value {
        match value {
            Value::Object(map) => {
                let mut sanitized = serde_json::Map::new();
                for (key, val) in map {
                    let sanitized_value = if Self::is_sensitive_field(key) {
                        Self::redact_value(key, val)
                    } else {
                        Self::sanitize_json(val)
                    };
                    sanitized.insert(key.clone(), sanitized_value);
                }
                Value::Object(sanitized)
            }
            Value::Array(arr) => Value::Array(arr.iter().map(|v| Self::sanitize_json(v)).collect()),
            _ => value.clone(),
        }
    }

    /// 判断是否为敏感字段
    fn is_sensitive_field(field_name: &str) -> bool {
        Self::patterns()
            .iter()
            .any(|pattern| pattern.field_name.is_match(field_name))
    }

    /// 脱敏值
    fn redact_value(field_name: &str, value: &Value) -> Value {
        let pattern = Self::patterns()
            .iter()
            .find(|p| p.field_name.is_match(field_name));

        if let Some(pattern) = pattern {
            if let Some(str_val) = value.as_str() {
                let redacted = Self::apply_redaction(str_val, &pattern.redaction_strategy);
                return Value::String(redacted);
            }
        }

        Value::String("***REDACTED***".to_string())
    }

    /// 应用脱敏策略
    fn apply_redaction(value: &str, strategy: &RedactionStrategy) -> String {
        match strategy {
            RedactionStrategy::FullRedact => "***REDACTED***".to_string(),

            RedactionStrategy::PartialShow(n) => {
                if value.len() <= *n {
                    "*".repeat(value.len())
                } else {
                    format!("{}***", &value[..*n])
                }
            }

            RedactionStrategy::PrefixSuffix(prefix_len, suffix_len) => {
                let total_show = prefix_len + suffix_len;
                if value.len() <= total_show {
                    "*".repeat(value.len())
                } else {
                    format!(
                        "{}...{}",
                        &value[..*prefix_len],
                        &value[value.len() - suffix_len..]
                    )
                }
            }

            RedactionStrategy::HashRedact => {
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(value.as_bytes());
                format!("HASH:{:x}", hash)[..16].to_string()
            }
        }
    }

    /// 脱敏字符串中的敏感信息
    pub fn sanitize_string(text: &str) -> String {
        let mut result = text.to_string();

        // 脱敏常见的敏感模式
        let patterns = vec![
            // 0x开头的长十六进制字符串（可能是私钥或签名）
            (r"0x[0-9a-fA-F]{64,}", "0x***REDACTED***"),
            // JWT Token模式
            (
                r"Bearer\s+[A-Za-z0-9-_=]+\.[A-Za-z0-9-_=]+\.[A-Za-z0-9-_=]+",
                "Bearer ***TOKEN***",
            ),
            // Email地址
            (
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b",
                "***@***",
            ),
        ];

        for (pattern, replacement) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                result = re.replace_all(&result, replacement).to_string();
            }
        }

        result
    }
}

/// 脱敏结构化日志
#[derive(Debug, Serialize, Deserialize)]
pub struct SanitizedLogEntry {
    pub level: String,
    pub message: String,
    pub timestamp: String,
    pub fields: Value,
}

impl SanitizedLogEntry {
    pub fn sanitize(self) -> Self {
        Self {
            level: self.level,
            message: LogSanitizer::sanitize_string(&self.message),
            timestamp: self.timestamp,
            fields: LogSanitizer::sanitize_json(&self.fields),
        }
    }
}

/// 安全日志宏
#[macro_export]
macro_rules! safe_log_info {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        let sanitized = $crate::infrastructure::log_sanitizer_enhanced::LogSanitizer::sanitize_string(&message);
        tracing::info!("{}", sanitized);
    }};
}

#[macro_export]
macro_rules! safe_log_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        let sanitized = $crate::infrastructure::log_sanitizer_enhanced::LogSanitizer::sanitize_string(&message);
        tracing::error!("{}", sanitized);
    }};
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_sanitize_private_key() {
        let input = json!({
            "private_key": "0x1234567890abcdef1234567890abcdef12345678",
            "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6"
        });

        let sanitized = LogSanitizer::sanitize_json(&input);

        assert_eq!(sanitized["private_key"], "***REDACTED***");
        assert_ne!(sanitized["address"], "***REDACTED***"); // 地址不脱敏
    }

    #[test]
    fn test_sanitize_mnemonic() {
        let input = json!({
            "mnemonic": "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            "wallet_name": "My Wallet"
        });

        let sanitized = LogSanitizer::sanitize_json(&input);

        assert_eq!(sanitized["mnemonic"], "***REDACTED***");
        assert_eq!(sanitized["wallet_name"], "My Wallet");
    }

    #[test]
    fn test_sanitize_signature() {
        let input = json!({
            "signed_tx": "0xf86c808504a817c800825208943535353535353535353535353535353535353535880de0b6b3a76400008025a028ef61340bd939bc2195fe537567866003e1a15d3c71ff63e1590620aa636276a067cbe9d8997f761aecb703304b3800ccf555c9f3dc64214b297fb1966a3b6d83"
        });

        let sanitized = LogSanitizer::sanitize_json(&input);
        let sanitized_tx = sanitized["signed_tx"].as_str().unwrap();

        // 签名应该被部分脱敏
        assert!(sanitized_tx.contains("0xf86c"));
        assert!(sanitized_tx.contains("..."));
    }

    #[test]
    fn test_sanitize_string_with_private_key() {
        let text = "Processing transaction with private_key: 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = LogSanitizer::sanitize_string(text);

        assert!(sanitized.contains("***REDACTED***"));
        assert!(!sanitized.contains("1234567890abcdef1234567890abcdef"));
    }

    #[test]
    fn test_sanitize_jwt_token() {
        let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let sanitized = LogSanitizer::sanitize_string(text);

        assert!(sanitized.contains("Bearer ***TOKEN***"));
        assert!(!sanitized.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    }
}
