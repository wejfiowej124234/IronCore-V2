//! 代币服务
//! 企业级实现，从数据库读取代币信息，替代硬编码

use std::sync::Arc;

use anyhow::{Context, Result};
use sqlx::PgPool;

use crate::repository::{PgTokenRepository, Token, TokenRepository};

/// 代币服务
pub struct TokenService {
    repository: Arc<dyn TokenRepository>,
}

impl TokenService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            repository: Arc::new(PgTokenRepository::new(pool)),
        }
    }

    /// 根据符号和链ID获取代币地址
    pub async fn get_token_address(&self, symbol: &str, chain_id: u64) -> Result<Option<String>> {
        let token = self
            .repository
            .get_by_symbol_and_chain(symbol, chain_id)
            .await
            .context("Failed to get token address")?;

        Ok(token.map(|t| t.address))
    }

    /// 根据符号和链ID获取代币小数位数
    pub async fn get_token_decimals(&self, symbol: &str, chain_id: u64) -> Result<Option<u32>> {
        let token = self
            .repository
            .get_by_symbol_and_chain(symbol, chain_id)
            .await
            .context("Failed to get token decimals")?;

        Ok(token.map(|t| t.decimals as u32))
    }

    /// 根据符号和链ID获取完整代币信息
    pub async fn get_token(&self, symbol: &str, chain_id: u64) -> Result<Option<Token>> {
        self.repository
            .get_by_symbol_and_chain(symbol, chain_id)
            .await
            .context("Failed to get token")
    }

    /// 获取链上所有启用的代币
    pub async fn list_tokens_by_chain(&self, chain_id: u64) -> Result<Vec<Token>> {
        self.repository
            .list_by_chain(chain_id)
            .await
            .context("Failed to list tokens by chain")
    }

    /// 获取链上的稳定币列表
    pub async fn list_stablecoins_by_chain(&self, chain_id: u64) -> Result<Vec<Token>> {
        self.repository
            .list_stablecoins_by_chain(chain_id)
            .await
            .context("Failed to list stablecoins by chain")
    }

    /// 验证代币是否支持（企业级验证）
    pub async fn is_token_supported(&self, symbol: &str, chain_id: u64) -> bool {
        self.get_token(symbol, chain_id)
            .await
            .map(|t| t.is_some())
            .unwrap_or(false)
    }
}

/// 网络名称到Chain ID映射✅完整支持
pub fn network_to_chain_id(network: &str) -> Option<u64> {
    match network.to_lowercase().as_str() {
        "eth" | "ethereum" => Some(1),
        "bsc" | "binance" => Some(56),
        "polygon" | "matic" => Some(137),
        "arbitrum" | "arb" => Some(42161),
        "optimism" | "op" => Some(10),
        "avalanche" | "avax" => Some(43114),
        "solana" | "sol" => Some(501),
        "bitcoin" | "btc" => Some(0),
        "ton" => Some(607),
        _ => None,
    }
}
