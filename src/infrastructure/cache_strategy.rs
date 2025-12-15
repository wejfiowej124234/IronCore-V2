//! 缓存策略模块
//! 提供Redis缓存热点数据的策略和工具

use std::{sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

use crate::infrastructure::cache::RedisCtx;

/// 缓存键前缀
pub mod cache_keys {
    pub const WALLET: &str = "cache:wallet:";
    pub const USER: &str = "cache:user:";
    pub const TENANT: &str = "cache:tenant:";
    pub const POLICY: &str = "cache:policy:";
    pub const API_KEY: &str = "cache:api_key:";
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub default_ttl: Duration,
    pub wallet_ttl: Duration,
    pub user_ttl: Duration,
    pub tenant_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(300), // 5分钟
            wallet_ttl: Duration::from_secs(600),  // 10分钟
            user_ttl: Duration::from_secs(300),    // 5分钟
            tenant_ttl: Duration::from_secs(1800), // 30分钟
        }
    }
}

/// 缓存管理器
pub struct CacheManager {
    redis: Arc<RedisCtx>,
    config: CacheConfig,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(redis: Arc<RedisCtx>, config: CacheConfig) -> Self {
        Self { redis, config }
    }

    /// 获取缓存值
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value: Option<String> = self.redis.get_session(key).await?;

        if let Some(v) = value {
            let deserialized: T = serde_json::from_str(&v)
                .map_err(|e| anyhow!("Failed to deserialize cache value: {}", e))?;
            Ok(Some(deserialized))
        } else {
            Ok(None)
        }
    }

    /// 设置缓存值
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<()>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(value)
            .map_err(|e| anyhow!("Failed to serialize cache value: {}", e))?;

        let ttl = ttl.unwrap_or(self.config.default_ttl);
        self.redis.set_session(key, &serialized, ttl).await?;

        Ok(())
    }

    /// 删除缓存值
    pub async fn delete(&self, key: &str) -> Result<()> {
        self.redis.delete_session(key).await?;
        Ok(())
    }

    /// 批量删除缓存（使用模式匹配）
    ///
    /// 注意：使用SCAN而不是KEYS，避免阻塞Redis
    /// COUNT参数控制每次扫描的键数量，默认100
    pub async fn delete_pattern(&self, pattern: &str) -> Result<()> {
        let mut conn = self
            .redis
            .client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| anyhow!("Failed to get Redis connection: {}", e))?;

        // 使用SCAN迭代匹配的键（避免阻塞Redis）
        let mut cursor: u64 = 0;
        let mut total_deleted = 0;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100) // 每次扫描100个键
                .query_async(&mut conn)
                .await
                .map_err(|e| anyhow!("Failed to scan Redis keys: {}", e))?;

            if !keys.is_empty() {
                let deleted: u64 = redis::cmd("DEL")
                    .arg(&keys)
                    .query_async(&mut conn)
                    .await
                    .map_err(|e| anyhow!("Failed to delete Redis keys: {}", e))?;
                total_deleted += deleted;
            }

            if new_cursor == 0 {
                break;
            }
            cursor = new_cursor;
        }

        if total_deleted > 0 {
            tracing::debug!(
                "Deleted {} keys matching pattern: {}",
                total_deleted,
                pattern
            );
        }

        Ok(())
    }

    /// 获取钱包缓存
    pub async fn get_wallet<T>(&self, wallet_id: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let key = format!("{}{}", cache_keys::WALLET, wallet_id);
        self.get(&key).await
    }

    /// 设置钱包缓存
    pub async fn set_wallet<T>(&self, wallet_id: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let key = format!("{}{}", cache_keys::WALLET, wallet_id);
        self.set(&key, value, Some(self.config.wallet_ttl)).await
    }

    /// 删除钱包缓存
    pub async fn delete_wallet(&self, wallet_id: &str) -> Result<()> {
        let key = format!("{}{}", cache_keys::WALLET, wallet_id);
        self.delete(&key).await
    }

    /// 获取用户缓存
    pub async fn get_user<T>(&self, user_id: &str) -> Result<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let key = format!("{}{}", cache_keys::USER, user_id);
        self.get(&key).await
    }

    /// 设置用户缓存
    pub async fn set_user<T>(&self, user_id: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let key = format!("{}{}", cache_keys::USER, user_id);
        self.set(&key, value, Some(self.config.user_ttl)).await
    }

    /// 删除用户缓存
    pub async fn delete_user(&self, user_id: &str) -> Result<()> {
        let key = format!("{}{}", cache_keys::USER, user_id);
        self.delete(&key).await
    }
}
