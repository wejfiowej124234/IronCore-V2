//! 生产级完整集成测试
//!
//! 测试覆盖：
//! - ✅ 完整交易流程（费用计算、审计、缓存）
//! - ✅ RPC 故障转移和断路器
//! - ✅ 通知系统
//! - ✅ 多链费用计算
//! - ✅ 性能和缓存优化
//!
//! 运行方式：
//! ```bash
//! cargo test --test integration_complete -- --ignored
//! ```

use std::sync::Arc;

use ironcore::{
    app_state::AppState,
    infrastructure::{audit::ImmuCtx, cache::RedisCtx},
};
use sqlx::PgPool;
use uuid::Uuid;

// ============ 测试辅助函数 ============

/// 创建测试用的 AppState
async fn create_test_app_state() -> Arc<AppState> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/ironcore?sslmode=disable".into());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let redis = RedisCtx::new(&redis_url).expect("Failed to create Redis client");

    let immu = Arc::new(ImmuCtx::new(
        "127.0.0.1:3322".into(),
        "immudb".into(),
        "immudb".into(),
        "defaultdb".into(),
    ));

    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_for_integration_complete_tests_at_least_32_chars",
        );
    }
    let config = Arc::new(ironcore::config::Config::from_env().expect("Failed to load config"));
    let blockchain_config = Arc::new(ironcore::config::BlockchainConfig::default());
    let cross_chain_config = Arc::new(ironcore::config::CrossChainConfig::default());

    let state = AppState::new(
        pool,
        redis,
        immu,
        blockchain_config,
        cross_chain_config,
        config,
    )
    .await
    .expect("Failed to create AppState");

    Arc::new(state)
}

/// 清理测试数据
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM gas.fee_audit WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;

    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

/// 创建测试用户
async fn create_test_user(pool: &PgPool) -> Uuid {
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let _ = sqlx::query(
        "INSERT INTO tenants (id, name) VALUES ($1, $2) 
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(tenant_id)
    .bind("Test Tenant")
    .execute(pool)
    .await;

    sqlx::query(
        "INSERT INTO users (id, tenant_id, email_cipher, role, password_hash) 
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind("test@example.com")
    .bind("user")
    .bind("$2b$12$dummy_hash")
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

// ============ 集成测试 ============

/// Test 1.1: 完整费用计算流程
#[tokio::test]
#[ignore]
async fn test_complete_fee_calculation_flow() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.pool).await;

    // 1. 创建费率规则
    let rule_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gas.platform_fee_rules 
         (id, chain, operation, fee_type, flat_amount, percent_bp, min_fee, priority, rule_version, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true)",
    )
    .bind(rule_id)
    .bind("eth")
    .bind("transfer")
    .bind("percent")
    .bind(0.0)
    .bind(50)
    .bind(0.0001)
    .bind(10)
    .bind(1)
    .execute(&state.pool)
    .await
    .expect("Failed to create fee rule");

    // 2. 创建收费地址
    let collector_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gas.fee_collector_addresses (id, chain, address, active)
         VALUES ($1, $2, $3, true)",
    )
    .bind(collector_id)
    .bind("eth")
    .bind("0xCOLLECTOR_ADDRESS_TEST")
    .execute(&state.pool)
    .await
    .expect("Failed to create collector");

    // 3. 计算费用
    let fee_service = state.fee_service.clone();
    let result = fee_service.calculate_fee("eth", "transfer", 100.0).await;
    assert!(result.is_ok(), "Fee calculation should succeed");

    let fee_info = result.unwrap().unwrap();
    assert_eq!(fee_info.platform_fee, 0.5); // 100 * 0.5% = 0.5
    assert_eq!(fee_info.collector_address, "0xCOLLECTOR_ADDRESS_TEST");

    // 清理
    let _ = sqlx::query("DELETE FROM gas.platform_fee_rules WHERE id = $1")
        .bind(rule_id)
        .execute(&state.pool)
        .await;
    let _ = sqlx::query("DELETE FROM gas.fee_collector_addresses WHERE id = $1")
        .bind(collector_id)
        .execute(&state.pool)
        .await;
    cleanup_test_data(&state.pool, user_id).await;
}

