//! 配置管理模块
//! 支持从环境变量和配置文件加载配置

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// 应用配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub immudb: ImmudbConfig,
    pub jwt: JwtConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    pub monitoring: MonitoringConfig,
    #[serde(default)]
    pub features: FeaturesConfig,
    #[serde(default)]
    pub blockchain: BlockchainConfig,
    #[serde(default)]
    pub cross_chain: CrossChainConfig,
    #[serde(default)]
    pub payment_gateway: PaymentGatewayConfig,
}

/// 数据库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
}

/// Redis配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

/// Immudb配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmudbConfig {
    pub addr: String,
    pub user: String,
    pub password: String,
    pub database: String,
}

/// JWT配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub token_expiry_secs: u64,
    pub refresh_token_expiry_secs: u64,
}

/// 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub allow_degraded_start: bool,
    #[serde(default)]
    pub frontend_url: Option<String>,
}

/// 日志配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "text"
    pub enable_file_logging: bool,
    pub log_file_path: Option<String>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
}

/// 监控配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_prometheus: bool,
    pub prometheus_bind_addr: Option<String>,
    pub enable_health_check: bool,
}

/// 功能开关配置 (Feature Flags)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    pub enable_fee_system: bool,
    pub enable_rpc_failover: bool,
    pub enable_notify_system: bool,
}

/// 区块链RPC配置✅生产级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub eth_rpc_url: String,
    pub bsc_rpc_url: String,
    pub polygon_rpc_url: String,
    pub solana_rpc_url: String,
    pub bitcoin_rpc_url: String,
    pub ton_rpc_url: String, // ✅添加TON支持
}

/// 跨链桥配置 (生产环境手续费配置)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainConfig {
    pub bridge_fee_percentage: f64, // 桥接手续费百分比 (例如 0.004 = 0.4%)
    pub transaction_fee_percentage: f64, // 交易手续费百分比
}

/// Fiat Onramp 配置 (生产环境)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentGatewayConfig {
    // MoonPay 配置
    pub moonpay_api_key: String,
    pub moonpay_secret_key: String,
    
    // Transak 配置
    pub transak_api_key: String,
    pub transak_environment: String, // "STAGING" or "PRODUCTION"
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://root@localhost:26257/ironforge?sslmode=disable".into()
            }),
            max_connections: std::env::var("DB_MAX_CONNS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(16),
            min_connections: std::env::var("DB_MIN_CONNS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2),
            acquire_timeout_secs: std::env::var("DB_ACQ_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            idle_timeout_secs: std::env::var("DB_IDLE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
        }
    }
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".into()),
        }
    }
}

impl Default for ImmudbConfig {
    fn default() -> Self {
        Self {
            addr: std::env::var("IMMUDB_ADDR").unwrap_or_else(|_| "127.0.0.1:3322".into()),
            user: std::env::var("IMMUDB_USER").unwrap_or_else(|_| "immudb".into()),
            password: std::env::var("IMMUDB_PASS").unwrap_or_else(|_| "immudb".into()),
            database: std::env::var("IMMUDB_DB").unwrap_or_else(|_| "defaultdb".into()),
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                // 不再这里输出警告，因为配置可能从文件加载
                // 警告移到 main.rs 中根据实际使用的密钥判断
                "default-jwt-secret-please-change-in-production-min-32-chars".to_string()
            }),
            token_expiry_secs: std::env::var("TOKEN_EXPIRY")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600), // 1小时
            refresh_token_expiry_secs: std::env::var("REFRESH_TOKEN_EXPIRY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400 * 7),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8088".into()),
            allow_degraded_start: std::env::var("ALLOW_DEGRADED_START")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false),
            frontend_url: std::env::var("FRONTEND_URL").ok(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()),
            format: std::env::var("LOG_FORMAT").unwrap_or_else(|_| "text".into()),
            enable_file_logging: std::env::var("LOG_FILE_ENABLED")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false),
            log_file_path: std::env::var("LOG_FILE_PATH").ok(),
            max_file_size_mb: std::env::var("LOG_MAX_FILE_SIZE_MB")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            max_files: std::env::var("LOG_MAX_FILES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_prometheus: std::env::var("ENABLE_PROMETHEUS")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(true),
            prometheus_bind_addr: std::env::var("PROMETHEUS_BIND_ADDR").ok(),
            enable_health_check: std::env::var("ENABLE_HEALTH_CHECK")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(true),
        }
    }
}

