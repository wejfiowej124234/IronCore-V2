//! 服务商管理服务
//! 企业级实现，真实管理第三方服务商配置和健康检查
use std::{collections::HashMap, sync::Arc};

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 服务商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub provider_type: String, // "aggregator" (聚合器) or "direct" (直连)
    pub is_enabled: bool,
    pub priority: i64, // CockroachDB uses INT8 by default
    pub fee_min_percent: rust_decimal::Decimal,
    pub fee_max_percent: rust_decimal::Decimal,
    pub api_url: String,
    pub webhook_url: Option<String>,
    pub timeout_seconds: i64,
    pub supported_countries: Vec<String>,
    pub supported_payment_methods: Vec<String>,
    pub health_status: String,
    pub last_health_check: Option<DateTime<Utc>>,
    pub consecutive_failures: i64,
    pub total_requests: i64,
    pub successful_requests: i64,
    pub average_response_time_ms: Option<i64>,
}

/// 服务商健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

impl From<String> for HealthStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "healthy" => HealthStatus::Healthy,
            "unhealthy" => HealthStatus::Unhealthy,
            _ => HealthStatus::Unknown,
        }
    }
}

impl From<HealthStatus> for String {
    fn from(status: HealthStatus) -> Self {
        match status {
            HealthStatus::Healthy => "healthy".to_string(),
            HealthStatus::Unhealthy => "unhealthy".to_string(),
            HealthStatus::Unknown => "unknown".to_string(),
        }
    }
}

/// 服务商统计信息
#[derive(Debug, Clone)]
pub struct ProviderStats {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub average_response_time_ms: Option<i64>,
    pub success_rate: f64,
}

impl ProviderStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }
}

pub struct ProviderService {
    pool: PgPool,
    cache: Arc<RwLock<HashMap<String, ProviderConfig>>>,
}

