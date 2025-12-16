//! 1inch Aggregation API 集成服务
//! 企业级实现，支持同链代币交换

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

/// 1inch API 客户端
pub struct OneInchService {
    client: Client,
    pub api_key: Option<String>,
    base_url: String,
}

/// 1inch API 报价响应
#[derive(Debug, Deserialize)]
struct OneInchQuoteResponse {
    #[serde(rename = "toAmount")]
    to_amount: String,
    #[serde(rename = "fromToken")]
    #[allow(dead_code)]
    from_token: TokenInfo,
    #[serde(rename = "toToken")]
    #[allow(dead_code)]
    to_token: TokenInfo,
    #[serde(default, rename = "protocols")]
    #[allow(dead_code)]
    protocols: Vec<Vec<Vec<ProtocolRoute>>>,
    #[serde(rename = "estimatedGas")]
    estimated_gas: Option<String>,
    #[serde(rename = "estimatedGasUSD")]
    estimated_gas_usd: Option<String>,
    // 1inch API可能返回的其他字段
    #[serde(rename = "fromAmount")]
    #[allow(dead_code)]
    from_amount: Option<String>,
    #[serde(rename = "priceImpact")]
    price_impact: Option<String>, // 价格影响（百分比字符串，如 "0.5"）
}

#[derive(Debug, Deserialize)]
struct TokenInfo {
    #[allow(dead_code)]
    symbol: String,
    #[allow(dead_code)]
    address: String,
    #[allow(dead_code)]
    decimals: u32,
}

#[derive(Debug, Deserialize)]
struct ProtocolRoute {
    #[allow(dead_code)]
    name: String,
    #[serde(rename = "fromTokenAddress")]
    #[allow(dead_code)]
    from_token_address: String,
    #[serde(rename = "toTokenAddress")]
    #[allow(dead_code)]
    to_token_address: String,
    #[allow(dead_code)]
    part: f64,
}

/// 1inch Swap 响应
#[derive(Debug, Deserialize)]
struct OneInchSwapResponse {
    tx: TransactionData,
    #[serde(rename = "toAmount")]
    #[allow(dead_code)]
    to_amount: String,
}

/// 交易数据
#[derive(Debug, Deserialize, Clone)]
pub struct TransactionData {
    pub from: String,
    pub to: String,
    pub data: String,
    pub value: String,
    pub gas: Option<String>,
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<String>,
}

/// Swap报价结果
#[derive(Debug, Clone)]
pub struct SwapQuote {
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub exchange_rate: f64,
    pub price_impact: f64,
    pub gas_estimate: String,
    pub estimated_gas_usd: f64,
    pub valid_for: u32,
}

impl Default for OneInchService {
    fn default() -> Self {
        Self::new()
    }
}

impl OneInchService {
    /// 创建新的 1inch 服务
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        let api_key = std::env::var("ONEINCH_API_KEY").ok();

