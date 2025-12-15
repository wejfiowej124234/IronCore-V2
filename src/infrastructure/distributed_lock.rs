//! 分布式锁实现
//! 企业级实现：基于Redis的分布式锁，防止多实例并发冲突
//! 用于：Nonce管理、幂等性保护、关键业务逻辑

use std::time::Duration;

use anyhow::{Context, Result};
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use uuid::Uuid;

/// 分布式锁
pub struct DistributedLock {
    redis_client: ConnectionManager,
}

/// 锁守卫（自动释放）
pub struct LockGuard<'a> {
    lock: &'a DistributedLock,
    lock_key: String,
    lock_value: String,
}

impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        // 异步释放锁（在Drop中使用block_on）
        let lock = self.lock.clone();
        let key = self.lock_key.clone();
        let value = self.lock_value.clone();

        tokio::spawn(async move {
            if let Err(e) = lock.release_internal(&key, &value).await {
                tracing::warn!(
                    error = ?e,
                    lock_key = %key,
                    "Failed to release lock in Drop"
                );
            }
        });
    }
}

impl Clone for DistributedLock {
    fn clone(&self) -> Self {
        Self {
            redis_client: self.redis_client.clone(),
        }
    }
}

impl DistributedLock {
    /// 创建分布式锁实例
    ///
    /// # 参数
    /// - `redis_url`: Redis连接字符串，格式：redis://host:port
    pub async fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url).context("Failed to create Redis client")?;
        let conn = ConnectionManager::new(client)
            .await
            .context("Failed to connect to Redis")?;

        Ok(Self { redis_client: conn })
    }

    /// 获取分布式锁（阻塞直到获取成功或超时）
    ///
    /// # 参数
    /// - `lock_key`: 锁的唯一标识
    /// - `ttl`: 锁的过期时间（秒）
    /// - `timeout`: 获取锁的超时时间
    ///
    /// # 返回
    /// - `LockGuard`: 锁守卫，超出作用域自动释放
    ///
    /// # 企业级特性
    /// - 自动重试（指数退避）
    /// - 防死锁（TTL自动过期）
    /// - 防误删（使用UUID标识锁拥有者）
    pub async fn acquire<'a>(
        &'a self,
        lock_key: &str,
        ttl_secs: u64,
        timeout: Duration,
    ) -> Result<LockGuard<'a>> {
        let lock_value = Uuid::new_v4().to_string();
        let start = std::time::Instant::now();
        let mut attempt = 0;

        loop {
            attempt += 1;

            // 尝试获取锁（SET NX EX）
            if self
                .try_acquire_internal(lock_key, &lock_value, ttl_secs)
                .await?
            {
                tracing::debug!(
                    lock_key = %lock_key,
                    attempt = attempt,
                    elapsed_ms = start.elapsed().as_millis(),
                    "Acquired distributed lock"
                );

                return Ok(LockGuard {
                    lock: self,
                    lock_key: lock_key.to_string(),
                    lock_value,
                });
            }

            // 检查超时
            if start.elapsed() >= timeout {
                anyhow::bail!(
                    "Failed to acquire lock '{}' within {:?} after {} attempts",
                    lock_key,
                    timeout,
                    attempt
                );
            }

            // 指数退避（最大500ms）
            let backoff_ms = std::cmp::min(50 * 2u64.pow(attempt.min(4)), 500);
            tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        }
    }

    /// 尝试获取锁（非阻塞）
    ///
    /// # 返回
    /// - `Ok(Some(LockGuard))`: 获取成功
    /// - `Ok(None)`: 锁已被占用
    pub async fn try_acquire<'a>(
        &'a self,
        lock_key: &str,
        ttl_secs: u64,
    ) -> Result<Option<LockGuard<'a>>> {
        let lock_value = Uuid::new_v4().to_string();

        if self
            .try_acquire_internal(lock_key, &lock_value, ttl_secs)
            .await?
        {
            Ok(Some(LockGuard {
                lock: self,
                lock_key: lock_key.to_string(),
                lock_value,
            }))
        } else {
            Ok(None)
        }
    }

    /// 内部方法：尝试获取锁
    async fn try_acquire_internal(
        &self,
        lock_key: &str,
        lock_value: &str,
        ttl_secs: u64,
    ) -> Result<bool> {
        let mut conn = self.redis_client.clone();

        // SET key value NX EX ttl
        // NX: 仅当key不存在时设置
        // EX: 设置过期时间（秒）
        let result: Option<String> = redis::cmd("SET")
            .arg(lock_key)
            .arg(lock_value)
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .context("Failed to execute SET NX EX")?;

        Ok(result.is_some())
    }

    /// 释放锁（使用Lua脚本保证原子性）
    ///
    /// # 企业级特性
    /// - 只有锁的拥有者才能释放（验证UUID）
    /// - 原子操作（使用Lua脚本）
    async fn release_internal(&self, lock_key: &str, lock_value: &str) -> Result<()> {
        let mut conn = self.redis_client.clone();

        // Lua脚本：验证锁拥有者后删除
        let script = r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("DEL", KEYS[1])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(lock_key)
            .arg(lock_value)
            .invoke_async(&mut conn)
            .await
            .context("Failed to release lock")?;

        if result == 1 {
            tracing::debug!(lock_key = %lock_key, "Released distributed lock");
        } else {
            tracing::warn!(
                lock_key = %lock_key,
                "Lock not owned by current instance (may have expired)"
            );
        }

        Ok(())
    }

    /// 续约锁（延长TTL）
    ///
    /// # 使用场景
    /// - 长时间操作需要持续持有锁
    pub async fn renew(&self, lock_key: &str, lock_value: &str, ttl_secs: u64) -> Result<bool> {
        let mut conn = self.redis_client.clone();

        // Lua脚本：验证锁拥有者后续约
        let script = r#"
            if redis.call("GET", KEYS[1]) == ARGV[1] then
                return redis.call("EXPIRE", KEYS[1], ARGV[2])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(lock_key)
            .arg(lock_value)
            .arg(ttl_secs)
            .invoke_async(&mut conn)
            .await
            .context("Failed to renew lock")?;

        Ok(result == 1)
    }

    /// 检查锁是否存在
    pub async fn is_locked(&self, lock_key: &str) -> Result<bool> {
        let mut conn = self.redis_client.clone();
        let exists: bool = conn.exists(lock_key).await?;
        Ok(exists)
    }

    /// 获取锁的剩余TTL（秒）
    pub async fn get_ttl(&self, lock_key: &str) -> Result<i64> {
        let mut conn = self.redis_client.clone();
        let ttl: i64 = conn.ttl(lock_key).await?;
        Ok(ttl)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要Redis实例
    async fn test_acquire_and_release() {
        let lock = DistributedLock::new("redis://127.0.0.1:6379")
            .await
            .unwrap();

        let lock_key = "test:lock:1";

        // 获取锁
        let guard = lock
            .acquire(lock_key, 10, Duration::from_secs(5))
            .await
            .unwrap();

        // 验证锁存在
        assert!(lock.is_locked(lock_key).await.unwrap());

        // 释放锁
        drop(guard);

        // 等待异步释放完成
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 验证锁已释放
        assert!(!lock.is_locked(lock_key).await.unwrap());
    }

    #[tokio::test]
    #[ignore]
    async fn test_concurrent_acquire() {
        let lock1 = DistributedLock::new("redis://127.0.0.1:6379")
            .await
            .unwrap();
        let lock2 = lock1.clone();

        let lock_key = "test:lock:concurrent";

        // 第一个实例获取锁
        let guard1 = lock1
            .acquire(lock_key, 10, Duration::from_secs(1))
            .await
            .unwrap();

        // 第二个实例尝试获取（应该失败）
        let result2 = lock2.try_acquire(lock_key, 10).await.unwrap();
        assert!(result2.is_none());

        // 释放第一个锁
        drop(guard1);
        tokio::time::sleep(Duration::from_millis(100)).await;

        // 第二个实例再次尝试（应该成功）
        let guard2 = lock2.try_acquire(lock_key, 10).await.unwrap();
        assert!(guard2.is_some());
    }
}
