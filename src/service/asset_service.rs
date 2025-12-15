use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::price_service::PriceService;

/// 资产响应（统一 USDT 展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetResponse {
    pub total_usdt: f64,                  // 总资产（USDT）
    pub total_wallets: usize,             // 钱包总数
    pub assets_by_chain: Vec<ChainAsset>, // 按链分组的资产
    pub last_updated: String,             // 最后更新时间
}

/// 单链资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainAsset {
    pub chain_symbol: String, // eth, sol, btc
    pub chain_name: String,   // Ethereum, Solana, Bitcoin
    pub balance: f64,         // 原生币余额
    pub balance_usdt: f64,    // USDT 等价值
    pub percentage: f64,      // 占总资产百分比
    pub wallet_count: usize,  // 该链上钱包数量
    pub current_price: f64,   // 当前价格（USDT）
}

/// 单个钱包资产
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAsset {
    pub wallet_id: String,
    pub wallet_name: String,
    pub chain_symbol: String,
    pub address: String,
    pub balance: f64,
    pub balance_usdt: f64,
    pub current_price: f64,
}

/// 资产聚合服务
pub struct AssetService {
    pool: PgPool,
    price_service: Arc<PriceService>,
    config: Arc<crate::config::BlockchainConfig>,
}

impl AssetService {
    pub fn new(
        pool: PgPool,
        price_service: Arc<PriceService>,
        config: Arc<crate::config::BlockchainConfig>,
    ) -> Self {
        Self {
            pool,
            price_service,
            config,
        }
    }

