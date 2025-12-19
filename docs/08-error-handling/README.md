# 错误处理与日志 (Error Handling & Logging)

> ⚠️ 错误码标准、异常处理、日志规范、故障排查

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [ERROR_HANDLING.md](./ERROR_HANDLING.md) | 错误处理完整指南 | ✅ 核心 |

---

## 🎯 快速导航

### 后端工程师
- ⚠️ **[错误处理指南](./ERROR_HANDLING.md)** - 错误码、异常处理

---

## ⚠️ 错误处理架构

### 错误分类

```
┌─────────────────────────────────────────────┐
│         错误分类 (Error Classification)      │
├─────────────────────────────────────────────┤
│                                              │
│  1️⃣ 客户端错误 (4xx)                       │
│     ├─ 400 Bad Request - 请求参数错误       │
│     ├─ 401 Unauthorized - 未授权            │
│     ├─ 403 Forbidden - 禁止访问             │
│     ├─ 404 Not Found - 资源不存在           │
│     ├─ 409 Conflict - 资源冲突              │
│     ├─ 422 Unprocessable Entity - 参数验证失败 │
│     └─ 429 Too Many Requests - 请求频率超限 │
│                                              │
│  2️⃣ 服务器错误 (5xx)                       │
│     ├─ 500 Internal Server Error - 服务器内部错误 │
│     ├─ 502 Bad Gateway - 网关错误           │
│     ├─ 503 Service Unavailable - 服务不可用 │
│     └─ 504 Gateway Timeout - 网关超时       │
│                                              │
│  3️⃣ 业务错误 (自定义错误码)                 │
│     ├─ AUTH_* - 认证相关错误                │
│     ├─ WALLET_* - 钱包相关错误              │
│     ├─ TX_* - 交易相关错误                  │
│     ├─ TOKEN_* - 代币相关错误               │
│     ├─ NFT_* - NFT 相关错误                 │
│     ├─ SWAP_* - Swap 相关错误               │
│     ├─ PAYMENT_* - 支付相关错误             │
│     └─ SYSTEM_* - 系统相关错误              │
│                                              │
└─────────────────────────────────────────────┘
```

### 错误码结构

```rust
pub enum ErrorCode {
    // 认证错误 (AUTH_*)
    AuthInvalidToken,           // AUTH_INVALID_TOKEN
    AuthExpiredToken,           // AUTH_EXPIRED_TOKEN
    AuthUnauthorized,           // AUTH_UNAUTHORIZED
    AuthInvalidCredentials,     // AUTH_INVALID_CREDENTIALS
    
    // 钱包错误 (WALLET_*)
    WalletNotFound,             // WALLET_NOT_FOUND
    WalletAlreadyExists,        // WALLET_ALREADY_EXISTS
    WalletInvalidAddress,       // WALLET_INVALID_ADDRESS
    
    // 交易错误 (TX_*)
    TxInsufficientBalance,      // TX_INSUFFICIENT_BALANCE
    TxInvalidAmount,            // TX_INVALID_AMOUNT
    TxGasTooHigh,               // TX_GAS_TOO_HIGH
    TxFailed,                   // TX_FAILED
    
    // 代币错误 (TOKEN_*)
    TokenNotSupported,          // TOKEN_NOT_SUPPORTED
    TokenNotFound,              // TOKEN_NOT_FOUND
    
    // 系统错误 (SYSTEM_*)
    SystemDatabaseError,        // SYSTEM_DATABASE_ERROR
    SystemRedisError,           // SYSTEM_REDIS_ERROR
    SystemRateLimit,            // SYSTEM_RATE_LIMIT
}
```

---

## 📚 错误处理文档详解

### 1️⃣ [错误处理指南](./ERROR_HANDLING.md) ⭐
**适合**: 后端工程师、前端工程师

