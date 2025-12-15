//! 健康检查模块
//! 提供增强的健康检查功能，包括数据库、Redis、immudb等

use crate::infrastructure::audit::ImmuCtx;
use crate::infrastructure::cache::RedisCtx;
use crate::infrastructure::db::PgPool;
use serde::Serialize;

/// 健康检查结果
#[derive(Debug, Serialize)]
pub struct HealthCheckResult {
    pub status: String, // "healthy" | "degraded" | "unhealthy"
    pub components: ComponentHealth,
    pub timestamp: String,
}

/// 组件健康状态
#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub database: ComponentStatus,
    pub redis: ComponentStatus,
    pub immudb: ComponentStatus,
}

/// 组件状态
#[derive(Debug, Serialize)]
pub struct ComponentStatus {
    pub status: String, // "ok" | "error"
    pub message: String,
    pub latency_ms: Option<u64>,
}

/// 执行完整健康检查
pub async fn check_health(pool: &PgPool, redis: &RedisCtx, immu: &ImmuCtx) -> HealthCheckResult {
    // 并行检查所有组件
    let (db_status, redis_status, immu_status) =
        tokio::join!(check_database(pool), check_redis(redis), check_immudb(immu),);

    let overall_status = determine_overall_status(&db_status, &redis_status, &immu_status);

    HealthCheckResult {
        status: overall_status,
        components: ComponentHealth {
            database: db_status,
            redis: redis_status,
            immudb: immu_status,
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// 检查数据库健康状态
async fn check_database(pool: &PgPool) -> ComponentStatus {
    let start = std::time::Instant::now();

    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as u64;
            ComponentStatus {
                status: "ok".to_string(),
                message: "Database connection healthy".to_string(),
                latency_ms: Some(latency),
            }
        }
        Err(e) => ComponentStatus {
            status: "error".to_string(),
            message: format!("Database error: {}", e),
            latency_ms: None,
        },
    }
}

/// 检查Redis健康状态
async fn check_redis(redis: &RedisCtx) -> ComponentStatus {
    let start = std::time::Instant::now();

    match redis.ping().await {
        Ok(_) => {
            let latency = start.elapsed().as_millis() as u64;
            ComponentStatus {
                status: "ok".to_string(),
                message: "Redis connection healthy".to_string(),
                latency_ms: Some(latency),
            }
        }
        Err(e) => ComponentStatus {
            status: "error".to_string(),
            message: format!("Redis error: {}", e),
            latency_ms: None,
        },
    }
}

/// 检查immudb健康状态
async fn check_immudb(immu: &ImmuCtx) -> ComponentStatus {
    let start = std::time::Instant::now();

    // 尝试调用immudb的健康检查API
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build();

    match client {
        Ok(client) => {
            // 尝试写入一个测试键来验证immudb可用性
            let test_key = format!("health_check:{}", chrono::Utc::now().timestamp());
            let test_value = "health_check";

            let url = format!("http://{}/v1/immudb/set", immu.addr);
            let body = serde_json::json!({
                "key": test_key,
                "value": test_value
            });

            match client
                .post(&url)
                .basic_auth(&immu.user, Some(&immu.pass))
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let latency = start.elapsed().as_millis() as u64;
                    if resp.status().is_success() {
                        ComponentStatus {
                            status: "ok".to_string(),
                            message: "immudb connection healthy".to_string(),
                            latency_ms: Some(latency),
                        }
                    } else {
                        ComponentStatus {
                            status: "error".to_string(),
                            message: format!("immudb returned status: {}", resp.status()),
                            latency_ms: Some(latency),
                        }
                    }
                }
                Err(e) => {
                    let latency = start.elapsed().as_millis() as u64;
                    ComponentStatus {
                        status: "error".to_string(),
                        message: format!("immudb connection error: {}", e),
                        latency_ms: Some(latency),
                    }
                }
            }
        }
        Err(e) => {
            let latency = start.elapsed().as_millis() as u64;
            ComponentStatus {
                status: "error".to_string(),
                message: format!("Failed to create HTTP client: {}", e),
                latency_ms: Some(latency),
            }
        }
    }
}

/// 确定整体健康状态
fn determine_overall_status(
    db: &ComponentStatus,
    redis: &ComponentStatus,
    immu: &ComponentStatus,
) -> String {
    let error_count = [db, redis, immu]
        .iter()
        .filter(|c| c.status == "error")
        .count();

    match error_count {
        0 => "healthy".to_string(),
        1 => "degraded".to_string(),  // 一个组件失败，服务降级
        _ => "unhealthy".to_string(), // 多个组件失败，服务不健康
    }
}

/// 快速健康检查（仅检查关键组件）
pub async fn quick_health_check(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}