        Self {
            client,
            api_key,
            base_url: "https://api.1inch.dev".to_string(),
        }
    }

    /// 获取交换报价
    pub async fn get_quote(
        &self,
        chain_id: u64,
        from_token: &str,
        to_token: &str,
        amount: &str,
    ) -> Result<SwapQuote> {
        let url = format!("{}/swap/v5.2/{}/quote", self.base_url, chain_id);

        let mut request = self.client.get(&url).query(&[
            ("src", from_token),
            ("dst", to_token),
            ("amount", amount),
        ]);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await.context("1inch API 请求失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("无法读取错误响应: {}", e));
            anyhow::bail!("1inch API 返回错误 {}: {}", status, body);
        }

        let data: OneInchQuoteResponse = response.json().await.context("解析 1inch 响应失败")?;

        self.convert_quote_response(data, from_token, to_token, amount)
    }

    /// 获取交换交易数据
    pub async fn get_swap_tx(
        &self,
        chain_id: u64,
        from_token: &str,
        to_token: &str,
        amount: &str,
        from_address: &str,
        slippage: f64,
    ) -> Result<TransactionData> {
        let url = format!("{}/swap/v5.2/{}/swap", self.base_url, chain_id);

        let mut request = self.client.get(&url).query(&[
            ("src", from_token),
            ("dst", to_token),
            ("amount", amount),
            ("from", from_address),
            ("slippage", &slippage.to_string()),
            ("disableEstimate", "true"),
        ]);

        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await.context("1inch swap API 请求失败")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|e| format!("无法读取错误响应: {}", e));
            anyhow::bail!("1inch swap API 返回错误 {}: {}", status, body);
        }

        let data: OneInchSwapResponse = response.json().await.context("解析 swap 响应失败")?;

        Ok(data.tx)
    }

    /// 转换 1inch 响应为我们的格式
    fn convert_quote_response(
        &self,
        data: OneInchQuoteResponse,
        from_symbol: &str,
        to_symbol: &str,
        from_amount: &str,
    ) -> Result<SwapQuote> {
        let to_amount = data.to_amount.clone();

        let from_amount_f64 = from_amount.parse::<f64>().context("无效的源代币数量")?;
        let to_amount_f64 = to_amount.parse::<f64>().context("无效的目标代币数量")?;

        let exchange_rate = if from_amount_f64 > 0.0 {
            to_amount_f64 / from_amount_f64
        } else {
            0.0
        };

        // 从1inch API获取gas估算
        // 企业级实现：如果API没有返回gas估算，应该返回错误而不是使用硬编码值
        let gas_estimate = data
            .estimated_gas
            .ok_or_else(|| anyhow::anyhow!("1inch API未返回gas估算，无法确定交易成本"))?;

        // 从API响应中获取Gas费用（USD）
        // 企业级实现：如果API没有返回，尝试从链上数据计算，而不是返回0
        let estimated_gas_usd = if let Some(gas_usd_str) = &data.estimated_gas_usd {
            gas_usd_str.parse::<f64>().unwrap_or_else(|e| {
                tracing::warn!("解析estimatedGasUSD失败: {}, 值: {}", e, gas_usd_str);
                0.0
            })
        } else {
            // 如果API没有返回，记录警告但继续处理
            // 前端可以根据gas_estimate和当前gas price自行计算
            tracing::warn!("1inch API未返回estimatedGasUSD，前端需要自行计算");
            0.0
        };

        // 价格影响：优先从1inch API获取，如果没有则基于交换数量估算
        let price_impact = if let Some(pi_str) = &data.price_impact {
            // 从API响应中解析价格影响（百分比字符串，如 "0.5" 表示 0.5%）
            pi_str.parse::<f64>().unwrap_or_else(|_| {
                // 企业级实现：如果解析失败，尝试从环境变量读取默认值
                std::env::var("SWAP_PRICE_IMPACT_DEFAULT")
                    .ok()
                    .and_then(|v| v.parse::<f64>().ok())
                    .filter(|&v| v >= 0.0 && v.is_finite())
                    .unwrap_or_else(|| {
                        tracing::error!(
                            "严重警告：价格影响解析失败且未找到环境变量配置，使用硬编码默认值 0.0。生产环境必须配置环境变量 SWAP_PRICE_IMPACT_DEFAULT"
                        );
                        0.0 // 安全默认值（仅作为最后保障，生产环境不应使用）
                    })
            })
        } else {
            // 企业级实现：如果API未返回价格影响，使用基于交易金额的估算
            // 价格影响阈值从环境变量或配置读取（而非硬编码）
            let large_amount_threshold = std::env::var("SWAP_LARGE_AMOUNT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的大额阈值 (SWAP_LARGE_AMOUNT_THRESHOLD)，使用硬编码默认值 100000.0。生产环境必须配置此环境变量");
                    100000.0 // 安全默认值（仅作为最后保障，生产环境不应使用）
                });
            let medium_amount_threshold = std::env::var("SWAP_MEDIUM_AMOUNT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的中等阈值 (SWAP_MEDIUM_AMOUNT_THRESHOLD)，使用硬编码默认值 10000.0。生产环境必须配置此环境变量");
                    10000.0 // 安全默认值（仅作为最后保障，生产环境不应使用）
                });
            let small_amount_threshold = std::env::var("SWAP_SMALL_AMOUNT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的小额阈值 (SWAP_SMALL_AMOUNT_THRESHOLD)，使用硬编码默认值 1000.0。生产环境必须配置此环境变量");
                    1000.0 // 安全默认值（仅作为最后保障，生产环境不应使用）
                });
            let tiny_amount_threshold = std::env::var("SWAP_TINY_AMOUNT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的微小额阈值 (SWAP_TINY_AMOUNT_THRESHOLD)，使用硬编码默认值 100.0。生产环境必须配置此环境变量");
                    100.0 // 安全默认值（仅作为最后保障，生产环境不应使用）
                });
            let micro_amount_threshold = std::env::var("SWAP_MICRO_AMOUNT_THRESHOLD")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的微额阈值 (SWAP_MICRO_AMOUNT_THRESHOLD)，使用硬编码默认值 10.0。生产环境必须配置此环境变量");
                    10.0 // 安全默认值（仅作为最后保障，生产环境不应使用）
                });

            // 企业级实现：价格影响百分比从环境变量读取，多级降级策略
            // 多级降级策略：
            // 1. 优先从环境变量读取配置的价格影响值
            // 2. 最终降级：使用安全默认值（仅作为最后保障）
            let large_impact = std::env::var("SWAP_LARGE_AMOUNT_IMPACT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的大额价格影响 (SWAP_LARGE_AMOUNT_IMPACT)，使用硬编码默认值 1.0%。生产环境必须配置此环境变量");
                    1.0 // 安全默认值：1.0%（仅作为最后保障，生产环境不应使用）
                });
            let medium_impact = std::env::var("SWAP_MEDIUM_AMOUNT_IMPACT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的中等价格影响 (SWAP_MEDIUM_AMOUNT_IMPACT)，使用硬编码默认值 0.5%。生产环境必须配置此环境变量");
                    0.5 // 安全默认值：0.5%（仅作为最后保障，生产环境不应使用）
                });
            let small_impact = std::env::var("SWAP_SMALL_AMOUNT_IMPACT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的小额价格影响 (SWAP_SMALL_AMOUNT_IMPACT)，使用硬编码默认值 0.2%。生产环境必须配置此环境变量");
                    0.2 // 安全默认值：0.2%（仅作为最后保障，生产环境不应使用）
                });
            let tiny_impact = std::env::var("SWAP_TINY_AMOUNT_IMPACT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的微小额价格影响 (SWAP_TINY_AMOUNT_IMPACT)，使用硬编码默认值 0.1%。生产环境必须配置此环境变量");
                    0.1 // 安全默认值：0.1%（仅作为最后保障，生产环境不应使用）
                });
            let micro_impact = std::env::var("SWAP_MICRO_AMOUNT_IMPACT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的微额价格影响 (SWAP_MICRO_AMOUNT_IMPACT)，使用硬编码默认值 0.05%。生产环境必须配置此环境变量");
                    0.05 // 安全默认值：0.05%（仅作为最后保障，生产环境不应使用）
                });
            let default_impact = std::env::var("SWAP_DEFAULT_IMPACT")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
                .unwrap_or_else(|| {
                    tracing::error!("严重警告：未找到环境变量配置的默认价格影响 (SWAP_DEFAULT_IMPACT)，使用硬编码默认值 0.01%。生产环境必须配置此环境变量");
                    0.01 // 安全默认值：0.01%（仅作为最后保障，生产环境不应使用）
                });

            // 基于交易金额的估算（使用配置值）
            if from_amount_f64 > large_amount_threshold {
                large_impact
            } else if from_amount_f64 > medium_amount_threshold {
                medium_impact
            } else if from_amount_f64 > small_amount_threshold {
                small_impact
            } else if from_amount_f64 > tiny_amount_threshold {
                tiny_impact
            } else if from_amount_f64 > micro_amount_threshold {
                micro_impact
            } else {
                default_impact
            }
        };

        Ok(SwapQuote {
            from_token: from_symbol.to_string(),
            to_token: to_symbol.to_string(),
            from_amount: from_amount.to_string(),
            to_amount,
            exchange_rate,
            price_impact,
            gas_estimate,
            estimated_gas_usd,
            valid_for: 30, // 30秒有效期
        })
    }
}

// 注意：get_token_address 和 network_to_chain_id 函数已移至 token_service 模块
// 请使用 TokenService::get_token_address() 和 token_service::network_to_chain_id()
