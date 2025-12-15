//! 密码哈希和验证模块
//! 使用 bcrypt 进行密码哈希

use anyhow::{anyhow, Result};
use bcrypt::{hash, verify, DEFAULT_COST};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// 密码包装器（使用Zeroize保护）
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Password(String);

impl Password {
    pub fn new(password: String) -> Self {
        Self(password)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 哈希密码
///
/// # Arguments
/// * `password` - 明文密码
///
/// # Returns
/// 返回bcrypt哈希后的密码
pub fn hash_password(password: &str) -> Result<String> {
    hash(password, DEFAULT_COST).map_err(|e| anyhow!("Failed to hash password: {}", e))
}

/// 验证密码
///
/// # Arguments
/// * `password` - 明文密码
/// * `hash` - bcrypt哈希值
///
/// # Returns
/// 如果密码匹配返回true，否则返回false
pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    verify(password, hash).map_err(|e| anyhow!("Failed to verify password: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hash_verify() {
        let password = "my_secure_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }
}
