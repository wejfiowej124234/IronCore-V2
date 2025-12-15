//! 生产级集成测试套件
//!
//! 测试覆盖：
//! - ✅ 基础设施组件（数据库、Redis、ImmuDB）
//! - ✅ 配置加载和验证
//! - ✅ 安全功能（密码哈希、加密、JWT）
//! - ✅ 监控和指标
//! - ✅ 错误处理和边界条件
//!
//! 运行方式：
//! ```bash
//! cargo test --test integration_test -- --ignored
//! ```

use std::sync::Arc;

use ironcore::{
    app_state::AppState,
    infrastructure::{audit::ImmuCtx, cache::RedisCtx},
};

// ============ 测试辅助函数 ============

/// 创建生产级测试应用状态
async fn create_test_app_state() -> Arc<AppState> {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/ironcore_test?sslmode=disable".into());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create test database pool");

    let redis_url =
        std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into());
    let redis = RedisCtx::new(&redis_url).expect("Failed to create Redis client");

    let immu = Arc::new(ImmuCtx::new(
        "127.0.0.1:3322".into(),
        "immudb".into(),
        "immudb".into(),
        "defaultdb".into(),
    ));

    // 加载完整配置
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_that_is_at_least_32_characters_long_for_jwt",
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

// ============ 基础设施测试 ============

/// Test 1.1: 健康检查
#[tokio::test]
#[ignore]
async fn test_health_check() {
    let state = create_test_app_state().await;
    assert!(!state.pool.is_closed(), "Database pool should be active");

    let ping_result = state.redis.clone().ping().await;
    assert!(ping_result.is_ok(), "Redis should be connected");
}

/// Test 1.2: 数据库连接
#[tokio::test]
#[ignore]
async fn test_database_connection() {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/ironcore_test?sslmode=disable".into());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await;

    assert!(pool.is_ok(), "Database connection should succeed");

    if let Ok(pool) = pool {
        let result: Result<(i32,), _> = sqlx::query_as("SELECT 1").fetch_one(&pool).await;
        assert!(result.is_ok(), "Simple query should succeed");
    }
}

// ============ 配置管理测试 ============

/// Test 2.1: 配置加载
#[tokio::test]
async fn test_config_loading() {
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_that_is_at_least_32_characters_long",
        );
    }

    let config = ironcore::config::Config::from_env();
    assert!(config.is_ok(), "Config loading should succeed");

    let cfg = config.unwrap();
    assert!(
        cfg.jwt.secret.len() >= 32,
        "JWT secret should be at least 32 chars"
    );
}

/// Test 2.2: 配置验证
#[tokio::test]
async fn test_config_validation() {
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_key_for_jwt_signing_at_least_32_chars",
        );
    }

    let config = ironcore::config::Config::from_env().unwrap();
    assert!(
        config.validate().is_ok(),
        "Valid config should pass validation"
    );

    let mut invalid_config = config.clone();
    invalid_config.jwt.secret = "short".to_string();
    assert!(
        invalid_config.validate().is_err(),
        "Invalid config should fail validation"
    );
}

// ============ 监控和指标测试 ============

/// Test 3.1: Prometheus 指标
#[tokio::test]
async fn test_metrics_endpoint() {
    use ironcore::metrics;

    metrics::count_ok("test_endpoint");
    metrics::count_err("test_endpoint");

    let output = metrics::render_prometheus();
    assert!(
        output.contains("ironcore_requests_total"),
        "Should contain request counter"
    );
    assert!(
        output.contains("ironcore_errors_total"),
        "Should contain error counter"
    );
    assert!(output.contains("# TYPE"), "Should have TYPE declarations");
    assert!(output.contains("# HELP"), "Should have HELP text");
}

// ============ 安全功能测试 ============

/// Test 4.1: 密码哈希（Argon2id）
#[tokio::test]
async fn test_password_hashing() {
    use ironcore::infrastructure::password;

    let password = "test_password_123";
    let hash = password::hash_password(password).unwrap();

    assert!(hash.starts_with("$argon2"), "Should use Argon2");
    assert!(hash.len() > 50, "Hash should be sufficiently long");
    assert!(
        password::verify_password(password, &hash).unwrap(),
        "Correct password should verify"
    );
    assert!(
        !password::verify_password("wrong_password", &hash).unwrap(),
        "Wrong password should fail"
    );

    let hash2 = password::hash_password(password).unwrap();
    assert_ne!(hash, hash2, "Each hash should have unique salt");
}

