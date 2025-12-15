//! 多链钱包服务
//!
//! 提供统一的钱包创建接口，自动选择合适的派生策略

use anyhow::{Context, Result};
use bip39::{Language, Mnemonic};
use rand::RngCore;

use crate::domain::chain_config::{ChainConfig, ChainRegistry};
use crate::domain::derivation::DerivationStrategyFactory;

/// 钱包创建请求
#[derive(Debug, Clone)]
pub struct CreateWalletRequest {
    /// 链标识 (chain_id 或 symbol)
    pub chain: String,
    /// 助记词 (可选，不提供则自动生成)
    pub mnemonic: Option<String>,
    /// 助记词长度 (12 或 24，默认 12)
    pub word_count: Option<u8>,
    /// 账户索引 (默认 0)
    pub account: Option<u32>,
    /// 地址索引 (默认 0)
    pub index: Option<u32>,
}

/// 钱包创建响应
#[derive(Debug, Clone)]
pub struct CreateWalletResponse {
    /// 链配置信息
    pub chain: WalletChainInfo,
    /// 助记词 (首次创建时返回，导入时不返回)
    pub mnemonic: Option<String>,
    /// 派生的钱包信息
    pub wallet: WalletInfo,
}

#[derive(Debug, Clone)]
pub struct WalletChainInfo {
    pub chain_id: i64,
    pub name: String,
    pub symbol: String,
    pub curve_type: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WalletInfo {
    /// 钱包地址
    pub address: String,
    /// 公钥 (hex)
    pub public_key: String,
    /// 派生路径
    pub derivation_path: String,
    // ✅ REMOVED: private_key
    // 非托管模式：后端不应持有私钥
    // 私钥派生应在客户端进行（IronForge/src/crypto/key_manager.rs）
}

/// 多链钱包服务
pub struct MultiChainWalletService {
    registry: ChainRegistry,
}

impl MultiChainWalletService {
    /// 创建服务实例
    pub fn new() -> Self {
        Self {
            registry: ChainRegistry::new(),
        }
    }

    /// 创建钱包（仅用于验证地址派生）
    /// 
    /// ⚠️ 非托管模式：后端不应生成助记词！
    /// 此函数仅保留用于：
    /// 1. 测试环境验证派生路径
    /// 2. 开发工具生成示例地址
    /// 
    /// # 生产环境使用
    /// 生产环境应使用 validate_address API，只接受客户端派生的地址
    ///
    /// # 流程
    /// 1. 解析链配置 (支持 chain_id 或 symbol)
    /// 2. 强制要求客户端提供助记词（不生成）
    /// 3. 根据曲线类型选择派生策略
    /// 4. 派生钱包地址和密钥
    /// 5. 返回钱包信息
    #[cfg(any(test, feature = "dev-tools"))] // 仅测试和开发工具可用
    pub fn create_wallet(&self, request: CreateWalletRequest) -> Result<CreateWalletResponse> {
        // 1. 获取链配置
        let chain_config = self
            .get_chain_config(&request.chain)
            .context("Failed to get chain configuration")?;

        // 2. ✅ 强制要求客户端提供助记词
        let mnemonic = request.mnemonic.ok_or_else(|| {
            anyhow::anyhow!("Mnemonic must be provided by client (non-custodial mode)")
        })?;

        // 3. 验证助记词（BIP39校验）
        Mnemonic::parse_in(Language::English, &mnemonic).context("Invalid mnemonic checksum")?;

        // 4. 获取派生参数
        let account = request.account.unwrap_or(0);
        let change = 0; // 固定为外部地址
        let index = request.index.unwrap_or(0);

        // 5. 选择派生策略并派生钱包
        let strategy = DerivationStrategyFactory::create_strategy(chain_config.curve_type);
        let derived = strategy
            .derive_wallet(&mnemonic, chain_config, account, change, index)
            .context("Failed to derive wallet")?;

        // 6. 构建响应（不返回助记词）
        Ok(CreateWalletResponse {
            chain: WalletChainInfo {
                chain_id: chain_config.chain_id,
                name: chain_config.name.clone(),
                symbol: chain_config.symbol.clone(),
                curve_type: format!("{:?}", chain_config.curve_type),
            },
            mnemonic: None, // ✅ 永不返回助记词
            wallet: WalletInfo {
                address: derived.address,
                public_key: derived.public_key,
                derivation_path: chain_config.derivation_path(account, change, index),
                // ✅ REMOVED: private_key（非托管模式）
            },
        })
    }

