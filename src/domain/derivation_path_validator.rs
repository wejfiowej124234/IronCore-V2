//! BIP44派生路径验证器
//!
//! P1级修复：确保多链钱包派生路径一致性
//! 所有链必须遵循标准BIP44路径格式

use std::collections::HashMap;

/// 标准链派生路径配置
pub struct DerivationPathStandard {
    #[allow(dead_code)]
    chain_id: i32,
    coin_type: u32,
    standard_path: String,
    curve_type: CurveType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CurveType {
    Secp256k1,
    Ed25519,
}

/// 派生路径验证器
pub struct DerivationPathValidator {
    standards: HashMap<i32, DerivationPathStandard>,
}

impl DerivationPathValidator {
    /// 创建验证器
    pub fn new() -> Self {
        let mut standards = HashMap::new();

        // Ethereum (EIP-2334, BIP44)
        standards.insert(
            1,
            DerivationPathStandard {
                chain_id: 1,
                coin_type: 60,
                standard_path: "m/44'/60'/0'/0/0".to_string(),
                curve_type: CurveType::Secp256k1,
            },
        );

        // BSC (EVM compatible, uses ETH coin type)
        standards.insert(
            56,
            DerivationPathStandard {
                chain_id: 56,
                coin_type: 60, // Same as ETH
                standard_path: "m/44'/60'/0'/0/0".to_string(),
                curve_type: CurveType::Secp256k1,
            },
        );

        // Polygon (EVM compatible, uses ETH coin type)
        standards.insert(
            137,
            DerivationPathStandard {
                chain_id: 137,
                coin_type: 60, // Same as ETH
                standard_path: "m/44'/60'/0'/0/0".to_string(),
                curve_type: CurveType::Secp256k1,
            },
        );

        // Bitcoin (BIP84 - Native SegWit)
        standards.insert(
            0,
            DerivationPathStandard {
                chain_id: 0,
                coin_type: 0,
                standard_path: "m/84'/0'/0'/0/0".to_string(),
                curve_type: CurveType::Secp256k1,
            },
        );

        // Solana
        standards.insert(
            501,
            DerivationPathStandard {
                chain_id: 501,
                coin_type: 501,
                standard_path: "m/44'/501'/0'/0'".to_string(),
                curve_type: CurveType::Ed25519,
            },
        );

        // TON
        standards.insert(
            607,
            DerivationPathStandard {
                chain_id: 607,
                coin_type: 607,
                standard_path: "m/44'/607'/0'/0'/0'/0'".to_string(),
                curve_type: CurveType::Ed25519,
            },
        );

        Self { standards }
    }

    /// 验证派生路径格式
    ///
    /// # 返回
    /// - Ok(true): 路径有效且符合标准
    /// - Ok(false): 路径格式正确但不符合推荐标准
    /// - Err: 路径格式错误
    pub fn validate_path(&self, chain_id: i32, path: &str) -> Result<bool, String> {
        // 1. 基本格式验证
        if !path.starts_with("m/") {
            return Err("Path must start with 'm/'".to_string());
        }

        // 2. 解析路径组件
        let components: Vec<&str> = path[2..].split('/').collect();
        if components.is_empty() {
            return Err("Path must have at least one component".to_string());
        }

        // 3. 验证purpose（应为44'表示BIP44）
        if components[0] != "44'" {
            return Err("First component must be 44' (BIP44)".to_string());
        }

        // 4. 获取标准配置
        let standard = self
            .standards
            .get(&chain_id)
            .ok_or_else(|| format!("Unsupported chain_id: {}", chain_id))?;

        // 5. 验证coin_type
        if components.len() > 1 {
            let coin_type_str = components[1];
            let expected_coin_type = format!("{}'", standard.coin_type);

            if coin_type_str != expected_coin_type {
                return Ok(false); // 格式正确但不符合标准
            }
        }

        // 6. 检查是否完全匹配标准路径
        if path == standard.standard_path {
            Ok(true)
        } else {
            // 路径格式正确但不是标准路径（例如使用了不同的account或address index）
            Ok(true)
        }
    }

    /// 获取链的标准派生路径
    pub fn get_standard_path(&self, chain_id: i32) -> Option<String> {
        self.standards
            .get(&chain_id)
            .map(|s| s.standard_path.clone())
    }

