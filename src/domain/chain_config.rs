//! 多链配置模块
//!
//! 定义所有支持的区块链及其加密曲线配置

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 加密曲线类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CurveType {
    /// secp256k1 曲线 (Bitcoin, Ethereum, BSC, Polygon, Avalanche, Arbitrum, Optimism)
    Secp256k1,
    /// ed25519 曲线 (Solana, Cardano, Stellar)
    Ed25519,
    /// sr25519 曲线 (Polkadot, Kusama)
    Sr25519,
    /// NIST P-256 (某些企业链)
    P256,
}

/// 地址编码格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressFormat {
    /// 十六进制 0x... (Ethereum 系列)
    Hex,
    /// Base58 编码 (Bitcoin legacy)
    Base58,
    /// Bech32 编码 (Bitcoin native segwit)
    Bech32,
    /// Bech32m 编码 (Bitcoin taproot)
    Bech32m,
    /// Base58 编码 (Solana)
    SolanaBase58,
    /// SS58 编码 (Polkadot/Substrate)
    SS58,
    /// Bech32 编码 (Cosmos)
    CosmosBech32,
}

/// HD 派生标准
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DerivationStandard {
    /// BIP44: m/44'/coin_type'/account'/change/index
    BIP44,
    /// BIP49: m/49'/coin_type'/account'/change/index (P2SH-wrapped segwit)
    BIP49,
    /// BIP84: m/84'/coin_type'/account'/change/index (native segwit)
    BIP84,
    /// BIP86: m/86'/coin_type'/account'/change/index (taproot)
    BIP86,
    /// SLIP-0010: 适用于 ed25519
    SLIP0010,
    /// CIP-1852: Cardano 专用
    CIP1852,
}

/// 链配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// 链 ID (EIP-155 或 SLIP-44)
    pub chain_id: i64,
    /// 链名称
    pub name: String,
    /// 链符号 (ETH, BTC, SOL, DOT, etc.)
    pub symbol: String,
    /// 加密曲线类型
    pub curve_type: CurveType,
    /// 地址格式
    pub address_format: AddressFormat,
    /// HD 派生标准
    pub derivation_standard: DerivationStandard,
    /// BIP44 coin type (用于派生路径)
    pub coin_type: u32,
    /// 默认派生路径模板
    pub derivation_path_template: String,
    /// 是否为测试网
    pub is_testnet: bool,
    /// RPC 端点 (可选)
    pub rpc_url: Option<String>,
}

impl ChainConfig {
    /// 生成派生路径
    ///
    /// # Arguments
    /// * `account` - 账户索引 (通常为 0)
    /// * `change` - 找零索引 (外部地址为 0，内部地址为 1)
    /// * `index` - 地址索引
    pub fn derivation_path(&self, account: u32, change: u32, index: u32) -> String {
        match self.derivation_standard {
            DerivationStandard::BIP44 => {
                format!(
                    "m/44'/{}'/{}'/{}/{}",
                    self.coin_type, account, change, index
                )
            }
            DerivationStandard::BIP49 => {
                format!(
                    "m/49'/{}'/{}'/{}/{}",
                    self.coin_type, account, change, index
                )
            }
            DerivationStandard::BIP84 => {
                format!(
                    "m/84'/{}'/{}'/{}/{}",
                    self.coin_type, account, change, index
                )
            }
            DerivationStandard::BIP86 => {
                format!(
                    "m/86'/{}'/{}'/{}/{}",
                    self.coin_type, account, change, index
                )
            }
            DerivationStandard::SLIP0010 => {
                // Solana/ed25519: m/44'/501'/account'/change'
                format!("m/44'/{}'/{}'/{}'/", self.coin_type, account, change)
            }
            DerivationStandard::CIP1852 => {
                // Cardano: m/1852'/1815'/account'/role/index
                format!(
                    "m/1852'/{}'/{}'/{}/{}",
                    self.coin_type, account, change, index
                )
            }
        }
    }
}

/// 链配置注册表
pub struct ChainRegistry {
    configs: HashMap<i64, ChainConfig>,
    symbol_map: HashMap<String, i64>,
}

impl ChainRegistry {
    /// 创建预配置的注册表
    pub fn new() -> Self {
        let mut registry = Self {
            configs: HashMap::new(),
            symbol_map: HashMap::new(),
        };

        registry.register_default_chains();
        registry
    }

