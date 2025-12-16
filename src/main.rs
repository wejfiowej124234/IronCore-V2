//! IronCore 主入口
//! 企业级多链钱包系统后端

use std::sync::Arc;

use anyhow::Result;
use ironcore::{api, app_state::AppState, config::BlockchainConfig, infrastructure::db::PgPool};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // ✅ 1. 加载环境变量
    dotenvy::dotenv().ok();

    // ✅ 1.5 加载配置文件并设置环境变量（如果存在CONFIG_PATH）
    let loaded_config = if let Ok(config_path) = std::env::var("CONFIG_PATH") {
        match ironcore::config::Config::from_env_and_file(Some(config_path.as_str())) {
            Ok(config) => {
                // 将配置中的JWT secret设置到环境变量，确保JWT模块能找到它
                if std::env::var("JWT_SECRET").is_err() {
                    std::env::set_var("JWT_SECRET", &config.jwt.secret);
                    tracing::info!("✅ JWT_SECRET loaded from config file");
                }
                // 同样设置JWT过期时间
                if std::env::var("JWT_TOKEN_EXPIRY_SECS").is_err() {
                    std::env::set_var(
                        "JWT_TOKEN_EXPIRY_SECS",
                        config.jwt.token_expiry_secs.to_string(),
                    );
                }
                Some(config)
            }
            Err(e) => {
                tracing::warn!("⚠️ Failed to load config file: {}", e);
                None
            }
        }
    } else {
        None
    };

    // ✅ 2. 初始化日志（企业级：结构化日志 + 脱敏）
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ironcore=debug,tower_http=debug,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        // TODO: 添加日志脱敏层
        // .with(ironcore::api::middleware::log_sanitizer::SanitizingLayer)
        .init();

    tracing::info!("🚀 Starting IronCore Multi-Chain Wallet System");

    // ✅ 3. 连接数据库
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;
    tracing::info!("✅ Database connected");

    // ✅ 4. 运行数据库迁移（可选，用于开发测试）
    // 注意：生产环境建议单独运行迁移
    if std::env::var("SKIP_MIGRATIONS").is_err() {
        match sqlx::migrate!("./migrations").run(&pool).await {
            Ok(_) => tracing::info!("✅ Database migrations completed"),
            Err(e) => {
                tracing::warn!("⚠️ Database migrations failed (continuing): {}", e);
                tracing::info!("💡 Tip: Set SKIP_MIGRATIONS=1 to skip migrations on startup");
            }
        }
    } else {
        tracing::info!("⏭️ Database migrations skipped (SKIP_MIGRATIONS=1)");
    }

    // ✅ 5. 初始化Redis（分布式锁 + 缓存）
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    let distributed_lock = Arc::new(
        ironcore::infrastructure::distributed_lock::DistributedLock::new(&redis_url).await?,
    );
    tracing::info!("✅ Redis distributed lock initialized");

    // ✅ 6. 初始化应用状态
    // TODO: 实现 BlockchainConfig::from_env()
    let blockchain_config = Arc::new(BlockchainConfig::default());
    let cross_chain_config = Arc::new(ironcore::config::CrossChainConfig::default());

    // 创建Redis上下文
    let redis_client = redis::Client::open(redis_url.as_str())?;
    let redis_ctx = ironcore::infrastructure::cache::RedisCtx {
        client: redis_client,
    };

    // 创建 ImmuCtx（审计数据库上下文）
    let immu_ctx = Arc::new(ironcore::infrastructure::audit::ImmuCtx {
        addr: std::env::var("IMMU_ADDR").unwrap_or_else(|_| "localhost:3322".to_string()),
        user: std::env::var("IMMU_USER").unwrap_or_else(|_| "immudb".to_string()),
        pass: std::env::var("IMMU_PASS").unwrap_or_else(|_| "immudb".to_string()),
        db: std::env::var("IMMU_DB").unwrap_or_else(|_| "defaultdb".to_string()),
    });

    let config_arc =
        Arc::new(loaded_config.unwrap_or_else(|| ironcore::config::Config::from_env().unwrap()));
    let state = Arc::new(
        AppState::new(
            pool.clone(),
            redis_ctx,
            immu_ctx,
            blockchain_config,
            cross_chain_config,
            config_arc.clone(),
        )
        .await?,
    );

    // ✅ 7. 初始化费率配置（首次启动）
    let fee_service =
        ironcore::service::unified_fee_config_service::UnifiedFeeConfigService::new(pool.clone());
    if let Err(e) = fee_service.initialize_defaults().await {
        tracing::warn!("Failed to initialize fee defaults: {}", e);
    }

    // ✅ 7.5 初始化法币支付服务商（首次启动）
    if let Err(e) = ironcore::service::fiat_provider_seeder::seed_providers(&pool).await {
        tracing::warn!("Failed to initialize fiat providers: {}", e);
    } else {
        tracing::info!("✅ Fiat payment providers initialized");
    }

    // ✅ 8. 启动后台服务

    // 8.1 交易监控服务
    let tx_monitor = Arc::new(
        ironcore::service::transaction_monitor::TransactionMonitor::new(
            pool.clone(),
            state.blockchain_client.clone(),
        ),
    );
    let tx_monitor_clone = tx_monitor.clone();
    tokio::spawn(async move {
        tx_monitor_clone.start_background_monitor().await;
    });
    tracing::info!("✅ Transaction monitor started");

    // 8.2 交易自动恢复服务（RBF）
    let nonce_manager = Arc::new(ironcore::service::nonce_manager::NonceManager::new(
        pool.clone(),
        distributed_lock.clone(),
    ));

    let tx_auto_recovery = Arc::new(
        ironcore::service::transaction_auto_recovery::TransactionAutoRecovery::new(
            pool.clone(),
            state.blockchain_client.clone(),
            nonce_manager.clone(),
        ),
    );
    let tx_auto_recovery_clone = tx_auto_recovery.clone();
    tokio::spawn(async move {
        tx_auto_recovery_clone.start_background_monitor().await;
    });
    tracing::info!("✅ Transaction auto-recovery started");

    // 8.3 跨链事件监听服务
    let cross_chain_listener = Arc::new(
        ironcore::service::cross_chain_event_listener::CrossChainEventListener::new(
            pool.clone(),
            state.blockchain_client.clone(),
        ),
    );
    let cross_chain_listener_clone = cross_chain_listener.clone();
    tokio::spawn(async move {
        cross_chain_listener_clone.start_background_listener().await;
    });
    tracing::info!("✅ Cross-chain event listener started");

    // ✅ 9. 构建API路由
    // 使用统一的 api::routes() 函数，包含完整的路由配置：
    // - 认证: /api/auth/* (register, login, logout, refresh, me...)
    // - 钱包: /api/wallets/*, /api/v1/wallets/*
    // - 兑换: /api/swap/*, /api/v1/swap/* (包括 /api/v1/swap/history)
    // - 限价单: /api/v1/limit-orders/*
    // - Gas: /api/gas/* (estimate, estimate-all, price)
    // - 其他所有业务模块...
    // 包含所有中间件：认证、CORS、速率限制、追踪等
    // 健康检查端点在 api::routes 中已定义: /api/health, /healthz
    let app = api::routes(state.clone());

    // ✅ 10. 启动服务器
    // 尝试从config_arc获取bind_addr，否则使用默认值
    let bind_addr =
        std::env::var("BIND_ADDR").unwrap_or_else(|_| config_arc.server.bind_addr.clone());

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("🎉 Server listening on http://{}", bind_addr);
    tracing::info!("📖 Swagger UI: http://{}/swagger-ui", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

#[allow(dead_code)]
fn api_doc() -> utoipa::openapi::OpenApi {
    use utoipa::OpenApi;

    #[derive(OpenApi)]
    #[openapi(
        info(
            title = "IronCore API",
            version = "1.0.0",
            description = "Enterprise Multi-Chain Wallet System"
        ),
        paths(
            // 列出所有API路径
            api::multi_chain_api::create_multi_chain_wallets,
            api::fee_config_api::calculate_fee,
            api::fee_config_api::list_fee_configs,
            api::withdrawal_api::create_withdrawal,
        ),
        tags(
            (name = "wallets", description = "多链钱包管理"),
            (name = "assets", description = "资产管理"),
            (name = "transactions", description = "交易管理"),
            (name = "fees", description = "费率配置"),
            (name = "withdrawals", description = "提现管理"),
        )
    )]
    struct ApiDoc;

    ApiDoc::openapi()
}
