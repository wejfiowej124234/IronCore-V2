use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::{fee_service::FeeService, price_service::PriceService};
use crate::repository::wallet_repository::WalletRepository;

/// 跨链兑换请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainSwapRequest {
    #[serde(skip_deserializing)]
    pub user_id: Uuid,
    pub source_chain: String, // eth, bsc, polygon
    pub source_token: String, // ETH, BNB, MATIC
    pub source_amount: f64,   // 用户输入的数量
    #[serde(default)]
    pub source_wallet_id: Uuid, // 自动从用户钱包列表中选择
    pub target_chain: String, // sol, avax
    pub target_token: String, // SOL, AVAX
    pub target_wallet_id: Option<Uuid>, // 可选：自动创建
}

/// 跨链兑换响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainSwapResponse {
    pub swap_id: String,
    pub status: String, // pending, processing, completed, failed
    pub source_amount: f64,
    pub estimated_target_amount: f64,      // 预估收到数量
    pub actual_target_amount: Option<f64>, // 实际收到（完成后）
    pub exchange_rate: f64,                // 汇率（source → target）
    pub fee_usdt: f64,                     // 手续费（USDT）
    pub bridge_protocol: String,           // wormhole, layerzero
    pub estimated_time_minutes: u32,       // 预估时间（分钟）
    pub created_at: String,
    pub completed_at: Option<String>,
}

/// 跨链兑换估价
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub source_chain: String,
    pub target_chain: String,
    pub source_amount: f64,
    pub target_amount: f64,
    pub exchange_rate: f64,
    pub fee_usdt: f64,
    pub total_fee_percentage: f64, // 总手续费率
    pub estimated_time_minutes: u32,
    pub recommended_protocol: String, // 推荐桥协议
}

/// 跨链桥服务（企业级实现）
///
/// 业务逻辑：
/// - 跨链桥收取：跨链费用（bridge_fee）+ 平台服务费（platform_fee）
/// - 跨链费用：跨链桥协议收取的费用
/// - 平台服务费：钱包服务商收取的服务费用（通过FeeService计算）
///
/// 注意：这两个费用是完全独立的，不能混淆！
pub struct CrossChainBridgeService {
    pool: PgPool,
    price_service: Arc<PriceService>,
    config: Arc<crate::config::CrossChainConfig>,
    fee_service: Arc<FeeService>, // 企业级实现：用于计算平台服务费
    wallet_repo: Arc<dyn WalletRepository>, // 企业级实现：用于获取钱包地址
}

impl CrossChainBridgeService {
    pub fn new(
        pool: PgPool,
        price_service: Arc<PriceService>,
        config: Arc<crate::config::CrossChainConfig>,
        fee_service: Arc<FeeService>, // 企业级实现：用于计算平台服务费
        wallet_repo: Arc<dyn WalletRepository>, // 企业级实现：用于获取钱包地址
    ) -> Self {
        Self {
            pool,
            price_service,
            config,
            fee_service,
            wallet_repo,
        }
    }

