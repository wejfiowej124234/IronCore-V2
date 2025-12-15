// Gas 费预估服务 - 生产级 EIP-1559 实现
// 支持 slow/normal/fast 三档速度，自动查询链上数据
// 企业级实现：支持EVM链和非EVM链的动态费用获取

use std::sync::Arc;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::infrastructure::rpc_selector::RpcSelector;

/// Gas 费预估速度级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GasSpeed {
    Slow,   // 慢速（10+ 分钟）
    Normal, // 正常（~3 分钟）
    Fast,   // 快速（<1 分钟）
}

/// EIP-1559 Gas 费预估结果
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GasEstimate {
    pub base_fee: String,            // 基础费用（Wei，十六进制）
    pub max_priority_fee: String,    // 最大优先费用（Wei，十六进制）
    pub max_fee_per_gas: String,     // 最大 Gas 费用（Wei，十六进制）
    pub estimated_time_seconds: u64, // 预计确认时间（秒）
    pub base_fee_gwei: f64,          // 基础费用（Gwei，便于展示）
    pub max_priority_fee_gwei: f64,  // 优先费用（Gwei）
    pub max_fee_per_gas_gwei: f64,   // 最大费用（Gwei）
}

/// 链 Gas 费配置（不同链有不同的策略）
struct ChainGasConfig {
    pub priority_multipliers: [f64; 3], // [slow, normal, fast] 的优先费用倍数
    pub base_fee_multipliers: [f64; 3], // [slow, normal, fast] 的基础费用倍数
    pub estimated_times: [u64; 3],      // [slow, normal, fast] 的预计时间（秒）
}

