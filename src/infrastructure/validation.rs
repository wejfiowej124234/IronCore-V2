//! 输入验证和清理模块
//! 提供输入验证、清理和业务规则验证功能

use anyhow::{anyhow, Result};

/// 验证邮箱格式（加密后的邮箱）
pub fn validate_email_cipher(email_cipher: &str) -> Result<()> {
    if email_cipher.is_empty() {
        return Err(anyhow!("Email cipher cannot be empty"));
    }
    if email_cipher.len() > 500 {
        return Err(anyhow!("Email cipher too long"));
    }
    Ok(())
}

/// 验证手机号格式（加密后的手机号）
pub fn validate_phone_cipher(phone_cipher: &str) -> Result<()> {
    if phone_cipher.is_empty() {
        return Err(anyhow!("Phone cipher cannot be empty"));
    }
    if phone_cipher.len() > 200 {
        return Err(anyhow!("Phone cipher too long"));
    }
    Ok(())
}

/// 验证密码强度
pub fn validate_password_strength(password: &str) -> Result<()> {
    if password.len() < 8 {
        return Err(anyhow!("Password must be at least 8 characters"));
    }
    if password.len() > 128 {
        return Err(anyhow!("Password too long"));
    }

    // 检查是否包含数字
    if !password.chars().any(|c| c.is_ascii_digit()) {
        return Err(anyhow!("Password must contain at least one digit"));
    }

    // 检查是否包含字母
    if !password.chars().any(|c| c.is_ascii_alphabetic()) {
        return Err(anyhow!("Password must contain at least one letter"));
    }

    Ok(())
}

/// 清理和验证字符串输入（防止XSS）
pub fn sanitize_string(input: &str, max_length: usize) -> Result<String> {
    if input.len() > max_length {
        return Err(anyhow!("Input too long (max {} characters)", max_length));
    }

    // 移除控制字符（保留换行和制表符）
    let cleaned: String = input
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();

    Ok(cleaned)
}

/// 验证UUID格式
pub fn validate_uuid(uuid_str: &str) -> Result<()> {
    uuid::Uuid::parse_str(uuid_str).map_err(|_| anyhow!("Invalid UUID format"))?;
    Ok(())
}

/// 验证角色名称
pub fn validate_role(role: &str) -> Result<()> {
    const ALLOWED_ROLES: &[&str] = &["admin", "operator", "viewer"];
    if !ALLOWED_ROLES.contains(&role) {
        return Err(anyhow!("Invalid role. Allowed roles: {:?}", ALLOWED_ROLES));
    }
    Ok(())
}

/// 验证API Key名称
pub fn validate_api_key_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("API key name cannot be empty"));
    }
    if name.len() > 100 {
        return Err(anyhow!("API key name too long (max 100 characters)"));
    }
    Ok(())
}

/// 验证租户名称
pub fn validate_tenant_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Tenant name cannot be empty"));
    }
    if name.len() > 200 {
        return Err(anyhow!("Tenant name too long (max 200 characters)"));
    }
    Ok(())
}

/// 验证策略名称
pub fn validate_policy_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(anyhow!("Policy name cannot be empty"));
    }
    if name.len() > 200 {
        return Err(anyhow!("Policy name too long (max 200 characters)"));
    }
    Ok(())
}

/// 验证JSON字符串
pub fn validate_json(json_str: &str) -> Result<()> {
    serde_json::from_str::<serde_json::Value>(json_str)
        .map_err(|e| anyhow!("Invalid JSON: {}", e))?;
    Ok(())
}

/// 验证交易金额✅企业级
pub fn validate_amount(amount_str: &str) -> Result<f64> {
    let amount = amount_str
        .trim()
        .parse::<f64>()
        .map_err(|_| anyhow!("Invalid amount format"))?;

    if amount <= 0.0 || !amount.is_finite() {
        return Err(anyhow!("Amount must be > 0 and finite"));
    }

    if amount > 1e15 {
        return Err(anyhow!("Amount too large"));
    }

    Ok(amount)
}

/// 验证链名称✅
pub fn validate_chain(chain: &str) -> Result<String> {
    let chain_lower = chain.trim().to_lowercase();
    if chain_lower.is_empty() {
        return Err(anyhow!("Chain identifier required"));
    }

    // 支持的链
    const SUPPORTED: &[&str] = &[
        "ethereum", "eth", "bsc", "binance", "polygon", "matic", "solana", "sol", "bitcoin", "btc",
        "ton",
    ];
    if !SUPPORTED.contains(&chain_lower.as_str()) {
        return Err(anyhow!("Unsupported chain: {}", chain));
    }

    Ok(chain_lower)
}

/// 验证地址格式✅
pub fn validate_address(address: &str, chain: &str) -> Result<()> {
    let addr = address.trim();
    if addr.is_empty() {
        return Err(anyhow!("Address required"));
    }

    match chain.to_lowercase().as_str() {
        "ethereum" | "eth" | "bsc" | "polygon" | "matic" => {
            if !addr.starts_with("0x") || addr.len() != 42 {
                return Err(anyhow!("Invalid EVM address"));
            }
        }
        "solana" | "sol" => {
            if addr.len() < 32 || addr.len() > 44 {
                return Err(anyhow!("Invalid Solana address"));
            }
        }
        "bitcoin" | "btc" => {
            if !addr.starts_with("bc1") && !addr.starts_with("1") && !addr.starts_with("3") {
                return Err(anyhow!("Invalid Bitcoin address"));
            }
        }
        "ton" => {
            if !addr.starts_with("EQ") && !addr.starts_with("0:") {
                return Err(anyhow!("Invalid TON address"));
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        assert!(validate_password_strength("password123").is_ok());
        assert!(validate_password_strength("short").is_err());
        assert!(validate_password_strength("12345678").is_err()); // 没有字母
        assert!(validate_password_strength("abcdefgh").is_err()); // 没有数字
    }

    #[test]
    fn test_role_validation() {
        assert!(validate_role("admin").is_ok());
        assert!(validate_role("operator").is_ok());
        assert!(validate_role("viewer").is_ok());
        assert!(validate_role("invalid").is_err());
    }

    #[test]
    fn test_sanitize_string() {
        let input = "Hello\x00World";
        let cleaned = sanitize_string(input, 100).unwrap();
        assert_eq!(cleaned, "HelloWorld");
    }
}