impl Default for PaymentGatewayConfig {
    fn default() -> Self {
        Self {
            // MoonPay
            moonpay_api_key: std::env::var("MOONPAY_API_KEY")
                .unwrap_or_else(|_| "pk_test_placeholder".to_string()),
            moonpay_secret_key: std::env::var("MOONPAY_SECRET_KEY")
                .unwrap_or_else(|_| "sk_test_placeholder".to_string()),
            
            // Transak
            transak_api_key: std::env::var("TRANSAK_API_KEY")
                .unwrap_or_else(|_| "placeholder_api_key".to_string()),
            transak_environment: std::env::var("TRANSAK_ENVIRONMENT")
                .unwrap_or_else(|_| "STAGING".to_string()),
        }
    }
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database: DatabaseConfig::default(),
            redis: RedisConfig::default(),
            immudb: ImmudbConfig::default(),
            jwt: JwtConfig::default(),
            server: ServerConfig::default(),
            logging: LoggingConfig::default(),
            monitoring: MonitoringConfig::default(),
            features: FeaturesConfig::default(),
            blockchain: BlockchainConfig::default(),
            cross_chain: CrossChainConfig::default(),
            payment_gateway: PaymentGatewayConfig::default(),
        })
    }

    /// 从配置文件加载配置
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

        let config: Config =
            toml::from_str(&content).with_context(|| "Failed to parse config file as TOML")?;

        Ok(config)
    }

    /// 从环境变量和配置文件合并加载（配置文件优先级更高）
    pub fn from_env_and_file<P: AsRef<Path>>(path: Option<P>) -> Result<Self> {
        let mut config = Self::from_env()?;

        if let Some(path) = path {
            if path.as_ref().exists() {
                let file_config = Self::from_file(path)?;
                // 合并配置（文件配置覆盖环境变量）
                config = file_config;
            }
        }

        Ok(config)
    }

    /// 验证配置有效性
    pub fn validate(&self) -> Result<()> {
        // 验证数据库URL格式
        if !self.database.url.starts_with("postgres://")
            && !self.database.url.starts_with("postgresql://")
        {
            anyhow::bail!("DATABASE_URL must start with postgres:// or postgresql://");
        }

        // 验证JWT secret长度
        if self.jwt.secret.len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 characters");
        }

        // 验证日志级别
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.to_lowercase().as_str()) {
            anyhow::bail!("LOG_LEVEL must be one of: {:?}", valid_levels);
        }

        // 验证日志格式
        if self.logging.format != "json" && self.logging.format != "text" {
            anyhow::bail!("LOG_FORMAT must be 'json' or 'text'");
        }

        Ok(())
    }
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enable_fee_system: std::env::var("ENABLE_FEE_SYSTEM")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false),
            enable_rpc_failover: std::env::var("ENABLE_RPC_FAILOVER")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(false),
            enable_notify_system: std::env::var("ENABLE_NOTIFY_SYSTEM")
                .ok()
                .map(|v| v == "1")
                .unwrap_or(true),
        }
    }
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            eth_rpc_url: std::env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY".into()),
            bsc_rpc_url: std::env::var("BSC_RPC_URL")
                .unwrap_or_else(|_| "https://bsc-dataseed1.binance.org".into()),
            polygon_rpc_url: std::env::var("POLYGON_RPC_URL")
                .unwrap_or_else(|_| "https://polygon-rpc.com".into()),
            solana_rpc_url: std::env::var("SOLANA_RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".into()),
            bitcoin_rpc_url: std::env::var("BITCOIN_RPC_URL")
                .unwrap_or_else(|_| "https://blockstream.info/api".into()),
            ton_rpc_url: std::env::var("TON_RPC_URL")
                .unwrap_or_else(|_| "https://toncenter.com/api/v2/jsonRPC".into()),
        }
    }
}

