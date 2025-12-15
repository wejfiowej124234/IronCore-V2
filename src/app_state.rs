use std::sync::Arc;

use crate::{
    api::middleware::csrf::CsrfManager,
    infrastructure::{audit::ImmuCtx, cache::RedisCtx, cache_strategy::CacheManager, db::PgPool},
};

/// 应用状态
/// 包含所有共享资源
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub redis: Arc<RedisCtx>,
    pub redis_pool: Arc<RedisCtx>, // 别名，兼容旧代码
    pub immu: Arc<ImmuCtx>,
    pub cache: Arc<CacheManager>,
    pub csrf: Arc<CsrfManager>,
    pub distributed_lock: Arc<crate::infrastructure::distributed_lock::DistributedLock>,
    pub fee_service: Arc<crate::service::fee_service::FeeService>,
    pub rpc_selector: Arc<crate::infrastructure::rpc_selector::RpcSelector>,
    pub blockchain_client: Arc<crate::service::blockchain_client::BlockchainClient>,
    pub balance_sync_service: Arc<crate::service::balance_sync_service::BalanceSyncService>,
    pub blockchain_config: Arc<crate::config::BlockchainConfig>,
    pub cross_chain_config: Arc<crate::config::CrossChainConfig>,
    /// ✅ 企业级优化：Gas 估算器单例（配置只读取一次，避免重复警告）
    pub gas_estimator: Arc<crate::service::gas_estimator::GasEstimator>,
    /// ✅ 生产级：完整配置（包含支付网关配置）
    pub config: Arc<crate::config::Config>,
    /// ✅ 生产级：实时价格服务（CoinGecko + Redis缓存）
    pub price_service: Arc<crate::service::price_service::PriceService>,
}

impl AppState {
    /// 创建新的应用状态
    pub async fn new(
        pool: PgPool,
        redis: RedisCtx,
        immu: Arc<ImmuCtx>,
        blockchain_config: Arc<crate::config::BlockchainConfig>,
        cross_chain_config: Arc<crate::config::CrossChainConfig>,
        config: Arc<crate::config::Config>,
    ) -> anyhow::Result<Self> {
        let redis = Arc::new(redis);
        let redis_pool = redis.clone(); // 别名

        let cache_config = crate::infrastructure::cache_strategy::CacheConfig::default();
        let cache = Arc::new(CacheManager::new(redis.clone(), cache_config));

        let csrf_ttl = std::env::var("CSRF_TOKEN_TTL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(86400); // 默认24小时
        let csrf = Arc::new(CsrfManager::new(csrf_ttl));

        // 初始化分布式锁
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let distributed_lock = Arc::new(
            crate::infrastructure::distributed_lock::DistributedLock::new(&redis_url)
                .await
                .expect("Failed to initialize distributed lock"),
        );

        // 使用带 Redis 二级缓存的服务（生产级性能优化）
        let fee_service = Arc::new(crate::service::fee_service::FeeService::with_redis(
            pool.clone(),
            redis.clone(),
        ));
        let rpc_selector = Arc::new(
            crate::infrastructure::rpc_selector::RpcSelector::with_redis(
                pool.clone(),
                redis.clone(),
            ),
        );
        let blockchain_client = Arc::new(crate::service::blockchain_client::BlockchainClient::new(
            rpc_selector.clone(),
        ));

        let balance_sync_service = Arc::new(
            crate::service::balance_sync_service::BalanceSyncService::new(
                pool.clone(),
                blockchain_client.clone(),
            ),
        );

        // ✅ 企业级优化：Gas 估算器在启动时初始化一次（配置读取和警告只出现一次）
        let gas_estimator = Arc::new(crate::service::gas_estimator::GasEstimator::new(
            rpc_selector.clone(),
        ));
        tracing::info!("✅ Gas estimator initialized with cached configuration");

        // ✅ 生产级：初始化价格服务（CoinGecko + Redis缓存）
        let price_service = Arc::new(crate::service::price_service::PriceService::new(
            pool.clone(),
            Some(std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())),
        ));
        tracing::info!("✅ Price service initialized with CoinGecko API");

        Ok(Self {
            pool,
            redis,
            redis_pool,
            immu,
            cache,
            csrf,
            distributed_lock,
            fee_service,
            rpc_selector,
            blockchain_client,
            balance_sync_service,
            blockchain_config,
            cross_chain_config,
            gas_estimator,
            config,
            price_service,
        })
    }

    /// 初始化默认 RPC 端点（需要在 main.rs 中调用）
    pub async fn init_default_rpc_endpoints(pool: &PgPool) -> anyhow::Result<()> {
        // 检查是否已有端点
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admin.rpc_endpoints")
            .fetch_one(pool)
            .await?;

        if count > 0 {
            tracing::info!("RPC endpoints already initialized ({} endpoints)", count);
            return Ok(());
        }

        tracing::info!("Initializing default RPC endpoints...");

        // 插入默认端点
        let endpoints = vec![
            ("ethereum", "https://ethereum-sepolia-rpc.publicnode.com", 1),
            ("ethereum", "https://rpc.sepolia.org", 2),
            (
                "ethereum",
                "https://sepolia.infura.io/v3/9aa3d95b3bc440fa88ea12eaa4456161",
                3,
            ),
        ];

        for (chain, url, priority) in endpoints {
            sqlx::query(
                "INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) 
                 VALUES ($1, $2, $3, true, 'closed')",
            )
            .bind(chain)
            .bind(url)
            .bind(priority)
            .execute(pool)
            .await?;
        }

        tracing::info!("Default RPC endpoints initialized successfully");
        Ok(())
    }
}