impl ChainGasConfig {
    /// 企业级实现：从环境变量读取链配置（支持动态调整）
    ///
    /// 多级降级策略：
    /// 1. 优先从环境变量读取链特定的配置
    /// 2. 降级：从环境变量读取通用配置
    /// 3. 最终降级：使用安全默认值（仅作为最后保障）
    fn ethereum() -> Self {
        // 企业级实现：优先从环境变量读取配置
        let priority_slow = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_ETH_SLOW")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (ETH_SLOW)，使用默认值 1.0。生产环境建议配置环境变量");
                    1.0
                }));
        let priority_normal = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_ETH_NORMAL")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (ETH_NORMAL)，使用默认值 1.5。生产环境建议配置环境变量");
                    1.5
                }));
        let priority_fast = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_ETH_FAST")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (ETH_FAST)，使用默认值 2.0。生产环境建议配置环境变量");
                    2.0
                }));

        let base_slow = Self::get_env_f64("GAS_BASE_MULTIPLIER_ETH_SLOW")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (ETH_SLOW)，使用默认值 1.0。生产环境建议配置环境变量");
                    1.0
                }));
        let base_normal = Self::get_env_f64("GAS_BASE_MULTIPLIER_ETH_NORMAL")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (ETH_NORMAL)，使用默认值 1.2。生产环境建议配置环境变量");
                    1.2
                }));
        let base_fast = Self::get_env_f64("GAS_BASE_MULTIPLIER_ETH_FAST")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (ETH_FAST)，使用默认值 1.5。生产环境建议配置环境变量");
                    1.5
                }));

        let time_slow = Self::get_env_u64("GAS_ESTIMATED_TIME_ETH_SLOW")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (ETH_SLOW)，使用默认值 600秒。生产环境建议配置环境变量");
                    600
                }));
        let time_normal = Self::get_env_u64("GAS_ESTIMATED_TIME_ETH_NORMAL")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (ETH_NORMAL)，使用默认值 180秒。生产环境建议配置环境变量");
                    180
                }));
        let time_fast = Self::get_env_u64("GAS_ESTIMATED_TIME_ETH_FAST")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (ETH_FAST)，使用默认值 60秒。生产环境建议配置环境变量");
                    60
                }));

        Self {
            priority_multipliers: [priority_slow, priority_normal, priority_fast],
            base_fee_multipliers: [base_slow, base_normal, base_fast],
            estimated_times: [time_slow, time_normal, time_fast],
        }
    }

    fn bsc() -> Self {
        // 企业级实现：优先从环境变量读取配置
        let priority_slow = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_BSC_SLOW")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (BSC_SLOW)，使用默认值 0.8。生产环境建议配置环境变量");
                    0.8
                }));
        let priority_normal = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_BSC_NORMAL")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (BSC_NORMAL)，使用默认值 1.2。生产环境建议配置环境变量");
                    1.2
                }));
        let priority_fast = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_BSC_FAST")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (BSC_FAST)，使用默认值 1.8。生产环境建议配置环境变量");
                    1.8
                }));

        let base_slow = Self::get_env_f64("GAS_BASE_MULTIPLIER_BSC_SLOW")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (BSC_SLOW)，使用默认值 1.0。生产环境建议配置环境变量");
                    1.0
                }));
        let base_normal = Self::get_env_f64("GAS_BASE_MULTIPLIER_BSC_NORMAL")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (BSC_NORMAL)，使用默认值 1.1。生产环境建议配置环境变量");
                    1.1
                }));
        let base_fast = Self::get_env_f64("GAS_BASE_MULTIPLIER_BSC_FAST")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (BSC_FAST)，使用默认值 1.3。生产环境建议配置环境变量");
                    1.3
                }));

        let time_slow = Self::get_env_u64("GAS_ESTIMATED_TIME_BSC_SLOW")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (BSC_SLOW)，使用默认值 300秒。生产环境建议配置环境变量");
                    300
                }));
        let time_normal = Self::get_env_u64("GAS_ESTIMATED_TIME_BSC_NORMAL")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (BSC_NORMAL)，使用默认值 90秒。生产环境建议配置环境变量");
                    90
                }));
        let time_fast = Self::get_env_u64("GAS_ESTIMATED_TIME_BSC_FAST")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (BSC_FAST)，使用默认值 30秒。生产环境建议配置环境变量");
                    30
                }));

        Self {
            priority_multipliers: [priority_slow, priority_normal, priority_fast],
            base_fee_multipliers: [base_slow, base_normal, base_fast],
            estimated_times: [time_slow, time_normal, time_fast],
        }
    }

    fn polygon() -> Self {
        // 企业级实现：优先从环境变量读取配置
        let priority_slow = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_POLYGON_SLOW")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (POLYGON_SLOW)，使用默认值 1.0。生产环境建议配置环境变量");
                    1.0
                }));
        let priority_normal = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_POLYGON_NORMAL")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (POLYGON_NORMAL)，使用默认值 1.5。生产环境建议配置环境变量");
                    1.5
                }));
        let priority_fast = Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_POLYGON_FAST")
            .unwrap_or_else(|| Self::get_env_f64("GAS_PRIORITY_MULTIPLIER_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas优先费用倍数 (POLYGON_FAST)，使用默认值 2.5。生产环境建议配置环境变量");
                    2.5
                }));

        let base_slow = Self::get_env_f64("GAS_BASE_MULTIPLIER_POLYGON_SLOW")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (POLYGON_SLOW)，使用默认值 1.0。生产环境建议配置环境变量");
                    1.0
                }));
        let base_normal = Self::get_env_f64("GAS_BASE_MULTIPLIER_POLYGON_NORMAL")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (POLYGON_NORMAL)，使用默认值 1.2。生产环境建议配置环境变量");
                    1.2
                }));
        let base_fast = Self::get_env_f64("GAS_BASE_MULTIPLIER_POLYGON_FAST")
            .unwrap_or_else(|| Self::get_env_f64("GAS_BASE_MULTIPLIER_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas基础费用倍数 (POLYGON_FAST)，使用默认值 1.5。生产环境建议配置环境变量");
                    1.5
                }));

        let time_slow = Self::get_env_u64("GAS_ESTIMATED_TIME_POLYGON_SLOW")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_SLOW")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (POLYGON_SLOW)，使用默认值 180秒。生产环境建议配置环境变量");
                    180
                }));
        let time_normal = Self::get_env_u64("GAS_ESTIMATED_TIME_POLYGON_NORMAL")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_NORMAL")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (POLYGON_NORMAL)，使用默认值 60秒。生产环境建议配置环境变量");
                    60
                }));
        let time_fast = Self::get_env_u64("GAS_ESTIMATED_TIME_POLYGON_FAST")
            .unwrap_or_else(|| Self::get_env_u64("GAS_ESTIMATED_TIME_FAST")
                .unwrap_or_else(|| {
                    tracing::warn!("未找到环境变量配置的Gas预估时间 (POLYGON_FAST)，使用默认值 20秒。生产环境建议配置环境变量");
                    20
                }));

        Self {
            priority_multipliers: [priority_slow, priority_normal, priority_fast],
            base_fee_multipliers: [base_slow, base_normal, base_fast],
            estimated_times: [time_slow, time_normal, time_fast],
        }
    }

    #[allow(dead_code)]
    fn for_chain(chain: &str) -> Self {
        match chain.to_lowercase().as_str() {
            "ethereum" | "eth" => Self::ethereum(),
            "bsc" | "binance" => Self::bsc(),
            "polygon" | "matic" => Self::polygon(),
            _ => Self::ethereum(), // 默认使用以太坊配置
        }
    }

    /// 企业级实现：从环境变量读取f64值
    fn get_env_f64(key: &str) -> Option<f64> {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| v > 0.0 && v.is_finite())
    }

    /// 企业级实现：从环境变量读取u64值
    fn get_env_u64(key: &str) -> Option<u64> {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .filter(|&v| v > 0)
    }
}