    /// 获取用户总资产（所有钱包，USDT 统一展示）
    pub async fn get_user_total_assets(&self, user_id: Uuid) -> Result<AssetResponse> {
        tracing::info!("Fetching total assets for user: {}", user_id);

        // 1. 获取用户所有钱包
        let wallets = sqlx::query_as::<_, (Uuid, Option<String>, Option<String>, String)>(
            "SELECT id, name, chain_symbol, address FROM wallets WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch wallets")?;

        if wallets.is_empty() {
            return Ok(AssetResponse {
                total_usdt: 0.0,
                total_wallets: 0,
                assets_by_chain: vec![],
                last_updated: chrono::Utc::now().to_rfc3339(),
            });
        }

        // 2. 按链分组钱包
        let mut chain_groups: HashMap<String, Vec<(Uuid, String, String)>> = HashMap::new();
        for (id, name, chain_symbol, address) in wallets {
            let chain = chain_symbol
                .unwrap_or_else(|| String::from("unknown"))
                .to_lowercase();
            let wallet_name = name.unwrap_or_else(|| format!("Wallet {}", &address[..8]));

            chain_groups
                .entry(chain.clone())
                .or_default()
                .push((id, wallet_name, address));
        }

        // 3. 获取所有需要的价格
        let symbols: Vec<&str> = chain_groups
            .keys()
            .map(|s| self.chain_symbol_to_token(s))
            .collect();

        let prices = self.price_service.get_prices(&symbols).await?;

        // 4. 聚合每条链的资产
        let mut assets_by_chain = Vec::new();
        let mut total_usdt = 0.0;

        for (chain_symbol, wallets) in &chain_groups {
            let token_symbol = self.chain_symbol_to_token(chain_symbol);
            let current_price = prices.get(token_symbol).copied().unwrap_or(0.0);

            // 查询真实链上余额（并发查询所有钱包）
            let mut balance = 0.0;
            for (_id, _name, address) in wallets {
                match self.fetch_balance_by_chain(chain_symbol, address).await {
                    Ok(wallet_balance) => balance += wallet_balance,
                    Err(e) => {
                        tracing::warn!(chain = %chain_symbol, address = %address, error = %e, "Failed to fetch wallet balance");
                        // 失败时不累加，记录警告继续
                    }
                }
            }
            let balance_usdt = balance * current_price;

            total_usdt += balance_usdt;

            assets_by_chain.push(ChainAsset {
                chain_symbol: chain_symbol.clone(),
                chain_name: self.chain_name(chain_symbol),
                balance,
                balance_usdt,
                percentage: 0.0, // 后面计算
                wallet_count: wallets.len(),
                current_price,
            });
        }

        // 5. 计算百分比
        for asset in &mut assets_by_chain {
            if total_usdt > 0.0 {
                asset.percentage = (asset.balance_usdt / total_usdt) * 100.0;
            }
        }

        // 6. 按 USDT 价值排序
        assets_by_chain.sort_by(|a, b| {
            b.balance_usdt
                .partial_cmp(&a.balance_usdt)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(AssetResponse {
            total_usdt,
            total_wallets: chain_groups.values().map(|v| v.len()).sum(),
            assets_by_chain,
            last_updated: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// 获取单个钱包资产
    pub async fn get_wallet_asset(&self, wallet_id: Uuid) -> Result<WalletAsset> {
        let wallet = sqlx::query_as::<_, (Uuid, Option<String>, Option<String>, String)>(
            "SELECT id, name, chain_symbol, address FROM wallets WHERE id = $1",
        )
        .bind(wallet_id)
        .fetch_one(&self.pool)
        .await
        .context("Wallet not found")?;

        let (id, name, chain_symbol, address) = wallet;
        let chain = chain_symbol
            .unwrap_or_else(|| String::from("unknown"))
            .to_lowercase();
        let token_symbol = self.chain_symbol_to_token(&chain);

        let current_price = self.price_service.get_price(token_symbol).await?;

        // 查询真实链上余额
        let balance = self.fetch_balance_by_chain(&chain, &address)
            .await
            .unwrap_or_else(|e| {
                tracing::error!(chain = %chain, address = %address, error = %e, "Failed to fetch balance");
                0.0 // 失败时返回0
            });
        let balance_usdt = balance * current_price;

        Ok(WalletAsset {
            wallet_id: id.to_string(),
            wallet_name: name.unwrap_or_else(|| format!("Wallet {}", &address[..8])),
            chain_symbol: chain,
            address,
            balance,
            balance_usdt,
            current_price,
        })
    }

    /// 链符号转代币符号
    fn chain_symbol_to_token(&self, chain: &str) -> &str {
        match chain.to_lowercase().as_str() {
            "eth" | "ethereum" => "ETH",
            "bsc" | "bnb" => "BNB",
            "polygon" | "matic" => "MATIC",
            "sol" | "solana" => "SOL",
            "btc" | "bitcoin" => "BTC",
            "avax" | "avalanche" => "AVAX",
            "dot" | "polkadot" => "DOT",
            "ada" | "cardano" => "ADA",
            _ => "ETH", // 默认
        }
    }

    /// 获取链名称
    fn chain_name(&self, chain: &str) -> String {
        match chain.to_lowercase().as_str() {
            "eth" => "Ethereum",
            "bsc" => "BNB Chain",
            "polygon" => "Polygon",
            "sol" => "Solana",
            "btc" => "Bitcoin",
            "avax" => "Avalanche",
            "dot" => "Polkadot",
            "ada" => "Cardano",
            _ => chain,
        }
        .to_string()
    }

    /// 查询链上余额（生产级真实 RPC 调用，使用配置文件的RPC地址）
    async fn fetch_balance_by_chain(&self, chain: &str, address: &str) -> Result<f64> {
        // 从配置读取真实 RPC 端点（生产环境需配置API密钥）
        match chain.to_lowercase().as_str() {
            "eth" | "ethereum" => {
                self.fetch_eth_balance(address, &self.config.eth_rpc_url)
                    .await
            }
            "bsc" | "binance" => {
                self.fetch_eth_balance(address, &self.config.bsc_rpc_url)
                    .await
            }
            "polygon" | "matic" => {
                self.fetch_eth_balance(address, &self.config.polygon_rpc_url)
                    .await
            }
            "sol" | "solana" => self.fetch_sol_balance(address).await,
            "btc" | "bitcoin" => self.fetch_btc_balance(address).await,
            _ => Ok(0.0),
        }
    }

    /// 查询 EVM 链余额（ETH/BSC/Polygon）
    async fn fetch_eth_balance(&self, address: &str, rpc_url: &str) -> Result<f64> {
        use std::str::FromStr;

        use ethers::{
            providers::{Http, Middleware, Provider},
            types::Address,
        };

        // 使用 ethers-rs 创建 provider
        let provider =
            Provider::<Http>::try_from(rpc_url).context("Failed to create Ethereum provider")?;

        // 解析地址
        let address = Address::from_str(address).context("Invalid Ethereum address")?;

        // 查询余额
        let balance = provider
            .get_balance(address, None)
            .await
            .context("Failed to fetch balance")?;

        // 转换为 ETH（Wei → ETH）
        let balance_eth = balance.as_u128() as f64 / 1e18;

        tracing::info!(
            address = %address,
            balance_wei = %balance,
            balance_eth = %balance_eth,
            rpc = %rpc_url,
            "Fetched EVM balance using ethers-rs"
        );

        Ok(balance_eth)
    }

    /// 查询 Solana 余额 - 使用真实的 JSON-RPC
    async fn fetch_sol_balance(&self, address: &str) -> Result<f64> {
        // 验证 Solana 地址格式（Base58，32字节）
        let decoded = bs58::decode(address)
            .into_vec()
            .context("Invalid Solana address format")?;

        if decoded.len() != 32 {
            anyhow::bail!("Solana address must be 32 bytes");
        }

        // 使用配置的 Solana RPC 端点
        let client = reqwest::Client::new();
        let rpc_url = self.config.solana_rpc_url.clone();

        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address]
        });

        let response = client
            .post(rpc_url)
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to send Solana RPC request")?;

        let result: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Solana RPC response")?;

        // 解析余额（lamports）
        let balance_lamports = result["result"]["value"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("Invalid balance response from Solana RPC"))?;

        // 转换为 SOL（lamports / 10^9）
        let balance_sol = balance_lamports as f64 / 1e9;

        tracing::info!(
            address = %address,
            balance_lamports = %balance_lamports,
            balance_sol = %balance_sol,
            rpc = %self.config.solana_rpc_url,
            "Fetched Solana balance using JSON-RPC"
        );

        Ok(balance_sol)
    }

    /// 查询 Bitcoin 余额
    async fn fetch_btc_balance(&self, address: &str) -> Result<f64> {
        use std::str::FromStr;

        use bitcoin::Address;

        // 验证 Bitcoin 地址格式
        let _btc_address = Address::from_str(address).context("Invalid Bitcoin address")?;

        // 验证是比特币主网地址
        if !_btc_address.is_valid_for_network(bitcoin::Network::Bitcoin) {
            anyhow::bail!("Address is not valid for Bitcoin mainnet");
        }

        // 使用 Blockchain.info API 查询余额
        let client = reqwest::Client::new();
        let url = format!("https://blockchain.info/q/addressbalance/{}", address);

        let response = client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to query Bitcoin balance")?;

        if !response.status().is_success() {
            anyhow::bail!("Bitcoin API returned error: {}", response.status());
        }

        let balance_satoshis: u64 = response
            .text()
            .await?
            .parse()
            .context("Failed to parse balance")?;

        // 转换为 BTC（satoshis / 10^8）
        let balance_btc = balance_satoshis as f64 / 1e8;

        tracing::info!(
            address = %address,
            balance_satoshis = %balance_satoshis,
            balance_btc = %balance_btc,
            "Fetched Bitcoin balance using Blockchain.info API"
        );

        Ok(balance_btc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires database connection"]
    fn test_chain_symbol_to_token() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let price_service = Arc::new(PriceService::new(pool.clone(), None));
        let blockchain_config = Arc::new(crate::config::BlockchainConfig::default());
        let service = AssetService::new(pool, price_service, blockchain_config);

        assert_eq!(service.chain_symbol_to_token("eth"), "ETH");
        assert_eq!(service.chain_symbol_to_token("sol"), "SOL");
        assert_eq!(service.chain_symbol_to_token("bsc"), "BNB");
    }

    #[test]
    #[ignore = "requires database connection"]
    fn test_chain_name() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let price_service = Arc::new(PriceService::new(pool.clone(), None));
        let blockchain_config = Arc::new(crate::config::BlockchainConfig::default());
        let service = AssetService::new(pool, price_service, blockchain_config);

        assert_eq!(service.chain_name("eth"), "Ethereum");
        assert_eq!(service.chain_name("sol"), "Solana");
    }
}