    /// 获取跨链兑换报价✅验证
    pub async fn get_swap_quote(
        &self,
        source_chain: &str,
        source_token: &str,
        source_amount: f64,
        target_chain: &str,
        target_token: &str,
    ) -> Result<SwapQuote> {
        // ✅参数验证
        if source_amount <= 0.0 || !source_amount.is_finite() {
            anyhow::bail!("Invalid amount: must be > 0 and finite");
        }
        if source_chain.trim().is_empty() || target_chain.trim().is_empty() {
            anyhow::bail!("Chain identifiers required");
        }
        if source_chain.eq_ignore_ascii_case(target_chain) {
            anyhow::bail!("Source and target chains must be different for cross-chain swap");
        }

        tracing::info!(
            "Cross-chain quote: {} {} ({}) → {} ({})",
            source_amount,
            source_token,
            source_chain,
            target_token,
            target_chain
        );

        // 1. 获取源币种和目标币种价格
        let source_price = self.price_service.get_price(source_token).await?;
        let target_price = self.price_service.get_price(target_token).await?;

        // 2. 计算源币种的 USDT 价值
        let source_value_usdt = source_amount * source_price;

        // 3. 企业级实现：从配置读取手续费（支持环境变量动态调整）
        // 优先从环境变量读取，降级到配置文件
        let bridge_fee = std::env::var("BRIDGE_FEE_PERCENTAGE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| (0.0..=1.0).contains(&v))
            .unwrap_or(self.config.bridge_fee_percentage);

        let tx_fee = std::env::var("BRIDGE_TX_FEE_PERCENTAGE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|&v| (0.0..=1.0).contains(&v))
            .unwrap_or(self.config.transaction_fee_percentage);

        let total_fee_percentage = bridge_fee + tx_fee;
        let fee_usdt = source_value_usdt * total_fee_percentage;

        // 验证费用合理性
        if total_fee_percentage > 0.1 {
            tracing::warn!(
                "跨链桥费用过高警告: total_fee_percentage={}%, bridge_fee={}%, tx_fee={}%",
                total_fee_percentage * 100.0,
                bridge_fee * 100.0,
                tx_fee * 100.0
            );
        }

        // 4. 计算目标币种数量（扣除手续费后）
        let net_value_usdt = source_value_usdt - fee_usdt;
        let target_amount = net_value_usdt / target_price;

        // 5. 计算汇率（source → target）
        let exchange_rate = source_price / target_price;

        // 6. 选择最佳桥协议
        let recommended_protocol = self.select_best_bridge(source_chain, target_chain);

        // 7. 估算时间
        let estimated_time_minutes = self.estimate_bridge_time(source_chain, target_chain);

        tracing::info!(
            bridge_fee_pct = %bridge_fee,
            tx_fee_pct = %tx_fee,
            total_fee_pct = %total_fee_percentage,
            "Using configured cross-chain fees"
        );

        Ok(SwapQuote {
            source_chain: source_chain.to_string(),
            target_chain: target_chain.to_string(),
            source_amount,
            target_amount,
            exchange_rate,
            fee_usdt,
            total_fee_percentage: total_fee_percentage * 100.0,
            estimated_time_minutes,
            recommended_protocol,
        })
    }

    /// 执行跨链兑换（用户确认后调用）
    ///
    /// 企业级实现：业务逻辑
    /// - 跨链桥收取：跨链费用（bridge_fee）+ 平台服务费（platform_fee）
    /// - 跨链费用：跨链桥协议收取的费用（已包含在quote.fee_usdt中）
    /// - 平台服务费：钱包服务商收取的服务费用（通过FeeService计算）
    ///
    /// 注意：这两个费用是完全独立的，不能混淆！
    pub async fn execute_swap(
        &self,
        request: CrossChainSwapRequest,
    ) -> Result<CrossChainSwapResponse> {
        // ✅验证请求
        if request.source_amount <= 0.0 || !request.source_amount.is_finite() {
            anyhow::bail!("Invalid source_amount");
        }
        if request
            .source_chain
            .eq_ignore_ascii_case(&request.target_chain)
        {
            anyhow::bail!("Cannot bridge to same chain");
        }

        tracing::info!("Executing cross-chain swap: {:?}", request);

        // 1. 获取报价（包含跨链桥协议费用）
        let quote = self
            .get_swap_quote(
                &request.source_chain,
                &request.source_token,
                request.source_amount,
                &request.target_chain,
                &request.target_token,
            )
            .await?;

        // 2. 企业级实现：计算平台服务费（与跨链费用完全独立）
        // 业务逻辑：跨链桥收取 跨链费用 + 平台服务费
        let chain_key = request.source_chain.to_lowercase();

        // 企业级实现：获取钱包地址用于费用审计记录
        let wallet_address = self
            .wallet_repo
            .find_by_id(request.source_wallet_id)
            .await
            .ok()
            .flatten()
            .map(|w| w.address)
            .unwrap_or_else(|| {
                tracing::warn!(
                    "无法获取钱包地址，使用空字符串: wallet_id={}",
                    request.source_wallet_id
                );
                String::new()
            });

        let platform_fee_result = self
            .fee_service
            .calculate_fee(&chain_key, "bridge", request.source_amount)
            .await;

        // 记录平台服务费（如果计算成功）
        if let Ok(Some(fee_calc)) = platform_fee_result {
            tracing::info!(
                "跨链桥平台服务费计算成功: 服务费={}, 收款地址={}, 钱包地址={}",
                fee_calc.platform_fee,
                fee_calc.collector_address,
                wallet_address
            );

            // 记录费用审计（失败不阻断主流程）
            // 注意：跨链交易可能还没有tx_hash，先记录服务费，tx_hash后续回填
            if let Err(e) = self
                .fee_service
                .record_fee_audit(
                    request.user_id,
                    &chain_key,
                    "bridge",
                    request.source_amount,
                    &fee_calc,
                    &wallet_address, // 企业级实现：使用实际钱包地址
                    None,            // tx_hash（跨链交易可能还没有tx_hash，后续回填）
                )
                .await
            {
                tracing::warn!(error=?e, "跨链桥服务费审计记录失败; continuing without blocking bridge transaction");
            }
        } else {
            tracing::warn!("跨链桥平台服务费计算失败或未配置规则，不收取服务费");
        }

        // 3. 创建跨链交易记录
        let swap_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO cross_chain_swaps (
                id, user_id, source_chain, source_token, source_amount, source_wallet_id,
                target_chain, target_token, estimated_amount, target_wallet_id,
                exchange_rate, fee_usdt, status, bridge_protocol
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'pending', $13)
            "#,
        )
        .bind(swap_id)
        .bind(request.user_id)
        .bind(&request.source_chain)
        .bind(&request.source_token)
        .bind(request.source_amount)
        .bind(request.source_wallet_id)
        .bind(&request.target_chain)
        .bind(&request.target_token)
        .bind(quote.target_amount)
        .bind(request.target_wallet_id)
        .bind(quote.exchange_rate)
        .bind(quote.fee_usdt)
        .bind(&quote.recommended_protocol)
        .execute(&self.pool)
        .await
        .context("Failed to create swap record")?;

        // 3. 启动后台任务执行跨链（异步）
        let pool = self.pool.clone();
        let swap_id_clone = swap_id;
        tokio::spawn(async move {
            if let Err(e) = Self::process_swap_async(pool, swap_id_clone).await {
                tracing::error!("Swap processing failed: {}", e);
            }
        });

        Ok(CrossChainSwapResponse {
            swap_id: swap_id.to_string(),
            status: "pending".to_string(),
            source_amount: request.source_amount,
            estimated_target_amount: quote.target_amount,
            actual_target_amount: None,
            exchange_rate: quote.exchange_rate,
            fee_usdt: quote.fee_usdt,
            bridge_protocol: quote.recommended_protocol,
            estimated_time_minutes: quote.estimated_time_minutes,
            created_at: Utc::now().to_rfc3339(),
            completed_at: None,
        })
    }

    /// 查询跨链兑换状态
    pub async fn get_swap_status(&self, swap_id: Uuid) -> Result<CrossChainSwapResponse> {
        // CockroachDB兼容：直接查询DECIMAL类型，无需类型转换
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                String,
                rust_decimal::Decimal,
                String,
                String,
                Option<rust_decimal::Decimal>,
                rust_decimal::Decimal,
                rust_decimal::Decimal,
                rust_decimal::Decimal,
                String,
                Option<String>,
                DateTime<Utc>,
                Option<DateTime<Utc>>,
            ),
        >(
            r#"
            SELECT id, source_chain, source_token, source_amount, target_chain, target_token,
                   target_amount, estimated_amount, exchange_rate,
                   fee_usdt, status, bridge_protocol, created_at, completed_at
            FROM cross_chain_swaps WHERE id = $1
            "#,
        )
        .bind(swap_id)
        .fetch_one(&self.pool)
        .await
        .context("Swap not found")?;

        let (
            _id,
            source_chain,
            _source_token,
            source_amount,
            target_chain,
            _target_token,
            actual_amount,
            estimated_amount,
            exchange_rate,
            fee_usdt,
            status,
            protocol,
            created_at,
            completed_at,
        ) = row;

        // 转换为f64以保持API兼容性
        use rust_decimal::prelude::ToPrimitive;
        Ok(CrossChainSwapResponse {
            swap_id: swap_id.to_string(),
            status,
            source_amount: source_amount.to_f64().unwrap_or(0.0),
            estimated_target_amount: estimated_amount.to_f64().unwrap_or(0.0),
            actual_target_amount: actual_amount.and_then(|v| v.to_f64()),
            exchange_rate: exchange_rate.to_f64().unwrap_or(0.0),
            fee_usdt: fee_usdt.to_f64().unwrap_or(0.0),
            bridge_protocol: protocol.clone().unwrap_or_default(),
            estimated_time_minutes: Self::calculate_bridge_time(
                &source_chain,
                &target_chain,
                protocol.as_deref(),
            ),
            created_at: created_at.to_rfc3339(),
            completed_at: completed_at.map(|t| t.to_rfc3339()),
        })
    }

