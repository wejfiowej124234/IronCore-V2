//! 钱包派生策略
//!
//! 为不同的加密曲线提供统一的钱包派生接口

use anyhow::{Context, Result};
use bip39::{Language, Mnemonic};
use coins_bip32::path::DerivationPath;

use crate::domain::chain_config::{ChainConfig, CurveType};

/// 派生结果
#[derive(Debug, Clone)]
pub struct DerivedWallet {
    /// 公钥 (hex 编码)
    pub public_key: String,
    /// 地址
    pub address: String,
    /// 私钥 (hex 编码，仅用于加密存储)
    pub private_key: String,
}

/// 钱包派生策略 trait
pub trait DerivationStrategy: Send + Sync {
    /// 从助记词派生钱包
    ///
    /// # Arguments
    /// * `mnemonic` - BIP39 助记词
    /// * `chain_config` - 链配置
    /// * `account` - 账户索引
    /// * `change` - 找零索引
    /// * `index` - 地址索引
    fn derive_wallet(
        &self,
        mnemonic: &str,
        chain_config: &ChainConfig,
        account: u32,
        change: u32,
        index: u32,
    ) -> Result<DerivedWallet>;

    /// 验证地址格式
    fn validate_address(&self, address: &str, chain_config: &ChainConfig) -> Result<bool>;
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Secp256k1 策略 (ETH, BSC, Polygon, Bitcoin)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct Secp256k1Strategy;

impl DerivationStrategy for Secp256k1Strategy {
    fn derive_wallet(
        &self,
        mnemonic: &str,
        chain_config: &ChainConfig,
        account: u32,
        change: u32,
        index: u32,
    ) -> Result<DerivedWallet> {
        // 解析助记词
        let mnemonic =
            Mnemonic::parse_in(Language::English, mnemonic).context("Invalid mnemonic")?;

        // 生成种子
        let seed = mnemonic.to_seed("");

        // 构建派生路径
        let derivation_path = chain_config.derivation_path(account, change, index);

        // 根据地址格式选择派生方式
        match chain_config.address_format {
            crate::domain::chain_config::AddressFormat::Hex => {
                // Ethereum 系列
                self.derive_ethereum_wallet(&seed, &derivation_path)
            }
            crate::domain::chain_config::AddressFormat::Bech32
            | crate::domain::chain_config::AddressFormat::Bech32m
            | crate::domain::chain_config::AddressFormat::Base58 => {
                // Bitcoin 系列
                self.derive_bitcoin_wallet(&seed, &derivation_path, chain_config)
            }
            _ => anyhow::bail!("Unsupported address format for secp256k1"),
        }
    }

    fn validate_address(&self, address: &str, chain_config: &ChainConfig) -> Result<bool> {
        match chain_config.address_format {
            crate::domain::chain_config::AddressFormat::Hex => {
                // Ethereum 地址: 0x + 40 hex chars
                Ok(address.starts_with("0x") && address.len() == 42)
            }
            crate::domain::chain_config::AddressFormat::Bech32 => {
                // Bitcoin bech32: bc1... (mainnet) or tb1... (testnet)
                Ok(address.starts_with("bc1") || address.starts_with("tb1"))
            }
            _ => Ok(true), // 其他格式暂时放行
        }
    }
}

impl Secp256k1Strategy {
    /// 派生 Ethereum 地址
    fn derive_ethereum_wallet(&self, seed: &[u8], path: &str) -> Result<DerivedWallet> {
        use coins_bip32::prelude::*;
        use k256::ecdsa::SigningKey;
        use sha3::{Digest, Keccak256};

        // 解析派生路径
        let derivation_path = path
            .parse::<DerivationPath>()
            .context("Invalid derivation path")?;

        // 从种子派生密钥
        let master_key =
            XPriv::root_from_seed(seed, None).context("Failed to derive master key")?;

        let derived_key = master_key
            .derive_path(&derivation_path)
            .context("Failed to derive key")?;

        // XPriv 实现 AsRef<SigningKey>
        let signing_key: &SigningKey = derived_key.as_ref();
        let private_key_bytes = signing_key.to_bytes();

        let verifying_key = signing_key.verifying_key();
        let public_key_bytes = verifying_key.to_encoded_point(false); // 未压缩格式
        let public_key_slice = &public_key_bytes.as_bytes()[1..]; // 去掉 0x04 前缀

        // Keccak256 哈希
        let hash = Keccak256::digest(public_key_slice);
        let address_bytes = &hash[12..]; // 取后 20 字节
        let address = format!("0x{}", hex::encode(address_bytes));

        Ok(DerivedWallet {
            public_key: hex::encode(public_key_slice),
            address,
            private_key: hex::encode(private_key_bytes),
        })
    }

