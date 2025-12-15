//! 日志系统配置模块
//! 支持结构化日志、日志级别配置和日志轮转

use crate::config::LoggingConfig;
use std::path::Path;
use tracing_appender::{non_blocking, rolling};
use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Registry,
};

/// 初始化日志系统
pub fn init_logging(config: &LoggingConfig) -> Result<(), Box<dyn std::error::Error>> {
    // 设置日志级别过滤器
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.level));

    // 根据配置选择日志格式
    if config.format == "json" {
        init_json_logging(filter, config)?;
    } else {
        init_text_logging(filter, config)?;
    }

    Ok(())
}

/// 初始化JSON格式日志（结构化日志）
fn init_json_logging(
    filter: EnvFilter,
    config: &LoggingConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.enable_file_logging {
        // 文件日志 + 控制台日志
        let log_dir = config
            .log_file_path
            .as_ref()
            .and_then(|p| Path::new(p).parent())
            .unwrap_or_else(|| Path::new("./logs"));

        std::fs::create_dir_all(log_dir)?;

        let file_appender = rolling::daily(log_dir, "app.log");
        let (non_blocking_appender, _guard) = non_blocking(file_appender);

        // 文件日志使用JSON格式
        let file_layer = fmt::layer()
            .json()
            .with_writer(non_blocking_appender)
            .with_timer(ChronoUtc::rfc_3339());

        // 控制台日志也使用JSON格式
        let stdout_layer = fmt::layer().json().with_timer(ChronoUtc::rfc_3339());

        Registry::default()
            .with(filter)
            .with(file_layer)
            .with(stdout_layer)
            .init();
    } else {
        // 仅控制台日志
        Registry::default()
            .with(filter)
            .with(fmt::layer().json().with_timer(ChronoUtc::rfc_3339()))
            .init();
    }

    Ok(())
}

/// 初始化文本格式日志
fn init_text_logging(
    filter: EnvFilter,
    config: &LoggingConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.enable_file_logging {
        // 文件日志 + 控制台日志
        let log_dir = config
            .log_file_path
            .as_ref()
            .and_then(|p| Path::new(p).parent())
            .unwrap_or_else(|| Path::new("./logs"));

        std::fs::create_dir_all(log_dir)?;

        let file_appender = rolling::daily(log_dir, "app.log");
        let (non_blocking_appender, _guard) = non_blocking(file_appender);

        // 文件日志
        let file_layer = fmt::layer()
            .with_writer(non_blocking_appender)
            .with_timer(ChronoUtc::rfc_3339())
            .with_ansi(false);

        // 控制台日志
        let stdout_layer = fmt::layer()
            .with_timer(ChronoUtc::rfc_3339())
            .with_ansi(true);

        Registry::default()
            .with(filter)
            .with(file_layer)
            .with(stdout_layer)
            .init();
    } else {
        // 仅控制台日志
        Registry::default()
            .with(filter)
            .with(
                fmt::layer()
                    .with_timer(ChronoUtc::rfc_3339())
                    .with_ansi(true),
            )
            .init();
    }

    Ok(())
}

/// 简化初始化（使用默认配置）
pub fn init_default_logging() {
    let config = crate::config::LoggingConfig::default();
    init_logging(&config).unwrap_or_else(|e| {
        eprintln!("Failed to initialize logging: {}", e);
        // 回退到最基本的日志初始化
        tracing_subscriber::fmt::init();
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_logging_config() {
        let config = crate::config::LoggingConfig {
            level: "debug".to_string(),
            format: "json".to_string(),
            enable_file_logging: false,
            log_file_path: None,
            max_file_size_mb: 100,
            max_files: 10,
        };

        // 测试配置创建
        assert_eq!(config.level, "debug");
        assert_eq!(config.format, "json");
    }
}
