//! 测试辅助模块
//! 提供测试工具和辅助函数

use ironcore::app_state::AppState;
use ironcore::infrastructure::db::PgPool;
use ironcore::infrastructure::cache::RedisCtx;
use ironcore::infrastructure::audit::ImmuCtx;
use std::sync::Arc;

/// 测试数据库URL
pub fn test_database_url() -> String {
    std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/test?sslmode=disable".into())
}

/// 测试Redis URL
pub fn test_redis_url() -> String {
    std::env::var("TEST_REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".into())
}

/// 创建测试数据库连接池
pub async fn create_test_pool() -> PgPool {
    let url = test_database_url();
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("Failed to create test database pool")
}

/// 创建测试应用状态
pub async fn create_test_app_state() -> Arc<AppState> {
    let pool = create_test_pool().await;
    
    let redis_url = test_redis_url();
    let redis = Arc::new(RedisCtx::new(&redis_url).expect("Failed to create Redis client"));

    let immu = Arc::new(ImmuCtx::new(
        "127.0.0.1:3322".into(),
        "immudb".into(),
        "immudb".into(),
        "defaultdb".into(),
    ));

    Arc::new(AppState::new(pool, (*redis).clone(), immu))
}

/// 清理测试数据
pub async fn cleanup_test_data(pool: &PgPool) {
    // 清理测试数据的SQL
    // 注意：实际应该根据测试需求实现
}

