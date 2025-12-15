# 错误处理指南

> ironforge_backend 错误处理完整文档

## 📋 目录

- [错误处理架构](#错误处理架构)
- [错误类型](#错误类型)
- [错误传播](#错误传播)
- [错误响应](#错误响应)
- [错误日志](#错误日志)
- [最佳实践](#最佳实践)

---

## 错误处理架构

### 错误处理流程

```
┌─────────────┐
│   Handler   │  ◄─── 1. 业务逻辑执行
└──────┬──────┘
       │ Error
       ▼
┌─────────────┐
│ Error Map   │  ◄─── 2. 错误映射
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Sanitize   │  ◄─── 3. 错误脱敏
└──────┬──────┘
       │
       ▼
┌─────────────┐
│    Log      │  ◄─── 4. 错误记录
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Response   │  ◄─── 5. 错误响应
└─────────────┘
```

---

## 错误类型

### 1. 自定义错误枚举

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Redis error: {0}")]
    Redis(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    #[error("Authorization error: {0}")]
    Authorization(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("External API error: {0}")]
    ExternalApi(String),
    
    #[error("Internal server error")]
    Internal(#[source] anyhow::Error),
}
```

### 2. 错误码定义

```rust
#[derive(Debug, Clone, Copy, Serialize)]
pub enum ErrorCode {
    // 通用错误 (1000-1999)
    InternalError = 1000,
    ValidationError = 1001,
    NotFound = 1002,
    
    // 认证错误 (2000-2999)
    Unauthorized = 2000,
    InvalidToken = 2001,
    TokenExpired = 2002,
    
    // 授权错误 (3000-3999)
    Forbidden = 3000,
    InsufficientPermissions = 3001,
    
    // 数据库错误 (4000-4999)
    DatabaseError = 4000,
    DuplicateEntry = 4001,
    ForeignKeyViolation = 4002,
    
    // 业务逻辑错误 (5000-5999)
    InsufficientBalance = 5000,
    WalletNotFound = 5001,
    TransactionFailed = 5002,
    
    // 外部服务错误 (6000-6999)
    RpcError = 6000,
    BlockchainError = 6001,
}

impl ErrorCode {
    pub fn as_u32(&self) -> u32 {
        *self as u32
    }
    
    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::InternalError | Self::DatabaseError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ValidationError => StatusCode::BAD_REQUEST,
            Self::NotFound | Self::WalletNotFound => StatusCode::NOT_FOUND,
            Self::Unauthorized | Self::InvalidToken | Self::TokenExpired => StatusCode::UNAUTHORIZED,
            Self::Forbidden | Self::InsufficientPermissions => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
```

### 3. 错误响应结构

```rust
// src/error_body.rs
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub code: u32,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub timestamp: String,
    pub request_id: Option<String>,
}

impl ErrorResponse {
    pub fn new(code: ErrorCode, message: String) -> Self {
        Self {
            code: code.as_u32(),
            message,
            details: None,
            timestamp: Utc::now().to_rfc3339(),
            request_id: None,
        }
    }
    
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
    
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }
}
```

---

## 错误传播

### 1. 使用 ? 操作符

```rust
pub async fn create_wallet(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
) -> Result<Wallet, ApiError> {
    // 验证输入
    validate_wallet_name(name)?;
    
    // 检查用户是否存在
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => ApiError::NotFound("User not found".to_string()),
            _ => ApiError::Database(e),
        })?;
    
    // 创建钱包
    let wallet = sqlx::query_as!(
        Wallet,
        "INSERT INTO wallets (user_id, name) VALUES ($1, $2) RETURNING *",
        user_id,
        name
    )
    .fetch_one(pool)
    .await?;
    
    Ok(wallet)
}
```

### 2. Context 添加上下文

```rust
use anyhow::Context;

pub async fn process_transaction(
    pool: &PgPool,
    tx_id: Uuid,
) -> Result<()> {
    let tx = get_transaction(pool, tx_id)
        .await
        .context(format!("Failed to get transaction {}", tx_id))?;
    
    validate_transaction(&tx)
        .context("Transaction validation failed")?;
    
    submit_to_blockchain(&tx)
        .await
        .context("Failed to submit transaction to blockchain")?;
    
    Ok(())
}
```

### 3. 自定义错误转换

```rust
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                ApiError::NotFound("Resource not found".to_string())
            }
            sqlx::Error::Database(db_err) => {
                if let Some(code) = db_err.code() {
                    if code == "23505" {  // 唯一约束违反
                        return ApiError::Validation("Duplicate entry".to_string());
                    }
                }
                ApiError::Database(err)
            }
            _ => ApiError::Database(err),
        }
    }
}
```

---

## 错误响应

### 1. 实现 IntoResponse

```rust
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            ApiError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::DatabaseError,
                "Database error occurred".to_string(),
            ),
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorCode::NotFound,
                msg,
            ),
            ApiError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorCode::ValidationError,
                msg,
            ),
            ApiError::Authentication(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
                msg,
            ),
            ApiError::Authorization(msg) => (
                StatusCode::FORBIDDEN,
                ErrorCode::Forbidden,
                msg,
            ),
            ApiError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::ValidationError,
                "Rate limit exceeded".to_string(),
            ),
            ApiError::Internal(e) => {
                error!("Internal error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::InternalError,
                    "Internal server error".to_string(),
                )
            }
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorCode::InternalError,
                "An error occurred".to_string(),
            ),
        };
        
        let body = ErrorResponse::new(code, message);
        (status, Json(body)).into_response()
    }
}
```

### 2. 统一错误响应格式

```json
{
  "code": 5001,
  "message": "Wallet not found",
  "details": {
    "wallet_id": "550e8400-e29b-41d4-a716-446655440000"
  },
  "timestamp": "2025-11-24T10:30:00Z",
  "request_id": "req_abc123"
}
```

### 3. 错误响应示例

```rust
// 验证错误
{
  "code": 1001,
  "message": "Validation error",
  "details": {
    "field": "email",
    "reason": "Invalid email format"
  },
  "timestamp": "2025-11-24T10:30:00Z"
}

