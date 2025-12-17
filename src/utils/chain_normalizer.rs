//! 链标识符标准化模块
//!
//! 企业级实现：统一所有链标识符的处理逻辑
//! 解决问题：C.1 - 链标识符不统一

use std::collections::HashMap;

use anyhow;
use once_cell::sync::Lazy;

/// 链标识符配置
#[derive(Debug, Clone)]
pub struct ChainIdentifier {
    /// 规范名称（小写，用于内部处理）
    pub canonical_name: &'static str,
    /// 链ID
    pub chain_id: i64,
    /// 符号（大写）
    pub symbol: &'static str,
    /// 全称
    pub full_name: &'static str,
    /// 别名列表
    pub aliases: &'static [&'static str],
}

/// 链标识符注册表（静态初始化）
static CHAIN_REGISTRY: Lazy<HashMap<String, ChainIdentifier>> = Lazy::new(|| {
    let chains = vec![
        ChainIdentifier {
            canonical_name: "ethereum",
            chain_id: 1,
            symbol: "ETH",
            full_name: "Ethereum Mainnet",
            aliases: &["eth", "ETH", "Ethereum", "ethereum", "mainnet"],
        },
        ChainIdentifier {
            canonical_name: "bsc",
            chain_id: 56,
            symbol: "BNB",
            full_name: "BNB Smart Chain",
            aliases: &["bsc", "BSC", "binance", "Binance", "BNB", "bnb"],
        },
        ChainIdentifier {
            canonical_name: "polygon",
            chain_id: 137,
            symbol: "MATIC",
            full_name: "Polygon",
            aliases: &["polygon", "Polygon", "POLYGON", "matic", "MATIC"],
        },
        ChainIdentifier {
            canonical_name: "arbitrum",
            chain_id: 42161,
            symbol: "ETH",
            full_name: "Arbitrum One",
            aliases: &["arbitrum", "Arbitrum", "ARBITRUM", "arb", "ARB"],
        },
        ChainIdentifier {
            canonical_name: "optimism",
            chain_id: 10,
            symbol: "ETH",
            full_name: "Optimism",
            aliases: &["optimism", "Optimism", "OPTIMISM", "op", "OP"],
        },
        ChainIdentifier {
            canonical_name: "avalanche",
            chain_id: 43114,
            symbol: "AVAX",
            full_name: "Avalanche C-Chain",
            aliases: &["avalanche", "Avalanche", "AVALANCHE", "avax", "AVAX"],
        },
        ChainIdentifier {
            canonical_name: "solana",
            chain_id: 501,
            symbol: "SOL",
            full_name: "Solana",
            aliases: &["solana", "Solana", "SOLANA", "sol", "SOL"],
        },
        ChainIdentifier {
            canonical_name: "bitcoin",
            chain_id: 0,
            symbol: "BTC",
            full_name: "Bitcoin",
            aliases: &["bitcoin", "Bitcoin", "BITCOIN", "btc", "BTC"],
        },
        ChainIdentifier {
            canonical_name: "ton",
            chain_id: 607,
            symbol: "TON",
            full_name: "The Open Network",
            aliases: &["ton", "TON", "Ton"],
        },
    ];

    let mut registry = HashMap::new();
    for chain in chains {
        // 注册规范名称
        registry.insert(chain.canonical_name.to_string(), chain.clone());

        // 注册所有别名
        for alias in chain.aliases {
            registry.insert(alias.to_string(), chain.clone());
        }

        // 注册chain_id作为字符串
        registry.insert(chain.chain_id.to_string(), chain.clone());
    }

    registry
});

/// 标准化链标识符
///
/// # 功能
/// - 接受任何格式的链标识符（chain_id、symbol、alias）
/// - 返回规范化的小写链名称
///
/// # 示例
/// ```rust
/// # use ironcore::utils::chain_normalizer::normalize_chain_identifier;
/// assert_eq!(normalize_chain_identifier("ETH").unwrap(), "ethereum");
/// assert_eq!(normalize_chain_identifier("1").unwrap(), "ethereum");
/// assert_eq!(normalize_chain_identifier("Ethereum").unwrap(), "ethereum");
/// ```
pub fn normalize_chain_identifier(input: &str) -> anyhow::Result<String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        anyhow::bail!("Chain identifier cannot be empty");
    }

    CHAIN_REGISTRY
        .get(trimmed)
        .map(|chain| chain.canonical_name.to_string())
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain identifier: {}", trimmed))
}

/// 获取链配置
pub fn get_chain_config(input: &str) -> anyhow::Result<&'static ChainIdentifier> {
    let trimmed = input.trim();
    CHAIN_REGISTRY
        .get(trimmed)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain identifier: {}", trimmed))
}

/// 判断是否为EVM链
pub fn is_evm_chain(chain: &str) -> bool {
    match normalize_chain_identifier(chain) {
        Ok(canonical) => matches!(
            canonical.as_str(),
            "ethereum" | "bsc" | "polygon" | "arbitrum" | "optimism" | "avalanche"
        ),
        Err(_) => false,
    }
}

/// 获取链ID
pub fn get_chain_id(chain: &str) -> anyhow::Result<i64> {
    get_chain_config(chain).map(|config| config.chain_id)
}

/// 获取链符号
pub fn get_chain_symbol(chain: &str) -> anyhow::Result<&'static str> {
    get_chain_config(chain).map(|config| config.symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_ethereum() {
        assert_eq!(normalize_chain_identifier("ETH").unwrap(), "ethereum");
        assert_eq!(normalize_chain_identifier("eth").unwrap(), "ethereum");
        assert_eq!(normalize_chain_identifier("Ethereum").unwrap(), "ethereum");
        assert_eq!(normalize_chain_identifier("1").unwrap(), "ethereum");
    }

    #[test]
    fn test_normalize_bsc() {
        assert_eq!(normalize_chain_identifier("BSC").unwrap(), "bsc");
        assert_eq!(normalize_chain_identifier("binance").unwrap(), "bsc");
        assert_eq!(normalize_chain_identifier("BNB").unwrap(), "bsc");
        assert_eq!(normalize_chain_identifier("56").unwrap(), "bsc");
    }

    #[test]
    fn test_is_evm_chain() {
        assert!(is_evm_chain("ethereum"));
        assert!(is_evm_chain("ETH"));
        assert!(is_evm_chain("bsc"));
        assert!(is_evm_chain("polygon"));
        assert!(!is_evm_chain("solana"));
        assert!(!is_evm_chain("bitcoin"));
    }

    #[test]
    fn test_get_chain_id() {
        assert_eq!(get_chain_id("ethereum").unwrap(), 1);
        assert_eq!(get_chain_id("ETH").unwrap(), 1);
        assert_eq!(get_chain_id("bsc").unwrap(), 56);
        assert_eq!(get_chain_id("solana").unwrap(), 501);
    }

    #[test]
    fn test_invalid_chain() {
        assert!(normalize_chain_identifier("invalid_chain").is_err());
        assert!(normalize_chain_identifier("").is_err());
    }
}