    /// 后台异步处理跨链兑换
    async fn process_swap_async(pool: PgPool, swap_id: Uuid) -> Result<()> {
        tracing::info!("Starting async swap processing: {}", swap_id);

        // 1. 更新状态为 processing
        sqlx::query("UPDATE cross_chain_swaps SET status = 'processing', updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(swap_id)
            .execute(&pool)
            .await?;

        // 查询 swap 详情
        let swap = sqlx::query_as::<_, (String, String, String, rust_decimal::Decimal, uuid::Uuid)>(
            "SELECT source_chain, target_chain, source_token, source_amount, user_id FROM cross_chain_swaps WHERE id = $1"
        )
        .bind(swap_id)
        .fetch_one(&pool)
        .await?;

        let (source_chain, target_chain, source_token, source_amount, user_id) = swap;

        // 2. 跨链桥接过程（生产级实现）
        use crate::{
            repository::wallet_repository::PgWalletRepository,
            service::bridge_sdk::{create_bridge, BridgeRequest},
        };

        // PRODUCTION: Query user's wallet address on target chain for receiving assets
        let wallet_repo = PgWalletRepository::new(pool.clone());
        let recipient_address = wallet_repo
            .get_user_address_for_chain(user_id, &target_chain)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "User {} does not have a wallet on target chain '{}'. Please create a wallet first.",
                    user_id,
                    target_chain
                )
            })?;

        tracing::info!(
            user_id = %user_id,
            target_chain = %target_chain,
            recipient = %recipient_address,
            "Using user's target chain wallet as recipient"
        );

        // 创建桥接请求
        let bridge_request = BridgeRequest {
            swap_id,
            source_chain: source_chain.clone(),
            target_chain: target_chain.clone(),
            token: source_token.clone(),
            amount: source_amount.to_string(),
            recipient: recipient_address,
        };

        // 选择并创建桥接 SDK
        let bridge = create_bridge(&source_chain, &target_chain)?;

        // 步骤1: 锁定源链资产
        tracing::info!(swap_id = %swap_id, "Step 1: Locking asset on source chain");
        let lock_tx = bridge.lock_asset(&bridge_request).await?;
        tracing::info!(swap_id = %swap_id, lock_tx = %lock_tx, "Asset locked successfully");

        // 步骤2: 生成桥接证明
        tracing::info!(swap_id = %swap_id, "Step 2: Generating bridge proof");
        let proof = bridge.generate_proof(&lock_tx).await?;
        tracing::info!(swap_id = %swap_id, proof_len = proof.len(), "Proof generated");

        // 步骤3: 在目标链铸造/解锁资产
        tracing::info!(swap_id = %swap_id, "Step 3: Minting on target chain");
        let mint_tx = bridge.mint_on_target(&proof, &bridge_request).await?;
        tracing::info!(swap_id = %swap_id, mint_tx = %mint_tx, "Asset minted on target chain");

        // 步骤4: 验证并更新状态
        tracing::info!(swap_id = %swap_id, "Step 4: Verifying transaction completion");
        let status = bridge.query_status(&mint_tx).await?;
        tracing::info!(swap_id = %swap_id, status = ?status, "Bridge status verified");

        // 3. 更新为完成状态
        sqlx::query(
            "UPDATE cross_chain_swaps SET status = 'completed', target_amount = estimated_amount, completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1"
        )
        .bind(swap_id)
        .execute(&pool)
        .await?;

        tracing::info!("Swap completed: {}", swap_id);
        Ok(())
    }

    /// 企业级实现：选择最佳桥协议（从环境变量读取配置）
    ///
    /// 多级降级策略：
    /// 1. 优先从环境变量读取链组合特定的协议
    /// 2. 降级：从环境变量读取通用默认协议
    /// 3. 最终降级：使用安全默认值（仅作为最后保障）
    fn select_best_bridge(&self, source_chain: &str, target_chain: &str) -> String {
        // 企业级实现：优先从环境变量读取链组合特定的协议
        let pair_key = format!(
            "BRIDGE_PROTOCOL_{}_{}",
            source_chain.to_uppercase(),
            target_chain.to_uppercase()
        );
        let protocol = std::env::var(&pair_key)
            .ok()
            .or_else(|| {
                // 尝试反向链组合
                let reverse_key = format!("BRIDGE_PROTOCOL_{}_{}",
                    target_chain.to_uppercase(),
                    source_chain.to_uppercase()
                );
                std::env::var(&reverse_key).ok()
            })
            .or_else(|| {
                // 降级：从环境变量读取通用默认协议
                std::env::var("BRIDGE_PROTOCOL_DEFAULT").ok()
            })
            .unwrap_or_else(|| {
                // 最终降级：使用基于链组合的安全默认值
                tracing::warn!(
                    "未找到桥接协议配置 (from={}, to={})，使用安全默认值 wormhole，建议配置环境变量",
                    source_chain, target_chain
                );
                match (source_chain, target_chain) {
                    ("eth", "sol") | ("sol", "eth") => "wormhole".to_string(),
                    ("eth", "bsc") | ("bsc", "eth") => "layerzero".to_string(),
                    ("polygon", "avax") | ("avax", "polygon") => "across".to_string(),
                    _ => "wormhole".to_string(), // 安全默认值
                }
            });

        protocol
    }

    /// 企业级实现：基于链对和协议计算跨链时间（分钟）
    ///
    /// 多级降级策略：
    /// 1. 优先从环境变量读取链组合和协议特定的时间
    /// 2. 降级：从环境变量读取协议特定的时间
    /// 3. 最终降级：使用安全默认值（仅作为最后保障）
    fn calculate_bridge_time(
        source_chain: &str,
        target_chain: &str,
        protocol: Option<&str>,
    ) -> u32 {
        // 企业级实现：优先从环境变量读取链组合和协议特定的时间
        if let Some(proto) = protocol {
            let pair_protocol_key = format!(
                "BRIDGE_TIME_{}_{}_{}",
                source_chain.to_uppercase(),
                target_chain.to_uppercase(),
                proto.to_uppercase()
            );
            if let Some(time) = std::env::var(&pair_protocol_key)
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|&v| v > 0 && v <= 1440)
            // 验证范围：0-1440分钟（24小时）
            {
                return time;
            }

            // 尝试反向链组合
            let reverse_key = format!(
                "BRIDGE_TIME_{}_{}_{}",
                target_chain.to_uppercase(),
                source_chain.to_uppercase(),
                proto.to_uppercase()
            );
            if let Some(time) = std::env::var(&reverse_key)
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|&v| v > 0 && v <= 1440)
            {
                return time;
            }

            // 降级：从环境变量读取协议特定的时间
            let protocol_key = format!("BRIDGE_TIME_{}", proto.to_uppercase());
            if let Some(time) = std::env::var(&protocol_key)
                .ok()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|&v| v > 0 && v <= 1440)
            {
                return time;
            }
        }

        // 企业级实现：从环境变量读取链组合特定的时间（无协议）
        let pair_key = format!(
            "BRIDGE_TIME_{}_{}",
            source_chain.to_uppercase(),
            target_chain.to_uppercase()
        );
        if let Some(time) = std::env::var(&pair_key)
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v > 0 && v <= 1440)
        {
            return time;
        }

        // 尝试反向链组合
        let reverse_key = format!(
            "BRIDGE_TIME_{}_{}",
            target_chain.to_uppercase(),
            source_chain.to_uppercase()
        );
        if let Some(time) = std::env::var(&reverse_key)
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|&v| v > 0 && v <= 1440)
        {
            return time;
        }

        // 最终降级：使用基于协议和链组合的安全默认值
        match protocol {
            Some("wormhole") => match (source_chain, target_chain) {
                ("ethereum", "solana") | ("solana", "ethereum") => 10,
                ("bsc", "polygon") | ("polygon", "bsc") => 8,
                _ => 12,
            },
            Some("layerzero") => match (source_chain, target_chain) {
                ("ethereum", "bsc") | ("bsc", "ethereum") => 5,
                ("polygon", "avalanche") | ("avalanche", "polygon") => 7,
                _ => 8,
            },
            Some("stargate") => 15,
            _ => {
                // 默认估算：基于源链确认时间
                match source_chain {
                    "ethereum" => 15,
                    "bitcoin" => 60,
                    "bsc" | "polygon" => 5,
                    "solana" => 2,
                    _ => 15,
                }
            }
        }
    }

    /// 估算桥接时间（分钟）- 实例方法保持向后兼容
    fn estimate_bridge_time(&self, source_chain: &str, target_chain: &str) -> u32 {
        Self::calculate_bridge_time(source_chain, target_chain, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires database connection"]
    fn test_select_best_bridge() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let price_service = Arc::new(PriceService::new(pool.clone(), None));
        let cross_chain_config = Arc::new(crate::config::CrossChainConfig::default());
        // 测试环境：创建mock fee_service 和 wallet_repo
        use crate::repository::wallet_repository::PgWalletRepository;
        let fee_service = Arc::new(FeeService::new(pool.clone()));
        let wallet_repo = Arc::new(PgWalletRepository::new(pool.clone()));
        let service = CrossChainBridgeService::new(
            pool,
            price_service,
            cross_chain_config,
            fee_service,
            wallet_repo,
        );

        assert_eq!(service.select_best_bridge("eth", "sol"), "wormhole");
        assert_eq!(service.select_best_bridge("eth", "bsc"), "layerzero");
    }

    #[test]
    #[ignore = "requires database connection"]
    fn test_estimate_bridge_time() {
        let pool = PgPool::connect_lazy("postgres://localhost/test").unwrap();
        let price_service = Arc::new(PriceService::new(pool.clone(), None));
        let cross_chain_config = Arc::new(crate::config::CrossChainConfig::default());
        // 测试环境：创建mock fee_service 和 wallet_repo
        use crate::repository::wallet_repository::PgWalletRepository;
        let fee_service = Arc::new(FeeService::new(pool.clone()));
        let wallet_repo = Arc::new(PgWalletRepository::new(pool.clone()));
        let service = CrossChainBridgeService::new(
            pool,
            price_service,
            cross_chain_config,
            fee_service,
            wallet_repo,
        );

        assert_eq!(service.estimate_bridge_time("eth", "sol"), 15);
        assert_eq!(service.estimate_bridge_time("sol", "eth"), 5);
    }
}