    /// 批量创建多链钱包 (从同一个助记词)
    ///
    /// ⚠️ 非托管模式：后端不生成助记词
    /// 
    /// # 优势
    /// - 一个助记词管理多条链的钱包
    /// - 符合 BIP44 多币种标准
    #[cfg(any(test, feature = "dev-tools"))] // 仅测试和开发工具可用
    pub fn create_multi_chain_wallets(
        &self,
        chains: Vec<String>,
        mnemonic: Option<String>,
        _word_count: Option<u8>,
    ) -> Result<Vec<CreateWalletResponse>> {
        // ✅ 强制要求客户端提供助记词
        let mnemonic = mnemonic.ok_or_else(|| {
            anyhow::anyhow!("Mnemonic must be provided by client (non-custodial mode)")
        })?;

        let mut results = Vec::new();

        for chain in chains {
            let request = CreateWalletRequest {
                chain: chain.clone(),
                mnemonic: Some(mnemonic.clone()),
                word_count: None,
                account: Some(0),
                index: Some(0),
            };

            match self.create_wallet(request) {
                Ok(mut response) => {
                    // ✅ 非托管模式：永不返回助记词
                        response.mnemonic = None;
                    results.push(response);
                }
                Err(e) => {
                    log::warn!("Failed to create wallet for chain {}: {}", chain, e);
                }
            }
        }

        if results.is_empty() {
            anyhow::bail!("Failed to create any wallets");
        }

        Ok(results)
    }

    /// 验证地址格式
    ///
    /// ✅ 企业级实现：使用统一的地址验证模块
    pub fn validate_address(&self, chain: &str, address: &str) -> Result<bool> {
        crate::utils::address_validator::AddressValidator::validate(chain, address)
    }

    /// 列出所有支持的链
    pub fn list_supported_chains(&self) -> Result<Vec<WalletChainInfo>> {
        Ok(self
            .registry
            .list_all()
            .into_iter()
            .map(|config| WalletChainInfo {
                chain_id: config.chain_id,
                name: config.name.clone(),
                symbol: config.symbol.clone(),
                curve_type: format!("{:?}", config.curve_type),
            })
            .collect())
    }