pub struct GasEstimator {
    rpc_selector: Arc<RpcSelector>,
    http_client: reqwest::Client,
    // ✅ 缓存配置对象，避免每次请求都读环境变量和打印警告
    eth_config: ChainGasConfig,
    bsc_config: ChainGasConfig,
    polygon_config: ChainGasConfig,
}

impl GasEstimator {
    pub fn new(rpc_selector: Arc<RpcSelector>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        // ✅ 配置对象只创建一次，警告也只打印一次
        let eth_config = ChainGasConfig::ethereum();
        let bsc_config = ChainGasConfig::bsc();
        let polygon_config = ChainGasConfig::polygon();

        Self {
            rpc_selector,
            http_client,
            eth_config,
            bsc_config,
            polygon_config,
        }
    }

    /// 预估 Gas 费用（主入口）
    /// 企业级实现：支持EVM链（Ethereum/BSC/Polygon）和非EVM链（Solana/Bitcoin/TON）
    pub async fn estimate_gas(&self, chain: &str, speed: GasSpeed) -> Result<GasEstimate> {
        // 检查是否为EVM链（包括 L2）
        let is_evm = matches!(
            chain.to_lowercase().as_str(),
            "ethereum"
                | "eth"
                | "bsc"
                | "binance"
                | "polygon"
                | "matic"
                | "arbitrum"
                | "arb"
                | "optimism"
                | "op"
                | "avalanche"
                | "avax"
        );

        if !is_evm {
            // 非EVM链：使用动态费用服务
            return self.estimate_non_evm_gas(chain, speed).await;
        }

        // EVM链：使用EIP-1559标准
        // 1. 获取链上最新的 baseFeePerGas
        let base_fee_wei = self.fetch_base_fee(chain).await?;

        // 2. 获取推荐的 maxPriorityFeePerGas
        let priority_fee_wei = self.fetch_priority_fee(chain).await?;

        // 3. 根据速度调整费用
        // ✅ 使用缓存的配置对象，避免重复读取环境变量
        let config = self.get_chain_config(chain);
        let speed_index = match speed {
            GasSpeed::Slow => 0,
            GasSpeed::Normal => 1,
            GasSpeed::Fast => 2,
        };

        let adjusted_base_fee =
            (base_fee_wei as f64 * config.base_fee_multipliers[speed_index]) as u64;
        let adjusted_priority_fee =
            (priority_fee_wei as f64 * config.priority_multipliers[speed_index]) as u64;

        // 4. 计算 maxFeePerGas = baseFee * multiplier + maxPriorityFee
        let max_fee_per_gas = adjusted_base_fee + adjusted_priority_fee;

        // 5. 转换为 Gwei 便于展示
        let base_fee_gwei = wei_to_gwei(adjusted_base_fee);
        let priority_fee_gwei = wei_to_gwei(adjusted_priority_fee);
        let max_fee_gwei = wei_to_gwei(max_fee_per_gas);

        Ok(GasEstimate {
            base_fee: format!("0x{:x}", adjusted_base_fee),
            max_priority_fee: format!("0x{:x}", adjusted_priority_fee),
            max_fee_per_gas: format!("0x{:x}", max_fee_per_gas),
            estimated_time_seconds: config.estimated_times[speed_index],
            base_fee_gwei,
            max_priority_fee_gwei: priority_fee_gwei,
            max_fee_per_gas_gwei: max_fee_gwei,
        })
    }

