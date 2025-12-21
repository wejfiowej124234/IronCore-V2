use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::RwLock;

/// 价格数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub symbol: String,
    pub price_usdt: Decimal,
    pub source: String,
    pub last_updated: DateTime<Utc>,
}

/// CoinGecko API 响应
#[derive(Debug, Deserialize)]
struct CoinGeckoResponse {
    #[serde(flatten)]
    prices: HashMap<String, CoinGeckoCoin>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoCoin {
    usd: f64,
}

/// 价格服务
pub struct PriceService {
    pool: PgPool,
    #[allow(dead_code)]
    redis: Option<redis::Client>,
    cache: Arc<RwLock<HashMap<String, Price>>>,
    client: reqwest::Client,
}

impl PriceService {
    pub fn new(pool: PgPool, redis_url: Option<String>) -> Self {
        let redis_client = redis_url.and_then(|url| redis::Client::open(url).ok());

        Self {
            pool,
            redis: redis_client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            client: reqwest::Client::new(),
        }
    }

    /// 获取单个币种价格（USDT）
    /// 返回f64以保持API兼容性，内部使用Decimal保证精度
    pub async fn get_price(&self, symbol: &str) -> Result<f64> {
        let decimal_price = self.get_price_decimal(symbol).await?;
        // 转换为f64，保持向后兼容
        decimal_price
            .to_f64()
            .ok_or_else(|| anyhow::anyhow!("Price value out of range for f64"))
    }

    /// 获取单个币种价格（USDT），返回Decimal类型（推荐使用）
    pub async fn get_price_decimal(&self, symbol: &str) -> Result<Decimal> {
        // 1. 先查内存缓存
        {
            let cache = self.cache.read().await;
            if let Some(price) = cache.get(symbol) {
                let age = Utc::now() - price.last_updated;
                if age.num_seconds() < 300 {
                    // 5分钟内有效
                    return Ok(price.price_usdt);
                }
            }
        }

        // 2. 查 Redis 缓存
        if let Ok(cached) = self.get_from_redis_decimal(symbol).await {
            return Ok(cached);
        }

        // 3. 查数据库
        // CockroachDB兼容：直接查询DECIMAL类型，无需类型转换
        let db_price = sqlx::query_as::<_, (String, Decimal, String, DateTime<Utc>)>(
            "SELECT symbol, price_usdt, source, last_updated FROM prices WHERE symbol = $1 ORDER BY last_updated DESC LIMIT 1"
        )
        .bind(symbol.to_uppercase())
        .fetch_optional(&self.pool)
        .await?;

        if let Some((sym, price, source, updated)) = db_price {
            let age = Utc::now() - updated;
            if age.num_seconds() < 300 {
                // 数据库数据也在5分钟内
                self.update_cache(sym.clone(), price, source, updated).await;
                return Ok(price);
            }
        }

        // 4. 从 CoinGecko 获取最新价格
        self.fetch_and_update_price(symbol).await
    }

    /// 批量获取价格
    pub async fn get_prices(&self, symbols: &[&str]) -> Result<HashMap<String, f64>> {
        let mut result = HashMap::new();
        for symbol in symbols {
            if let Ok(price) = self.get_price(symbol).await {
                result.insert(symbol.to_string(), price);
            }
        }
        Ok(result)
    }

    /// 批量获取价格（Decimal类型，推荐使用）
    pub async fn get_prices_decimal(&self, symbols: &[&str]) -> Result<HashMap<String, Decimal>> {
        let mut result = HashMap::new();
        for symbol in symbols {
            if let Ok(price) = self.get_price_decimal(symbol).await {
                result.insert(symbol.to_string(), price);
            }
        }
        Ok(result)
    }

