// 日志脱敏模块 - 防止敏感信息泄露

use regex::Regex;
use std::sync::LazyLock;

// 以太坊地址正则（40个十六进制字符，可选0x前缀）
static ADDRESS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)0x[a-f0-9]{40}").unwrap());

// 交易哈希正则（64个十六进制字符，可选0x前缀）
static TX_HASH_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)0x[a-f0-9]{64}").unwrap());

// 私钥正则（64个十六进制字符，可选0x前缀）
static PRIVATE_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)0x[a-f0-9]{64}").unwrap());

// 助记词正则（匹配常见的助记词格式）
static MNEMONIC_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:[a-z]+\s+){11,23}[a-z]+\b").unwrap());

// 金额正则（匹配大数字，可能是余额或金额）
static AMOUNT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b\d{10,}\b").unwrap() // 10位以上的数字
});

/// 脱敏地址（保留前4位和后4位）
pub fn sanitize_address(addr: &str) -> String {
    if addr.len() < 12 {
        return "***".to_string();
    }
    let prefix = &addr[..6]; // 0x + 前2位
    let suffix = &addr[addr.len() - 4..];
    format!("{}...{}", prefix, suffix)
}

/// 脱敏交易哈希（保留前8位和后8位）
pub fn sanitize_tx_hash(hash: &str) -> String {
    if hash.len() < 18 {
        return "***".to_string();
    }
    let prefix = &hash[..10]; // 0x + 前4位
    let suffix = &hash[hash.len() - 8..];
    format!("{}...{}", prefix, suffix)
}

/// 脱敏金额（只显示数量级）
pub fn sanitize_amount(amount: &str) -> String {
    if let Ok(num) = amount.parse::<u128>() {
        if num == 0 {
            return "0".to_string();
        }
        // 计算数量级
        let magnitude = (num as f64).log10().floor() as u32;
        format!("~10^{}", magnitude)
    } else {
        "***".to_string()
    }
}

/// 脱敏字符串中的敏感信息
pub fn sanitize_log_message(msg: &str) -> String {
    let mut sanitized = msg.to_string();

    // 脱敏地址
    sanitized = ADDRESS_REGEX
        .replace_all(&sanitized, |caps: &regex::Captures| {
            caps.get(0)
                .map(|m| sanitize_address(m.as_str()))
                .unwrap_or_else(|| "***".to_string())
        })
        .to_string();

    // 脱敏交易哈希
    sanitized = TX_HASH_REGEX
        .replace_all(&sanitized, |caps: &regex::Captures| {
            caps.get(0)
                .map(|m| sanitize_tx_hash(m.as_str()))
                .unwrap_or_else(|| "***".to_string())
        })
        .to_string();

    // 脱敏私钥（如果存在）
    sanitized = PRIVATE_KEY_REGEX
        .replace_all(&sanitized, "***PRIVATE_KEY***")
        .to_string();

    // 脱敏助记词
    sanitized = MNEMONIC_REGEX
        .replace_all(&sanitized, "***MNEMONIC***")
        .to_string();

    // 脱敏大金额（在特定上下文中）
    if msg.contains("balance") || msg.contains("amount") || msg.contains("value") {
        sanitized = AMOUNT_REGEX
            .replace_all(&sanitized, |caps: &regex::Captures| {
                caps.get(0)
                    .map(|m| sanitize_amount(m.as_str()))
                    .unwrap_or_else(|| "***".to_string())
            })
            .to_string();
    }

    sanitized
}

/// 检查字符串是否包含敏感信息
pub fn contains_sensitive_data(msg: &str) -> bool {
    ADDRESS_REGEX.is_match(msg)
        || TX_HASH_REGEX.is_match(msg)
        || PRIVATE_KEY_REGEX.is_match(msg)
        || MNEMONIC_REGEX.is_match(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_address() {
        let addr = "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb";
        let sanitized = sanitize_address(addr);
        assert!(sanitized.contains("..."));
        assert!(!sanitized.contains("742d35Cc6634C0532925a3b844Bc9e7595f0bEb"));
    }

    #[test]
    fn test_sanitize_tx_hash() {
        let hash = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let sanitized = sanitize_tx_hash(hash);
        assert!(sanitized.contains("..."));
    }

    #[test]
    fn test_sanitize_log_message() {
        let msg = "User 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb has balance 1000000000000000000";
        let sanitized = sanitize_log_message(msg);
        assert!(!sanitized.contains("742d35Cc6634C0532925a3b844Bc9e7595f0bEb"));
        assert!(sanitized.contains("..."));
    }

    #[test]
    fn test_contains_sensitive_data() {
        assert!(contains_sensitive_data(
            "Address: 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"
        ));
        assert!(!contains_sensitive_data("Normal log message"));
    }
}