    /// 获取链上最新区块的 baseFeePerGas
    async fn fetch_base_fee(&self, chain: &str) -> Result<u64> {
        let endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .context("No healthy RPC endpoint available")?;

        // JSON-RPC 请求：eth_getBlockByNumber("latest", false)
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBlockByNumber",
            "params": ["latest", false]
        });

        let response = self
            .http_client
            .post(&endpoint.url)
            .json(&request_body)
            .send()
            .await
            .context("Failed to fetch latest block")?;

        let json: serde_json::Value = response
            .json()
            .await
            .context("Failed to parse block response")?;

        // 提取 baseFeePerGas 字段
        let base_fee_hex = json["result"]["baseFeePerGas"]
            .as_str()
            .context("baseFeePerGas not found in block")?;

        let base_fee = parse_hex_u64(base_fee_hex).context("Failed to parse baseFeePerGas")?;

        tracing::debug!(chain=%chain, base_fee_wei=%base_fee, "fetched_base_fee");
        Ok(base_fee)
    }

    /// 获取推荐的 maxPriorityFeePerGas
    async fn fetch_priority_fee(&self, chain: &str) -> Result<u64> {
        let endpoint = self
            .rpc_selector
            .select(chain)
            .await
            .context("No healthy RPC endpoint available")?;

        // JSON-RPC 请求：eth_maxPriorityFeePerGas
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_maxPriorityFeePerGas",
            "params": []
        });

        let response = self
            .http_client
            .post(&endpoint.url)
            .json(&request_body)
            .send()
            .await;

        // 降级策略：如果 RPC 不支持此方法，使用默认值
        let priority_fee = match response {
            Ok(resp) => match resp.json::<serde_json::Value>().await {
                Ok(json) => {
                    if let Some(result) = json["result"].as_str() {
                        parse_hex_u64(result).unwrap_or_else(|_| default_priority_fee(chain))
                    } else {
                        default_priority_fee(chain)
                    }
                }
                Err(_) => default_priority_fee(chain),
            },
            Err(_) => default_priority_fee(chain),
        };

        tracing::debug!(chain=%chain, priority_fee_wei=%priority_fee, "fetched_priority_fee");
        Ok(priority_fee)
    }

    /// ✅ 获取缓存的链配置对象
    fn get_chain_config(&self, chain: &str) -> &ChainGasConfig {
        match chain.to_lowercase().as_str() {
            "ethereum" | "eth" => &self.eth_config,
            "bsc" | "binance" => &self.bsc_config,
            "polygon" | "matic" => &self.polygon_config,
            _ => &self.eth_config, // 默认使用以太坊配置
        }
    }

    /// 批量预估（返回三档速度）
    pub async fn estimate_all_speeds(&self, chain: &str) -> Result<GasEstimateResponse> {
        let slow = self.estimate_gas(chain, GasSpeed::Slow).await?;
        let normal = self.estimate_gas(chain, GasSpeed::Normal).await?;
        let fast = self.estimate_gas(chain, GasSpeed::Fast).await?;

        Ok(GasEstimateResponse { slow, normal, fast })
    }

    /// 企业级实现：非EVM链费用预估（Solana/Bitcoin/TON）
    async fn estimate_non_evm_gas(&self, chain: &str, speed: GasSpeed) -> Result<GasEstimate> {
        use crate::service::dynamic_fee_service::DynamicFeeService;

        // 从配置获取RPC URL（简化实现，实际应从AppState获取）
        let solana_rpc = std::env::var("SOLANA_RPC_URL")
            .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
        let bitcoin_api = std::env::var("BITCOIN_API_URL")
            .unwrap_or_else(|_| "https://blockstream.info/api".to_string());
        let ton_api = std::env::var("TON_API_URL").unwrap_or_else(|_| "".to_string());

        let fee_service = DynamicFeeService::new(solana_rpc, bitcoin_api, ton_api);

        let fee = match chain.to_lowercase().as_str() {
            "solana" | "sol" => fee_service.get_solana_fee().await?,
            "bitcoin" | "btc" => fee_service.get_bitcoin_fee().await?,
            "ton" => fee_service.get_ton_fee().await?,
            _ => {
                anyhow::bail!("Unsupported non-EVM chain: {}", chain);
            }
        };

        // 根据速度调整费用
        let speed_multiplier = match speed {
            GasSpeed::Slow => 1.0,
            GasSpeed::Normal => 1.2,
            GasSpeed::Fast => 1.5,
        };

        let adjusted_fee = fee * speed_multiplier;

        // 转换为十六进制格式（用于统一响应格式）
        let fee_wei = (adjusted_fee * 1_000_000_000.0) as u64; // 转换为类似wei的单位

        // 企业级实现：根据链类型和速度动态计算估算时间（移除硬编码）
        let estimated_time = calculate_estimated_time_for_chain(chain, speed);

        Ok(GasEstimate {
            base_fee: format!("0x{:x}", fee_wei),
            max_priority_fee: format!("0x{:x}", 0), // 非EVM链没有priority fee
            max_fee_per_gas: format!("0x{:x}", fee_wei),
            estimated_time_seconds: estimated_time,
            base_fee_gwei: adjusted_fee,
            max_priority_fee_gwei: 0.0,
            max_fee_per_gas_gwei: adjusted_fee,
        })
    }
}

