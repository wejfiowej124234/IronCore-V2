//! 余额同步服务
//!
//! 企业级实现：自动同步链上余额，确保用户看到最新的资产信息
//! 在法币充值、跨链兑换、交易完成后自动触发余额同步

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result};
use sqlx::PgPool;
use tokio::time::interval;
use uuid::Uuid;

/// 余额同步服务
pub struct BalanceSyncService {
    pool: PgPool,
    blockchain_client: Arc<crate::service::blockchain_client::BlockchainClient>,
}

impl BalanceSyncService {
    /// 创建余额同步服务
    pub fn new(
        pool: PgPool,
        blockchain_client: Arc<crate::service::blockchain_client::BlockchainClient>,
    ) -> Self {
        Self {
            pool,
            blockchain_client,
        }
    }

    /// 同步指定钱包的余额
    ///
    /// # Arguments
    /// * `wallet_id` - 钱包ID
    /// * `chain` - 链标识
    /// * `address` - 钱包地址
    pub async fn sync_wallet_balance(
        &self,
        wallet_id: Uuid,
        chain: &str,
        address: &str,
    ) -> Result<String> {
        // 1. 查询链上余额
        let balance = self
            .query_chain_balance(chain, address)
            .await
            .context("Failed to query chain balance")?;

        // 2. 更新数据库中的余额缓存
        sqlx::query(
            r#"
            UPDATE user_wallets
            SET balance_cache = $1,
                balance_updated_at = CURRENT_TIMESTAMP
            WHERE id = $2
            "#,
        )
        .bind(&balance)
        .bind(wallet_id)
        .execute(&self.pool)
        .await
        .context("Failed to update balance cache")?;

        tracing::info!(
            wallet_id = %wallet_id,
            chain = %chain,
            address = %address,
            balance = %balance,
            "Wallet balance synced"
        );

        Ok(balance)
    }

    /// 查询链上余额
    async fn query_chain_balance(&self, chain: &str, address: &str) -> Result<String> {
        // 根据链类型选择查询方式
        match chain.to_lowercase().as_str() {
            "eth" | "ethereum" | "bsc" | "polygon" => {
                // Ethereum系列：使用eth_getBalance
                self.query_ethereum_balance(address).await
            }
            "sol" | "solana" => {
                // Solana：使用getBalance
                self.query_solana_balance(address).await
            }
            "btc" | "bitcoin" => {
                // Bitcoin：使用getaddressbalance
                self.query_bitcoin_balance(address).await
            }
            "ton" => {
                // TON：使用getAddressInformation
                self.query_ton_balance(address).await
            }
            _ => anyhow::bail!("Unsupported chain for balance query: {}", chain),
        }
    }

    /// 查询Ethereum系列余额
    /// 企业级实现：使用blockchain_client查询真实余额
    async fn query_ethereum_balance(&self, address: &str) -> Result<String> {
        // 使用blockchain_client查询原生代币余额
        let balance = self
            .blockchain_client
            .get_native_balance("ethereum", address)
            .await
            .context("Failed to query Ethereum balance")?;

        Ok(balance.to_string())
    }

    /// 查询Solana余额
    /// 企业级实现：使用Solana RPC查询真实余额
    async fn query_solana_balance(&self, address: &str) -> Result<String> {
        // Solana余额查询需要特定的RPC调用
        // 简化实现：使用blockchain_client（如果支持）或直接调用RPC
        // 实际生产环境应使用solana-client库
        use reqwest::Client;
        use serde_json::json;

        let client = Client::new();
        let rpc_url = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address]
        });

        let response = client
            .post(&rpc_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to call Solana RPC")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse Solana RPC response")?;

        if let Some(result) = json.get("result").and_then(|r| r.get("value")) {
            Ok(result.as_u64().unwrap_or(0).to_string())
        } else {
            anyhow::bail!("Invalid Solana RPC response: {:?}", json)
        }
    }

    /// 查询Bitcoin余额
    /// 企业级实现：使用Bitcoin RPC查询真实余额
    async fn query_bitcoin_balance(&self, address: &str) -> Result<String> {
        // Bitcoin余额查询需要UTXO聚合
        // 使用Blockstream API或Bitcoin RPC
        use reqwest::Client;

        let client = Client::new();
        let api_url = std::env::var("BITCOIN_API_URL")
            .unwrap_or_else(|_| "https://blockstream.info/api".to_string());

        let url = format!("{}/address/{}/utxo", api_url, address);

        #[derive(serde::Deserialize)]
        struct Utxo {
            value: u64,
        }

        let utxos: Vec<Utxo> = client
            .get(&url)
            .send()
            .await
            .context("Failed to call Bitcoin API")?
            .json()
            .await
            .context("Failed to parse Bitcoin API response")?;

        let total: u64 = utxos.iter().map(|u| u.value).sum();
        Ok(total.to_string())
    }

    /// 查询TON余额
    /// 企业级实现：使用TON API查询真实余额
    async fn query_ton_balance(&self, address: &str) -> Result<String> {
        // TON余额查询使用TON Center API
        use reqwest::Client;

        let client = Client::new();
        let api_url = std::env::var("TON_API_URL")
            .unwrap_or_else(|_| "https://toncenter.com/api/v2".to_string());

        let url = format!("{}/getAddressInformation?address={}", api_url, address);

        #[derive(serde::Deserialize)]
        struct TonResponse {
            ok: bool,
            result: TonResult,
        }

        #[derive(serde::Deserialize)]
        struct TonResult {
            balance: String, // Nanotons
        }

        let response: TonResponse = client
            .get(&url)
            .send()
            .await
            .context("Failed to call TON API")?
            .json()
            .await
            .context("Failed to parse TON API response")?;

        if response.ok {
            Ok(response.result.balance)
        } else {
            anyhow::bail!("TON API returned error")
        }
    }

    /// 启动后台余额同步任务
    ///
    /// 定期同步所有活跃钱包的余额
    pub async fn start_background_sync(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(300)); // 每5分钟同步一次

        tracing::info!("Balance sync service started");

        loop {
            ticker.tick().await;

            match self.sync_all_active_wallets().await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(count = count, "Synced wallet balances");
                    }
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to sync wallet balances");
                }
            }
        }
    }

    /// 同步所有活跃钱包的余额
    async fn sync_all_active_wallets(&self) -> Result<usize> {
        // 查询最近24小时内有活动的钱包
        let wallets = sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT id, chain, address
            FROM user_wallets
            WHERE updated_at > CURRENT_TIMESTAMP - INTERVAL '24 hours'
            ORDER BY updated_at DESC
            LIMIT 100
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to query active wallets")?;

        let mut synced_count = 0;

        for (wallet_id, chain, address) in wallets {
            match self.sync_wallet_balance(wallet_id, &chain, &address).await {
                Ok(_) => synced_count += 1,
                Err(e) => {
                    tracing::warn!(
                        wallet_id = %wallet_id,
                        chain = %chain,
                        error = ?e,
                        "Failed to sync wallet balance"
                    );
                }
            }
        }

        Ok(synced_count)
    }

    /// 在法币充值完成后同步余额
    pub async fn sync_after_fiat_deposit(
        &self,
        wallet_id: Uuid,
        chain: &str,
        address: &str,
    ) -> Result<()> {
        self.sync_wallet_balance(wallet_id, chain, address)
            .await
            .map(|_| ())
    }

    /// 在跨链兑换完成后同步余额
    pub async fn sync_after_swap(&self, wallet_id: Uuid, chain: &str, address: &str) -> Result<()> {
        self.sync_wallet_balance(wallet_id, chain, address)
            .await
            .map(|_| ())
    }
}
