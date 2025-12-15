//! Webhook签名验证模块
//! 支持Onramper, TransFi, Alchemy Pay的签名验证算法

use anyhow::{anyhow, Result};
use axum::http::HeaderMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Onramper Webhook签名验证
/// 算法: HMAC-SHA256(secret, request_body)
/// Header: X-Onramper-Signature
pub fn verify_onramper_signature(headers: &HeaderMap, body: &str, secret: &str) -> Result<()> {
    let signature = headers
        .get("X-Onramper-Signature")
        .ok_or_else(|| anyhow!("Missing X-Onramper-Signature header"))?
        .to_str()
        .map_err(|_| anyhow!("Invalid signature format"))?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow!("Invalid HMAC key: {}", e))?;
    mac.update(body.as_bytes());

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if signature.to_lowercase() != expected_signature.to_lowercase() {
        return Err(anyhow!("Signature verification failed for Onramper"));
    }

    Ok(())
}

/// TransFi Webhook签名验证
/// 算法: HMAC-SHA256(secret, timestamp + method + path + body)
/// Headers: X-TransFi-Signature, X-TransFi-Timestamp
pub fn verify_transfi_signature(
    headers: &HeaderMap,
    body: &str,
    method: &str,
    path: &str,
    secret: &str,
) -> Result<()> {
    let signature = headers
        .get("X-TransFi-Signature")
        .ok_or_else(|| anyhow!("Missing X-TransFi-Signature header"))?
        .to_str()
        .map_err(|_| anyhow!("Invalid signature format"))?;

    let timestamp = headers
        .get("X-TransFi-Timestamp")
        .ok_or_else(|| anyhow!("Missing X-TransFi-Timestamp header"))?
        .to_str()
        .map_err(|_| anyhow!("Invalid timestamp format"))?;

    // 防重放攻击：检查时间戳（允许5分钟偏差）
    let now = chrono::Utc::now().timestamp();
    let webhook_time: i64 = timestamp
        .parse()
        .map_err(|_| anyhow!("Invalid timestamp value"))?;

    if (now - webhook_time).abs() > 300 {
        return Err(anyhow!("Webhook timestamp expired (>5 minutes)"));
    }

    // 构造签名payload
    let payload = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow!("Invalid HMAC key: {}", e))?;
    mac.update(payload.as_bytes());

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if signature.to_lowercase() != expected_signature.to_lowercase() {
        return Err(anyhow!("Signature verification failed for TransFi"));
    }

    Ok(())
}

/// Alchemy Pay Webhook签名验证
/// 算法: HMAC-SHA256(secret, request_body)
/// Header: X-Alchemy-Signature
pub fn verify_alchemypay_signature(headers: &HeaderMap, body: &str, secret: &str) -> Result<()> {
    let signature = headers
        .get("X-Alchemy-Signature")
        .ok_or_else(|| anyhow!("Missing X-Alchemy-Signature header"))?
        .to_str()
        .map_err(|_| anyhow!("Invalid signature format"))?;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| anyhow!("Invalid HMAC key: {}", e))?;
    mac.update(body.as_bytes());

    let expected_signature = hex::encode(mac.finalize().into_bytes());

    if signature.to_lowercase() != expected_signature.to_lowercase() {
        return Err(anyhow!("Signature verification failed for Alchemy Pay"));
    }

    Ok(())
}

/// 测试模式签名验证（开发环境）
/// 跳过签名验证，仅检查存在性
pub fn verify_test_signature(headers: &HeaderMap, provider: &str) -> Result<()> {
    let header_name = match provider {
        "onramper" => "X-Onramper-Signature",
        "transfi" => "X-TransFi-Signature",
        "alchemypay" => "X-Alchemy-Signature",
        _ => return Err(anyhow!("Unknown provider: {}", provider)),
    };

    if headers.get(header_name).is_none() {
        return Err(anyhow!("Missing signature header: {}", header_name));
    }

    tracing::warn!(
        "⚠️ [SECURITY] Test mode: Skipping signature verification for {}",
        provider
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderMap;

    use super::*;

    #[test]
    fn test_onramper_signature() {
        let mut headers = HeaderMap::new();
        let body = r#"{"orderId":"123","status":"completed"}"#;
        let secret = "test_secret_key";

        // 生成正确的签名
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        headers.insert("X-Onramper-Signature", signature.parse().unwrap());

        assert!(verify_onramper_signature(&headers, body, secret).is_ok());
    }

    #[test]
    fn test_transfi_signature() {
        let mut headers = HeaderMap::new();
        let body = r#"{"orderId":"456","status":"processing"}"#;
        let secret = "test_transfi_secret";
        let method = "POST";
        let path = "/api/v1/fiat/webhook/transfi";
        let timestamp = chrono::Utc::now().timestamp().to_string();

        // 构造payload
        let payload = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        headers.insert("X-TransFi-Signature", signature.parse().unwrap());
        headers.insert("X-TransFi-Timestamp", timestamp.parse().unwrap());

        assert!(verify_transfi_signature(&headers, body, method, path, secret).is_ok());
    }

    #[test]
    fn test_alchemypay_signature() {
        let mut headers = HeaderMap::new();
        let body = r#"{"orderId":"789","status":"completed"}"#;
        let secret = "test_alchemy_secret";

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        headers.insert("X-Alchemy-Signature", signature.parse().unwrap());

        assert!(verify_alchemypay_signature(&headers, body, secret).is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let mut headers = HeaderMap::new();
        let body = r#"{"orderId":"123"}"#;
        let secret = "test_secret";

        headers.insert("X-Onramper-Signature", "invalid_signature".parse().unwrap());

        assert!(verify_onramper_signature(&headers, body, secret).is_err());
    }

    #[test]
    fn test_timestamp_expired() {
        let mut headers = HeaderMap::new();
        let body = r#"{"orderId":"456"}"#;
        let secret = "test_secret";
        let method = "POST";
        let path = "/webhook";

        // 使用6分钟前的时间戳（超过5分钟限制）
        let old_timestamp = (chrono::Utc::now().timestamp() - 360).to_string();

        let payload = format!("{}{}{}{}", old_timestamp, method.to_uppercase(), path, body);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        headers.insert("X-TransFi-Signature", signature.parse().unwrap());
        headers.insert("X-TransFi-Timestamp", old_timestamp.parse().unwrap());

        let result = verify_transfi_signature(&headers, body, method, path, secret);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("expired"));
    }
}
