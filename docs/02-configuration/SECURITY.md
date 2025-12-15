# 安全策略与实践

> ironforge_backend 安全设计完整指南

## 📋 目录

- [安全架构](#安全架构)
- [认证与授权](#认证与授权)
- [数据安全](#数据安全)
- [网络安全](#网络安全)
- [密码学](#密码学)
- [安全审计](#安全审计)
- [最佳实践](#最佳实践)

---

## 安全架构

### 多层防御架构

```
┌─────────────────────────────────────────┐
│         1. 网络层（Network Layer）       │
│  - 防火墙                                │
│  - DDoS 防护                             │
│  - TLS/SSL                               │
└────────────────┬────────────────────────┘
                 ▼
┌─────────────────────────────────────────┐
│        2. 应用层（Application Layer）    │
│  - 速率限制                              │
│  - CSRF 保护                             │
│  - 输入验证                              │
└────────────────┬────────────────────────┘
                 ▼
┌─────────────────────────────────────────┐
│         3. 认证层（Auth Layer）          │
│  - JWT 验证                              │
│  - API 密钥                              │
│  - 会话管理                              │
└────────────────┬────────────────────────┘
                 ▼
┌─────────────────────────────────────────┐
│         4. 业务层（Business Layer）      │
│  - 权限控制                              │
│  - 审批流程                              │
│  - 资产隔离                              │
└────────────────┬────────────────────────┘
                 ▼
┌─────────────────────────────────────────┐
│          5. 数据层（Data Layer）         │
│  - 数据加密                              │
│  - 审计日志                              │
│  - 备份恢复                              │
└─────────────────────────────────────────┘
```

### 非托管架构核心原则

⚠️ **关键设计**: 后端**绝不接触私钥**

- 私钥存储: ✅ 客户端（LocalStorage/Secure Enclave）
- 私钥存储: ❌ 后端数据库/缓存/日志
- 交易签名: ✅ 客户端本地签名
- 交易签名: ❌ 后端签名服务

---

## 认证与授权

### JWT 认证

#### JWT 结构

```rust
pub struct Claims {
    pub sub: String,        // 用户ID
    pub email: String,      // 邮箱
    pub exp: usize,         // 过期时间
    pub iat: usize,         // 签发时间
    pub tenant_id: Option<String>, // 租户ID
}
```

#### JWT 配置

```toml
[jwt]
secret = "your-secure-secret-min-32-chars"
token_expiry_secs = 3600  # 1小时
```

#### JWT 生成

```rust
use jsonwebtoken::{encode, EncodingKey, Header};

pub fn generate_jwt(user_id: &str, email: &str) -> Result<String> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(3600))
        .unwrap()
        .timestamp() as usize;
    
    let claims = Claims {
        sub: user_id.to_owned(),
        email: email.to_owned(),
        exp: expiration,
        iat: Utc::now().timestamp() as usize,
        tenant_id: None,
    };
    
    let secret = std::env::var("JWT_SECRET")?;
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    
    Ok(token)
}
```

#### JWT 验证中间件

```rust
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    let auth_header = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| anyhow!("Missing Authorization header"))?;
    
    // 注意：不使用 "Bearer " 前缀
    let token = auth_header.trim();
    
    let claims = verify_jwt(token)?;
    
    // 将用户信息注入请求
    req.extensions_mut().insert(claims);
    
    Ok(next.run(req).await)
}
```

### API 密钥认证

#### API 密钥生成

```rust
use rand::Rng;
use sha2::{Sha256, Digest};

pub fn generate_api_key() -> (String, String) {
    // 生成随机密钥
    let key: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    
    // 计算哈希（存储在数据库）
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let key_hash = format!("{:x}", hasher.finalize());
    
    (key, key_hash)  // 返回明文和哈希
}
```

#### API 密钥验证

```rust
pub async fn verify_api_key(
    pool: &PgPool,
    api_key: &str,
) -> Result<Uuid> {
    let mut hasher = Sha256::new();
    hasher.update(api_key.as_bytes());
    let key_hash = format!("{:x}", hasher.finalize());
    
    let record = sqlx::query!(
        "SELECT user_id, is_active, expires_at FROM api_keys WHERE key_hash = $1",
        key_hash
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| anyhow!("Invalid API key"))?;
    
    if !record.is_active {
        return Err(anyhow!("API key is inactive"));
    }
    
    if let Some(expires_at) = record.expires_at {
        if Utc::now() > expires_at {
            return Err(anyhow!("API key has expired"));
        }
    }
    
    // 更新最后使用时间
    sqlx::query!(
        "UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1",
        key_hash
    )
    .execute(pool)
    .await?;
    
    Ok(record.user_id)
}
```

### 权限控制（RBAC）

#### 角色定义

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Admin,      // 管理员
    User,       // 普通用户
    Approver,   // 审批者
    Viewer,     // 只读用户
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub resource: String,  // "wallets", "transactions"
    pub action: String,    // "read", "write", "delete"
}
```

#### 权限检查中间件

```rust
pub async fn require_permission(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let user_id = Uuid::parse_str(&claims.sub)?;
    
    // 从数据库加载用户权限
    let permissions = load_user_permissions(&state.pool, user_id).await?;
    
    // 检查请求的资源和操作
    let resource = req.uri().path();
    let action = match *req.method() {
        Method::GET => "read",
        Method::POST => "write",
        Method::DELETE => "delete",
        _ => "unknown",
    };
    
    if !has_permission(&permissions, resource, action) {
        return Err(anyhow!("Permission denied"));
    }
    
    Ok(next.run(req).await)
}
```

---

## 数据安全

### 密码哈希（Argon2id）

```rust
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{rand_core::OsRng, SaltString}
};

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    
    Ok(password_hash)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let parsed_hash = PasswordHash::new(hash)?;
    let argon2 = Argon2::default();
    
    Ok(argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}
```

**参数配置：**

- **Memory**: 64 MB
- **Iterations**: 3
- **Parallelism**: 4 threads
- **Salt**: 16 bytes (自动生成)

### 敏感数据加密（AES-256-GCM）

```rust
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce
};

pub fn encrypt_data(plaintext: &str, key: &[u8; 32]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(b"unique nonce"); // 实际使用应随机生成
    
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| anyhow!("Encryption failed: {}", e))?;
    
    Ok(ciphertext)
}

pub fn decrypt_data(ciphertext: &[u8], key: &[u8; 32]) -> Result<String> {
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(b"unique nonce");
    
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow!("Decryption failed: {}", e))?;
    
    Ok(String::from_utf8(plaintext)?)
}
```

### 数据脱敏

```rust
pub fn sanitize_error(error: &anyhow::Error) -> String {
    let error_str = error.to_string();
    
    // 移除敏感信息
    error_str
        .replace(&env::var("JWT_SECRET").unwrap_or_default(), "***")
        .replace(&env::var("DATABASE_URL").unwrap_or_default(), "***")
        .lines()
        .filter(|line| !line.contains("/home/") && !line.contains("C:\\"))
        .collect::<Vec<_>>()
        .join("\n")
}
```

---

## 网络安全

### TLS/SSL 配置

```toml
[server]
bind_addr = "0.0.0.0:8088"
tls_cert_path = "/etc/ssl/certs/server.crt"
tls_key_path = "/etc/ssl/private/server.key"
```

```rust
use axum_server::tls_rustls::RustlsConfig;

let tls_config = RustlsConfig::from_pem_file(
    "/etc/ssl/certs/server.crt",
    "/etc/ssl/private/server.key",
).await?;

axum_server::bind_rustls(addr, tls_config)
    .serve(app.into_make_service())
    .await?;
```

### 速率限制

```rust
use governor::{Quota, RateLimiter};

pub struct RateLimitMiddleware {
    limiter: Arc<RateLimiter<String>>,
}

impl RateLimitMiddleware {
    pub fn new() -> Self {
        let quota = Quota::per_minute(nonzero!(100_u32));
        let limiter = RateLimiter::keyed(quota);
        
        Self {
            limiter: Arc::new(limiter),
        }
    }
    
    pub async fn check(&self, ip: &str) -> Result<()> {
        self.limiter
            .check_key(&ip.to_string())
            .map_err(|_| anyhow!("Rate limit exceeded"))?;
        
        Ok(())
    }
}
```

### CSRF 保护

```rust
use axum::http::header::SET_COOKIE;

pub struct CsrfToken {
    pub token: String,
}

impl CsrfToken {
    pub fn generate() -> Self {
        let token: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        Self { token }
    }
    
    pub fn verify(&self, submitted_token: &str) -> bool {
        self.token == submitted_token
    }
}

// 在响应中设置 CSRF token
pub fn set_csrf_cookie(token: &str) -> HeaderValue {
    format!("csrf_token={}; SameSite=Strict; Secure; HttpOnly", token)
        .parse()
        .unwrap()
}
```

### CORS 配置

```rust
use tower_http::cors::{Any, CorsLayer};

let cors = CorsLayer::new()
    .allow_origin("https://app.ironforge.io".parse::<HeaderValue>()?)
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    .allow_credentials(true);

let app = Router::new()
    .route("/api/health", get(health_check))
    .layer(cors);
```

---

## 密码学

### 区块链签名验证

```rust
use secp256k1::{Secp256k1, Message, PublicKey};

pub fn verify_signature(
    message: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> Result<bool> {
    let secp = Secp256k1::new();
    
    let msg = Message::from_slice(message)?;
    let sig = secp256k1::Signature::from_compact(signature)?;
    let pubkey = PublicKey::from_slice(public_key)?;
    
    Ok(secp.verify(&msg, &sig, &pubkey).is_ok())
}
```

### 随机数生成

```rust
use rand::{rngs::OsRng, RngCore};

pub fn generate_random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    OsRng.fill_bytes(&mut bytes);
    bytes
}
```

---

## 安全审计

### 审计日志（Immudb）

```rust
pub async fn log_audit_event(
    immu: &ImmuCtx,
    event: AuditEvent,
) -> Result<()> {
    let key = format!("audit:{}:{}", event.user_id, event.timestamp);
    let value = serde_json::to_string(&event)?;
    
    immu.set(&key, value.as_bytes()).await?;
    
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub user_id: Uuid,
    pub action: String,
    pub resource: String,
    pub timestamp: DateTime<Utc>,
    pub ip_address: String,
    pub user_agent: String,
    pub result: String,  // "success" or "failure"
}
```

### 安全事件监控

```rust
pub async fn detect_suspicious_activity(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<SecurityAlert>> {
    let mut alerts = Vec::new();
    
    // 检测异常登录位置
    let recent_logins = get_recent_logins(pool, user_id).await?;
    if has_unusual_location(&recent_logins) {
        alerts.push(SecurityAlert::UnusualLocation);
    }
    
    // 检测大额交易
    let recent_tx = get_recent_transactions(pool, user_id).await?;
    if has_large_transaction(&recent_tx) {
        alerts.push(SecurityAlert::LargeTransaction);
    }
    
    // 检测频繁失败的尝试
    let failed_attempts = get_failed_login_attempts(pool, user_id).await?;
    if failed_attempts > 5 {
        alerts.push(SecurityAlert::MultipleFailedLogins);
    }
    
    Ok(alerts)
}
```

---

## 最佳实践

### ✅ 应该做的

1. **使用强密码策略**
   - 最小长度 12 字符
   - 包含大小写字母、数字、特殊字符
   - 定期更换密码

2. **启用多因素认证（MFA）**
   - TOTP（Google Authenticator）
   - SMS 验证码
   - 硬件令牌

3. **最小权限原则**
   - 用户只能访问必需的资源
   - API 密钥限制特定权限

4. **定期安全审计**
   - 代码审查
   - 渗透测试
   - 依赖项漏洞扫描

5. **安全的密钥管理**
   - 使用环境变量存储密钥
   - 生产环境使用 HashiCorp Vault
   - 定期轮换密钥

### ❌ 不应该做的

1. **不要在代码中硬编码密钥**
```rust
// ❌ 错误
let jwt_secret = "hardcoded-secret";

// ✅ 正确
let jwt_secret = env::var("JWT_SECRET")?;
```

2. **不要记录敏感信息**
```rust
// ❌ 错误
log::info!("User password: {}", password);

// ✅ 正确
log::info!("User login attempt for: {}", username);
```

3. **不要使用弱加密算法**
```rust
// ❌ 错误：MD5
use md5::Md5;

// ✅ 正确：Argon2id
use argon2::Argon2;
```

4. **不要忽略错误处理**
```rust
// ❌ 错误
let user = get_user(id).unwrap();

// ✅ 正确
let user = get_user(id).context("Failed to get user")?;
```

5. **不要信任客户端输入**
```rust
// ❌ 错误：直接使用
let amount = req.amount;

// ✅ 正确：验证后使用
let amount = validate_amount(req.amount)?;
```

---

## 安全检查清单

### 部署前检查

- [ ] JWT_SECRET 已设置且足够强（≥32字符）
- [ ] 数据库密码已更换（不使用默认密码）
- [ ] TLS/SSL 已启用
- [ ] CORS 已正确配置
- [ ] 速率限制已启用
- [ ] 日志不包含敏感信息
- [ ] 所有依赖项已更新到最新版本
- [ ] 安全头已设置（CSP、HSTS等）
- [ ] 错误信息已脱敏
- [ ] 审计日志已启用

### 定期检查

- [ ] 每月进行依赖项漏洞扫描
- [ ] 每季度进行渗透测试
- [ ] 每年进行全面安全审计
- [ ] 监控异常登录行为
- [ ] 检查未使用的 API 密钥

---

## 应急响应

### 安全事件处理流程

1. **发现阶段**
   - 监控告警
   - 用户报告
   - 自动检测

2. **遏制阶段**
   - 隔离受影响系统
   - 禁用泄露的密钥
   - 临时关闭受影响功能

3. **根除阶段**
   - 修复漏洞
   - 更新依赖
   - 加固防御

4. **恢复阶段**
   - 恢复服务
   - 验证修复
   - 监控异常

5. **总结阶段**
   - 事后分析
   - 更新流程
   - 培训团队

---

## 相关文档

- [配置管理](./CONFIG_MANAGEMENT.md)
- [数据库模式](./DATABASE_SCHEMA.md)
- [API 文档](../03-api/API_CLEANUP_SUMMARY.md)

---

**最后更新**: 2025-11-24  
**维护者**: Security Team
