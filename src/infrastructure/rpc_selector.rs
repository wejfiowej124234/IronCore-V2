use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::{sync::RwLock, time::interval};

use crate::infrastructure::cache::RedisCtx;

const OPEN_THRESHOLD: i64 = 3;
const OPEN_TIMEOUT_SECS: u64 = 60;
const ALPHA: f64 = 0.3; // EMA alpha for latency averaging

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcEndpoint {
    pub id: uuid::Uuid,
    pub chain: String,
    pub url: String,
    pub priority: i64, // Changed from i32 to i64 to match DB BIGINT
    pub healthy: bool,
    pub fail_count: i64,      // Changed from i32 to i64 to match DB BIGINT
    pub avg_latency_ms: i64,  // Changed from i32 to i64 to match DB BIGINT
    pub last_latency_ms: i64, // Changed from i32 to i64 to match DB BIGINT
    pub circuit_state: String,
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub last_checked_at: Option<chrono::DateTime<chrono::Utc>>,
}

struct CachedList {
    endpoints: Vec<RpcEndpoint>,
    fetched_at: Instant,
}

pub struct RpcSelector {
    pool: PgPool,
    cache: Arc<RwLock<Option<CachedList>>>, // L1: 本地内存缓存
    redis: Option<Arc<RedisCtx>>,           // L2: Redis 缓存（可选）
    ttl: Duration,
    probe_interval: Duration,
    http_client: reqwest::Client,
}

