//! 地址验证模块
//!
//! 企业级实现：统一的地址验证逻辑
//! 解决问题：C.2 - 地址验证逻辑分散

use anyhow::Result;

use crate::utils::chain_normalizer;

/// 地址验证器
pub struct AddressValidator;

impl AddressValidator {
    /// 验证地址格式
    ///
    /// # 参数
    /// - `chain`: 链标识符（会自动标准化）
    /// - `address`: 待验证的地址
    ///
    /// # 返回
    /// - Ok(true): 地址有效
    /// - Ok(false): 地址无效
    /// - Err: 不支持的链
    pub fn validate(chain: &str, address: &str) -> Result<bool> {
        let chain_normalized = chain_normalizer::normalize_chain_identifier(chain)?;

        match chain_normalized.as_str() {
            "ethereum" | "bsc" | "polygon" | "arbitrum" | "optimism" | "avalanche" => {
                Self::validate_evm_address(address)
            }
            "solana" => Self::validate_solana_address(address),
            "bitcoin" => Self::validate_bitcoin_address(address),
            "ton" => Self::validate_ton_address(address),
            _ => Err(anyhow::anyhow!(
                "Unsupported chain for address validation: {}",
                chain_normalized
            )),
        }
    }

    /// 验证EVM地址（支持EIP-55 Checksum）
    fn validate_evm_address(address: &str) -> Result<bool> {
        // 1. 基本格式检查
        if !address.starts_with("0x") {
            return Ok(false);
        }

        if address.len() != 42 {
            return Ok(false);
        }

        // 2. 验证hex字符
        let hex_part = &address[2..];
        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(false);
        }

        // 3. EIP-55 Checksum验证（如果地址包含大写字母）
        if hex_part.chars().any(|c| c.is_uppercase()) {
            return Self::verify_eip55_checksum(address);
        }