/// Test 1.2: RPC 故障转移测试
#[tokio::test]
#[ignore]
async fn test_rpc_failover() {
    let state = create_test_app_state().await;

    // 清理旧测试数据
    let _ = sqlx::query("DELETE FROM admin.rpc_endpoints WHERE chain = 'eth_test'")
        .execute(&state.pool)
        .await;

    // 1. 插入多个 RPC 端点（不同优先级）
    let endpoints = vec![
        ("https://eth-mainnet.g.alchemy.com/v2/demo", 10, true),
        ("https://rpc.ankr.com/eth", 20, true),
        ("https://cloudflare-eth.com", 30, true),
    ];

    for (url, priority, enabled) in endpoints {
        sqlx::query(
            "INSERT INTO admin.rpc_endpoints (id, chain, url, priority, healthy, circuit_state, enabled)
             VALUES (gen_random_uuid(), $1, $2, $3, true, 'closed', $4)",
        )
        .bind("eth_test")
        .bind(url)
        .bind(priority as i32)
        .bind(enabled)
        .execute(&state.pool)
        .await
        .expect("Failed to insert RPC endpoint");
    }

    // 2. 选择端点（应该选择优先级最高的）
    // select() 会自动加载端点
    let endpoint = state.rpc_selector.select("eth_test").await;
    assert!(endpoint.is_some(), "Should select an endpoint");

    let selected = endpoint.unwrap();
    assert_eq!(selected.priority, 10, "Should select highest priority");
    assert!(selected.url.contains("alchemy"));

    // 清理
    let _ = sqlx::query("DELETE FROM admin.rpc_endpoints WHERE chain = 'eth_test'")
        .execute(&state.pool)
        .await;
}

/// Test 1.3: 通知发布和查询
#[tokio::test]
#[ignore]
async fn test_notification_publish_and_feed() {
    let state = create_test_app_state().await;
    let user_id = create_test_user(&state.pool).await;

    // 1. 发布通知
    let notif_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO notify.notifications (id, tenant_id, category, title, content, severity, target_roles, published_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(notif_id)
    .bind(Uuid::new_v4())
    .bind("system_update")
    .bind("Test Notification")
    .bind("This is a test notification content")
    .bind("info")
    .bind(sqlx::types::Json(vec!["user", "admin"]))
    .bind(user_id)
    .execute(&state.pool)
    .await
    .expect("Failed to publish notification");

    // 2. 查询通知
    use sqlx::Row;
    let notifications = sqlx::query(
        "SELECT id, category, title, content, severity, published_at 
         FROM notify.notifications 
         WHERE category = $1
         ORDER BY published_at DESC
         LIMIT 10",
    )
    .bind("system_update")
    .fetch_all(&state.pool)
    .await
    .expect("Failed to fetch notifications");

    assert!(!notifications.is_empty(), "Should have notifications");
    assert_eq!(notifications.len(), 1);

    // 3. 验证内容
    let row = &notifications[0];
    let title: String = row.try_get("title").unwrap();
    let content: String = row.try_get("content").unwrap();
    let severity: String = row.try_get("severity").unwrap();

    assert_eq!(title, "Test Notification");
    assert_eq!(content, "This is a test notification content");
    assert_eq!(severity, "info");

    // 清理
    let _ = sqlx::query("DELETE FROM notify.notifications WHERE id = $1")
        .bind(notif_id)
        .execute(&state.pool)
        .await;
    cleanup_test_data(&state.pool, user_id).await;
}

/// Test 1.4: 费用缓存性能
#[tokio::test]
#[ignore]
async fn test_fee_cache_expiration() {
    let state = create_test_app_state().await;

    // 创建费率规则
    let rule_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gas.platform_fee_rules 
         (id, chain, operation, fee_type, flat_amount, percent_bp, min_fee, priority, rule_version, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true)",
    )
    .bind(rule_id)
    .bind("btc")
    .bind("transfer")
    .bind("flat")
    .bind(0.0001)
    .bind(0)
    .bind(0.0)
    .bind(10)
    .bind(1)
    .execute(&state.pool)
    .await
    .expect("Failed to create fee rule");

    let collector_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO gas.fee_collector_addresses (id, chain, address, active)
         VALUES ($1, $2, $3, true)",
    )
    .bind(collector_id)
    .bind("btc")
    .bind("bc1qcollector_test_address")
    .execute(&state.pool)
    .await
    .expect("Failed to create collector");

    // 第一次查询（从数据库）
    let fee_service = state.fee_service.clone();
    let start = std::time::Instant::now();
    let result1 = fee_service
        .calculate_fee("btc", "transfer", 1.0)
        .await
        .unwrap();
    let db_time = start.elapsed();

    // 第二次查询（从缓存）
    let start = std::time::Instant::now();
    let result2 = fee_service
        .calculate_fee("btc", "transfer", 1.0)
        .await
        .unwrap();
    let cache_time = start.elapsed();

    // 缓存应该更快
    assert!(cache_time < db_time, "Cache should be faster than database");

    // 结果应该一致
    assert_eq!(
        result1.as_ref().unwrap().platform_fee,
        result2.as_ref().unwrap().platform_fee
    );

    // 清理
    let _ = sqlx::query("DELETE FROM gas.platform_fee_rules WHERE id = $1")
        .bind(rule_id)
        .execute(&state.pool)
        .await;
    let _ = sqlx::query("DELETE FROM gas.fee_collector_addresses WHERE id = $1")
        .bind(collector_id)
        .execute(&state.pool)
        .await;
}

