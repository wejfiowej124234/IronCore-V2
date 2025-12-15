//! PBKDF2 密钥派生模块
//! 用于从用户密码派生加密密钥

use anyhow::{anyhow, Result};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;

/// PBKDF2 密钥派生参数
const PBKDF2_ITERATIONS: u32 = 100_000; // 100,000次迭代
const SALT_LENGTH: usize = 16; // 16字节盐值
const KEY_LENGTH: usize = 32; // 32字节密钥（AES-256）

/// 从密码派生加密密钥
///
/// # Arguments
/// * `password` - 用户密码
/// * `salt` - 盐值（如果为None，将生成随机盐值）
///
/// # Returns
/// 返回 (密钥, 盐值) 元组
pub fn derive_key_from_password(password: &str, salt: Option<&[u8]>) -> Result<(Vec<u8>, Vec<u8>)> {
    let salt_bytes = if let Some(s) = salt {
        if s.len() != SALT_LENGTH {
            return Err(anyhow!("Salt must be {} bytes", SALT_LENGTH));
        }
        s.to_vec()
    } else {
        // 生成随机盐值
        let mut salt = vec![0u8; SALT_LENGTH];
        rand::thread_rng().fill_bytes(&mut salt);
        salt
    };

    // 派生密钥
    let mut key = vec![0u8; KEY_LENGTH];
    pbkdf2_hmac::<Sha256>(
        password.as_bytes(),
        &salt_bytes,
        PBKDF2_ITERATIONS,
        &mut key,
    );

    Ok((key, salt_bytes))
}

/// 从密码和盐值派生密钥（用于验证）
///
/// # Arguments
/// * `password` - 用户密码
/// * `salt` - 盐值
///
/// # Returns
/// 返回派生出的密钥
pub fn derive_key_with_salt(password: &str, salt: &[u8]) -> Result<Vec<u8>> {
    if salt.len() != SALT_LENGTH {
        return Err(anyhow!("Salt must be {} bytes", SALT_LENGTH));
    }

    let mut key = vec![0u8; KEY_LENGTH];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);

    Ok(key)
}

/// 获取PBKDF2迭代次数
pub fn get_iterations() -> u32 {
    PBKDF2_ITERATIONS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbkdf2_derive() {
        let password = "test_password_123";
        let (key1, salt) = derive_key_from_password(password, None).unwrap();
        assert_eq!(key1.len(), KEY_LENGTH);
        assert_eq!(salt.len(), SALT_LENGTH);

        // 使用相同密码和盐值应该得到相同密钥
        let key2 = derive_key_with_salt(password, &salt).unwrap();
        assert_eq!(key1, key2);

        // 不同密码应该得到不同密钥
        let key3 = derive_key_with_salt("different_password", &salt).unwrap();
        assert_ne!(key1, key3);
    }
}
