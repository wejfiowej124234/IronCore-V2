//! 环境变量验证器
//! 确保必需的环境变量已设置，并在启动时验证配置

use std::env;

#[derive(Debug)]
pub struct EnvValidator;

impl EnvValidator {
    /// 验证所有必需的环境变量
    pub fn validate_all() -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // 必需的环境变量
        let required = vec!["DATABASE_URL"];

        for var in required {
            if env::var(var).is_err() {
                errors.push(format!("{} is required but not set", var));
            }
        }

        // 验证 DATABASE_URL 格式
        if let Ok(db_url) = env::var("DATABASE_URL") {
            if !db_url.starts_with("postgres://") && !db_url.starts_with("postgresql://") {
                errors
                    .push("DATABASE_URL must start with postgres:// or postgresql://".to_string());
            }
        }

        // 验证 REDIS_URL 格式（如果设置）
        if let Ok(redis_url) = env::var("REDIS_URL") {
            if !redis_url.starts_with("redis://") && !redis_url.starts_with("rediss://") {
                errors.push("REDIS_URL must start with redis:// or rediss://".to_string());
            }
        }

        // 验证 WALLET_ENC_KEY（如果设置）
        if let Ok(enc_key) = env::var("WALLET_ENC_KEY") {
            // 密钥可以是32字节字符串、64字节hex字符串，或任意长度（将使用SHA256）
            if enc_key.is_empty() {
                errors.push("WALLET_ENC_KEY cannot be empty".to_string());
            }
            // 建议密钥长度至少16字符（生产环境）
            #[cfg(not(debug_assertions))]
            {
                if enc_key.len() < 16 {
                    errors.push(
                        "WALLET_ENC_KEY should be at least 16 characters in production".to_string(),
                    );
                }
            }
        } else {
            // 在Release构建中，WALLET_ENC_KEY是必需的
            #[cfg(not(debug_assertions))]
            {
                errors.push("WALLET_ENC_KEY is required in release builds".to_string());
            }
            // 在Debug构建中，建议设置（使用默认测试密钥）
            #[cfg(debug_assertions)]
            {
                tracing::warn!("WALLET_ENC_KEY not set in debug build, using default test key (NOT FOR PRODUCTION)");
            }
        }

        // 验证 JWT_SECRET（如果设置）
        if let Ok(jwt_secret) = env::var("JWT_SECRET") {
            if jwt_secret.len() < 32 {
                errors.push("JWT_SECRET must be at least 32 characters".to_string());
            }
        } else {
            // JWT_SECRET是必需的
            errors.push("JWT_SECRET is required".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 验证并打印错误
    pub fn validate_and_log() -> Result<(), anyhow::Error> {
        match Self::validate_all() {
            Ok(()) => {
                tracing::info!("Environment variables validation passed");
                Ok(())
            }
            Err(errors) => {
                for error in &errors {
                    tracing::error!("{}", error);
                }
                Err(anyhow::anyhow!(
                    "Environment validation failed: {} error(s)",
                    errors.len()
                ))
            }
        }
    }
}
