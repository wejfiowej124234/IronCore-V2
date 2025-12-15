//! AES-256-GCM 加密/解密模块
//! 用于敏感数据加密存储

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Result};
use hex;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// 加密数据
///
/// # Arguments
/// * `data` - 要加密的原始数据
/// * `key` - 32字节加密密钥
///
/// # Returns
/// 返回加密后的数据（nonce + ciphertext）
pub fn encrypt_data(data: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(anyhow!("Key must be 32 bytes for AES-256"));
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("Invalid key: {}", e))?;

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;

    // 将 nonce (12字节) 和 ciphertext 组合
    let mut result = nonce.to_vec();
    result.extend_from_slice(&ciphertext);

    Ok(result)
}

/// 解密数据
///
/// # Arguments
/// * `encrypted` - 加密的数据（nonce + ciphertext）
/// * `key` - 32字节加密密钥
///
/// # Returns
/// 返回解密后的原始数据
pub fn decrypt_data(encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>> {
    if key.len() != 32 {
        return Err(anyhow!("Key must be 32 bytes for AES-256"));
    }

    if encrypted.len() < 12 {
        return Err(anyhow!("Encrypted data too short"));
    }

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| anyhow!("Invalid key: {}", e))?;

    // 提取 nonce（前12字节）
    let nonce = Nonce::from_slice(&encrypted[..12]);
    let ciphertext = &encrypted[12..];

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;

    Ok(plaintext)
}

/// 加密密钥（使用Zeroize保护）
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct EncryptionKey {
    key: [u8; 32],
}

impl EncryptionKey {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.key
    }
}

/// 从环境变量获取加密密钥
///
/// # Returns
/// 返回32字节密钥
pub fn get_encryption_key() -> Result<Vec<u8>> {
    let key_str = std::env::var("WALLET_ENC_KEY")
        .map_err(|_| anyhow!("WALLET_ENC_KEY environment variable not set"))?;

    // ✅支持多格式密钥
    if key_str.is_empty() {
        return Err(anyhow!("WALLET_ENC_KEY empty"));
    }

    if key_str.len() == 64 {
        hex::decode(&key_str).map_err(|e| anyhow!("Invalid hex key: {}", e))
    } else if key_str.len() == 32 {
        Ok(key_str.as_bytes().to_vec())
    } else if key_str.len() >= 16 {
        let mut hasher = Sha256::new();
        hasher.update(key_str.as_bytes());
        Ok(hasher.finalize().to_vec())
    } else {
        Err(anyhow!("WALLET_ENC_KEY too short (min 16)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let key = b"01234567890123456789012345678901"; // 32 bytes
        let data = b"Hello, World!";

        let encrypted = encrypt_data(data, key).unwrap();
        assert_ne!(encrypted, data);

        let decrypted = decrypt_data(&encrypted, key).unwrap();
        assert_eq!(decrypted, data);
    }
}