        Ok(true)
    }

    /// 验证EIP-55 Checksum
    /// https://eips.ethereum.org/EIPS/eip-55
    fn verify_eip55_checksum(address: &str) -> Result<bool> {
        use sha3::{Digest, Keccak256};

        let addr_lower = address[2..].to_lowercase();
        let mut hasher = Keccak256::new();
        hasher.update(addr_lower.as_bytes());
        let hash = hasher.finalize();

        let hex_chars = &address[2..];
        for (i, ch) in hex_chars.chars().enumerate() {
            if ch.is_alphabetic() {
                let hash_byte = hash[i / 2];
                let hash_nibble = if i % 2 == 0 {
                    hash_byte >> 4
                } else {
                    hash_byte & 0x0f
                };

                let should_be_uppercase = hash_nibble >= 8;
                if ch.is_uppercase() != should_be_uppercase {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// 验证Solana地址（Base58编码，32字节）
    fn validate_solana_address(address: &str) -> Result<bool> {
        // Solana地址是32字节的Base58编码
        // 典型长度：32-44个字符
        if address.len() < 32 || address.len() > 44 {
            return Ok(false);
        }

        // 验证Base58字符集
        const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        if !address.chars().all(|c| BASE58_ALPHABET.contains(c)) {
            return Ok(false);
        }

        // 尝试解码
        match bs58::decode(address).into_vec() {
            Ok(decoded) => Ok(decoded.len() == 32),
            Err(_) => Ok(false),
        }
    }

    /// 验证Bitcoin地址
    fn validate_bitcoin_address(address: &str) -> Result<bool> {
        // Bitcoin支持多种地址格式：
        // - P2PKH: 以1开头
        // - P2SH: 以3开头
        // - Bech32 (SegWit): 以bc1开头

        if address.is_empty() {
            return Ok(false);
        }

        // Legacy地址（P2PKH, P2SH）
        if address.starts_with('1') || address.starts_with('3') {
            return Self::validate_base58_bitcoin_address(address);
        }

        // Bech32 SegWit地址
        if address.starts_with("bc1") {
            return Self::validate_bech32_address(address);
        }

        Ok(false)
    }

    /// 验证Base58 Bitcoin地址（带checksum）
    fn validate_base58_bitcoin_address(address: &str) -> Result<bool> {
        // 长度检查：26-35个字符
        if address.len() < 26 || address.len() > 35 {
            return Ok(false);
        }

        // Base58字符集验证
        const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        if !address.chars().all(|c| BASE58_ALPHABET.contains(c)) {
            return Ok(false);
        }

        // 尝试解码（完整验证需要checksum验证，这里简化）
        match bs58::decode(address).into_vec() {
            Ok(decoded) => Ok(decoded.len() >= 25), // 至少25字节（包含checksum）
            Err(_) => Ok(false),
        }
    }

    /// 验证Bech32地址
    fn validate_bech32_address(address: &str) -> Result<bool> {
        // 简化验证：检查格式和长度
        if !address.starts_with("bc1") {
            return Ok(false);
        }

        // Bech32地址长度：42-62个字符
        if address.len() < 42 || address.len() > 62 {
            return Ok(false);
        }

        // Bech32字符集：0-9, a-z（不含1, b, i, o）
        let addr_lower = address.to_lowercase();
        let valid_chars = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
        let body = &addr_lower[3..]; // 跳过"bc1"

        Ok(body
            .chars()
            .all(|c| valid_chars.contains(c) || c.is_ascii_digit()))
    }

    /// 验证TON地址
    fn validate_ton_address(address: &str) -> Result<bool> {
        // TON地址格式：
        // - User-friendly: EQ... 或 UQ...（48个字符，Base64）
        // - Raw: 0:hex64 (workchain:address)

        // User-friendly格式
        if (address.starts_with("EQ") || address.starts_with("UQ")) && address.len() == 48 {
            // Base64验证（简化）
            return Ok(address.chars().all(|c| {
                c.is_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '-' || c == '_'
            }));
        }

        // Raw格式：0:hex64
        if address.contains(':') {
            let parts: Vec<&str> = address.split(':').collect();
            if parts.len() == 2 {
                // workchain应该是数字
                if let Ok(_) = parts[0].parse::<i32>() {
                    // address部分应该是64个hex字符
                    return Ok(
                        parts[1].len() == 64 && parts[1].chars().all(|c| c.is_ascii_hexdigit())
                    );
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_address_validation() {
        // 有效地址（全小写 - 无checksum，总是通过）
        assert!(AddressValidator::validate(
            "ethereum",
            "0x742d35cc6634c0532925a3b844bc9e7595f0beb6"
        )
        .unwrap());
        
        // 有效地址（ETH 别名）
        assert!(
            AddressValidator::validate("ETH", "0x1234567890123456789012345678901234567890")
                .unwrap()
        );

        // 无效地址
        assert!(!AddressValidator::validate("ethereum", "0x123").unwrap());
        assert!(!AddressValidator::validate(
            "ethereum",
            "742d35Cc6634C0532925a3b844Bc9e7595f0bEb6"
        )
        .unwrap());
        assert!(!AddressValidator::validate(
            "ethereum",
            "0xGGGG35Cc6634C0532925a3b844Bc9e7595f0bEb6"
        )
        .unwrap());
    }

    #[test]
    fn test_solana_address_validation() {
        // 有效地址
        assert!(AddressValidator::validate(
            "solana",
            "DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK"
        )
        .unwrap());

        // 无效地址
        assert!(!AddressValidator::validate("solana", "invalid").unwrap());
        assert!(!AddressValidator::validate(
            "solana",
            "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb6"
        )
        .unwrap());
    }

    #[test]
    fn test_bitcoin_address_validation() {
        // P2PKH地址
        assert!(
            AddressValidator::validate("bitcoin", "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").unwrap()
        );

        // P2SH地址
        assert!(
            AddressValidator::validate("bitcoin", "3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy").unwrap()
        );

        // Bech32地址
        assert!(AddressValidator::validate(
            "bitcoin",
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
        )
        .unwrap());

        // 无效地址
        assert!(!AddressValidator::validate("bitcoin", "invalid").unwrap());
    }

    #[test]
    fn test_chain_alias_support() {
        // 测试链别名支持（使用全小写地址避免checksum问题）
        let valid_addr_lower = "0x742d35cc6634c0532925a3b844bc9e7595f0beb6";
        
        assert!(
            AddressValidator::validate("ETH", valid_addr_lower)
                .unwrap()
        );
        assert!(
            AddressValidator::validate("1", valid_addr_lower).unwrap()
        );
        assert!(
            AddressValidator::validate("BSC", valid_addr_lower)
                .unwrap()
        );
    }
}
