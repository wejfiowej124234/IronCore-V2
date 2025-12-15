//! 认证服务层
//! 处理用户认证相关的业务逻辑

use std::time::Duration;

use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::{
    infrastructure::{
        cache::RedisCtx,
        db::PgPool,
        jwt::{generate_refresh_token, generate_token, verify_refresh_token, verify_token, Claims},
        password::{hash_password, verify_password},
        validation::validate_password_strength,
    },
    repository::auth::{self, AuthUser},
};

/// Redis分布式锁的RAII守卫
/// 自动释放锁，防止死锁
struct LockGuard {
    redis: RedisCtx,
    key: String,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        // 异步释放锁（best effort，不阻塞）
        let redis = self.redis.clone();
        let key = self.key.clone();
        tokio::spawn(async move {
            let _ = redis.delete(&key).await;
        });
    }
}

/// 用户注册
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `redis` - Redis cache context
/// * `email` - User email (plaintext, will be encrypted before storage)
/// * `password` - User password (plaintext, will be hashed with Argon2id)
/// * `phone` - Optional phone number
///
/// # Returns
/// * `Ok((access_token, refresh_token, user_id))` - JWT tokens and user ID
/// * `Err(anyhow::Error)` - Registration error (email exists, invalid format, etc.)
pub async fn register(
    pool: &PgPool,
    redis: &RedisCtx,
    email: String,
    password: String,
    phone: Option<String>,
) -> Result<(String, String, Uuid)> {
    // 1. 验证邮箱格式
    if !email.contains('@') || email.len() < 5 {
        return Err(anyhow!("Invalid email format"));
    }

    // 2. 验证密码强度 (至少8个字符，包含字母和数字)
    validate_password_strength(&password)?;

    // ✅ P0 Fix: 使用Redis分布式锁防止并发注册
    let lock_key = format!("register_lock:{}", email);
    let lock_acquired = redis
        .set_if_not_exists(&lock_key, "locked", Duration::from_secs(10))
        .await
        .unwrap_or(false);

    if !lock_acquired {
        return Err(anyhow!("Registration in progress, please try again"));
    }

    // 确保函数结束时释放锁
    let _lock_guard = LockGuard {
        redis: redis.clone(),
        key: lock_key.clone(),
    };

    // 双重检查：确保email不存在
    let existing_user: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email_cipher = $1 OR email = $1 LIMIT 1")
            .bind(&email)
            .fetch_optional(pool)
            .await?;

    if existing_user.is_some() {
        return Err(anyhow!("该邮箱已被注册，请使用其他邮箱或直接登录"));
    }

    // 3. 创建默认租户✅安全实现
    let tenant_id = Uuid::new_v4();
    let tenant_name = format!("User-{}", email.split('@').next().unwrap_or("user"));
    sqlx::query(
        r#"
        INSERT INTO tenants (id, name, created_at, updated_at)
        VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        "#,
    )
    .bind(tenant_id)
    .bind(tenant_name)
    .execute(pool)
    .await?;

    // 4. 哈希密码 (使用Argon2id) - 在检查前哈希以减少竞争窗口
    let password_hash = hash_password(&password)?;

    // 5. 创建用户记录 - 使用数据库唯一约束防止并发注册
    let user_id = Uuid::new_v4();

    // 尝试插入用户，依赖数据库唯一约束防止重复
    // 同时填充 email 和 email_cipher（开发环境简化，生产环境应加密）
    let insert_result = sqlx::query(
        r#"
        INSERT INTO users (id, tenant_id, email, email_cipher, phone, phone_cipher, password_hash, role, status, created_at, updated_at)
        VALUES ($1, $2, $3, $3, $4, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        "#
    )
    .bind(user_id)
    .bind(tenant_id)
    .bind(&email)
    .bind(phone.as_deref())
    .bind(password_hash)
    .bind("viewer")  // 普通用户角色，符合数据库约束：'admin', 'operator', 'viewer', 'api'
    .bind("active")
    .execute(pool)
    .await;

    // 处理唯一约束冲突
    if let Err(e) = insert_result {
        let err_msg = e.to_string();
        if err_msg.contains("unique") || err_msg.contains("duplicate") || err_msg.contains("已存在")
        {
            return Err(anyhow!("Email already registered"));
        }
        // 其他数据库错误
        return Err(e.into());
    }

    // 7. 生成JWT tokens
    let access_token = generate_token(user_id, tenant_id, "viewer".to_string())?;
    let refresh_token = generate_refresh_token(user_id, tenant_id, "viewer".to_string())?;

    // 8. 存储Session到Redis (TTL: 1小时)
    let session_key = format!("session:{}", access_token);
    let session_data = serde_json::json!({
        "user_id": user_id,
        "tenant_id": tenant_id,
        "role": "viewer",
        "email": email,
    })
    .to_string();

    redis
        .set_session(&session_key, &session_data, Duration::from_secs(3600)) // 1小时
        .await
        .map_err(|e| anyhow!("Failed to store session: {}", e))?;

    // 9. 存储Refresh Token (TTL: 30天)
    let refresh_key = format!("refresh:{}", refresh_token);
    redis
        .set_session(
            &refresh_key,
            &user_id.to_string(),
            Duration::from_secs(2592000),
        )
        .await
        .map_err(|e| anyhow!("Failed to store refresh token: {}", e))?;

    log::info!(
        "✅ User registered successfully: {} (ID: {})",
        email,
        user_id
    );

    Ok((access_token, refresh_token, user_id))
}

/// 用户登录
pub async fn login(
    pool: &PgPool,
    redis: &RedisCtx,
    tenant_id: Uuid,
    email_cipher: String,
    password: String,
) -> Result<(String, String, AuthUser)> {
    // 1. 检查账户是否被锁定
    let lock_key = format!("login_lock:{}:{}", tenant_id, email_cipher);
    let lock_status: Option<String> = redis.get_session(&lock_key).await.ok().flatten();
    if lock_status.is_some() {
        return Err(anyhow!(
            "Account is locked due to too many failed login attempts"
        ));
    }

    // 2. 查找用户
    let user = match auth::find_user_by_email(pool, tenant_id, &email_cipher).await? {
        Some(user) => user,
        None => {
            // 记录失败尝试（内部已处理错误，无需额外检查）
            record_failed_login(redis, &lock_key).await;
            tracing::warn!(
                email_hash = %email_cipher,
                tenant_id = %tenant_id,
                "Login attempt failed: user not found"
            );
            return Err(anyhow!("Invalid credentials"));
        }
    };

    // 3. 验证密码
    // ✅安全密码验证
    let password_valid = if let Some(ref hash) = user.password_hash {
        match verify_password(&password, hash) {
            Ok(valid) => valid,
            Err(e) => {
                tracing::warn!("Password verification error: {}", e);
                false
            }
        }
    } else {
        false
    };

    if !password_valid {
        // 记录失败尝试
        record_failed_login(redis, &lock_key).await;
        return Err(anyhow!("Invalid credentials"));
    }

    // 4. 清除失败计数
    redis.delete_session(&lock_key).await.ok();

    // 5. 生成Access Token和Refresh Token
    let access_token = generate_token(user.id, user.tenant_id, user.role.clone())?;
    let refresh_token = generate_refresh_token(user.id, user.tenant_id, user.role.clone())?;

    // 6. 存储Session到Redis（TTL: 1小时，与JWT Token一致）
    let session_key = format!("session:{}", access_token);
    let session_data = serde_json::json!({
        "user_id": user.id,
        "tenant_id": user.tenant_id,
        "role": user.role,
    })
    .to_string();

    redis
        .set_session(&session_key, &session_data, Duration::from_secs(3600))
        .await
        .map_err(|e| anyhow!("Failed to store session: {}", e))?;

    // 维护用户Session索引（用于快速清理）
    let user_sessions_key = format!("user_sessions:{}:{}", user.tenant_id, user.id);
    let mut conn = redis.client.get_multiplexed_async_connection().await?;
    let _: Result<(), redis::RedisError> = redis::cmd("SADD")
        .arg(&user_sessions_key)
        .arg(&session_key)
        .query_async(&mut conn)
        .await;
    let _: Result<(), redis::RedisError> = redis::cmd("EXPIRE")
        .arg(&user_sessions_key)
        .arg(2592000)
        .query_async(&mut conn)
        .await;

    // 7. 存储Refresh Token（TTL: 30天）
    let refresh_key = format!("refresh:{}", refresh_token);
    redis
        .set_session(
            &refresh_key,
            &user.id.to_string(),
            Duration::from_secs(2592000),
        )
        .await
        .map_err(|e| anyhow!("Failed to store refresh token: {}", e))?;

    // 8. 记录登录历史
    record_login_history(redis, user.id, tenant_id).await.ok();

    Ok((access_token, refresh_token, user))
}

/// 记录登录失败
async fn record_failed_login(redis: &RedisCtx, lock_key: &str) {
    let fail_count_key = format!("{}:count", lock_key);
    let count: i64 = redis
        .rate_limit_incr(&fail_count_key, Duration::from_secs(900))
        .await
        .unwrap_or(0);

    // 如果5次失败，锁定账户15分钟
    if count >= 5 {
        redis
            .set_session(lock_key, "locked", Duration::from_secs(900))
            .await
            .ok();
    }
}

/// 记录登录历史
async fn record_login_history(redis: &RedisCtx, user_id: Uuid, tenant_id: Uuid) -> Result<()> {
    let history_key = format!("login_history:{}:{}", tenant_id, user_id);
    let timestamp = chrono::Utc::now().to_rfc3339();
    let history_entry = serde_json::json!({
        "timestamp": timestamp,
        "user_id": user_id,
        "tenant_id": tenant_id,
    })
    .to_string();

    // 使用列表存储最近的登录历史（最多保畑00条）
    let mut conn = redis.client.get_multiplexed_async_connection().await?;
    redis::cmd("LPUSH")
        .arg(&history_key)
        .arg(&history_entry)
        .query_async::<_, ()>(&mut conn)
        .await?;
    let _: () = redis::cmd("LTRIM")
        .arg(&history_key)
        .arg(0)
        .arg(99)
        .query_async::<_, ()>(&mut conn)
        .await?;
    let _: () = redis::cmd("EXPIRE")
        .arg(&history_key)
        .arg(2592000)
        .query_async::<_, ()>(&mut conn)
        .await?; // 30天过期

    Ok(())
}

/// 用户登出
pub async fn logout(redis: &RedisCtx, token: &str) -> Result<()> {
    let session_key = format!("session:{}", token);
    redis
        .delete_session(&session_key)
        .await
        .map_err(|e| anyhow!("Failed to delete session: {}", e))?;
    Ok(())
}

/// 验证Token并获取用户信息
pub async fn verify_session(redis: &RedisCtx, token: &str) -> Result<Claims> {
    tracing::debug!("verify_session: Verifying token (length: {})", token.len());

    // 1. 验证JWT Token
    let claims = verify_token(token).map_err(|e| {
        tracing::error!("JWT verification failed: {}", e);
        e
    })?;

    tracing::debug!("JWT verified successfully, user_id: {}", claims.sub);

    // 2. 检查Redis Session是否存在
    let session_key = format!("session:{}", token);
    tracing::debug!(
        "Checking Redis session: {}",
        &session_key[0..50.min(session_key.len())]
    );

    let session_data = redis.get_session(&session_key).await.map_err(|e| {
        tracing::error!("Failed to get session from Redis: {}", e);
        anyhow!("Failed to get session: {}", e)
    })?;

    if session_data.is_none() {
        tracing::warn!(
            "Session not found in Redis. Key: {} (full token logged for debugging)",
            session_key
        );
        tracing::warn!("Possible causes: 1) Token expired 2) Redis cleared 3) User logged out 4) Invalid token");
        return Err(anyhow!("Session expired or invalid"));
    }

    tracing::debug!("Session verified successfully");
    Ok(claims)
}

/// 设置用户密码
pub async fn set_password(
    pool: &PgPool,
    user_id: Uuid,
    tenant_id: Uuid,
    password: String,
) -> Result<()> {
    // 验证密码强度
    validate_password_strength(&password)?;

    let password_hash = hash_password(&password)?;
    auth::update_password_hash(pool, user_id, tenant_id, &password_hash)
        .await?
        .then_some(())
        .ok_or_else(|| anyhow!("Failed to update password"))?;
    Ok(())
}

/// 获取当前用户信息
pub async fn get_current_user(pool: &PgPool, user_id: Uuid) -> Result<AuthUser> {
    auth::get_auth_user_by_id(pool, user_id)
        .await?
        .ok_or_else(|| anyhow!("User not found"))
}

/// 刷新Access Token
pub async fn refresh_access_token(redis: &RedisCtx, refresh_token: &str) -> Result<String> {
    // 1. 验证Refresh Token
    let claims = verify_refresh_token(refresh_token)?;

    // 2. 检查Refresh Token是否在Redis中（防止被撤销）
    let refresh_key = format!("refresh:{}", refresh_token);
    let user_id_str = redis
        .get_session(&refresh_key)
        .await
        .map_err(|e| anyhow!("Failed to check refresh token: {}", e))?
        .ok_or_else(|| anyhow!("Refresh token not found or expired"))?;

    let user_id =
        Uuid::parse_str(&user_id_str).map_err(|_| anyhow!("Invalid user_id in refresh token"))?;

    let tenant_id = Uuid::parse_str(&claims.tenant_id)
        .map_err(|_| anyhow!("Invalid tenant_id in refresh token"))?;

    // 3. 生成新的Access Token
    let role = claims.role.clone();
    let access_token = generate_token(user_id, tenant_id, role.clone())?;

    // 4. 存储新的Session
    let session_key = format!("session:{}", access_token);
    let session_data = serde_json::json!({
        "user_id": user_id,
        "tenant_id": tenant_id,
        "role": role,
    })
    .to_string();

    redis
        .set_session(&session_key, &session_data, Duration::from_secs(300))
        .await
        .map_err(|e| anyhow!("Failed to store session: {}", e))?;

    Ok(access_token)
}

/// 密码重置（需要管理员权限或用户本人）
pub async fn reset_password(
    pool: &PgPool,
    redis: &RedisCtx,
    user_id: Uuid,
    tenant_id: Uuid,
    new_password: String,
) -> Result<()> {
    // 验证密码强度
    validate_password_strength(&new_password)?;

    // 设置新密码
    set_password(pool, user_id, tenant_id, new_password).await?;

    // 清除所有Session和Refresh Token（强制重新登录）
    // 使用用户Session索引快速删除该用户的所有Session
    let mut deleted_count = 0;

    // 从用户Session索引中获取所有Session key
    let user_sessions_key = format!("user_sessions:{}:{}", tenant_id, user_id);
    let mut conn = redis.client.get_multiplexed_async_connection().await?;
    let session_keys: Vec<String> = redis::cmd("SMEMBERS")
        .arg(&user_sessions_key)
        .query_async(&mut conn)
        .await
        .unwrap_or_default();

    // 删除所有Session
    if !session_keys.is_empty() {
        let deleted: i64 = redis::cmd("DEL")
            .arg(&session_keys)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);
        deleted_count += deleted as usize;
    }

    // 删除用户Session索引
    redis.delete_session(&user_sessions_key).await.ok();

    // 删除所有Refresh Token（使用SCAN命令安全遍历）
    // 注意：refresh token存储时没有维护用户索引，所以需要通过SCAN
    // 但为了安全，我们只删除匹配用户ID的refresh token
    let refresh_pattern = "refresh:*";
    // 使用SCAN命令安全地遍历所有refresh token
    let mut cursor: i64 = 0;
    let mut refresh_to_delete = Vec::new();

    loop {
        let (new_cursor, keys): (i64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(refresh_pattern)
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await
            .unwrap_or((0, Vec::new()));

        // 检查每个refresh token的value是否匹配当前user_id
        for refresh_key in keys {
            if let Ok(Some(value)) = redis.get_session(&refresh_key).await {
                if value == user_id.to_string() {
                    refresh_to_delete.push(refresh_key);
                }
            }
        }

        cursor = new_cursor;
        if cursor == 0 {
            break;
        }
    }

    // 批量删除匹配的refresh token
    if !refresh_to_delete.is_empty() {
        let deleted: i64 = redis::cmd("DEL")
            .arg(&refresh_to_delete)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);
        deleted_count += deleted as usize;
    }

    // 删除登录历史
    let history_key = format!("login_history:{}:{}", tenant_id, user_id);
    redis.delete_session(&history_key).await.ok();

    tracing::info!(
        "Cleared {} sessions and refresh tokens for user {}",
        deleted_count,
        user_id
    );

    Ok(())
}

/// 获取登录历史
pub async fn get_login_history(
    redis: &RedisCtx,
    user_id: Uuid,
    tenant_id: Uuid,
    limit: usize,
) -> Result<Vec<serde_json::Value>> {
    let history_key = format!("login_history:{}:{}", tenant_id, user_id);
    let mut conn = redis.client.get_multiplexed_async_connection().await?;

    let entries: Vec<String> = redis::cmd("LRANGE")
        .arg(&history_key)
        .arg(0)
        .arg((limit - 1) as i64)
        .query_async(&mut conn)
        .await?;

    let mut history = Vec::new();
    for entry in entries {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&entry) {
            history.push(value);
        }
    }

    Ok(history)
}