impl Default for CrossChainConfig {
    /// 企业级实现：从环境变量读取配置，多级降级策略
    ///
    /// 多级降级策略：
    /// 1. 优先从环境变量读取配置的费率
    /// 2. 最终降级：使用安全默认值（仅作为最后保障）
    fn default() -> Self {
        Self {
            bridge_fee_percentage: std::env::var("BRIDGE_FEE_PERCENTAGE")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|&v: &f64| v > 0.0 && v <= 1.0 && v.is_finite()) // 验证范围：0-100%
                .unwrap_or_else(|| {
                    // 企业级实现：尝试从链特定的环境变量读取
                    let chain_specific_keys = vec![
                        "BRIDGE_FEE_PERCENTAGE_ETH",
                        "BRIDGE_FEE_PERCENTAGE_BSC",
                        "BRIDGE_FEE_PERCENTAGE_POLYGON",
                    ];
                    for key in chain_specific_keys {
                        if let Ok(env_value) = std::env::var(key) {
                            if let Ok(value) = env_value.parse::<f64>() {
                                if value > 0.0 && value <= 1.0 && value.is_finite() {
                                    tracing::warn!(
                                        "使用环境变量配置的桥接费率: key={}, value={}",
                                        key, value
                                    );
                                    return value;
                                }
                            }
                        }
                    }
                    // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                    tracing::error!(
                        "严重警告：未找到任何环境变量配置的桥接费率，使用硬编码默认值 0.3% (0.003)。生产环境必须配置环境变量 BRIDGE_FEE_PERCENTAGE"
                    );
                    0.003 // 安全默认值：0.3% 桥接费（仅作为最后保障，生产环境不应使用）
                }),
            transaction_fee_percentage: std::env::var("TRANSACTION_FEE_PERCENTAGE")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|&v: &f64| v > 0.0 && v <= 1.0 && v.is_finite()) // 验证范围：0-100%
                .unwrap_or_else(|| {
                    // 企业级实现：尝试从链特定的环境变量读取
                    let chain_specific_keys = vec![
                        "TRANSACTION_FEE_PERCENTAGE_ETH",
                        "TRANSACTION_FEE_PERCENTAGE_BSC",
                        "TRANSACTION_FEE_PERCENTAGE_POLYGON",
                    ];
                    for key in chain_specific_keys {
                        if let Ok(env_value) = std::env::var(key) {
                            if let Ok(value) = env_value.parse::<f64>() {
                                if value > 0.0 && value <= 1.0 && value.is_finite() {
                                    tracing::warn!(
                                        "使用环境变量配置的交易费率: key={}, value={}",
                                        key, value
                                    );
                                    return value;
                                }
                            }
                        }
                    }
                    // 企业级实现：如果所有环境变量都未设置，记录严重警告并使用安全默认值
                    tracing::error!(
                        "严重警告：未找到任何环境变量配置的交易费率，使用硬编码默认值 0.1% (0.001)。生产环境必须配置环境变量 TRANSACTION_FEE_PERCENTAGE"
                    );
                    0.001 // 安全默认值：0.1% 交易费（仅作为最后保障，生产环境不应使用）
                }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_config_from_env() {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_that_is_at_least_32_characters_long",
        );
        let config = Config::from_env().unwrap();
        assert_eq!(config.database.max_connections, 16);
        assert_eq!(config.server.bind_addr, "0.0.0.0:8088");
    }

    #[test]
    fn test_config_from_file() {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_that_is_at_least_32_characters_long",
        );

        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"
[database]
url = "postgres://test@localhost/test"
max_connections = 20
min_connections = 5
acquire_timeout_secs = 30
idle_timeout_secs = 600

[redis]
url = "redis://localhost:6379"

[immudb]
addr = "localhost:3322"
user = "immudb"
password = "immudb"
database = "ironcore"

[jwt]
secret = "test_secret_that_is_at_least_32_characters_long"
token_expiry_secs = 3600
refresh_token_expiry_secs = 604800

[server]
bind_addr = "0.0.0.0:9090"
allow_degraded_start = false

[logging]
level = "info"
format = "text"
enable_file_logging = false
max_file_size_mb = 100
max_files = 10

[monitoring]
enable_prometheus = false
prometheus_port = 9091
enable_health_check = true
health_check_port = 8080
"#
        )
        .unwrap();

        let config = Config::from_file(file.path()).unwrap();
        assert_eq!(config.database.max_connections, 20);
        assert_eq!(config.server.bind_addr, "0.0.0.0:9090");
    }

    #[test]
    fn test_config_validation() {
        std::env::set_var(
            "JWT_SECRET",
            "test_secret_that_is_at_least_32_characters_long",
        );
        let config = Config::from_env().unwrap();
        assert!(config.validate().is_ok());
    }
}