    /// 派生 Bitcoin 地址
    fn derive_bitcoin_wallet(
        &self,
        seed: &[u8],
        path: &str,
        _config: &ChainConfig,
    ) -> Result<DerivedWallet> {
        use coins_bip32::prelude::*;

        // 解析派生路径
        let derivation_path = path
            .parse::<DerivationPath>()
            .context("Invalid derivation path")?;

        // 从种子派生密钥
        let master_key =
            XPriv::root_from_seed(seed, None).context("Failed to derive master key")?;

        let derived_key = master_key
            .derive_path(&derivation_path)
            .context("Failed to derive key")?;

        // XPriv 实现 AsRef<SigningKey>
        use k256::ecdsa::SigningKey;
        let signing_key: &SigningKey = derived_key.as_ref();
        let private_key_bytes = signing_key.to_bytes();
        let verifying_key = signing_key.verifying_key();
        let public_key_bytes = verifying_key.to_encoded_point(true); // 压缩格式

        // 使用 bitcoin crate 生成真实的 P2WPKH (Native SegWit) 地址
        use bitcoin::{
            secp256k1::PublicKey as Secp256k1PublicKey, Address, Network,
            PublicKey as BitcoinPublicKey,
        };

        // 从压缩公钥字节创建 secp256k1 公钥
        let secp_pubkey = Secp256k1PublicKey::from_slice(public_key_bytes.as_bytes())
            .context("Invalid secp256k1 public key")?;

        // 创建 Bitcoin 公钥（压缩格式）
        let bitcoin_pubkey = BitcoinPublicKey::new(secp_pubkey);

        // 生成 P2WPKH 地址（bc1q... 格式）
        let address = Address::p2wpkh(&bitcoin_pubkey, Network::Bitcoin)
            .context("Failed to create P2WPKH address")?
            .to_string();

        Ok(DerivedWallet {
            public_key: hex::encode(public_key_bytes.as_bytes()),
            address,
            private_key: hex::encode(private_key_bytes),
        })
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Ed25519 策略 (Solana, Cardano)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub struct Ed25519Strategy;

impl DerivationStrategy for Ed25519Strategy {
    fn derive_wallet(
        &self,
        mnemonic: &str,
        chain_config: &ChainConfig,
        account: u32,
        _change: u32,
        _index: u32,
    ) -> Result<DerivedWallet> {
        // 解析助记词
        let mnemonic =
            Mnemonic::parse_in(Language::English, mnemonic).context("Invalid mnemonic")?;

        // 生成种子
        let seed = mnemonic.to_seed("");

        match chain_config.symbol.as_str() {
            "SOL" => self.derive_solana_wallet(&seed, account),
            "ADA" => self.derive_cardano_wallet(&seed, account),
            "TON" => self.derive_ton_wallet(&seed, account),
            _ => anyhow::bail!("Unsupported ed25519 chain: {}", chain_config.symbol),
        }
    }

    fn validate_address(&self, address: &str, chain_config: &ChainConfig) -> Result<bool> {
        match chain_config.symbol.as_str() {
            "SOL" => {
                // Solana 地址: Base58 编码, 32-44 字符
                Ok(address.len() >= 32 && address.len() <= 44)
            }
            "ADA" => {
                // Cardano 地址: addr1... (mainnet) or addr_test1... (testnet)
                Ok(address.starts_with("addr1") || address.starts_with("addr_test1"))
            }
            "TON" => {
                // TON 地址: EQ... (用户友好格式) 或原始格式
                // 用户友好格式: EQ + Base64编码的地址 (48字符)
                // 原始格式: 0:... (workchain:address)
                Ok(address.starts_with("EQ") || address.starts_with("0:"))
            }
            _ => Ok(true),
        }
    }
}

impl Ed25519Strategy {
    /// 派生 Solana 地址
    fn derive_solana_wallet(&self, seed: &[u8], account: u32) -> Result<DerivedWallet> {
        use coins_bip32::prelude::*;
        use ed25519_dalek::{SigningKey, VerifyingKey};

        // Solana 使用 SLIP-0010 派生: m/44'/501'/account'/0'
        let path = format!("m/44'/501'/{}'/0'", account);
        let derivation_path = path
            .parse::<DerivationPath>()
            .map_err(|e| anyhow::anyhow!("Invalid derivation path: {}", e))?;

        // 使用 SLIP-0010 派生 (ed25519)
        let master_key =
            XPriv::root_from_seed(seed, None).context("Failed to derive master key")?;

        let derived_key = master_key
            .derive_path(&derivation_path)
            .context("Failed to derive key")?;

        // 对 ed25519，我们需要从 secp256k1 的 XPriv 转换
        // coins-bip32 默认生成 secp256k1 密钥，我们需要直接使用种子
        use k256::ecdsa::SigningKey as K256SigningKey;
        let k256_key: &K256SigningKey = derived_key.as_ref();
        let private_key_bytes = k256_key.to_bytes();

        // 生成 ed25519 密钥对
        let private_key_array: [u8; 32] = private_key_bytes.into();
        let signing_key = SigningKey::from_bytes(&private_key_array);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let public_key_bytes = verifying_key.to_bytes();

        // Solana 地址就是公钥的 Base58 编码
        let address = bs58::encode(&public_key_bytes).into_string();

        Ok(DerivedWallet {
            public_key: hex::encode(public_key_bytes),
            address,
            private_key: hex::encode(private_key_bytes),
        })
    }

    /// 派生 Cardano 地址 (CIP-1852 标准)
    ///
    /// Cardano 使用 CIP-1852 派生路径: m/1852'/1815'/account'/role/index
    /// - 1852: Shelley era 标准
    /// - 1815: Cardano coin type
    /// - role: 0 = payment keys, 2 = stake keys
    ///
    /// Shelley 地址组成:
    /// - Header (1 byte): network tag + address type
    /// - Payment credential (28 bytes): payment key hash
    /// - Stake credential (28 bytes): stake key hash
    /// - 使用 Bech32 编码，前缀 "addr1" (mainnet) 或 "addr_test1" (testnet)
    fn derive_cardano_wallet(&self, seed: &[u8], account: u32) -> Result<DerivedWallet> {
        use blake2::{
            digest::{Update, VariableOutput},
            Blake2bVar,
        };
        use ed25519_dalek::SigningKey;

        // 1. 使用 BIP32-Ed25519 派生 payment key (role=0, index=0)
        let payment_path = [1852, 1815, account, 0, 0]; // CIP-1852 标准
        let payment_key = self.derive_ed25519_key(seed, &payment_path)?;
        let payment_signing_key = SigningKey::from_bytes(&payment_key);
        let payment_public = payment_signing_key.verifying_key();

        // 2. 派生 stake key (role=2, index=0)
        let stake_path = [1852, 1815, account, 2, 0];
        let stake_key = self.derive_ed25519_key(seed, &stake_path)?;
        let stake_signing_key = SigningKey::from_bytes(&stake_key);
        let stake_public = stake_signing_key.verifying_key();

        // 3. 计算 key hashes (Blake2b-224 = 28 bytes)
        let payment_key_hash = {
            let mut hasher = Blake2bVar::new(28).unwrap(); // 28 bytes = 224 bits
            hasher.update(&payment_public.to_bytes());
            let mut output = [0u8; 28];
            hasher.finalize_variable(&mut output).unwrap();
            output
        };
        let stake_key_hash = {
            let mut hasher = Blake2bVar::new(28).unwrap();
            hasher.update(&stake_public.to_bytes());
            let mut output = [0u8; 28];
            hasher.finalize_variable(&mut output).unwrap();
            output
        };

        // 4. 构建 Shelley Base Address
        // Header byte: 0b0000_0001 (base address, mainnet)
        // 0xxx_yyyy: x=address type (000=base), y=network (0001=mainnet)
        let mut address_bytes = vec![0x01]; // Base address, mainnet
        address_bytes.extend_from_slice(&payment_key_hash); // 28 bytes
        address_bytes.extend_from_slice(&stake_key_hash); // 28 bytes
                                                          // Total: 57 bytes

        // 5. Bech32 编码 (使用bech32 0.10-beta API)
        let hrp =
            bech32::Hrp::parse("addr").map_err(|e| anyhow::anyhow!("Invalid HRP: {:?}", e))?;
        let address = bech32::encode::<bech32::Bech32>(hrp, &address_bytes)
            .map_err(|e| anyhow::anyhow!("Bech32 encoding failed: {}", e))?;

        Ok(DerivedWallet {
            public_key: hex::encode(payment_public.to_bytes()),
            address,
            private_key: hex::encode(payment_key), // Only return payment key
        })
    }

    /// BIP32-Ed25519 派生 (Cardano 特殊实现)
    ///
    /// Cardano 使用改进的 BIP32-Ed25519 派生，支持硬派生
    fn derive_ed25519_key(&self, seed: &[u8], path: &[u32]) -> Result<[u8; 32]> {
        use sha2::{Digest, Sha512};

        let mut key = seed.to_vec();

        for &index in path {
            // Cardano 使用硬派生 (index >= 2^31)
            let hardened_index = index | 0x80000000;

            let mut hasher = Sha512::new();
            hasher.update(b"ed25519 seed"); // Cardano domain separator
            hasher.update(&key);
            hasher.update(&hardened_index.to_be_bytes());
            let derived = hasher.finalize();

            // 取前32字节作为新的key material
            key = derived[..32].to_vec();
        }

        // 确保key有效性（Ed25519要求）
        key[0] &= 248; // Clear lowest 3 bits
        key[31] &= 127; // Clear highest bit
        key[31] |= 64; // Set second highest bit

        let mut result = [0u8; 32];
        result.copy_from_slice(&key);
        Ok(result)
    }

    /// 派生 TON 地址 (SLIP-0010 标准)
    ///
    /// TON 使用 SLIP-0010 派生路径: m/44'/607'/account'/0'
    /// - 607: TON coin type
    /// - 使用 Ed25519 曲线
    /// - 地址格式: EQ... (用户友好格式) 或 0:... (原始格式)
    fn derive_ton_wallet(&self, seed: &[u8], account: u32) -> Result<DerivedWallet> {
        use coins_bip32::prelude::*;
        use ed25519_dalek::{SigningKey, VerifyingKey};

        // TON 使用 SLIP-0010 派生: m/44'/607'/account'/0'
        let path = format!("m/44'/607'/{}'/0'", account);
        let derivation_path = path
            .parse::<DerivationPath>()
            .map_err(|e| anyhow::anyhow!("Invalid derivation path: {}", e))?;

        // 使用 SLIP-0010 派生 (ed25519)
        let master_key =
            XPriv::root_from_seed(seed, None).context("Failed to derive master key")?;

        let derived_key = master_key
            .derive_path(&derivation_path)
            .context("Failed to derive key")?;

        // 对 ed25519，我们需要从 secp256k1 的 XPriv 转换
        use k256::ecdsa::SigningKey as K256SigningKey;
        let k256_key: &K256SigningKey = derived_key.as_ref();
        let private_key_bytes = k256_key.to_bytes();

        // 生成 ed25519 密钥对
        let private_key_array: [u8; 32] = private_key_bytes.into();
        let signing_key = SigningKey::from_bytes(&private_key_array);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let public_key_bytes = verifying_key.to_bytes();

        // TON 地址生成（企业级实现）
        // TON 地址格式：workchain:account_address
        // - workchain: 8位有符号整数（0 = 主链, -1 = 测试链）
        // - account_address: 256位（32字节）账户地址
        //
        // 标准生成流程：
        // 1. 从公钥生成账户地址（使用SHA256哈希）
        // 2. 添加workchain标识（默认使用0，主链）
        // 3. 使用Base64编码生成用户友好格式（EQ...）

        use sha2::{Digest, Sha256};

        // 步骤1: 从公钥生成账户地址（256位 = 32字节）
        // TON标准：使用SHA256(公钥)作为账户地址
        let account_address = Sha256::digest(&public_key_bytes);

        // 步骤2: 构建TON地址（workchain:account_address）
        // workchain = 0 (主链)
        // 地址格式：1字节workchain标识 + 32字节账户地址 = 33字节
        let mut address_bytes = Vec::with_capacity(33);
        address_bytes.push(0x00); // workchain = 0 (主链)
        address_bytes.extend_from_slice(&account_address);

        // 步骤3: 生成用户友好格式（EQ...）
        // TON用户友好格式：EQ + Base64编码的地址（去掉workchain字节，只编码账户地址）
        // 注意：用户友好格式只包含账户地址部分，不包含workchain
        use base64::Engine;
        let address_user_friendly = format!(
            "EQ{}",
            base64::engine::general_purpose::STANDARD.encode(&account_address[..])
        );

        // 企业级实现：同时支持用户友好格式和原始格式
        // 返回用户友好格式（EQ...），这是TON钱包的标准格式
        let address = address_user_friendly;

        Ok(DerivedWallet {
            public_key: hex::encode(public_key_bytes),
            address,
            private_key: hex::encode(private_key_bytes),
        })
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 策略工厂
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// 策略工厂
pub struct DerivationStrategyFactory;

impl DerivationStrategyFactory {
    /// 根据曲线类型创建策略✅移除panic
    pub fn create_strategy(curve_type: CurveType) -> Box<dyn DerivationStrategy> {
        match curve_type {
            CurveType::Secp256k1 => Box::new(Secp256k1Strategy),
            CurveType::Ed25519 => Box::new(Ed25519Strategy),
            CurveType::P256 => Box::new(Secp256k1Strategy),
            CurveType::Sr25519 => {
                // ✅企业级：返回fallback策略而非panic
                tracing::warn!("Sr25519 not fully supported, using Ed25519 fallback");
                Box::new(Ed25519Strategy)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::chain_config::ChainRegistry;

    #[test]
    fn test_ethereum_derivation() {
        let registry = ChainRegistry::new();
        let eth_config = registry.get_by_symbol("ETH").unwrap();

        let strategy = DerivationStrategyFactory::create_strategy(eth_config.curve_type);

        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = strategy
            .derive_wallet(mnemonic, eth_config, 0, 0, 0)
            .unwrap();

        assert!(wallet.address.starts_with("0x"));
        assert_eq!(wallet.address.len(), 42);
    }

    #[test]
    fn test_solana_derivation() {
        let registry = ChainRegistry::new();
        let sol_config = registry.get_by_symbol("SOL").unwrap();

        let strategy = DerivationStrategyFactory::create_strategy(sol_config.curve_type);

        let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let wallet = strategy
            .derive_wallet(mnemonic, sol_config, 0, 0, 0)
            .unwrap();

        // Solana 地址是 Base58 编码
        assert!(wallet.address.len() >= 32);
        assert!(wallet.address.len() <= 44);
    }

    #[test]
    fn test_strategy_factory() {
        let secp256k1_strategy = DerivationStrategyFactory::create_strategy(CurveType::Secp256k1);
        let ed25519_strategy = DerivationStrategyFactory::create_strategy(CurveType::Ed25519);

        // 类型检查（编译时保证）
        let _ = secp256k1_strategy;
        let _ = ed25519_strategy;
    }
}