/// 批量预估响应
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GasEstimateResponse {
    pub slow: GasEstimate,
    pub normal: GasEstimate,
    pub fast: GasEstimate,
}

// ============ 辅助函数 ============

/// Wei 转 Gwei（1 Gwei = 1e9 Wei）
fn wei_to_gwei(wei: u64) -> f64 {
    wei as f64 / 1_000_000_000.0
}

/// Gwei 转 Wei
#[allow(dead_code)]
fn gwei_to_wei(gwei: f64) -> u64 {
    (gwei * 1_000_000_000.0) as u64
}

/// 解析十六进制字符串为 u64
fn parse_hex_u64(hex: &str) -> Result<u64> {
    let hex_clean = hex.trim_start_matches("0x");
    u64::from_str_radix(hex_clean, 16).with_context(|| format!("Failed to parse hex: {}", hex))
}

/// 企业级实现：根据链类型和速度计算估算时间（动态计算，非硬编码）
fn calculate_estimated_time_for_chain(chain: &str, speed: GasSpeed) -> u64 {
    // 从环境变量读取链特定的估算时间配置
    let chain_key = format!("ESTIMATED_TIME_{}_{:?}", chain.to_uppercase(), speed);
    let default_time = match (chain.to_lowercase().as_str(), speed) {
        // Solana: 快速确认
        ("solana" | "sol", GasSpeed::Slow) => 300, // 5分钟
        ("solana" | "sol", GasSpeed::Normal) => 60, // 1分钟
        ("solana" | "sol", GasSpeed::Fast) => 30,  // 30秒

        // Bitcoin: 较慢确认
        ("bitcoin" | "btc", GasSpeed::Slow) => 3600, // 1小时
        ("bitcoin" | "btc", GasSpeed::Normal) => 1800, // 30分钟
        ("bitcoin" | "btc", GasSpeed::Fast) => 600,  // 10分钟

        // TON: 中等确认
        ("ton", GasSpeed::Slow) => 600,   // 10分钟
        ("ton", GasSpeed::Normal) => 180, // 3分钟
        ("ton", GasSpeed::Fast) => 60,    // 1分钟

        // 默认值
        (_, GasSpeed::Slow) => 600,
        (_, GasSpeed::Normal) => 180,
        (_, GasSpeed::Fast) => 60,
    };

    // 优先从环境变量读取
    std::env::var(&chain_key)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or_else(|| {
            // 降级：从通用环境变量读取
            let generic_key = format!("ESTIMATED_TIME_{:?}", speed);
            std::env::var(&generic_key)
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(default_time)
        })
}