impl ProviderService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 获取所有启用的服务商
    pub async fn get_enabled_providers(&self) -> Result<Vec<ProviderConfig>> {
        tracing::info!("[ProviderService] Fetching enabled providers from database...");

        let rows = sqlx::query(
            r#"
            SELECT
                id, name, display_name, is_enabled, priority,
                fee_min_percent, fee_max_percent, api_url, webhook_url,
                timeout_seconds, supported_countries, supported_payment_methods,
                health_status, last_health_check, consecutive_failures,
                total_requests, successful_requests, average_response_time_ms
            FROM fiat.providers
            WHERE is_enabled = TRUE
            ORDER BY priority ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch enabled providers")?;

        tracing::info!(
            "[ProviderService] Fetched {} rows from database",
            rows.len()
        );

        let mut providers = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            tracing::debug!("[ProviderService] Parsing row {}/{}", idx + 1, rows.len());

            // 尝试多种方式解析TEXT[]数组：Vec<String> 或 Vec<Option<String>>
            let supported_countries: Vec<String> = row
                .try_get::<Vec<String>, _>("supported_countries")
                .or_else(|_| {
                    row.try_get::<Vec<Option<String>>, _>("supported_countries")
                        .map(|v| v.into_iter().flatten().collect())
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("[ProviderService] Failed to parse supported_countries for row {}, using empty vec: {:?}", idx, e);
                    Vec::new()
                });

            let supported_payment_methods: Vec<String> = row
                .try_get::<Vec<String>, _>("supported_payment_methods")
                .or_else(|_| {
                    row.try_get::<Vec<Option<String>>, _>("supported_payment_methods")
                        .map(|v| v.into_iter().flatten().collect())
                })
                .unwrap_or_else(|e| {
                    tracing::warn!("[ProviderService] Failed to parse supported_payment_methods for row {}, using empty vec: {:?}", idx, e);
                    Vec::new()
                });

            // 逐字段解析，捕获具体错误
            let provider = match (|| -> Result<ProviderConfig> {
                Ok(ProviderConfig {
                    id: row.try_get("id").context("Failed to parse id")?,
                    name: row.try_get("name").context("Failed to parse name")?,
                    display_name: row
                        .try_get("display_name")
                        .context("Failed to parse display_name")?,
                    provider_type: row
                        .try_get("provider_type")
                        .unwrap_or_else(|_| "direct".to_string()),
                    is_enabled: row
                        .try_get("is_enabled")
                        .context("Failed to parse is_enabled")?,
                    priority: row
                        .try_get("priority")
                        .context("Failed to parse priority")?,
                    fee_min_percent: row
                        .try_get("fee_min_percent")
                        .context("Failed to parse fee_min_percent")?,
                    fee_max_percent: row
                        .try_get("fee_max_percent")
                        .context("Failed to parse fee_max_percent")?,
                    api_url: row.try_get("api_url").context("Failed to parse api_url")?,
                    webhook_url: row
                        .try_get("webhook_url")
                        .context("Failed to parse webhook_url")?,
                    timeout_seconds: row
                        .try_get("timeout_seconds")
                        .context("Failed to parse timeout_seconds")?,
                    supported_countries: supported_countries.clone(),
                    supported_payment_methods: supported_payment_methods.clone(),
                    health_status: row
                        .try_get("health_status")
                        .context("Failed to parse health_status")?,
                    last_health_check: row
                        .try_get("last_health_check")
                        .context("Failed to parse last_health_check")?,
                    consecutive_failures: row
                        .try_get("consecutive_failures")
                        .context("Failed to parse consecutive_failures")?,
                    total_requests: row
                        .try_get("total_requests")
                        .context("Failed to parse total_requests")?,
                    successful_requests: row
                        .try_get("successful_requests")
                        .context("Failed to parse successful_requests")?,
                    average_response_time_ms: row
                        .try_get("average_response_time_ms")
                        .context("Failed to parse average_response_time_ms")?,
                })
            })() {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        "[ProviderService] Failed to parse provider row {}: {:?}",
                        idx,
                        e
                    );
                    continue; // 跳过这个provider，继续处理下一个
                }
            };

            providers.push(provider);
        }

        Ok(providers)
    }

    /// 检查服务商是否支持指定国家
    pub async fn check_country_support(
        &self,
        provider_name: &str,
        country_code: &str,
    ) -> Result<bool> {
        let row = sqlx::query(
            r#"
            SELECT
                CASE
                    WHEN EXISTS (
                        SELECT 1 FROM fiat.provider_country_support pcs
                        JOIN fiat.providers p ON p.id = pcs.provider_id
                        WHERE p.name = $1 AND pcs.country_code = $2 AND pcs.is_supported = TRUE
                    ) THEN TRUE
                    WHEN EXISTS (
                        SELECT 1 FROM fiat.providers
                        WHERE name = $1 AND $2 = ANY(supported_countries)
                    ) THEN TRUE
                    ELSE FALSE
                END as is_supported
            "#,
        )
        .bind(provider_name)
        .bind(country_code)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to check country support")?;

        Ok(row
            .and_then(|r| r.try_get::<bool, _>("is_supported").ok())
            .unwrap_or(false))
    }

    /// 获取服务商支持的国家列表
    pub async fn get_supported_countries(&self, provider_name: &str) -> Result<Vec<String>> {
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(
                    ARRAY_AGG(DISTINCT pcs.country_code) FILTER (WHERE pcs.is_supported = TRUE),
                    ARRAY[]::TEXT[]
                ) as countries
            FROM fiat.providers p
            LEFT JOIN fiat.provider_country_support pcs ON p.id = pcs.provider_id
            WHERE p.name = $1 AND p.is_enabled = TRUE
            "#,
        )
        .bind(provider_name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch supported countries")?;

        let countries: Vec<String> = row
            .and_then(|r| {
                r.try_get::<Vec<Option<String>>, _>("countries")
                    .ok()
                    .map(|v| v.into_iter().flatten().collect())
            })
            .unwrap_or_default();

        Ok(countries)
    }

    /// 更新服务商健康状态
    pub async fn update_health_status(
        &self,
        provider_name: &str,
        status: HealthStatus,
        response_time_ms: Option<i32>,
    ) -> Result<()> {
        let now = Utc::now();
        let status_str: String = status.into();

        sqlx::query(
            r#"
            UPDATE fiat.providers
            SET
                health_status = $1,
                last_health_check = $2,
                consecutive_failures = CASE
                    WHEN $1 = 'healthy' THEN 0
                    ELSE consecutive_failures + 1
                END,
                average_response_time_ms = CASE
                    WHEN $3 IS NOT NULL THEN COALESCE(
                        (average_response_time_ms + $3) / 2,
                        $3
                    )
                    ELSE average_response_time_ms
                END,
                updated_at = $2
            WHERE name = $4
            "#,
        )
        .bind(&status_str)
        .bind(now)
        .bind(response_time_ms)
        .bind(provider_name)
        .execute(&self.pool)
        .await
        .context("Failed to update provider health status")?;

        // 清除缓存
        let mut cache = self.cache.write().await;
        cache.remove(provider_name);

        Ok(())
    }

    /// 更新服务商统计信息
    pub async fn update_stats(
        &self,
        provider_name: &str,
        success: bool,
        response_time_ms: Option<i32>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE fiat.providers
            SET
                total_requests = total_requests + 1,
                successful_requests = successful_requests + CASE WHEN $1 THEN 1 ELSE 0 END,
                average_response_time_ms = CASE
                    WHEN $2 IS NOT NULL THEN COALESCE(
                        (average_response_time_ms + $2) / 2,
                        $2
                    )
                    ELSE average_response_time_ms
                END,
                updated_at = CURRENT_TIMESTAMP
            WHERE name = $3
            "#,
        )
        .bind(success)
        .bind(response_time_ms)
        .bind(provider_name)
        .execute(&self.pool)
        .await
        .context("Failed to update provider stats")?;

        Ok(())
    }

    /// 健康检查（ping服务商）
    pub async fn health_check(&self, provider: &ProviderConfig) -> Result<(bool, Option<i32>)> {
        let start = std::time::Instant::now();

        // 尝试ping服务商的健康检查端点
        let health_url = format!("{}/health", provider.api_url.trim_end_matches('/'));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .context("Failed to create HTTP client")?;

        match client.get(&health_url).send().await {
            Ok(resp) => {
                let elapsed_ms = start.elapsed().as_millis() as i32;
                let is_healthy = resp.status().is_success();
                Ok((is_healthy, Some(elapsed_ms)))
            }
            Err(_) => {
                let elapsed_ms = start.elapsed().as_millis() as i32;
                Ok((false, Some(elapsed_ms)))
            }
        }
    }

    /// 定期健康检查所有服务商
    pub async fn check_all_providers_health(&self) -> Result<()> {
        let providers = self.get_enabled_providers().await?;

        for provider in providers {
            let (is_healthy, response_time) =
                self.health_check(&provider).await.unwrap_or((false, None));

            let status = if is_healthy {
                HealthStatus::Healthy
            } else {
                HealthStatus::Unhealthy
            };

            self.update_health_status(&provider.name, status, response_time)
                .await?;
        }

        Ok(())
    }

    /// 根据名称获取单个服务商配置
    pub async fn get_provider_by_name(&self, name: &str) -> Result<ProviderConfig> {
        // 1. 尝试从缓存获取
        {
            let cache = self.cache.read().await;
            if let Some(provider) = cache.get(name) {
                return Ok(provider.clone());
            }
        }

        // 2. 从数据库查询
        let providers = self.get_enabled_providers().await?;
        let provider = providers
            .into_iter()
            .find(|p| p.name == name)
            .ok_or_else(|| anyhow!("Provider not found or disabled: {}", name))?;

        // 3. 更新缓存
        {
            let mut cache = self.cache.write().await;
            cache.insert(name.to_string(), provider.clone());
        }

        Ok(provider)
    }
}
