//! 统一费率配置服务
//! 企业级实现：所有费率统一管理，前后端统一调用

use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::RwLock;

/// 费率类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeeType {
    /// 平台服务费（兑换）
    SwapServiceFee,
    /// Gas费率（EVM链）
    GasFee,
    /// 跨链桥费率
    BridgeFee,
    /// 提现费率
    WithdrawalFee,
    /// 法币充值费率
    FiatDepositFee,
    /// 法币提现费率
    FiatWithdrawalFee,
}

/// 费率配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeConfig {
    /// 费率类型
    pub fee_type: FeeType,
    /// 链标识（可选，某些费率是链特定的）
    pub chain: Option<String>,
    /// 费率值（百分比，0-100）
    pub rate_percentage: f64,
    /// 最小费用（USD）
    pub min_fee_usd: Option<f64>,
    /// 最大费用（USD）
    pub max_fee_usd: Option<f64>,
    /// 固定费用（USD，优先于百分比）
    pub fixed_fee_usd: Option<f64>,
    /// 是否启用
    pub enabled: bool,
    /// 描述
    pub description: String,
    /// 最后更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 费率计算结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeCalculationResult {
    /// 费率类型
    pub fee_type: FeeType,
    /// 原始金额（USD）
    pub amount_usd: f64,
    /// 计算的费用（USD）
    pub fee_usd: f64,
    /// 费率（百分比）
    pub rate_percentage: f64,
    /// 实际扣除后金额
    pub net_amount_usd: f64,
}

/// 统一费率配置服务
pub struct UnifiedFeeConfigService {
    pool: PgPool,
    /// 内存缓存（5分钟刷新）
    cache: Arc<RwLock<HashMap<String, FeeConfig>>>,
    last_refresh: Arc<RwLock<chrono::DateTime<chrono::Utc>>>,
}