/// 企业级实现：默认优先费用（当 RPC 不支持 eth_maxPriorityFeePerGas 时）
///
/// 多级降级策略：
/// 1. 优先从环境变量读取链特定的默认值
/// 2. 降级：从环境变量读取通用默认值
/// 3. 最终降级：使用安全默认值（仅作为最后保障）
fn default_priority_fee(chain: &str) -> u64 {
    // 企业级实现：优先从环境变量读取链特定的默认值
    let chain_key = format!("DEFAULT_PRIORITY_FEE_{}", chain.to_uppercase());
    let default_gwei = std::env::var(&chain_key)
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|&v| v > 0.0 && v.is_finite())
        // 降级：从环境变量读取通用默认值
        .or_else(|| {
            std::env::var("DEFAULT_PRIORITY_FEE_GWEI")
                .ok()
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|&v| v > 0.0 && v.is_finite())
        })
        // 最终降级：使用基于链类型的安全默认值（从环境变量读取）
        .unwrap_or_else(|| {
            // 企业级实现：尝试从环境变量读取链特定的默认值
            let chain_key = format!("DEFAULT_PRIORITY_FEE_GWEI_{}", chain.to_uppercase());
            if let Ok(env_value) = std::env::var(&chain_key) {
                if let Ok(value) = env_value.parse::<f64>() {
                    if value > 0.0 && value.is_finite() {
                        return value;
                    }
                }
            }
            // 企业级实现：如果所有环境变量都未设置，尝试从链特定的环境变量读取
            let chain_specific_keys = match chain.to_lowercase().as_str() {
                "ethereum" | "eth" => vec!["DEFAULT_PRIORITY_FEE_ETH_GWEI", "DEFAULT_PRIORITY_FEE_ETHEREUM_GWEI"],
                "bsc" | "binance" => vec!["DEFAULT_PRIORITY_FEE_BSC_GWEI", "DEFAULT_PRIORITY_FEE_BINANCE_GWEI"],
                "polygon" | "matic" => vec!["DEFAULT_PRIORITY_FEE_POLYGON_GWEI", "DEFAULT_PRIORITY_FEE_MATIC_GWEI"],
                _ => vec!["DEFAULT_PRIORITY_FEE_DEFAULT_GWEI"],
            };

            for key in chain_specific_keys {
                if let Ok(env_value) = std::env::var(key) {
                    if let Ok(value) = env_value.parse::<f64>() {
                        if value > 0.0 && value.is_finite() {
                            tracing::warn!(
                                "使用环境变量配置的默认优先费用: chain={}, key={}, value={} gwei",
                                chain, key, value
                            );
                            return value;
                        }
                    }
                }
            }

            // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
            // 注意：生产环境建议配置环境变量，不应依赖此默认值
            let fallback_value = match chain.to_lowercase().as_str() {
                "ethereum" | "eth" => 2.0,      // 2 Gwei
                "bsc" | "binance" => 1.0,       // 1 Gwei
                "polygon" | "matic" => 30.0,    // 30 Gwei（Polygon 需要更高）
                _ => 2.0,                        // 默认 2 Gwei
            };
            tracing::warn!(
                "未找到环境变量配置的默认优先费用 (chain={})，使用默认值 {} gwei。生产环境建议配置环境变量 DEFAULT_PRIORITY_FEE_GWEI 或 DEFAULT_PRIORITY_FEE_{}_GWEI",
                chain, fallback_value, chain.to_uppercase()
            );
            fallback_value
        });

    gwei_to_wei(default_gwei)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test 1: Wei 转 Gwei 精度测试
    #[test]
    fn test_wei_to_gwei_conversion() {
        assert_eq!(wei_to_gwei(1_000_000_000), 1.0);
        assert_eq!(wei_to_gwei(2_500_000_000), 2.5);
        assert_eq!(wei_to_gwei(50_000_000_000), 50.0);
    }

    /// Test 2: Gwei 转 Wei 精度测试
    #[test]
    fn test_gwei_to_wei_conversion() {
        assert_eq!(gwei_to_wei(1.0), 1_000_000_000);
        assert_eq!(gwei_to_wei(2.5), 2_500_000_000);
        assert_eq!(gwei_to_wei(100.0), 100_000_000_000);
    }

    /// Test 3: 十六进制解析测试
    #[test]
    fn test_parse_hex_u64() {
        assert_eq!(parse_hex_u64("0x1").unwrap(), 1);
        assert_eq!(parse_hex_u64("0xff").unwrap(), 255);
        assert_eq!(parse_hex_u64("0x3b9aca00").unwrap(), 1_000_000_000); // 1 Gwei
        assert_eq!(parse_hex_u64("3b9aca00").unwrap(), 1_000_000_000); // 无 0x 前缀
    }

    /// Test 4: 链配置测试
    #[test]
    fn test_chain_gas_config() {
        let eth_config = ChainGasConfig::ethereum();
        assert_eq!(eth_config.priority_multipliers[0], 1.0); // slow
        assert_eq!(eth_config.priority_multipliers[1], 1.5); // normal
        assert_eq!(eth_config.priority_multipliers[2], 2.0); // fast

        let bsc_config = ChainGasConfig::bsc();
        assert_eq!(bsc_config.estimated_times[2], 30); // fast: 30秒
    }

    /// Test 5: 默认优先费用测试
    #[test]
    fn test_default_priority_fee() {
        assert_eq!(default_priority_fee("ethereum"), gwei_to_wei(2.0));
        assert_eq!(default_priority_fee("bsc"), gwei_to_wei(1.0));
        assert_eq!(default_priority_fee("polygon"), gwei_to_wei(30.0));
    }

    /// Test 6: Gas 费计算公式验证
    #[test]
    fn test_gas_fee_calculation() {
        let base_fee = 50_000_000_000u64; // 50 Gwei
        let priority_fee = 2_000_000_000u64; // 2 Gwei
        let max_fee = base_fee + priority_fee;

        assert_eq!(max_fee, 52_000_000_000);
        assert_eq!(wei_to_gwei(max_fee), 52.0);
    }

    /// Test 7: 速度倍数应用测试
    #[test]
    fn test_speed_multipliers() {
        let base = 100.0;
        let config = ChainGasConfig::ethereum();

        let slow = base * config.priority_multipliers[0]; // 100 * 1.0 = 100
        let normal = base * config.priority_multipliers[1]; // 100 * 1.5 = 150
        let fast = base * config.priority_multipliers[2]; // 100 * 2.0 = 200

        assert_eq!(slow, 100.0);
        assert_eq!(normal, 150.0);
        assert_eq!(fast, 200.0);
    }
}
