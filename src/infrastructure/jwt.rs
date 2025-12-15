//! JWT Token 生成和验证模块

use anyhow::{anyhow, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // Subject (user ID)
    pub tenant_id: String, // Tenant ID
    pub role: String,      // User role
    pub exp: i64,          // Expiration time
    pub iat: i64,          // Issued at
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typ: Option<String>, // Token type: "access" or "refresh"
    pub jti: String,       // JWT ID (unique identifier) - 确保每个token唯一
}

impl Claims {
    /// 获取用户 ID（UUID）
    pub fn user_id(&self) -> Result<Uuid> {
        Uuid::parse_str(&self.sub).map_err(|e| anyhow!("Invalid user ID in claims: {}", e))
    }
}

impl Claims {
    pub fn new(user_id: Uuid, tenant_id: Uuid, role: String, expires_in_secs: i64) -> Self {
        Self::new_with_type(
            user_id,
            tenant_id,
            role,
            expires_in_secs,
            Some("access".to_string()),
        )
    }

    pub fn new_with_type(
        user_id: Uuid,
        tenant_id: Uuid,
        role: String,
        expires_in_secs: i64,
        typ: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            role,
            exp: (now + Duration::seconds(expires_in_secs)).timestamp(),
            iat: now.timestamp(),
            typ,
            jti: Uuid::new_v4().to_string(), // ✅ 每个token生成唯一ID
        }
    }
}

/// 生成JWT Token
pub fn generate_token(user_id: Uuid, tenant_id: Uuid, role: String) -> Result<String> {
    // 从配置读取过期时间，如果失败则使用 1 小时（3600秒）作为默认值
    let expires_in_secs = std::env::var("JWT_TOKEN_EXPIRY_SECS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(3600); // 默认1小时

    generate_token_with_expiry(user_id, tenant_id, role, expires_in_secs)
}

/// 生成JWT Token（指定过期时间）
pub fn generate_token_with_expiry(
    user_id: Uuid,
    tenant_id: Uuid,
    role: String,
    expires_in_secs: i64,
) -> Result<String> {
    let secret = get_jwt_secret()?;
    let claims = Claims::new(user_id, tenant_id, role, expires_in_secs);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("Failed to encode token: {}", e))
}

/// 生成刷新Token（30天过期）
pub fn generate_refresh_token(user_id: Uuid, tenant_id: Uuid, role: String) -> Result<String> {
    let secret = get_jwt_secret()?;
    let claims = Claims::new_with_type(
        user_id,
        tenant_id,
        role,
        2592000, // 30天过期
        Some("refresh".to_string()),
    );

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| anyhow!("Failed to encode refresh token: {}", e))
}

/// 验证刷新Token
pub fn verify_refresh_token(token: &str) -> Result<Claims> {
    let claims = verify_token(token)?;

    // 检查Token类型
    if claims.typ.as_deref() != Some("refresh") {
        return Err(anyhow!("Invalid token type, expected refresh token"));
    }

    Ok(claims)
}

/// 验证JWT Token✅增强验证 + 调试日志
pub fn verify_token(token: &str) -> Result<Claims> {
    let secret = get_jwt_secret()?;
    tracing::debug!("JWT: starting verification, token_len={}", token.len());

    let mut validation = Validation::default();
    validation.validate_exp = true; // ✅强制验证过期时间
    validation.leeway = 10; // 允许10秒时钟偏差

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map_err(|e| {
        tracing::warn!("JWT: token verification failed: {}", e);
        anyhow!("Token verification failed: {}", e)
    })?;

    // ✅额外验证
    let claims = token_data.claims;

    tracing::debug!(
        "JWT: claims decoded, sub={}, tenant_id={}, role={}",
        claims.sub,
        claims.tenant_id,
        claims.role
    );

    // 验证user_id和tenant_id格式
    Uuid::parse_str(&claims.sub)
        .map_err(|e| anyhow!("Invalid user_id format in token: {}", e))?;
    Uuid::parse_str(&claims.tenant_id)
        .map_err(|e| anyhow!("Invalid tenant_id format in token: {}", e))?;

    Ok(claims)
}

/// 从环境变量获取JWT密钥 + 调试日志
fn get_jwt_secret() -> Result<String> {
    match std::env::var("JWT_SECRET") {
        Ok(secret) => {
            tracing::debug!("JWT: using secret from env, len={}", secret.as_bytes().len());
            Ok(secret)
        }
        Err(_) => Err(anyhow!("JWT_SECRET environment variable not set")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_roundtrip() {
        unsafe {
            std::env::set_var("JWT_SECRET", "test_secret_key_for_jwt_signing");
        }

        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let role = "admin".to_string();

        let token = generate_token(user_id, tenant_id, role.clone()).unwrap();
        let claims = verify_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.role, role);
    }
}