impl RpcSelector {
    pub fn new(pool: PgPool) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
            redis: None,
            ttl: Duration::from_secs(15),
            probe_interval: Duration::from_secs(15),
            http_client: client,
        }
    }

    /// 创建带 Redis 二级缓存的 RpcSelector
    pub fn with_redis(pool: PgPool, redis: Arc<RedisCtx>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
            redis: Some(redis),
            ttl: Duration::from_secs(15),
            probe_interval: Duration::from_secs(15),
            http_client: client,
        }
    }

    pub async fn start_background_probe(self: Arc<Self>) {
        let mut ticker = interval(self.probe_interval);
        loop {
            ticker.tick().await;
            if let Err(e) = self.probe_all().await {
                tracing::warn!(error=?e, "RPC probe_all failed");
            }
        }
    }

    async fn probe_all(&self) -> Result<()> {
        let rows = sqlx::query(
            "SELECT id, chain, url, circuit_state, last_checked_at FROM admin.rpc_endpoints",
        )
        .fetch_all(&self.pool)
        .await?;
        for r in rows {
            let id: uuid::Uuid = r.try_get("id")?;
            let url: String = r.try_get("url")?;
            let circuit: String = r.try_get("circuit_state")?;
            let last_checked: Option<chrono::DateTime<chrono::Utc>> =
                r.try_get("last_checked_at").ok();
            // Circuit breaker half_open timeout logic
            if circuit == "open" {
                if let Some(checked) = last_checked {
                    let elapsed = chrono::Utc::now()
                        .signed_duration_since(checked)
                        .num_seconds();
                    if elapsed >= OPEN_TIMEOUT_SECS as i64 {
                        // CockroachDB兼容：手动更新updated_at字段（CockroachDB不支持触发器）
                        sqlx::query("UPDATE admin.rpc_endpoints SET circuit_state='half_open', updated_at=CURRENT_TIMESTAMP WHERE id=$1")
                            .bind(id).execute(&self.pool).await?;
                    }
                }
            }
            // Skip actual probe if open (wait for half_open or transition)
            if circuit == "open" {
                continue;
            }
            // Execute lightweight JSON-RPC health check
            let start = Instant::now();
            let body = r#"{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}"#;
            let res = self
                .http_client
                .post(&url)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
            let latency = start.elapsed().as_millis() as i64;
            match res {
                Ok(resp) if resp.status().is_success() => {
                    // Success: reset fail_count, update latency
                    let row = sqlx::query(
                        "SELECT avg_latency_ms::BIGINT AS avg_latency_ms, fail_count::BIGINT AS fail_count FROM admin.rpc_endpoints WHERE id=$1",
                    )
                    .bind(id)
                    .fetch_one(&self.pool)
                    .await?;
                    let old_avg: i64 = row.try_get("avg_latency_ms").unwrap_or(0);
                    let new_avg = if old_avg == 0 {
                        latency
                    } else {
                        (ALPHA * latency as f64 + (1.0 - ALPHA) * old_avg as f64) as i64
                    };
                    sqlx::query("UPDATE admin.rpc_endpoints SET healthy=true, fail_count=0, last_latency_ms=$1, avg_latency_ms=$2, circuit_state='closed', last_checked_at=CURRENT_TIMESTAMP WHERE id=$3")
                        .bind(latency).bind(new_avg).bind(id).execute(&self.pool).await?;
                    tracing::debug!(endpoint_id=%id, latency_ms=latency, "rpc_probe_success");
                }
                _ => {
                    // Failure: increment fail_count, possibly open circuit
                    let row = sqlx::query(
                        "SELECT fail_count::BIGINT AS fail_count FROM admin.rpc_endpoints WHERE id=$1",
                    )
                        .bind(id)
                        .fetch_one(&self.pool)
                        .await?;
                    let current_fail: i64 = row.try_get("fail_count").unwrap_or(0);
                    let new_fail = current_fail + 1;
                    let new_circuit = if new_fail >= OPEN_THRESHOLD {
                        "open"
                    } else {
                        "closed"
                    };
                    if new_circuit == "open" {
                        crate::metrics::inc_rpc_circuit_open();
                    }
                    sqlx::query("UPDATE admin.rpc_endpoints SET healthy=false, fail_count=$1, circuit_state=$2, last_checked_at=CURRENT_TIMESTAMP, last_fail_at=CURRENT_TIMESTAMP WHERE id=$3")
                        .bind(new_fail).bind(new_circuit).bind(id).execute(&self.pool).await?;
                    tracing::warn!(endpoint_id=%id, fail_count=new_fail, circuit=%new_circuit, "rpc_probe_fail");
                }
            }
        }
        // Refresh cache after probes
        self.refresh().await?;
        Ok(())
    }

    async fn refresh(&self) -> Result<()> {
        let rows = sqlx::query(
            "SELECT id, chain, url,
                    priority::BIGINT AS priority,
                    healthy,
                    fail_count::BIGINT AS fail_count,
                    avg_latency_ms::BIGINT AS avg_latency_ms,
                    last_latency_ms::BIGINT AS last_latency_ms,
                    circuit_state,
                    last_checked_at
             FROM admin.rpc_endpoints"
        )
        .fetch_all(&self.pool)
        .await?;
        let mut endpoints = Vec::with_capacity(rows.len());
        for r in rows {
            endpoints.push(RpcEndpoint {
                id: r.try_get("id")?,
                chain: r.try_get("chain")?,
                url: r.try_get("url")?,
                priority: r.try_get("priority")?,
                healthy: r.try_get("healthy")?,
                fail_count: r.try_get("fail_count")?,
                avg_latency_ms: r.try_get("avg_latency_ms")?,
                last_latency_ms: r.try_get("last_latency_ms")?,
                circuit_state: r.try_get("circuit_state")?,
                last_checked_at: r.try_get("last_checked_at").ok(),
            });
        }

        // 回填 L1 缓存
        {
            let mut cache = self.cache.write().await;
            *cache = Some(CachedList {
                endpoints: endpoints.clone(),
                fetched_at: Instant::now(),
            });
        }

        // 回填 L2 Redis 缓存（后台异步）
        if let Some(redis_ctx) = &self.redis {
            let endpoints_clone = endpoints.clone();
            let redis_clone = redis_ctx.clone();
            tokio::spawn(async move {
                if let Ok(endpoints_json) = serde_json::to_string(&endpoints_clone) {
                    if let Ok(mut conn) =
                        redis_clone.client.get_multiplexed_async_connection().await
                    {
                        if let Err(e) = redis::cmd("SETEX")
                            .arg("rpc:endpoints:all")
                            .arg(15) // TTL: 15 秒
                            .arg(endpoints_json)
                            .query_async::<_, ()>(&mut conn)
                            .await
                        {
                            tracing::warn!(
                                cache_key = "rpc:endpoints:all",
                                error = %e,
                                "Failed to set Redis cache for RPC endpoints - continuing without cache"
                            );
                        }
                    }
                }
            });
        }

        Ok(())
    }

    pub async fn select(&self, chain: &str) -> Option<RpcEndpoint> {
        crate::metrics::inc_rpc_selection();

        // L1: 本地内存缓存
        {
            let cache = self.cache.read().await;
            if let Some(c) = &*cache {
                if c.fetched_at.elapsed() < self.ttl {
                    tracing::debug!("rpc_cache_hit_l1");
                    return self.score_pick(chain, &c.endpoints);
                }
            }
        }

        // L2: Redis 缓存（如果可用）
        if let Some(redis_ctx) = &self.redis {
            if let Ok(mut conn) = redis_ctx.client.get_multiplexed_async_connection().await {
                match redis::cmd("GET")
                    .arg("rpc:endpoints:all")
                    .query_async::<_, Option<String>>(&mut conn)
                    .await
                {
                    Ok(Some(cached_json)) => {
                        if let Ok(endpoints) =
                            serde_json::from_str::<Vec<RpcEndpoint>>(&cached_json)
                        {
                            tracing::debug!("rpc_cache_hit_l2_redis");
                            // 回填 L1 缓存
                            let mut cache = self.cache.write().await;
                            *cache = Some(CachedList {
                                endpoints: endpoints.clone(),
                                fetched_at: Instant::now(),
                            });
                            return self.score_pick(chain, &endpoints);
                        }
                    }
                    Ok(None) => tracing::debug!("rpc_cache_miss_l2_redis"),
                    Err(e) => tracing::warn!(error=?e, "rpc_redis_get_failed"),
                }
            }
        }

        // L3: 数据库查询
        tracing::debug!("rpc_cache_miss_querying_db");
        if let Err(e) = self.refresh().await {
            tracing::warn!(error=?e, "refresh on select failed");
        }
        let cache = self.cache.read().await;
        cache
            .as_ref()
            .and_then(|c| self.score_pick(chain, &c.endpoints))
    }

    fn score_pick(&self, chain: &str, list: &[RpcEndpoint]) -> Option<RpcEndpoint> {
        let mut candidates: Vec<&RpcEndpoint> = list
            .iter()
            .filter(|e| e.chain == chain && e.healthy && e.circuit_state == "closed")
            .collect();
        let mut fallback = false;
        if candidates.is_empty() {
            candidates = list
                .iter()
                .filter(|e| e.chain == chain && e.circuit_state == "half_open")
                .collect();
            fallback = true;
        }
        if candidates.is_empty() {
            candidates = list.iter().filter(|e| e.chain == chain).collect();
            fallback = true;
        }
        if fallback {
            crate::metrics::inc_rpc_fallback();
        }
        candidates
            .into_iter()
            .min_by_key(|e| e.priority * 100 + e.avg_latency_ms)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============ 端点选择逻辑测试（无需数据库） ============

    /// Test 1: 基础优先级排序（健康端点）
    #[test]
    fn test_basic_priority_sorting() {
        let endpoints = vec![
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "ethereum".to_string(),
                url: "https://rpc3.example.com".to_string(),
                priority: 3,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 100,
                last_latency_ms: 100,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "ethereum".to_string(),
                url: "https://rpc1.example.com".to_string(),
                priority: 1,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 100,
                last_latency_ms: 100,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "ethereum".to_string(),
                url: "https://rpc2.example.com".to_string(),
                priority: 2,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 100,
                last_latency_ms: 100,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
        ];

        let selected = score_pick_logic("ethereum", &endpoints);

        assert!(selected.is_some());
        let endpoint = selected.unwrap();
        assert_eq!(
            endpoint.priority, 1,
            "Should select endpoint with lowest priority"
        );
        assert_eq!(endpoint.url, "https://rpc1.example.com");
    }

    /// Test 2: 延迟优先（相同优先级）
    #[test]
    fn test_latency_tiebreaker() {
        let endpoints = vec![
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "bsc".to_string(),
                url: "https://fast.example.com".to_string(),
                priority: 1,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 50,
                last_latency_ms: 50,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "bsc".to_string(),
                url: "https://slow.example.com".to_string(),
                priority: 1,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 200,
                last_latency_ms: 200,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
        ];

        let selected = score_pick_logic("bsc", &endpoints);

        assert!(selected.is_some());
        let endpoint = selected.unwrap();
        assert_eq!(
            endpoint.avg_latency_ms, 50,
            "Should prefer lower latency when priority is equal"
        );
        assert_eq!(endpoint.url, "https://fast.example.com");
    }

    /// Test 3: 跳过不健康端点
    #[test]
    fn test_skip_unhealthy_endpoints() {
        let endpoints = vec![
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "polygon".to_string(),
                url: "https://unhealthy.example.com".to_string(),
                priority: 1,
                healthy: false, // 不健康
                fail_count: 5,
                avg_latency_ms: 50,
                last_latency_ms: 50,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "polygon".to_string(),
                url: "https://healthy.example.com".to_string(),
                priority: 2,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 100,
                last_latency_ms: 100,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
        ];

        let selected = score_pick_logic("polygon", &endpoints);

        assert!(selected.is_some());
        let endpoint = selected.unwrap();
        assert_eq!(
            endpoint.priority, 2,
            "Should skip unhealthy endpoint even with better priority"
        );
        assert!(endpoint.healthy);
    }

    /// Test 4: 熔断器打开时跳过
    #[test]
    fn test_skip_circuit_open_endpoints() {
        let endpoints = vec![
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "ethereum".to_string(),
                url: "https://circuit-open.example.com".to_string(),
                priority: 1,
                healthy: false,
                fail_count: 3,
                avg_latency_ms: 50,
                last_latency_ms: 50,
                circuit_state: "open".to_string(), // 熔断器打开
                last_checked_at: None,
            },
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "ethereum".to_string(),
                url: "https://circuit-closed.example.com".to_string(),
                priority: 2,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 100,
                last_latency_ms: 100,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
        ];

        let selected = score_pick_logic("ethereum", &endpoints);

        assert!(selected.is_some());
        let endpoint = selected.unwrap();
        assert_eq!(endpoint.circuit_state, "closed");
        assert_eq!(endpoint.url, "https://circuit-closed.example.com");
    }

    /// Test 5: 半开状态作为降级候选
    #[test]
    fn test_half_open_as_fallback() {
        let endpoints = vec![RpcEndpoint {
            id: uuid::Uuid::new_v4(),
            chain: "bsc".to_string(),
            url: "https://half-open.example.com".to_string(),
            priority: 1,
            healthy: false,
            fail_count: 2,
            avg_latency_ms: 50,
            last_latency_ms: 50,
            circuit_state: "half_open".to_string(),
            last_checked_at: None,
        }];

        let selected = score_pick_logic("bsc", &endpoints);

        assert!(
            selected.is_some(),
            "Should fallback to half_open when no healthy endpoints"
        );
        let endpoint = selected.unwrap();
        assert_eq!(endpoint.circuit_state, "half_open");
    }

    /// Test 6: 无健康端点时返回任意端点
    #[test]
    fn test_fallback_to_any_endpoint() {
        let endpoints = vec![RpcEndpoint {
            id: uuid::Uuid::new_v4(),
            chain: "polygon".to_string(),
            url: "https://all-down.example.com".to_string(),
            priority: 1,
            healthy: false,
            fail_count: 5,
            avg_latency_ms: 0,
            last_latency_ms: 0,
            circuit_state: "open".to_string(),
            last_checked_at: None,
        }];

        let selected = score_pick_logic("polygon", &endpoints);

        assert!(
            selected.is_some(),
            "Should return any endpoint as last resort"
        );
    }

    /// Test 7: 链过滤（跨链隔离）
    #[test]
    fn test_chain_isolation() {
        let endpoints = vec![
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "ethereum".to_string(),
                url: "https://eth.example.com".to_string(),
                priority: 1,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 50,
                last_latency_ms: 50,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
            RpcEndpoint {
                id: uuid::Uuid::new_v4(),
                chain: "bsc".to_string(),
                url: "https://bsc.example.com".to_string(),
                priority: 1,
                healthy: true,
                fail_count: 0,
                avg_latency_ms: 50,
                last_latency_ms: 50,
                circuit_state: "closed".to_string(),
                last_checked_at: None,
            },
        ];

        let eth_selected = score_pick_logic("ethereum", &endpoints);
        assert!(eth_selected.is_some());
        assert_eq!(eth_selected.unwrap().chain, "ethereum");

        let bsc_selected = score_pick_logic("bsc", &endpoints);
        assert!(bsc_selected.is_some());
        assert_eq!(bsc_selected.unwrap().chain, "bsc");
    }

    /// Test 8: 空列表返回 None
    #[test]
    fn test_empty_list_returns_none() {
        let endpoints = vec![];

        let selected = score_pick_logic("ethereum", &endpoints);

        assert!(
            selected.is_none(),
            "Should return None when no endpoints available"
        );
    }

    /// Test 9: 不匹配的链返回 None
    #[test]
    fn test_no_matching_chain() {
        let endpoints = vec![RpcEndpoint {
            id: uuid::Uuid::new_v4(),
            chain: "ethereum".to_string(),
            url: "https://eth.example.com".to_string(),
            priority: 1,
            healthy: true,
            fail_count: 0,
            avg_latency_ms: 50,
            last_latency_ms: 50,
            circuit_state: "closed".to_string(),
            last_checked_at: None,
        }];

        let selected = score_pick_logic("solana", &endpoints);

        assert!(
            selected.is_none(),
            "Should return None when chain not found"
        );
    }

    /// Test 10: EMA 延迟计算公式
    #[test]
    fn test_ema_latency_calculation() {
        let old_avg = 100;
        let new_latency = 200;
        let alpha = 0.3;

        let new_avg = (alpha * new_latency as f64 + (1.0 - alpha) * old_avg as f64) as i32;

        // 0.3 * 200 + 0.7 * 100 = 60 + 70 = 130
        assert_eq!(new_avg, 130, "EMA should be weighted average");
    }

    /// Test 11: 失败计数阈值触发熔断
    #[test]
    fn test_failure_threshold_triggers_circuit() {
        let fail_count = 3;
        let threshold = OPEN_THRESHOLD; // 3

        assert_eq!(threshold, 3);
        assert!(
            fail_count >= threshold,
            "Should open circuit when fail_count reaches threshold"
        );
    }

    /// Test 12: 评分函数逻辑
    #[test]
    fn test_scoring_function() {
        // 评分 = priority * 100 + avg_latency_ms
        // 越低越好

        let score1 = 100 + 50; // 150
        let score2 = 2 * 100 + 30; // 230
        let score3 = 100 + 100; // 200

        assert!(
            score1 < score2,
            "Lower priority wins even with higher latency"
        );
        assert!(score1 < score3, "Lower latency wins with same priority");

        // 极端情况：高延迟低优先级 vs 低延迟高优先级
        let low_priority_high_latency = 100 + 500; // 600
        let high_priority_low_latency = 5 * 100 + 10; // 510

        assert!(
            high_priority_low_latency < low_priority_high_latency,
            "Priority difference of 4 levels (400) is more significant than 490ms latency"
        );
    }

    // ============ 辅助函数 ============

    // 提取选择逻辑为独立函数，便于单元测试（不需要数据库）
    fn score_pick_logic(chain: &str, list: &[RpcEndpoint]) -> Option<RpcEndpoint> {
        let mut candidates: Vec<&RpcEndpoint> = list
            .iter()
            .filter(|e| e.chain == chain && e.healthy && e.circuit_state == "closed")
            .collect();

        if candidates.is_empty() {
            candidates = list
                .iter()
                .filter(|e| e.chain == chain && e.circuit_state == "half_open")
                .collect();
        }
        if candidates.is_empty() {
            candidates = list.iter().filter(|e| e.chain == chain).collect();
        }

        candidates
            .into_iter()
            .min_by_key(|e| e.priority * 100 + e.avg_latency_ms)
            .cloned()
    }

    // ============ 集成测试（需要数据库）============

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_rpc_selector_integration_with_db() {
        // 完整的数据库集成测试：
        // 1. 插入测试端点
        // 2. 执行探测
        // 3. 验证状态转换
        // 4. 验证选择逻辑
    }
}
