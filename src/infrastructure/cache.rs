//! Redis 封装：会话、幂等键、速率限制与热点缓存简易接口
//! 使用 MultiplexedConnection 替代已废弃的 Connection

use std::time::Duration;

#[derive(Clone)]
pub struct RedisCtx {
    pub client: redis::Client,
}

impl RedisCtx {
    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        Ok(Self { client })
    }

    pub async fn ping(&self) -> Result<String, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
        Ok(pong)
    }

    /// 幂等键：成功占位返回 true，已存在返回 false
    pub async fn put_idempotency_key(
        &self,
        key: &str,
        ttl: Duration,
    ) -> Result<bool, redis::RedisError> {
        use redis::FromRedisValue;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let reply: redis::Value = redis::cmd("SET")
            .arg(key)
            .arg("1")
            .arg("NX")
            .arg("EX")
            .arg(ttl.as_secs() as usize)
            .query_async(&mut conn)
            .await?;
        // When SET with NX succeeds, Redis returns "OK". If key existed, returns Nil.
        if let Ok(status) = String::from_redis_value(&reply) {
            Ok(status.eq_ignore_ascii_case("OK"))
        } else {
            Ok(false)
        }
    }

    /// 简单速率限制：返回当前窗口计数
    pub async fn rate_limit_incr(
        &self,
        key: &str,
        window: Duration,
    ) -> Result<i64, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        // INCR first; if this is a new key (count == 1), set TTL
        let cnt: i64 = redis::cmd("INCR").arg(key).query_async(&mut conn).await?;
        if cnt == 1 {
            let _: () = redis::cmd("EXPIRE")
                .arg(key)
                .arg(window.as_secs() as usize)
                .query_async(&mut conn)
                .await?;
        }
        Ok(cnt)
    }

    /// 存储Session
    pub async fn set_session(
        &self,
        session_key: &str,
        user_data: &str,
        ttl: Duration,
    ) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("SETEX")
            .arg(session_key)
            .arg(ttl.as_secs() as usize)
            .arg(user_data)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(())
    }

    /// 获取Session
    pub async fn get_session(
        &self,
        session_key: &str,
    ) -> Result<Option<String>, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let value: Option<String> = redis::cmd("GET")
            .arg(session_key)
            .query_async(&mut conn)
            .await?;
        Ok(value)
    }

    /// 删除Session
    pub async fn delete_session(&self, session_key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("DEL")
            .arg(session_key)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(())
    }

    /// 使用SCAN命令删除匹配模式的所有键（用于清理用户Session）
    pub async fn delete_keys_by_pattern(&self, pattern: &str) -> Result<usize, redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let mut cursor: i64 = 0;
        let mut total_deleted = 0;

        loop {
            let (new_cursor, keys): (i64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await?;

            if !keys.is_empty() {
                let deleted: i64 = redis::cmd("DEL").arg(&keys).query_async(&mut conn).await?;
                total_deleted += deleted as usize;
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(total_deleted)
    }

    /// SET if Not Exists - 用于分布式锁
    /// 返回 true 表示成功获取锁，false 表示锁已存在
    pub async fn set_if_not_exists(
        &self,
        key: &str,
        value: &str,
        ttl: Duration,
    ) -> Result<bool, redis::RedisError> {
        use redis::FromRedisValue;
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        let reply: redis::Value = redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("NX")  // Only set if not exists
            .arg("EX")  // Expire in seconds
            .arg(ttl.as_secs() as usize)
            .query_async(&mut conn)
            .await?;
        
        // Redis returns "OK" on success, Nil if key exists
        if let Ok(status) = String::from_redis_value(&reply) {
            Ok(status.eq_ignore_ascii_case("OK"))
        } else {
            Ok(false)
        }
    }

    /// 删除指定key
    pub async fn delete(&self, key: &str) -> Result<(), redis::RedisError> {
        let mut conn = self.client.get_multiplexed_async_connection().await?;
        redis::cmd("DEL")
            .arg(key)
            .query_async::<_, ()>(&mut conn)
            .await?;
        Ok(())
    }
}