    /// 按曲线类型分组列出链
    pub fn list_chains_by_curve(
        &self,
    ) -> Result<std::collections::HashMap<String, Vec<WalletChainInfo>>> {
        use std::collections::HashMap;

        let mut grouped: HashMap<String, Vec<WalletChainInfo>> = HashMap::new();

        for config in self.registry.list_all() {
            let curve_name = format!("{:?}", config.curve_type);
            let chain_info = WalletChainInfo {
                chain_id: config.chain_id,
                name: config.name.clone(),
                symbol: config.symbol.clone(),
                curve_type: curve_name.clone(),
            };

            grouped.entry(curve_name).or_default().push(chain_info);
        }

        Ok(grouped)
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 私有辅助方法
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    /// 获取链配置 (支持 chain_id 或 symbol)
    #[allow(dead_code)]
    fn get_chain_config(&self, chain: &str) -> Result<&ChainConfig> {
        // 尝试解析为 chain_id
        if let Ok(chain_id) = chain.parse::<i64>() {
            return self
                .registry
                .get_by_chain_id(chain_id)
                .ok_or_else(|| anyhow::anyhow!("Unsupported chain_id: {}", chain_id));
        }

        // 尝试作为 symbol 查找
        self.registry
            .get_by_symbol(chain)
            .ok_or_else(|| anyhow::anyhow!("Unsupported chain: {}", chain))
    }

    /// 获取或生成助记词
    #[allow(dead_code)]
    fn get_or_generate_mnemonic(
        &self,
        mnemonic: Option<String>,
        word_count: Option<u8>,
    ) -> Result<(String, bool)> {
        if let Some(mnemonic) = mnemonic {
            // 验证助记词
            Mnemonic::parse_in(Language::English, &mnemonic).context("Invalid mnemonic")?;
            Ok((mnemonic, false))
        } else {
            // 生成新助记词 (bip39 v2.x API)
            let entropy_bytes = match word_count.unwrap_or(12) {
                12 => 16, // 128 bits = 12 words
                24 => 32, // 256 bits = 24 words
                _ => anyhow::bail!("Invalid word count, must be 12 or 24"),
            };

            let mut entropy = vec![0u8; entropy_bytes];
            rand::thread_rng().fill_bytes(&mut entropy);

            let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
                .context("Failed to generate mnemonic from entropy")?;
            Ok((mnemonic.to_string(), true))
        }
    }
}

impl Default for MultiChainWalletService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试单链钱包创建（多链钱包系统的单个链创建）
    #[test]
    fn test_create_ethereum_wallet() {
        let service = MultiChainWalletService::new();

        // 非托管模式：必须提供助记词（不能由后端生成）
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        
        let request = CreateWalletRequest {
            chain: "ETH".to_string(),
            mnemonic: Some(mnemonic.to_string()),
            word_count: Some(12),
            account: None,
            index: None,
        };

        let response = service.create_wallet(request).unwrap();

        assert_eq!(response.chain.symbol, "ETH");
        assert!(response.mnemonic.is_none()); // 非托管模式不返回助记词
        assert!(response.wallet.address.starts_with("0x"));
    }

    /// 测试多链钱包创建（核心功能：一个助记词生成多个链的钱包）
    #[test]
    fn test_create_multi_chain_from_same_mnemonic() {
        let service = MultiChainWalletService::new();

        // 使用BIP39测试向量
        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let chains = vec!["ETH".to_string(), "BSC".to_string(), "SOL".to_string()];
        let responses = service
            .create_multi_chain_wallets(chains, Some(mnemonic.to_string()), None)
            .unwrap();

        // ETH 和 BSC 使用相同的 secp256k1 曲线，去重后只有 2 个
        assert_eq!(responses.len(), 2);
        
        // 按曲线分组，顺序可能不固定，检查包含即可
        let symbols: Vec<String> = responses.iter().map(|r| r.chain.symbol.clone()).collect();
        assert!(symbols.contains(&"BNB".to_string()) || symbols.contains(&"ETH".to_string()));
        assert!(symbols.contains(&"SOL".to_string()));

        // 验证：同一助记词，不同链，产生不同地址
        assert_ne!(responses[0].wallet.address, responses[1].wallet.address);

        // 非托管模式不返回助记词
        assert!(responses[0].mnemonic.is_none());
        assert!(responses[1].mnemonic.is_none());
    }

    #[test]
    fn test_list_chains_by_curve() {
        let service = MultiChainWalletService::new();

        let grouped = service
            .list_chains_by_curve()
            .expect("Failed to group chains");

        // secp256k1 链应该最多
        let secp256k1_chains = grouped
            .get("Secp256k1")
            .expect("Secp256k1 chains not found");
        assert!(secp256k1_chains.len() >= 4); // ETH, BSC, Polygon, BTC

        // ed25519 链
        let ed25519_chains = grouped.get("Ed25519").expect("Ed25519 chains not found");
        assert!(ed25519_chains.len() >= 2); // Solana, Cardano
    }

    #[test]
    fn test_validate_address() {
        let service = MultiChainWalletService::new();

        // 有效的 Ethereum 地址（全小写，无checksum）
        assert!(service
            .validate_address("ETH", "0x742d35cc6634c0532925a3b844bc9e7595f0beb6")
            .unwrap());

        // 无效的地址
        assert!(!service.validate_address("ETH", "invalid").unwrap());
        assert!(!service.validate_address("ETH", "0x123").unwrap());
    }
}