impl UnifiedFeeConfigService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
            last_refresh: Arc::new(RwLock::new(chrono::Utc::now())),
        }
    }

    /// 初始化默认费率配置（首次部署）
    pub async fn initialize_defaults(&self) -> Result<()> {
        let defaults = vec![
            FeeConfig {
                fee_type: FeeType::SwapServiceFee,
                chain: Some("global".to_string()),
                rate_percentage: 0.3, // 0.3%
                min_fee_usd: Some(0.1),
                max_fee_usd: Some(100.0),
                fixed_fee_usd: None,
                enabled: true,
                description: "Swap service fee".to_string(),
                updated_at: chrono::Utc::now(),
            },
            FeeConfig {
                fee_type: FeeType::BridgeFee,
                chain: Some("global".to_string()),
                rate_percentage: 0.1, // 0.1%
                min_fee_usd: Some(1.0),
                max_fee_usd: Some(500.0),
                fixed_fee_usd: None,
                enabled: true,
                description: "Cross-chain bridge fee".to_string(),
                updated_at: chrono::Utc::now(),
            },
            FeeConfig {
                fee_type: FeeType::WithdrawalFee,
                chain: Some("ETH".to_string()),
                rate_percentage: 0.0,
                min_fee_usd: None,
                max_fee_usd: None,
                fixed_fee_usd: Some(2.0),
                enabled: true,
                description: "ETH withdrawal fixed fee".to_string(),
                updated_at: chrono::Utc::now(),
            },
            FeeConfig {
                fee_type: FeeType::FiatDepositFee,
                chain: Some("global".to_string()),
                rate_percentage: 0.0,
                min_fee_usd: None,
                max_fee_usd: None,
                fixed_fee_usd: Some(0.0), // 充值免费
                enabled: true,
                description: "Fiat deposit fee (free)".to_string(),
                updated_at: chrono::Utc::now(),
            },
            FeeConfig {
                fee_type: FeeType::FiatWithdrawalFee,
                chain: Some("global".to_string()),
                rate_percentage: 0.5, // 0.5%
                min_fee_usd: Some(1.0),
                max_fee_usd: Some(50.0),
                fixed_fee_usd: None,
                enabled: true,
                description: "Fiat withdrawal fee".to_string(),
                updated_at: chrono::Utc::now(),
            },
        ];

        for config in defaults {
            self.upsert_fee_config(&config).await?;
        }

        tracing::info!("Initialized default fee configurations");
        Ok(())
    }

    /// 获取费率配置
    ///
    /// # 参数
    /// - `fee_type`: 费率类型
    /// - `chain`: 链标识（可选）
    pub async fn get_fee_config(
        &self,
        fee_type: FeeType,
        chain: Option<&str>,
    ) -> Result<FeeConfig> {
        // 刷新缓存（如果过期）
        self.refresh_cache_if_needed().await?;

        let cache_key = Self::make_cache_key(&fee_type, chain);
        let cache = self.cache.read().await;

        if let Some(config) = cache.get(&cache_key) {
            return Ok(config.clone());
        }

        drop(cache);

        // 缓存未命中，从数据库加载
        let config = self.load_fee_config_from_db(&fee_type, chain).await?;

        // 更新缓存
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, config.clone());

        Ok(config)
    }

    /// 计算费用
    ///
    /// # 参数
    /// - `fee_type`: 费率类型
    /// - `amount_usd`: 原始金额（USD）
    /// - `chain`: 链标识（可选）
    pub async fn calculate_fee(
        &self,
        fee_type: FeeType,
        amount_usd: f64,
        chain: Option<&str>,
    ) -> Result<FeeCalculationResult> {
        let config = self.get_fee_config(fee_type.clone(), chain).await?;

        if !config.enabled {
            return Ok(FeeCalculationResult {
                fee_type,
                amount_usd,
                fee_usd: 0.0,
                rate_percentage: 0.0,
                net_amount_usd: amount_usd,
            });
        }

        // 计算费用
        let fee_usd = if let Some(fixed_fee) = config.fixed_fee_usd {
            // 使用固定费用
            fixed_fee
        } else {
            // 使用百分比费率
            let calculated_fee = amount_usd * (config.rate_percentage / 100.0);

            // 应用最小/最大限制
            let fee_with_min = if let Some(min_fee) = config.min_fee_usd {
                calculated_fee.max(min_fee)
            } else {
                calculated_fee
            };

            if let Some(max_fee) = config.max_fee_usd {
                fee_with_min.min(max_fee)
            } else {
                fee_with_min
            }
        };

        Ok(FeeCalculationResult {
            fee_type,
            amount_usd,
            fee_usd,
            rate_percentage: config.rate_percentage,
            net_amount_usd: amount_usd - fee_usd,
        })
    }

    /// 批量计算多种费用
    pub async fn calculate_multiple_fees(
        &self,
        amount_usd: f64,
        fee_types: Vec<(FeeType, Option<String>)>,
    ) -> Result<Vec<FeeCalculationResult>> {
        let mut results = Vec::new();

        for (fee_type, chain) in fee_types {
            let result = self
                .calculate_fee(fee_type, amount_usd, chain.as_deref())
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    /// 更新费率配置（管理员操作）
    pub async fn update_fee_config(&self, config: &FeeConfig) -> Result<()> {
        self.upsert_fee_config(config).await?;

        // 清除缓存
        self.invalidate_cache().await;

        tracing::info!(
            fee_type = ?config.fee_type,
            chain = ?config.chain,
            rate = config.rate_percentage,
            "Updated fee configuration"
        );

        Ok(())
    }

    /// 列出所有费率配置
    pub async fn list_all_configs(&self) -> Result<Vec<FeeConfig>> {
        let configs = sqlx::query_as::<_, FeeConfigRow>(
            "SELECT fee_type, chain, rate_percentage, min_fee_usd, max_fee_usd, 
                    fixed_fee_usd, enabled, description, updated_at
             FROM fee_configurations
             ORDER BY fee_type, chain",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(configs.into_iter().map(|row| row.into()).collect())
    }

    // ===== Private Methods =====

    fn make_cache_key(fee_type: &FeeType, chain: Option<&str>) -> String {
        format!("{:?}:{}", fee_type, chain.unwrap_or("global"))
    }

    async fn refresh_cache_if_needed(&self) -> Result<()> {
        let last_refresh = *self.last_refresh.read().await;
        let now = chrono::Utc::now();

        // 5分钟刷新一次
        if now.signed_duration_since(last_refresh).num_minutes() >= 5 {
            self.refresh_cache().await?;
        }

        Ok(())
    }

    async fn refresh_cache(&self) -> Result<()> {
        let configs = self.list_all_configs().await?;

        let mut cache = self.cache.write().await;
        cache.clear();

        for config in configs {
            let key = Self::make_cache_key(&config.fee_type, config.chain.as_deref());
            cache.insert(key, config);
        }

        *self.last_refresh.write().await = chrono::Utc::now();

        Ok(())
    }

    async fn invalidate_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        *self.last_refresh.write().await = chrono::Utc::now() - chrono::Duration::minutes(10);
    }

    async fn load_fee_config_from_db(
        &self,
        fee_type: &FeeType,
        chain: Option<&str>,
    ) -> Result<FeeConfig> {
        // 将 None 转换为 "global"
        let chain_str = chain.unwrap_or("global");
        
        let row = sqlx::query_as::<_, FeeConfigRow>(
            "SELECT fee_type, chain, rate_percentage, min_fee_usd, max_fee_usd, 
                    fixed_fee_usd, enabled, description, updated_at
             FROM fee_configurations
             WHERE fee_type = $1 AND chain = $2
             LIMIT 1",
        )
        .bind(format!("{:?}", fee_type))
        .bind(chain_str)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn upsert_fee_config(&self, config: &FeeConfig) -> Result<()> {
        sqlx::query(
            "INSERT INTO fee_configurations 
             (fee_type, chain, rate_percentage, min_fee_usd, max_fee_usd, fixed_fee_usd, enabled, description, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (fee_type, chain) 
             DO UPDATE SET 
                rate_percentage = EXCLUDED.rate_percentage,
                min_fee_usd = EXCLUDED.min_fee_usd,
                max_fee_usd = EXCLUDED.max_fee_usd,
                fixed_fee_usd = EXCLUDED.fixed_fee_usd,
                enabled = EXCLUDED.enabled,
                description = EXCLUDED.description,
                updated_at = EXCLUDED.updated_at"
        )
        .bind(format!("{:?}", config.fee_type))
        .bind(config.chain.as_deref().unwrap_or("global"))
        .bind(config.rate_percentage)
        .bind(config.min_fee_usd)
        .bind(config.max_fee_usd)
        .bind(config.fixed_fee_usd)
        .bind(config.enabled)
        .bind(&config.description)
        .bind(config.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// 数据库行结构
#[derive(sqlx::FromRow)]
struct FeeConfigRow {
    fee_type: String,
    chain: String,  // 数据库中不再允许 NULL，使用 "global" 表示全局
    rate_percentage: f64,
    min_fee_usd: Option<f64>,
    max_fee_usd: Option<f64>,
    fixed_fee_usd: Option<f64>,
    enabled: bool,
    description: String,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<FeeConfigRow> for FeeConfig {
    fn from(row: FeeConfigRow) -> Self {
        let fee_type = match row.fee_type.as_str() {
            "SwapServiceFee" => FeeType::SwapServiceFee,
            "GasFee" => FeeType::GasFee,
            "BridgeFee" => FeeType::BridgeFee,
            "WithdrawalFee" => FeeType::WithdrawalFee,
            "FiatDepositFee" => FeeType::FiatDepositFee,
            "FiatWithdrawalFee" => FeeType::FiatWithdrawalFee,
            _ => FeeType::SwapServiceFee, // 默认值
        };

        FeeConfig {
            fee_type,
            chain: if row.chain == "global" { None } else { Some(row.chain) },
            rate_percentage: row.rate_percentage,
            min_fee_usd: row.min_fee_usd,
            max_fee_usd: row.max_fee_usd,
            fixed_fee_usd: row.fixed_fee_usd,
            enabled: row.enabled,
            description: row.description,
            updated_at: row.updated_at,
        }
    }
}