// 认证错误
{
  "code": 2001,
  "message": "Invalid token",
  "timestamp": "2025-11-24T10:30:00Z"
}

// 业务逻辑错误
{
  "code": 5000,
  "message": "Insufficient balance",
  "details": {
    "required": "100.0",
    "available": "50.0"
  },
  "timestamp": "2025-11-24T10:30:00Z"
}
```

---

## 错误日志

### 1. 结构化错误日志

```rust
use tracing::{error, warn, instrument};

#[instrument(skip(pool))]
pub async fn create_wallet(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
) -> Result<Wallet, ApiError> {
    match do_create_wallet(pool, user_id, name).await {
        Ok(wallet) => {
            info!(
                wallet_id = %wallet.id,
                user_id = %user_id,
                "Wallet created successfully"
            );
            Ok(wallet)
        }
        Err(e) => {
            error!(
                user_id = %user_id,
                wallet_name = name,
                error = %e,
                error_type = ?e,
                "Failed to create wallet"
            );
            Err(e)
        }
    }
}
```

### 2. 错误级别分类

```rust
pub fn log_error(error: &ApiError) {
    match error {
        // ERROR: 需要立即关注
        ApiError::Database(_) | ApiError::Internal(_) => {
            error!("Critical error: {:?}", error);
        }
        
        // WARN: 需要关注但不紧急
        ApiError::ExternalApi(_) | ApiError::Redis(_) => {
            warn!("Service degradation: {:?}", error);
        }
        
        // INFO: 正常业务异常
        ApiError::NotFound(_) | ApiError::Validation(_) => {
            info!("Business error: {:?}", error);
        }
        
        _ => {}
    }
}
```

### 3. 错误审计日志

```rust
pub async fn audit_error(
    immu: &ImmuCtx,
    user_id: Option<Uuid>,
    error: &ApiError,
    request_id: &str,
) -> Result<()> {
    let audit_log = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "request_id": request_id,
        "user_id": user_id.map(|id| id.to_string()),
        "error_type": format!("{:?}", error),
        "error_message": error.to_string(),
    });
    
    let key = format!("audit:error:{}:{}", Utc::now().timestamp(), request_id);
    immu.set(&key, serde_json::to_vec(&audit_log)?).await?;
    
    Ok(())
}
```

---

## 最佳实践

### 1. ✅ 应该做的

#### 使用具体的错误类型

```rust
// ✅ 好：具体的错误
return Err(ApiError::NotFound(format!("Wallet {} not found", wallet_id)));

// ❌ 差：通用的错误
return Err(ApiError::Internal(anyhow!("Error")));
```

#### 添加错误上下文

```rust
// ✅ 好：包含上下文
get_user(pool, user_id)
    .await
    .context(format!("Failed to get user {}", user_id))?;

// ❌ 差：没有上下文
get_user(pool, user_id).await?;
```

#### 脱敏敏感信息

```rust
// ✅ 好：脱敏后的错误
pub fn sanitize_error(error: &anyhow::Error) -> String {
    error.to_string()
        .replace(&env::var("JWT_SECRET").unwrap_or_default(), "***")
        .replace(&env::var("DATABASE_URL").unwrap_or_default(), "***")
}

// ❌ 差：直接返回原始错误
error.to_string()
```

#### 记录完整错误链

```rust
// ✅ 好：记录完整错误链
error!(
    error = %e,
    error_chain = ?e.chain().collect::<Vec<_>>(),
    "Operation failed"
);
```

### 2. ❌ 不应该做的

#### 不要吞没错误

```rust
// ❌ 错误：忽略错误
let _ = update_cache(key, value).await;

// ✅ 正确：处理或传播错误
if let Err(e) = update_cache(key, value).await {
    warn!("Failed to update cache: {}", e);
}
```

#### 不要panic

```rust
// ❌ 错误：使用 panic
let user = get_user(id).unwrap();