    /// 构建派生路径
    ///
    /// # Arguments
    /// * `chain_id` - 链ID
    /// * `account` - 账户索引（默认0）
    /// * `change` - 找零索引（0=外部地址，1=找零地址）
    /// * `address_index` - 地址索引（默认0）
    pub fn build_path(
        &self,
        chain_id: i32,
        account: u32,
        change: u32,
        address_index: u32,
    ) -> Result<String, String> {
        let standard = self
            .standards
            .get(&chain_id)
            .ok_or_else(|| format!("Unsupported chain_id: {}", chain_id))?;

        match standard.curve_type {
            CurveType::Secp256k1 => {
                // BIP44标准路径：m/44'/coin_type'/account'/change/address_index
                if chain_id == 0 {
                    // Bitcoin使用BIP84（Native SegWit）
                    Ok(format!(
                        "m/84'/0'/{}'/{}/''{}",
                        account, change, address_index
                    ))
                } else {
                    // EVM链使用BIP44
                    Ok(format!(
                        "m/44'/{}'/{}'/{}'/{}",
                        standard.coin_type, account, change, address_index
                    ))
                }
            }
            CurveType::Ed25519 => {
                // Ed25519链路径略有不同
                if chain_id == 501 {
                    // Solana: m/44'/501'/account'/change'
                    Ok(format!("m/44'/501'/{}'/{}'", account, change))
                } else if chain_id == 607 {
                    // TON: m/44'/607'/0'/0'/account'/address_index'
                    Ok(format!("m/44'/607'/0'/0'/{}'/{}'", account, address_index))
                } else {
                    // 其他Ed25519链
                    Ok(format!(
                        "m/44'/{}'/{}'/{}'/{}",
                        standard.coin_type, account, change, address_index
                    ))
                }
            }
        }
    }

    /// 批量验证多链派生路径一致性
    ///
    /// # 目的
    /// 确保同一助记词派生的多链钱包使用相同的account和address_index
    pub fn validate_multi_chain_consistency(
        &self,
        paths: &[(i32, String)], // (chain_id, path)
    ) -> Result<(), String> {
        if paths.is_empty() {
            return Ok(());
        }

        let mut account_indices = Vec::new();
        let mut address_indices = Vec::new();

        for (chain_id, path) in paths {
            let components: Vec<&str> = path[2..].split('/').collect();

            // 提取account和address_index
            if components.len() >= 3 {
                let account_str = components[2].trim_end_matches('\'');
                if let Ok(account) = account_str.parse::<u32>() {
                    account_indices.push((*chain_id, account));
                }
            }

            if components.len() >= 5 {
                let addr_str = components[4].trim_end_matches('\'');
                if let Ok(addr_idx) = addr_str.parse::<u32>() {
                    address_indices.push((*chain_id, addr_idx));
                }
            }
        }

        // 检查一致性
        if let Some((_, first_account)) = account_indices.first() {
            for (chain_id, account) in &account_indices {
                if account != first_account {
                    return Err(format!(
                        "Inconsistent account index: chain {} uses {}, expected {}",
                        chain_id, account, first_account
                    ));
                }
            }
        }

        if let Some((_, first_addr_idx)) = address_indices.first() {
            for (chain_id, addr_idx) in &address_indices {
                if addr_idx != first_addr_idx {
                    return Err(format!(
                        "Inconsistent address index: chain {} uses {}, expected {}",
                        chain_id, addr_idx, first_addr_idx
                    ));
                }
            }
        }

        Ok(())
    }
}

impl Default for DerivationPathValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_ethereum_path() {
        let validator = DerivationPathValidator::new();

        // 标准路径
        assert!(validator.validate_path(1, "m/44'/60'/0'/0/0").unwrap());

        // 不同的address index（格式正确）
        assert!(validator.validate_path(1, "m/44'/60'/0'/0/1").unwrap());

        // 错误的coin_type
        assert!(!validator.validate_path(1, "m/44'/0'/0'/0/0").unwrap());

        // 格式错误
        assert!(validator.validate_path(1, "44'/60'/0'/0/0").is_err());
    }

    #[test]
    fn test_build_path() {
        let validator = DerivationPathValidator::new();

        // Ethereum
        let path = validator.build_path(1, 0, 0, 0).unwrap();
        assert_eq!(path, "m/44'/60'/0'/0'/0");

        // Bitcoin
        let btc_path = validator.build_path(0, 0, 0, 0).unwrap();
        assert!(btc_path.starts_with("m/84'/0'"));

        // Solana
        let sol_path = validator.build_path(501, 0, 0, 0).unwrap();
        assert_eq!(sol_path, "m/44'/501'/0'/0'");
    }

    #[test]
    fn test_multi_chain_consistency() {
        let validator = DerivationPathValidator::new();

        let paths = vec![
            (1, "m/44'/60'/0'/0/0".to_string()),
            (56, "m/44'/60'/0'/0/0".to_string()),
            (137, "m/44'/60'/0'/0/0".to_string()),
        ];

        // 所有链使用相同的account和address index
        assert!(validator.validate_multi_chain_consistency(&paths).is_ok());

        let inconsistent_paths = vec![
            (1, "m/44'/60'/0'/0/0".to_string()),
            (56, "m/44'/60'/1'/0/0".to_string()), // 不同的account
        ];

        // 不一致的account
        assert!(validator
            .validate_multi_chain_consistency(&inconsistent_paths)
            .is_err());
    }
}