**核心内容**:
- ⚠️ **错误码定义** - 100+ 标准错误码
- 🎯 **错误处理最佳实践** - 错误捕获与传播
- 📝 **错误日志记录** - 结构化日志
- 🔍 **故障排查** - 常见错误排查

**标准错误响应格式**:
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "WALLET_NOT_FOUND",
    "message": "Wallet not found",
    "details": {
      "wallet_id": "550e8400-e29b-41d4-a716-446655440000"
    },
    "trace_id": "abc123xyz"
  },
  "timestamp": "2025-12-06T12:00:00Z"
}
```

**错误处理示例**:
```rust
use anyhow::{Context, Result};
use thiserror::Error;

// 1. 自定义错误类型
#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet not found: {id}")]
    NotFound { id: String },
    
    #[error("Wallet already exists: {address}")]
    AlreadyExists { address: String },
    
    #[error("Invalid wallet address: {address}")]
    InvalidAddress { address: String },
}

// 2. Service 层错误处理
impl WalletService {
    pub async fn get_wallet(&self, id: &str) -> Result<Wallet> {
        self.repository
            .find_by_id(id)
            .await
            .context("Failed to query database")?
            .ok_or_else(|| WalletError::NotFound { id: id.to_string() }.into())
    }
}

// 3. API Handler 错误处理
pub async fn get_wallet_handler(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Wallet>>, ApiError> {
    let wallet = state.wallet_service
        .get_wallet(&id)
        .await
        .map_err(|e| {
            // 记录错误日志
            tracing::error!(
                error = ?e,
                wallet_id = %id,
                "Failed to get wallet"
            );
            
            // 转换为 API 错误
            ApiError::from(e)
        })?;
    
    Ok(Json(ApiResponse::success(wallet)))
}
```

**错误日志示例**:
```json
{
  "timestamp": "2025-12-06T12:00:00.123Z",
  "level": "ERROR",
  "target": "ironforge_backend::api::wallet",
  "message": "Failed to get wallet",
  "fields": {
    "wallet_id": "550e8400-e29b-41d4-a716-446655440000",
    "error": "Wallet not found",
    "trace_id": "abc123xyz",
    "user_id": "123",
    "request_path": "/api/v1/wallets/550e8400"
  }
}
```

**阅读时长**: 30 分钟

---

## 📝 日志规范

### 日志级别

```
TRACE (最详细)
  ├─ 用途: 追踪代码执行路径
  └─ 示例: "Entering function create_wallet"

DEBUG (调试信息)
  ├─ 用途: 开发调试
  └─ 示例: "Database query: SELECT * FROM wallets"

INFO (信息)
  ├─ 用途: 重要业务事件
  └─ 示例: "User created wallet: wallet_id=123"

WARN (警告)
  ├─ 用途: 潜在问题
  └─ 示例: "High memory usage: 85%"

ERROR (错误)
  ├─ 用途: 错误但可恢复
  └─ 示例: "Failed to connect to Redis, using fallback"

CRITICAL (严重错误)
  ├─ 用途: 严重错误，服务受影响
  └─ 示例: "Database connection lost"
```

### 结构化日志

```rust
use tracing::{info, warn, error, instrument};

// 1. 函数级追踪
#[instrument(skip(self), fields(wallet_id = %id))]
pub async fn get_wallet(&self, id: &str) -> Result<Wallet> {
    info!("Getting wallet");
    // ...
}

// 2. 记录业务事件
info!(
    user_id = %user_id,
    wallet_id = %wallet_id,
    chain = %chain,
    "Wallet created successfully"
);

// 3. 记录错误
error!(
    error = ?err,
    wallet_id = %id,
    "Failed to get wallet"
);

// 4. 记录性能指标
warn!(
    duration_ms = duration.as_millis(),
    "Slow query detected"
);
```

---

## 🔍 故障排查指南

### 常见错误排查

| 错误码 | 原因 | 排查步骤 | 解决方案 |
|--------|------|----------|----------|
| `AUTH_INVALID_TOKEN` | Token 无效或过期 | 检查 JWT secret, Token 过期时间 | 刷新 Token |
| `WALLET_NOT_FOUND` | 钱包不存在 | 检查 wallet_id 是否正确 | 确认钱包是否已创建 |
| `TX_INSUFFICIENT_BALANCE` | 余额不足 | 查询钱包余额 | 充值或减少交易金额 |
| `SYSTEM_DATABASE_ERROR` | 数据库错误 | 检查数据库连接、日志 | 重启数据库或检查配置 |
| `SYSTEM_RATE_LIMIT` | 请求频率超限 | 检查 IP、用户请求频率 | 等待限流窗口重置 |

### 错误日志查询

```bash
# 查看最近 100 条错误日志
docker compose logs --tail=100 ironcore | grep ERROR

# 查看特定错误码
docker compose logs ironcore | grep "WALLET_NOT_FOUND"

# 查看特定用户的错误
docker compose logs ironcore | grep "user_id=123" | grep ERROR

# 统计错误数量
docker compose logs ironcore | grep ERROR | wc -l
```

### 使用 Loki 查询日志

```logql
# 查看错误日志
{job="ironcore"} |= "ERROR"

# 查看特定错误码
{job="ironcore"} |= "WALLET_NOT_FOUND"

# 查看特定用户错误
{job="ironcore"} |= "user_id=123" |= "ERROR"

# 统计错误率
rate({job="ironcore"} |= "ERROR" [5m])
```

---

## 📊 错误监控指标

### 错误率监控

```promql
# 总错误率
rate(http_requests_total{status=~"5.."}[5m])

# 按状态码分组
rate(http_requests_total[5m]) by (status)

# 按端点分组
rate(http_requests_total{status=~"5.."}[5m]) by (path)

# 错误率百分比
rate(http_requests_total{status=~"5.."}[5m]) 
  / 
rate(http_requests_total[5m])
```

### 错误统计报表

| 时间段 | 总请求数 | 错误数 | 错误率 | 主要错误 |
|--------|---------|--------|--------|----------|
| 2025-12-06 00:00-01:00 | 12,000 | 60 | 0.5% | SYSTEM_DATABASE_ERROR |
| 2025-12-06 01:00-02:00 | 10,500 | 42 | 0.4% | TX_INSUFFICIENT_BALANCE |
| 2025-12-06 02:00-03:00 | 8,000 | 24 | 0.3% | WALLET_NOT_FOUND |

---

## 🔧 错误处理工具

### Rust 错误处理库

| 库 | 用途 | 文档 |
|----|------|------|
| `anyhow` | 简化错误处理 | https://docs.rs/anyhow |
| `thiserror` | 自定义错误类型 | https://docs.rs/thiserror |
| `tracing` | 结构化日志 | https://docs.rs/tracing |
| `tracing-subscriber` | 日志订阅器 | https://docs.rs/tracing-subscriber |

### 日志配置

```rust
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// 配置日志
tracing_subscriber::registry()
    .with(fmt::layer().json())  // JSON 格式
    .with(EnvFilter::from_default_env())  // 从环境变量读取日志级别
    .init();

// 环境变量配置
// RUST_LOG=info,ironforge_backend=debug
```

---

## 🔗 相关文档

- **API 错误码**: [03-api/API_ERROR_CODES_STANDARD.md](../../API_ERROR_CODES_STANDARD.md)
- **监控告警**: [07-monitoring/MONITORING.md](../07-monitoring/MONITORING.md)
- **运维手册**: [06-operations/OPERATIONS.md](../06-operations/OPERATIONS.md)
- **测试指南**: [04-testing/API_TESTING.md](../04-testing/API_TESTING.md)

---

**最后更新**: 2025-12-06  
**维护者**: Backend Engineering Team  
**审查者**: Backend Lead, SRE Lead