    /// 注册默认支持的链
    fn register_default_chains(&mut self) {
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Secp256k1 系列 (可共享实现)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // Ethereum Mainnet
        self.register(ChainConfig {
            chain_id: 1,
            name: "Ethereum".to_string(),
            symbol: "ETH".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60,
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: Some("https://eth-mainnet.g.alchemy.com/v2/".to_string()),
        });

        // Ethereum Sepolia Testnet
        self.register(ChainConfig {
            chain_id: 11155111,
            name: "Ethereum Sepolia".to_string(),
            symbol: "ETH".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60,
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: true,
            rpc_url: Some("https://sepolia.infura.io/v3/".to_string()),
        });

        // BSC (Binance Smart Chain)
        self.register(ChainConfig {
            chain_id: 56,
            name: "BNB Smart Chain".to_string(),
            symbol: "BNB".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60, // BSC 使用与 ETH 相同的派生路径
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: Some("https://bsc-dataseed.binance.org/".to_string()),
        });

        // Polygon
        self.register(ChainConfig {
            chain_id: 137,
            name: "Polygon".to_string(),
            symbol: "MATIC".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60, // Polygon 也使用 ETH 派生路径
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: Some("https://polygon-rpc.com/".to_string()),
        });

        // Bitcoin (BIP84 - native segwit)
        self.register(ChainConfig {
            chain_id: 0,
            name: "Bitcoin".to_string(),
            symbol: "BTC".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Bech32,
            derivation_standard: DerivationStandard::BIP84,
            coin_type: 0,
            derivation_path_template: "m/84'/0'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: None,
        });

        // ✅Arbitrum (L2)
        self.register(ChainConfig {
            chain_id: 42161,
            name: "Arbitrum One".to_string(),
            symbol: "ETH".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60,
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: Some("https://arb1.arbitrum.io/rpc".to_string()),
        });

        // ✅Optimism (L2)
        self.register(ChainConfig {
            chain_id: 10,
            name: "Optimism".to_string(),
            symbol: "ETH".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60,
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: Some("https://mainnet.optimism.io".to_string()),
        });

        // ✅Avalanche C-Chain
        self.register(ChainConfig {
            chain_id: 43114,
            name: "Avalanche C-Chain".to_string(),
            symbol: "AVAX".to_string(),
            curve_type: CurveType::Secp256k1,
            address_format: AddressFormat::Hex,
            derivation_standard: DerivationStandard::BIP44,
            coin_type: 60,
            derivation_path_template: "m/44'/60'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: Some("https://api.avax.network/ext/bc/C/rpc".to_string()),
        });

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Ed25519 系列 (独立实现)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // Solana
        self.register(ChainConfig {
            chain_id: 501,
            name: "Solana".to_string(),
            symbol: "SOL".to_string(),
            curve_type: CurveType::Ed25519,
            address_format: AddressFormat::SolanaBase58,
            derivation_standard: DerivationStandard::SLIP0010,
            coin_type: 501,
            derivation_path_template: "m/44'/501'/0'/0'".to_string(),
            is_testnet: false,
            rpc_url: Some("https://api.mainnet-beta.solana.com".to_string()),
        });

        // Cardano
        self.register(ChainConfig {
            chain_id: 1815,
            name: "Cardano".to_string(),
            symbol: "ADA".to_string(),
            curve_type: CurveType::Ed25519,
            address_format: AddressFormat::Bech32,
            derivation_standard: DerivationStandard::CIP1852,
            coin_type: 1815,
            derivation_path_template: "m/1852'/1815'/0'/0/{index}".to_string(),
            is_testnet: false,
            rpc_url: None,
        });

        // TON (The Open Network) - 使用 Ed25519 曲线
        // TON 地址格式: 用户友好的格式 (EQ...) 或 原始格式
        // 派生路径: m/44'/607'/account'/0' (SLIP-0010 标准，coin_type = 607)
        self.register(ChainConfig {
            chain_id: 607, // TON coin type
            name: "TON".to_string(),
            symbol: "TON".to_string(),
            curve_type: CurveType::Ed25519,
            address_format: AddressFormat::Base58, // TON 使用 Base58 编码
            derivation_standard: DerivationStandard::SLIP0010,
            coin_type: 607,
            derivation_path_template: "m/44'/607'/0'/0'".to_string(),
            is_testnet: false,
            rpc_url: Some("https://toncenter.com/api/v2".to_string()),
        });

        // Polkadot/Kusama (Sr25519) removed - frontend does not support DOT/KSM
    }