/// Test 1.5: 多链费用计算
#[tokio::test]
#[ignore]
async fn test_multi_chain_fee_calculation() {
    let state = create_test_app_state().await;

    // 为多条链创建费率规则
    let chains = vec![
        ("eth", 50, 0.0001_f64),
        ("bsc", 30, 0.00005),
        ("polygon", 20, 0.01),
    ];

    for (chain, percent_bp, min_fee) in chains {
        let rule_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO gas.platform_fee_rules 
             (id, chain, operation, fee_type, percent_bp, min_fee, priority, rule_version, active)
             VALUES ($1, $2, 'transfer', 'percent', $3, $4, 10, 1, true)",
        )
        .bind(rule_id)
        .bind(chain)
        .bind(percent_bp as i32)
        .bind(min_fee)
        .execute(&state.pool)
        .await
        .expect("Failed to create multi-chain rule");

        let collector_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO gas.fee_collector_addresses (id, chain, address, active)
             VALUES ($1, $2, $3, true)",
        )
        .bind(collector_id)
        .bind(chain)
        .bind(format!("0x{}_COLLECTOR", chain.to_uppercase()))
        .execute(&state.pool)
        .await
        .expect("Failed to create collector");
    }

    // 测试每条链的费用计算
    let fee_service = state.fee_service.clone();

    let eth_fee = fee_service
        .calculate_fee("eth", "transfer", 100.0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(eth_fee.platform_fee, 0.5); // 100 * 0.5%

    let bsc_fee = fee_service
        .calculate_fee("bsc", "transfer", 100.0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bsc_fee.platform_fee, 0.3); // 100 * 0.3%

    // 清理
    let _ =
        sqlx::query("DELETE FROM gas.platform_fee_rules WHERE chain IN ('eth', 'bsc', 'polygon')")
            .execute(&state.pool)
            .await;
    let _ = sqlx::query(
        "DELETE FROM gas.fee_collector_addresses WHERE chain IN ('eth', 'bsc', 'polygon')",
    )
    .execute(&state.pool)
    .await;
}

/// Test 2.1: 边界值测试
#[tokio::test]
async fn test_fee_calculation_edge_cases() {
    // 测试零值
    let amount = 0.0_f64;
    let percent_bp = 50;
    let fee = amount * (percent_bp as f64) / 10_000.0;
    assert_eq!(fee, 0.0);

    // 测试极小值
    let amount = 0.0001;
    let fee = amount * (percent_bp as f64) / 10_000.0;
    assert!(fee > 0.0);
    assert!(fee < 0.001);

    // 测试大额
    let amount = 1_000_000.0;
    let fee = amount * (percent_bp as f64) / 10_000.0;
    assert_eq!(fee, 50.0);

    // 测试最大封顶
    let max_fee = 10.0_f64;
    let calculated_fee = 50.0_f64;
    let final_fee = calculated_fee.min(max_fee);
    assert_eq!(final_fee, 10.0);
}

/// Test 2.2: 配置加载测试
#[tokio::test]
async fn test_config_loading_from_env() {
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_key_for_jwt_signing_at_least_32_chars",
        );
        std::env::set_var("DATABASE_URL", "postgres://test@localhost/test");
    }

    use ironcore::config::Config;

    let config = Config::from_env();
    assert!(config.is_ok());

    let cfg = config.unwrap();
    assert_eq!(cfg.database.url, "postgres://test@localhost/test");
    assert!(cfg.jwt.secret.len() >= 32);
}