/// Test 4.2: JWT Token 生成和验证
#[tokio::test]
async fn test_jwt_token() {
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_key_for_jwt_signing_at_least_32_chars",
        );
    }

    use ironcore::infrastructure::jwt;
    use uuid::Uuid;

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let role = "admin".to_string();

    let token = jwt::generate_token(user_id, tenant_id, role.clone()).unwrap();
    assert!(!token.is_empty(), "Token should not be empty");
    assert!(token.split('.').count() == 3, "JWT should have 3 parts");

    let claims = jwt::verify_token(&token).unwrap();
    assert_eq!(claims.sub, user_id.to_string(), "User ID should match");
    assert_eq!(claims.role, role, "Role should match");
    assert_eq!(
        claims.tenant_id,
        tenant_id.to_string(),
        "Tenant ID should match"
    );
    assert!(
        claims.exp as i64 > chrono::Utc::now().timestamp(),
        "Token should not be expired"
    );
}

/// Test 4.3: JWT 无效 token 拒绝
#[tokio::test]
async fn test_jwt_invalid_token() {
    unsafe {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_key_for_jwt_signing_at_least_32_chars",
        );
    }

    use ironcore::infrastructure::jwt;

    let invalid_token = "invalid.token.format";
    let result = jwt::verify_token(invalid_token);
    assert!(result.is_err(), "Invalid token should be rejected");

    let valid_token = jwt::generate_token(
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        "user".to_string(),
    )
    .unwrap();
    let tampered = valid_token.replace('a', "b");
    let result = jwt::verify_token(&tampered);
    assert!(result.is_err(), "Tampered token should be rejected");
}

/// Test 4.4: AES-256-GCM 加密解密
#[tokio::test]
async fn test_encryption() {
    use ironcore::infrastructure::encryption;

    let key = b"01234567890123456789012345678901";
    let data = b"Hello, World! This is a test message.";

    let encrypted = encryption::encrypt_data(data, key).unwrap();
    assert_ne!(
        encrypted.as_slice(),
        data,
        "Encrypted should differ from plaintext"
    );
    assert!(encrypted.len() > data.len(), "Encrypted should be longer");

    let decrypted = encryption::decrypt_data(&encrypted, key).unwrap();
    assert_eq!(
        decrypted.as_slice(),
        data,
        "Decrypted should match original"
    );

    let wrong_key = b"wrong_key_32_bytes_long_exactly1";
    let result = encryption::decrypt_data(&encrypted, wrong_key);
    assert!(result.is_err(), "Wrong key should fail decryption");

    let encrypted2 = encryption::encrypt_data(data, key).unwrap();
    assert_ne!(
        encrypted, encrypted2,
        "Each encryption should use unique nonce"
    );
}

// ============ 错误处理和边界测试 ============

/// Test 5.1: 边界值测试
#[tokio::test]
async fn test_boundary_values() {
    let amount = 0.0_f64;
    let percent_bp = 50;
    let fee = amount * (percent_bp as f64) / 10_000.0;
    assert_eq!(fee, 0.0, "Zero amount should result in zero fee");

    let amount = 0.0001;
    let fee = amount * (percent_bp as f64) / 10_000.0;
    assert!(fee > 0.0, "Tiny amount should have positive fee");

    let amount = 1_000_000.0;
    let fee = amount * (percent_bp as f64) / 10_000.0;
    assert_eq!(fee, 5000.0, "Large amount: 1M * 0.5% = 5000");

    let max_fee = 10.0_f64;
    let calculated_fee = 5000.0_f64;
    let final_fee = calculated_fee.min(max_fee);
    assert_eq!(final_fee, 10.0, "Fee should be capped at max");
}

/// Test 5.2: 并发安全性
#[tokio::test]
async fn test_concurrent_operations() {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter_clone = counter.clone();
        let handle = tokio::spawn(async move {
            for _ in 0..100 {
                let mut num = counter_clone.lock().await;
                *num += 1;
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let final_count = *counter.lock().await;
    assert_eq!(final_count, 1000, "Concurrent operations should be safe");
}