    /// 从 CoinGecko 获取价格并更新
    async fn fetch_and_update_price(&self, symbol: &str) -> Result<Decimal> {
        let symbol_upper = symbol.trim().to_uppercase();
        let symbol_lower = symbol_upper.to_lowercase();
        let is_stablecoin = matches!(symbol_lower.as_str(), "usdt" | "usdc" | "dai" | "busd");

        let coin_id = self.symbol_to_coingecko_id(&symbol_upper);

        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
            coin_id
        );

        tracing::info!("Fetching price from CoinGecko: {}", url);

        let response = match self
            .client
            .get(&url)
            .header("User-Agent", "IronForge/1.0")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if is_stablecoin {
                    tracing::warn!(
                        symbol = %symbol_upper,
                        "CoinGecko fetch failed; using static stablecoin price=1: {e}"
                    );
                    return self.store_static_price(&symbol_upper).await;
                }
                return Err(anyhow::Error::new(e)).context("Failed to fetch price from CoinGecko");
            }
        };

        if !response.status().is_success() {
            if is_stablecoin {
                tracing::warn!(
                    symbol = %symbol_upper,
                    status = %response.status(),
                    "CoinGecko returned error; using static stablecoin price=1"
                );
                return self.store_static_price(&symbol_upper).await;
            }
            anyhow::bail!("CoinGecko API error: {}", response.status());
        }

        let data: CoinGeckoResponse = response
            .json()
            .await
            .context("Failed to parse CoinGecko response")?;

        let price_f64 = match data.prices.get(&coin_id) {
            Some(v) => v.usd,
            None => {
                if is_stablecoin {
                    tracing::warn!(
                        symbol = %symbol_upper,
                        coingecko_id = %coin_id,
                        "CoinGecko response missing id; using static stablecoin price=1"
                    );
                    return self.store_static_price(&symbol_upper).await;
                }
                anyhow::bail!("Price not found for {}", coin_id);
            }
        };

        // 转换为Decimal以保持精度
        let price = Decimal::from_f64_retain(price_f64)
            .ok_or_else(|| anyhow::anyhow!("Invalid price value: {}", price_f64))?;

        // 更新数据库
        // CockroachDB兼容：使用唯一约束，ON CONFLICT语法正确
        sqlx::query(
            "INSERT INTO prices (symbol, price_usdt, source, last_updated)
             VALUES ($1, $2, 'coingecko', CURRENT_TIMESTAMP)
             ON CONFLICT (symbol, source)
             DO UPDATE SET price_usdt = EXCLUDED.price_usdt, last_updated = CURRENT_TIMESTAMP",
        )
        .bind(&symbol_upper)
        .bind(price)
        .execute(&self.pool)
        .await
        .context("Failed to update price in database")?;

        // 更新缓存
        self.update_cache(
            symbol_upper.clone(),
            price,
            "coingecko".to_string(),
            Utc::now(),
        )
        .await;

        // 更新 Redis
        self.set_to_redis_decimal(&symbol_upper, price).await?;

        Ok(price)
    }

    async fn store_static_price(&self, symbol_upper: &str) -> Result<Decimal> {
        let price = Decimal::ONE;

        sqlx::query(
            "INSERT INTO prices (symbol, price_usdt, source, last_updated)
             VALUES ($1, $2, 'static', CURRENT_TIMESTAMP)
             ON CONFLICT (symbol, source)
             DO UPDATE SET price_usdt = EXCLUDED.price_usdt, last_updated = CURRENT_TIMESTAMP",
        )
        .bind(symbol_upper)
        .bind(price)
        .execute(&self.pool)
        .await
        .context("Failed to update static price in database")?;

        self.update_cache(
            symbol_upper.to_string(),
            price,
            "static".to_string(),
            Utc::now(),
        )
        .await;

        let _ = self.set_to_redis_decimal(symbol_upper, price).await;

        Ok(price)
    }

    /// 符号转 CoinGecko ID
    fn symbol_to_coingecko_id(&self, symbol: &str) -> String {
        match symbol.to_lowercase().as_str() {
            // majors
            "eth" => "ethereum".to_string(),
            "btc" => "bitcoin".to_string(),
            "sol" => "solana".to_string(),
            "bnb" => "binancecoin".to_string(),
            "matic" => "matic-network".to_string(),
            "avax" => "avalanche-2".to_string(),
            "dot" => "polkadot".to_string(),
            "ada" => "cardano".to_string(),

            // stablecoins (critical for bridge quote in prod)
            "usdt" => "tether".to_string(),
            "usdc" => "usd-coin".to_string(),
            "dai" => "dai".to_string(),
            "busd" => "binance-usd".to_string(),

            // default: try lowercase symbol as id (best-effort)
            other => other.to_string(),
        }
    }

    /// 更新内存缓存
    async fn update_cache(
        &self,
        symbol: String,
        price: Decimal,
        source: String,
        updated: DateTime<Utc>,
    ) {
        let mut cache = self.cache.write().await;
        cache.insert(
            symbol.to_uppercase(),
            Price {
                symbol: symbol.to_uppercase(),
                price_usdt: price,
                source,
                last_updated: updated,
            },
        );
    }

    /// 从 Redis 获取价格（可选，支持优雅降级）
    #[allow(dead_code)]
    async fn get_from_redis(&self, _symbol: &str) -> Result<f64> {
        // Redis未配置时优雅降级到数据库
        anyhow::bail!("Redis not configured or unavailable")
    }

    /// 从 Redis 获取价格（Decimal类型）
    async fn get_from_redis_decimal(&self, _symbol: &str) -> Result<Decimal> {
        // Redis未配置时优雅降级到数据库
        anyhow::bail!("Redis not configured or unavailable")
    }

    /// 存入 Redis缓存（5分钟过期，可选）
    #[allow(dead_code)]
    async fn set_to_redis(&self, _symbol: &str, _price: f64) -> Result<()> {
        // Redis未配置时静默跳过，不影响主流程
        Ok(())
    }

    /// 存入 Redis缓存（Decimal类型）
    async fn set_to_redis_decimal(&self, _symbol: &str, _price: Decimal) -> Result<()> {
        // Redis未配置时静默跳过，不影响主流程
        Ok(())
    }

    /// 后台任务：定时更新所有支持的币种价格
    pub async fn start_price_updater(self: Arc<Self>) {
        let supported_symbols = vec![
            "ETH", "SOL", "BTC", "BNB", "MATIC", "AVAX", "DOT", "ADA", "USDT", "USDC", "DAI",
            "BUSD",
        ];

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5分钟

            loop {
                interval.tick().await;

                tracing::info!("Starting background price update...");

                for symbol in &supported_symbols {
                    match self.fetch_and_update_price(symbol).await {
                        Ok(price) => {
                            tracing::info!("Updated {} price: {} USDT", symbol, price);
                        }
                        Err(e) => {
                            tracing::error!("Failed to update {} price: {}", symbol, e);
                        }
                    }

                    // 避免触发 API 限流
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }

                tracing::info!("Background price update completed");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实数据库连接
    async fn test_get_price() {
        // 测试需要配置数据库连接
    }

    #[test]
    #[ignore = "requires database connection"]
    fn test_symbol_to_coingecko_id() {
        // Mock池和Redis
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let service = PriceService::new(pool, None);

        assert_eq!(service.symbol_to_coingecko_id("ETH"), "ethereum");
        assert_eq!(service.symbol_to_coingecko_id("SOL"), "solana");
        assert_eq!(service.symbol_to_coingecko_id("BTC"), "bitcoin");

        assert_eq!(service.symbol_to_coingecko_id("USDT"), "tether");
        assert_eq!(service.symbol_to_coingecko_id("USDC"), "usd-coin");
        assert_eq!(service.symbol_to_coingecko_id("DAI"), "dai");
        assert_eq!(service.symbol_to_coingecko_id("BUSD"), "binance-usd");
    }
}
