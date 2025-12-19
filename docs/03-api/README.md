# API 设计与文档 (API Design & Documentation)

> 📡 46+ REST API 完整参考、OpenAPI 规范、错误码标准

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [API_REFERENCE.md](./API_REFERENCE.md) | 完整 API 参考文档 | ✅ 核心 |
| [API_ERROR_CODES_STANDARD.md](../../API_ERROR_CODES_STANDARD.md) | 错误码标准 | ✅ 核心 |
| [GAS_ESTIMATION_API_GUIDE.md](../../GAS_ESTIMATION_API_GUIDE.md) | Gas 估算 API 指南 | ✅ 完成 |

---

## 🎯 快速导航

### API 开发者
- 📘 **[API 完整参考](./API_REFERENCE.md)** - 46+ 端点详细说明
- ⚠️ **[错误码标准](../../API_ERROR_CODES_STANDARD.md)** - 所有错误码

### 前端集成
- ⛽ **[Gas 估算 API](../../GAS_ESTIMATION_API_GUIDE.md)** - 手续费估算

---

## 📡 API 架构

### RESTful 设计原则

```
资源 (Resources)
    ↓
动作 (HTTP Methods)
    ├─ GET    - 获取资源
    ├─ POST   - 创建资源
    ├─ PUT    - 更新完整资源
    ├─ PATCH  - 更新部分资源
    └─ DELETE - 删除资源
    ↓
状态码 (Status Codes)
    ├─ 2xx - 成功
    ├─ 4xx - 客户端错误
    └─ 5xx - 服务器错误
```

### API 分类概览

```
┌─────────────────────────────────────────────┐
│          IronCore Backend API               │
│          46+ REST Endpoints                 │
├─────────────────────────────────────────────┤
│                                              │
│  🔐 Auth (3 endpoints)                      │
│     POST   /api/v1/auth/register           │
│     POST   /api/v1/auth/login              │
│     POST   /api/v1/auth/refresh            │
│                                              │
│  👛 Wallets (8 endpoints)                   │
│     GET    /api/v1/wallets                 │
│     GET    /api/v1/wallets/:id             │
│     DELETE /api/v1/wallets/:id             │
│     POST   /api/v1/wallets/batch           │
│     POST   /api/v1/wallets/unlock          │
│     POST   /api/v1/wallets/lock            │
│     GET    /api/v1/wallets/:id/assets      │
│     GET    /api/v1/wallets/assets          │
│                                              │
│  💸 Transactions (6 endpoints)              │
│     GET    /api/v1/transactions            │
│     POST   /api/v1/transactions            │
│     GET    /api/v1/transactions/{hash}/status│
│     GET    /api/v1/transactions/nonce      │
│     GET    /api/v1/transactions/history    │
│     POST   /api/v1/tx                       │
│                                              │
│  🪙 Tokens (5 endpoints)                    │
│     GET    /api/v1/tokens/list             │
│     GET    /api/v1/tokens/:address/info    │
│     GET    /api/v1/tokens/:token_address/balance│
│     GET    /api/v1/tokens/search           │
│     GET    /api/v1/tokens/popular          │
│                                              │
│  🔄 Swap (4 endpoints)                      │
│     GET    /api/v1/swap/quote              │
│     POST   /api/v1/swap/execute            │
│     GET    /api/v1/swap/history            │
│     GET    /api/v1/swap/history/:id        │
│                                              │
│  🔔 Notification (3 endpoints)              │
│     POST   /api/v1/notifications/publish   │
│     GET    /api/v1/notifications/feed      │
│                                              │
│  ⚙️ System (5 endpoints)                    │
│     GET    /api/health                     │
│     GET    /openapi.json                   │
│     GET    /openapi.yaml                   │
│     GET    /docs                           │
│                                              │
└─────────────────────────────────────────────┘
```

---

## 📚 API 文档详解

### 1️⃣ [API 完整参考](./API_REFERENCE.md) ⭐
**适合**: 所有开发人员

**核心内容**:
- 📋 **46+ 端点详细说明** - 请求/响应格式
- 🔐 **认证要求** - 哪些 API 需要 JWT
- 📝 **请求示例** - curl 命令
- 📊 **响应示例** - JSON 格式
- ⚠️ **错误处理** - 错误码说明

**标准响应格式**:
```json
{
  "code": 0,
  "message": "success",
  "data": { "...": "..." }
}
```

**标准错误格式**:
```json
{
  "code": "not_found",
  "message": "Wallet not found",
  "trace_id": "..."
}
```

