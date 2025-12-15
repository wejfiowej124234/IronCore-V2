//! Webhook签名验证服务
//!
//! 企业级实现：验证第三方支付服务商的Webhook签名
//! 解决问题：E.2 - Webhook验证机制未实现

use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Webhook验证器
pub struct WebhookValidator {
    /// 服务商密钥配置
    secrets: std::collections::HashMap<String, String>,
}

impl WebhookValidator {
    /// 创建验证器
    pub fn new() -> Self {
        let mut secrets = std::collections::HashMap::new();

        // 从环境变量加载各服务商的密钥
        if let Ok(ramp_secret) = std::env::var("RAMP_WEBHOOK_SECRET") {
            secrets.insert("ramp".to_string(), ramp_secret);
        }
        if let Ok(moonpay_secret) = std::env::var("MOONPAY_WEBHOOK_SECRET") {
            secrets.insert("moonpay".to_string(), moonpay_secret);
        }
        if let Ok(transak_secret) = std::env::var("TRANSAK_WEBHOOK_SECRET") {
            secrets.insert("transak".to_string(), transak_secret);
        }

        Self { secrets }
    }

    /// 验证Webhook签名
    ///
    /// # 参数
    /// - `provider`: 服务商名称（ramp/moonpay/transak）
    /// - `body`: 请求体（原始字符串）
    /// - `signature`: 签名值（从HTTP头提取）
    ///
    /// # 返回
    /// - Ok(()): 签名验证通过
    /// - Err: 签名验证失败
    pub fn verify_signature(&self, provider: &str, body: &str, signature: &str) -> Result<()> {
        let provider_lower = provider.to_lowercase();

        match provider_lower.as_str() {
            "ramp" => self.verify_ramp_signature(body, signature),
            "moonpay" => self.verify_moonpay_signature(body, signature),
            "transak" => self.verify_transak_signature(body, signature),
            _ => anyhow::bail!("Unsupported webhook provider: {}", provider),
        }
    }

    /// 验证Ramp签名
    /// Ramp使用HMAC-SHA256
    fn verify_ramp_signature(&self, body: &str, signature: &str) -> Result<()> {
        let secret = self
            .secrets
            .get("ramp")
            .context("Ramp webhook secret not configured")?;

        let expected_signature = self.compute_hmac_sha256(secret, body)?;

        if constant_time_compare(&expected_signature, signature) {
            Ok(())
        } else {
            anyhow::bail!("Invalid Ramp webhook signature")
        }
    }

    /// 验证MoonPay签名
    /// MoonPay使用HMAC-SHA256，签名格式为 hex
    fn verify_moonpay_signature(&self, body: &str, signature: &str) -> Result<()> {
        let secret = self
            .secrets
            .get("moonpay")
            .context("MoonPay webhook secret not configured")?;

        let expected_signature = self.compute_hmac_sha256(secret, body)?;

        if constant_time_compare(&expected_signature, signature) {
            Ok(())
        } else {
            anyhow::bail!("Invalid MoonPay webhook signature")
        }
    }

    /// 验证Transak签名
    /// Transak使用HMAC-SHA256
    fn verify_transak_signature(&self, body: &str, signature: &str) -> Result<()> {
        let secret = self
            .secrets
            .get("transak")
            .context("Transak webhook secret not configured")?;

        let expected_signature = self.compute_hmac_sha256(secret, body)?;

        if constant_time_compare(&expected_signature, signature) {
            Ok(())
        } else {
            anyhow::bail!("Invalid Transak webhook signature")
        }
    }

    /// 计算HMAC-SHA256签名
    fn compute_hmac_sha256(&self, secret: &str, message: &str) -> Result<String> {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).context("Invalid HMAC key length")?;

        mac.update(message.as_bytes());

        let result = mac.finalize();
        let code_bytes = result.into_bytes();

        // 转换为hex字符串
        Ok(hex::encode(code_bytes))
    }
}

impl Default for WebhookValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// 常量时间比较（防止时序攻击）
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (byte_a, byte_b) in a.bytes().zip(b.bytes()) {
        result |= byte_a ^ byte_b;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256() {
        let validator = WebhookValidator::new();
        let secret = "test_secret";
        let message = "test_message";

        let signature = validator.compute_hmac_sha256(secret, message).unwrap();
        assert!(!signature.is_empty());
        assert_eq!(signature.len(), 64); // SHA256 hex = 64 chars
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("abc", "abc"));
        assert!(!constant_time_compare("abc", "abd"));
        assert!(!constant_time_compare("abc", "ab"));
    }
}