// ✅ 正确：返回 Result
let user = get_user(id)?;
```

#### 不要泄露内部信息

```rust
// ❌ 错误：泄露数据库路径
format!("Database error at /var/lib/postgres: {}", e)

// ✅ 正确：通用错误消息
"Database error occurred".to_string()
```

### 3. 错误处理模式

#### 重试模式

```rust
use tokio::time::{sleep, Duration};

pub async fn retry_with_backoff<F, T>(
    mut f: F,
    max_retries: u32,
) -> Result<T, ApiError>
where
    F: FnMut() -> Pin<Box<dyn Future<Output = Result<T, ApiError>>>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if retries < max_retries => {
                warn!("Retry {}/{}: {}", retries + 1, max_retries, e);
                sleep(Duration::from_millis(100 * 2_u64.pow(retries))).await;
                retries += 1;
            }
            Err(e) => return Err(e),
        }
    }
}
```

#### 降级模式

```rust
pub async fn get_balance_with_fallback(
    pool: &PgPool,
    redis: &RedisCtx,
    wallet_id: Uuid,
) -> Result<Decimal> {
    // 尝试从 Redis 获取
    match get_balance_from_redis(redis, wallet_id).await {
        Ok(balance) => return Ok(balance),
        Err(e) => warn!("Redis failed, falling back to database: {}", e),
    }
    
    // 降级到数据库
    get_balance_from_db(pool, wallet_id).await
}
```

#### 熔断模式

```rust
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct CircuitBreaker {
    failure_count: Arc<AtomicU32>,
    threshold: u32,
    timeout: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(AtomicU32::new(0)),
            threshold,
            timeout,
        }
    }
    
    pub async fn call<F, T>(&self, f: F) -> Result<T, ApiError>
    where
        F: Future<Output = Result<T, ApiError>>,
    {
        if self.failure_count.load(Ordering::Relaxed) >= self.threshold {
            return Err(ApiError::ExternalApi("Circuit breaker open".to_string()));
        }
        
        match f.await {
            Ok(result) => {
                self.failure_count.store(0, Ordering::Relaxed);
                Ok(result)
            }
            Err(e) => {
                self.failure_count.fetch_add(1, Ordering::Relaxed);
                Err(e)
            }
        }
    }
}
```

---

## 错误处理检查清单

### 代码层面

- [ ] 使用具体的错误类型
- [ ] 添加错误上下文
- [ ] 脱敏敏感信息
- [ ] 记录完整错误链
- [ ] 避免 panic
- [ ] 正确传播错误

### API 层面

- [ ] 统一错误响应格式
- [ ] 返回正确的 HTTP 状态码
- [ ] 包含错误码和消息
- [ ] 添加 request_id
- [ ] 限制错误详情（生产环境）

### 日志层面

- [ ] 使用结构化日志
- [ ] 正确的日志级别
- [ ] 包含关键上下文
- [ ] 审计重要错误
- [ ] 不记录敏感信息

### 监控层面

- [ ] 监控错误率
- [ ] 设置告警阈值
- [ ] 追踪错误趋势
- [ ] 定期审查错误日志

---

## 常见错误场景

### 1. 数据库错误

```rust
match sqlx::query("...").execute(pool).await {
    Err(sqlx::Error::Database(db_err)) => {
        if let Some(code) = db_err.code() {
            match code.as_ref() {
                "23505" => Err(ApiError::Validation("Duplicate entry".into())),
                "23503" => Err(ApiError::Validation("Foreign key violation".into())),
                _ => Err(ApiError::Database(sqlx::Error::Database(db_err))),
            }
        } else {
            Err(ApiError::Database(sqlx::Error::Database(db_err)))
        }
    }
    Err(e) => Err(ApiError::Database(e)),
    Ok(result) => Ok(result),
}
```

### 2. 外部 API 错误

```rust
match reqwest::get(url).await {
    Ok(resp) if resp.status().is_success() => {
        resp.json().await.map_err(|e| {
            ApiError::ExternalApi(format!("Failed to parse response: {}", e))
        })
    }
    Ok(resp) => {
        Err(ApiError::ExternalApi(format!("HTTP {}: {}", resp.status(), resp.text().await?)))
    }
    Err(e) => {
        Err(ApiError::ExternalApi(format!("Request failed: {}", e)))
    }
}
```

### 3. 验证错误

```rust
pub fn validate_wallet_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::Validation("Wallet name cannot be empty".into()));
    }
    
    if name.len() > 255 {
        return Err(ApiError::Validation("Wallet name too long (max 255 chars)".into()));
    }
    
    Ok(())
}
```

---

## 相关文档

- [安全策略](../02-configuration/SECURITY.md)
- [监控告警](../07-monitoring/MONITORING.md)
- [API 文档](../03-api/API_CLEANUP_SUMMARY.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