    /// 注册链配置
    pub fn register(&mut self, config: ChainConfig) {
        let chain_id = config.chain_id;
        let symbol = config.symbol.to_lowercase();

        self.symbol_map.insert(symbol, chain_id);
        self.configs.insert(chain_id, config);
    }

    /// 通过 chain_id 获取配置
    pub fn get_by_chain_id(&self, chain_id: i64) -> Option<&ChainConfig> {
        self.configs.get(&chain_id)
    }

    /// 通过符号获取配置
    pub fn get_by_symbol(&self, symbol: &str) -> Option<&ChainConfig> {
        let chain_id = self.symbol_map.get(&symbol.to_lowercase())?;
        self.configs.get(chain_id)
    }

    /// 按曲线类型分组获取所有链
    pub fn get_by_curve_type(&self, curve_type: CurveType) -> Vec<&ChainConfig> {
        self.configs
            .values()
            .filter(|c| c.curve_type == curve_type)
            .collect()
    }

    /// 列出所有支持的链
    pub fn list_all(&self) -> Vec<&ChainConfig> {
        self.configs.values().collect()
    }

    /// 验证链配置完整性
    /// 企业级实现：确保所有链配置正确且完整
    pub fn validate_configs(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        for (chain_id, config) in &self.configs {
            // 验证chain_id
            if *chain_id < 0 && *chain_id != 0 {
                errors.push(format!(
                    "Chain {} has invalid chain_id: {}",
                    config.name, chain_id
                ));
            }

            // 验证名称和符号
            if config.name.is_empty() {
                errors.push(format!("Chain {} has empty name", chain_id));
            }
            if config.symbol.is_empty() {
                errors.push(format!("Chain {} has empty symbol", chain_id));
            }

            // 验证派生路径模板
            if config.derivation_path_template.is_empty() {
                errors.push(format!(
                    "Chain {} has empty derivation_path_template",
                    config.name
                ));
            }

            // 验证coin_type
            if config.coin_type == 0 && config.symbol != "BTC" {
                errors.push(format!(
                    "Chain {} has invalid coin_type: 0 (only BTC should use 0)",
                    config.name
                ));
            }

            // 验证曲线类型和地址格式匹配
            match (config.curve_type, config.address_format) {
                (CurveType::Secp256k1, AddressFormat::Hex) => {
                    // ETH系列：正确
                }
                (CurveType::Secp256k1, AddressFormat::Bech32 | AddressFormat::Bech32m) => {
                    // BTC系列：正确
                }
                (CurveType::Ed25519, AddressFormat::SolanaBase58) => {
                    // Solana：正确
                }
                (CurveType::Ed25519, AddressFormat::Bech32) => {
                    // Cardano/TON：正确
                }
                (CurveType::Ed25519, AddressFormat::Base58) => {
                    // TON：正确
                }
                _ => {
                    errors.push(format!(
                        "Chain {} has incompatible curve_type and address_format: {:?} / {:?}",
                        config.name, config.curve_type, config.address_format
                    ));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_registry() {
        let registry = ChainRegistry::new();

        // 测试通过 chain_id 查找
        let eth = registry.get_by_chain_id(1).unwrap();
        assert_eq!(eth.name, "Ethereum");
        assert_eq!(eth.curve_type, CurveType::Secp256k1);

        // 测试通过符号查找
        let sol = registry.get_by_symbol("SOL").unwrap();
        assert_eq!(sol.chain_id, 501);
        assert_eq!(sol.curve_type, CurveType::Ed25519);

        // 测试派生路径生成
        let btc = registry.get_by_symbol("BTC").unwrap();
        let path = btc.derivation_path(0, 0, 0);
        assert_eq!(path, "m/84'/0'/0'/0/0");
    }

    #[test]
    fn test_curve_grouping() {
        let registry = ChainRegistry::new();

        // 所有 secp256k1 链应该能共享实现
        let secp256k1_chains = registry.get_by_curve_type(CurveType::Secp256k1);
        assert!(secp256k1_chains.len() >= 4); // ETH, BSC, Polygon, BTC

        // 所有 ed25519 链
        let ed25519_chains = registry.get_by_curve_type(CurveType::Ed25519);
        assert!(ed25519_chains.len() >= 2); // Solana, Cardano
    }
}