**认证示例**:
```bash
# 1. 登录获取 Token
curl -X POST http://localhost:8088/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'

# Response
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "...",
    "user": { "id": "...", "email": "user@example.com", "created_at": "..." }
  }
}

# 2. 使用 Token 调用 API
curl -X GET http://localhost:8088/api/v1/wallets \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**阅读时长**: 45 分钟

---

### 2️⃣ [错误码标准](../../API_ERROR_CODES_STANDARD.md) ⭐
**适合**: 前端工程师、测试工程师

**核心内容**:
- ⚠️ **标准错误码** - 100+ 错误码定义
- 📊 **错误分类** - 按模块分类
- 🔍 **错误处理建议** - 如何处理每种错误

**错误码分类**:
| 前缀 | 模块 | 示例 |
|------|------|------|
| `AUTH_*` | 认证 | AUTH_INVALID_TOKEN |
| `WALLET_*` | 钱包 | WALLET_NOT_FOUND |
| `TX_*` | 交易 | TX_INSUFFICIENT_BALANCE |
| `TOKEN_*` | 代币 | TOKEN_NOT_SUPPORTED |
| `NFT_*` | NFT | NFT_NOT_FOUND |
| `SWAP_*` | Swap | SWAP_INSUFFICIENT_LIQUIDITY |
| `PAYMENT_*` | 支付 | PAYMENT_FAILED |
| `SYSTEM_*` | 系统 | SYSTEM_DATABASE_ERROR |

**常见错误码**:
```typescript
// 认证错误
AUTH_INVALID_TOKEN: "Token 无效或已过期"
AUTH_UNAUTHORIZED: "未授权访问"

// 钱包错误
WALLET_NOT_FOUND: "钱包不存在"
WALLET_ALREADY_EXISTS: "钱包已存在"

// 交易错误
TX_INSUFFICIENT_BALANCE: "余额不足"
TX_GAS_TOO_HIGH: "Gas 费用过高"

// 系统错误
SYSTEM_DATABASE_ERROR: "数据库错误"
SYSTEM_RATE_LIMIT: "请求频率超限"
```

**阅读时长**: 15 分钟

---

### 3️⃣ [Gas 估算 API](../../GAS_ESTIMATION_API_GUIDE.md)
**适合**: 前端工程师、区块链集成人员

**核心内容**:
- ⛽ **Gas Price 获取** - 实时 Gas 价格
- 📊 **Gas Limit 估算** - 交易 Gas 估算
- 💰 **手续费计算** - 总费用计算
- 🔄 **EIP-1559 支持** - 基础费 + 优先费

**API 示例**:
```bash
# 多链 Gas 估算（推荐）
curl "http://localhost:8088/api/v1/gas/estimate-all?speed=normal"

# 单链 Gas 估算
curl "http://localhost:8088/api/v1/gas/estimate?chain=ethereum&speed=normal"

# Response（字段以 OpenAPI 为准）
{
  "code": 0,
  "message": "success",
  "data": { "...": "..." }
}
```

**阅读时长**: 10 分钟

---

## 🔍 API 设计原则

### 1. RESTful 最佳实践
- ✅ 使用名词表示资源（`/wallets` 而非 `/getWallets`）
- ✅ 使用 HTTP 方法表示动作（GET, POST, PUT, DELETE）
- ✅ 使用复数形式（`/wallets` 而非 `/wallet`）
- ✅ 使用层级结构（`/wallets/:id/transactions`）
- ✅ 版本控制（`/api/v1/wallets`）

### 2. 命名规范
- ✅ URL 使用小写 + 中划线（`/api/v1/wallet-groups`）
- ✅ JSON 字段使用 snake_case（`user_id`, `created_at`）
- ✅ 错误码使用大写 + 下划线（`WALLET_NOT_FOUND`）

### 3. 分页规范
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "items": [...],
    "pagination": {
      "page": 1,
      "page_size": 20,
      "total_items": 100,
      "total_pages": 5
    }
  }
}
```

### 4. 过滤排序
```
GET /api/v1/wallets?chain=ethereum&sort=created_at:desc&page=1&page_size=20
```

---

## 📊 API 性能指标

| 端点 | 目标延迟 (p95) | 当前延迟 | 状态 |
|------|----------------|----------|------|
| GET /api/v1/wallets | < 50ms | 38ms | ✅ |
| POST /api/v1/wallets/batch | < 100ms | 75ms | ✅ |
| GET /api/v1/transactions | < 80ms | 65ms | ✅ |
| POST /api/v1/transactions | < 200ms | 150ms | ✅ |
| GET /api/v1/swap/quote | < 500ms | 420ms | ✅ |
| GET /api/health | < 10ms | 5ms | ✅ |

---

## 🔗 相关文档

- **系统架构**: [01-architecture/API_ROUTES_MAP.md](../01-architecture/API_ROUTES_MAP.md)
- **认证授权**: [02-configuration/SECURITY.md](../02-configuration/SECURITY.md)
- **错误处理**: [08-error-handling/ERROR_HANDLING.md](../08-error-handling/ERROR_HANDLING.md)
- **测试指南**: [04-testing/API_TESTING.md](../04-testing/API_TESTING.md)

---

**最后更新**: 2025-12-06  
**维护者**: Backend API Team  
**审查者**: API Architect, Lead Engineers
