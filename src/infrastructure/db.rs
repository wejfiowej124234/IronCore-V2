//! SQLx Postgres(CockroachDB) 连接池初始化与健康检查
//!
//! CockroachDB兼容性说明：
//! - 使用PostgreSQL协议，完全兼容sqlx
//! - 推荐连接池大小：生产环境 50-100，开发环境 16-32
//! - 注意：CockroachDB v23.2不支持触发器，需要在应用层更新updated_at
//! - 注意：CockroachDB不支持advisory locks，迁移可能需要手动执行
//!
//! 用法：
//! let pool = init_pool(&env::var("DATABASE_URL")?).await?;
//! health_check(&pool).await?;

use std::{env, time::Duration};

use anyhow::Result;

pub type PgPool = sqlx::Pool<sqlx::Postgres>;

/// 初始化CockroachDB连接池✅生产优化
///
/// # CockroachDB优化建议
/// - max_connections: 50-100(生产) 16-32(开发)
/// - min_connections: 5-10个预热连接
/// - idle_timeout: 300秒，避免连接泄漏
/// - max_lifetime: 1800秒，定期刷新
pub async fn init_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let max_conns = env::var("DB_MAX_CONNS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&n| n > 0 && n <= 200)
        .unwrap_or(16);
    let min_conns = env::var("DB_MIN_CONNS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&n| n > 0 && n <= max_conns)
        .unwrap_or(2);
    let acquire_secs = env::var("DB_ACQ_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    let idle_secs = env::var("DB_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300);
    let max_lifetime_secs = env::var("DB_MAX_LIFETIME_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1800);

    // CockroachDB优化：设置连接池参数
    // 注意：CockroachDB推荐使用较大的连接池（50-100生产环境）
    // 但需要根据实际负载调整，避免过度连接导致资源浪费
    let pool_opts = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_conns)
        .min_connections(min_conns)
        .acquire_timeout(Duration::from_secs(acquire_secs))
        .idle_timeout(Duration::from_secs(idle_secs))
        .max_lifetime(Duration::from_secs(max_lifetime_secs))
        // CockroachDB兼容：设置测试连接的超时时间
        // 确保连接在使用前是有效的，避免使用已断开的连接
        .test_before_acquire(true);

    let pool = pool_opts.connect(database_url).await.map_err(|e| {
        tracing::error!("Failed to connect to CockroachDB: {}", e);
        e
    })?;

    // 验证连接
    health_check(&pool).await?;

    Ok(pool)
}

/// 当 allow_lazy=true 时，使用 lazy 连接（不在启动时触发实际连接），便于无依赖环境联调
pub async fn init_pool_maybe_lazy(
    database_url: &str,
    allow_lazy: bool,
) -> Result<PgPool, sqlx::Error> {
    let max_conns = env::var("DB_MAX_CONNS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(16);
    let min_conns = env::var("DB_MIN_CONNS")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(2);
    let acquire_secs = env::var("DB_ACQ_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(5);
    let idle_secs = env::var("DB_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300);

    let max_lifetime_secs = env::var("DB_MAX_LIFETIME_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1800);

    // CockroachDB优化：设置连接池参数
    // 注意：CockroachDB推荐使用较大的连接池（50-100生产环境）
    // 但需要根据实际负载调整，避免过度连接导致资源浪费
    let opts = sqlx::postgres::PgPoolOptions::new()
        .max_connections(max_conns)
        .min_connections(min_conns)
        .acquire_timeout(Duration::from_secs(acquire_secs))
        .idle_timeout(Duration::from_secs(idle_secs))
        .max_lifetime(Duration::from_secs(max_lifetime_secs))
        // CockroachDB兼容：设置测试连接的超时时间
        // 确保连接在使用前是有效的，避免使用已断开的连接
        .test_before_acquire(true);

    if allow_lazy {
        // lazy 不需要 await，但会在首次使用时验证连接
        let pool = opts.connect_lazy(database_url)?;
        Ok(pool)
    } else {
        let pool = opts.connect(database_url).await.map_err(|e| {
            tracing::error!("Failed to connect to CockroachDB: {}", e);
            e
        })?;
        // 验证连接
        health_check(&pool).await?;
        Ok(pool)
    }
}

/// 确保数据库存在（如果不存在则创建）
///
/// 从数据库URL中提取数据库名，连接到默认数据库（defaultdb），然后创建目标数据库
pub async fn ensure_database_exists(database_url: &str) -> Result<()> {
    // 从URL中提取数据库名
    // 格式: postgresql://user@host:port/dbname?params
    // 或: postgresql://user@host:port/dbname
    let db_name = if let Some(db_part) = database_url.split('/').nth(3) {
        let name = db_part.split('?').next().unwrap_or(db_part).trim();
        if name.is_empty() || name.contains('@') {
            None
        } else {
            Some(name)
        }
    } else {
        None
    };

    let db_name = match db_name {
        Some(name) if name != "defaultdb" && name != "postgres" => name,
        _ => {
            tracing::debug!("Using default database, no need to create");
            return Ok(()); // 使用默认数据库，无需创建
        }
    };

    // 构建连接到默认数据库的URL
    let default_url = if database_url.contains(&format!("/{}", db_name)) {
        database_url.replace(&format!("/{}", db_name), "/defaultdb")
    } else if database_url.ends_with(db_name) {
        database_url.replace(db_name, "defaultdb")
    } else {
        // 如果无法替换，尝试构建新URL
        let base = database_url
            .split('/')
            .take(3)
            .collect::<Vec<_>>()
            .join("/");
        format!("{}/defaultdb", base)
    };

    tracing::debug!("Ensuring database '{}' exists...", db_name);

    // 连接到默认数据库
    let default_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&default_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to connect to default database: {}", e))?;

    // 检查数据库是否存在
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(db_name)
            .fetch_one(&default_pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to check database existence: {}", e))?;

    if !exists {
        tracing::info!("Database '{}' does not exist, creating...", db_name);
        // 创建数据库（CockroachDB 不支持 IF NOT EXISTS，所以先检查）
        // 注意：数据库名来自URL，相对安全，但仍需注意SQL注入
        let create_query = format!("CREATE DATABASE \"{}\"", db_name.replace('"', "\"\""));
        sqlx::query(&create_query)
            .execute(&default_pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create database '{}': {}", db_name, e))?;
        tracing::info!("✅ Database '{}' created successfully", db_name);
    } else {
        tracing::debug!("Database '{}' already exists", db_name);
    }

    default_pool.close().await;
    Ok(())
}

/// CockroachDB健康检查
///
/// 使用简单的SELECT CURRENT_TIMESTAMP查询验证连接和数据库响应
/// CockroachDB完全兼容PostgreSQL协议，使用标准SQL函数
pub async fn health_check(pool: &PgPool) -> Result<(), sqlx::Error> {
    // CockroachDB兼容：使用CURRENT_TIMESTAMP替代now()，更标准化
    let _: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as("SELECT CURRENT_TIMESTAMP")
        .fetch_one(pool)
        .await?;
    Ok(())
}
