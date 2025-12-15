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
│     POST   /api/auth/register              │
│     POST   /api/auth/login                 │
│     POST   /api/auth/refresh               │
│                                              │
│  👛 Wallets (8 endpoints)                   │
│     GET    /api/wallets                    │
│     POST   /api/wallets                    │
│     GET    /api/wallets/:id                │
│     PUT    /api/wallets/:id                │
│     DELETE /api/wallets/:id                │
│     POST   /api/wallets/batch              │
│     GET    /api/wallets/:id/balance        │
│     GET    /api/wallets/:id/tokens         │
│                                              │
│  💸 Transactions (6 endpoints)              │
│     GET    /api/transactions               │
│     POST   /api/transactions               │
│     GET    /api/transactions/:id           │
│     GET    /api/wallets/:id/transactions   │
│     POST   /api/transactions/estimate      │
│     POST   /api/transactions/broadcast     │
│                                              │
│  🪙 Tokens (5 endpoints)                    │
│     GET    /api/tokens                     │
│     GET    /api/tokens/:address            │
│     GET    /api/tokens/balance             │
│     GET    /api/tokens/price               │
│     GET    /api/tokens/search              │
│                                              │
│  🎨 NFTs (4 endpoints)                      │
│     GET    /api/nfts                       │
│     GET    /api/nfts/:id                   │
│     GET    /api/wallets/:id/nfts           │
│     POST   /api/nfts/transfer              │
│                                              │
│  🔄 Swap (4 endpoints)                      │
│     POST   /api/swap/quote                 │
│     POST   /api/swap/execute               │
│     GET    /api/swap/history               │
│     GET    /api/swap/pairs                 │
│                                              │
│  💳 Payment (3 endpoints)                   │
│     POST   /api/payments/moonpay/url       │
│     POST   /api/payments/webhook           │
│     GET    /api/payments/status/:id        │
│                                              │
│  👤 User (4 endpoints)                      │
│     GET    /api/users/profile              │
│     PUT    /api/users/profile              │
│     GET    /api/users/settings             │
│     PUT    /api/users/settings             │
│                                              │
│  🔔 Notification (3 endpoints)              │
│     GET    /api/notifications              │
│     PUT    /api/notifications/:id/read     │
│     DELETE /api/notifications/:id          │
│                                              │
│  📊 Stats (5 endpoints)                     │
│     GET    /api/stats/dashboard            │
│     GET    /api/stats/portfolio            │
│     GET    /api/stats/transactions         │
│     GET    /api/stats/tokens               │
│     GET    /api/stats/charts               │
│                                              │
│  ⚙️ System (5 endpoints)                    │
│     GET    /api/health                     │
│     GET    /api/version                    │
│     GET    /api/info                       │
│     GET    /api-docs/openapi.json          │
│     GET    /api-docs/openapi.yaml          │
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
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2025-12-06T12:00:00Z"
}
```

**标准错误格式**:
```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "WALLET_NOT_FOUND",
    "message": "Wallet not found",
    "details": { "wallet_id": "..." }
  },
  "timestamp": "2025-12-06T12:00:00Z"
}
```

**认证示例**:
```bash
# 1. 登录获取 Token
curl -X POST http://localhost:8088/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "password123"}'

# Response
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "...",
    "expires_in": 3600
  }
}

# 2. 使用 Token 调用 API
curl -X GET http://localhost:8088/api/wallets \
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
# 估算交易手续费
POST /api/transactions/estimate
Content-Type: application/json
Authorization: Bearer <token>

{
  "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "to": "0x8ba1f109551bD432803012645Ac136ddd64DBA72",
  "value": "1000000000000000000",  # 1 ETH
  "chain": "ethereum"
}

# Response
{
  "success": true,
  "data": {
    "gas_price": "30000000000",      # 30 Gwei
    "gas_limit": "21000",
    "total_fee": "630000000000000",  # 0.00063 ETH
    "estimated_usd": "2.52",
    "eip1559": {
      "base_fee": "25000000000",
      "priority_fee": "5000000000"
    }
  }
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
- ✅ URL 使用小写 + 中划线（`/api/wallet-groups`）
- ✅ JSON 字段使用 snake_case（`user_id`, `created_at`）
- ✅ 错误码使用大写 + 下划线（`WALLET_NOT_FOUND`）

### 3. 分页规范
```json
{
  "success": true,
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
GET /api/wallets?chain=ethereum&sort=created_at:desc&page=1&page_size=20
```

---

## 📊 API 性能指标

| 端点 | 目标延迟 (p95) | 当前延迟 | 状态 |
|------|----------------|----------|------|
| GET /api/wallets | < 50ms | 38ms | ✅ |
| POST /api/wallets | < 100ms | 75ms | ✅ |
| GET /api/transactions | < 80ms | 65ms | ✅ |
| POST /api/transactions/estimate | < 200ms | 150ms | ✅ |
| POST /api/swap/quote | < 500ms | 420ms | ✅ |
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
