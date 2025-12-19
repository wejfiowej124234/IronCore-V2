# 📖 API 完整参考手册

> IronCore-V2 Backend API 参考文档（以 `/openapi.yaml` 与 `/docs` 为准）

**基础URL**: `http://localhost:8088`  
**API版本**: v1（除健康检查外统一使用 `/api/v1/...`）  
**认证方式**: JWT Bearer Token

---

## 📋 目录

- [认证 API](#认证-api)
- [多链钱包 API](#多链钱包-api)
- [交易 API](#交易-api)
- [Gas 估算 API](#gas-估算-api)
- [管理员 API](#管理员-api)
- [健康检查 API](#健康检查-api)
- [错误码说明](#错误码说明)

---

## 🔐 认证方式

所有受保护的API需要在请求头中包含JWT Token：

```http
Authorization: Bearer <your_jwt_token>
```

> 响应统一使用 `{ code, message, data }` 包装格式；下文如未特别说明，示例响应展示的是 `data` 字段内容。

**获取Token**: 通过 `/api/v1/auth/login` 接口

---

## 认证 API

### 用户注册

**端点**: `POST /api/v1/auth/register`  
**认证**: 不需要  
**描述**: 创建新用户账户

**请求体**:
```json
{
  "email": "user@example.com",
  "password": "SecurePass123!",
  "tenant_name": "My Company"
}
```

**响应 200**:
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
  "email": "user@example.com",
  "role": "user"
}
```

**错误码**:
- `400` - 参数验证失败
- `409` - 邮箱已存在

---

### 用户登录

**端点**: `POST /api/v1/auth/login`  
**认证**: 不需要  
**描述**: 用户登录获取JWT Token

**请求体**:
```json
{
  "email": "user@example.com",
  "password": "SecurePass123!"
}
```

**响应 200**:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "user": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "role": "user"
  }
}
```

**错误码**:
- `401` - 邮箱或密码错误
- `403` - 账户已被禁用

---

### 刷新Token

**端点**: `POST /api/v1/auth/refresh`  
**认证**: 需要有效的JWT Token  
**描述**: 刷新JWT Token

**响应 200**:
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

---

### 获取当前用户信息

**端点**: `GET /api/v1/auth/me`  
**认证**: 需要JWT Token  
**描述**: 获取当前登录用户的详细信息

**响应 200**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "tenant_id": "660e8400-e29b-41d4-a716-446655440001",
  "email": "user@example.com",
  "role": "user",
  "created_at": "2025-11-24T10:00:00Z"
}
```

---

### 登出

**端点**: `POST /api/v1/auth/logout`  
**认证**: 需要JWT Token  
**描述**: 用户登出（使Token失效）

**响应 200**:
```json
{
  "message": "Logged out successfully"
}
```

---

## 💰 多链钱包 API

### 批量登记钱包（非托管）⭐

**端点**: `POST /api/v1/wallets/batch`  
**认证**: 需要JWT Token  
**描述**: 批量登记多链钱包（后端只接受公开信息：地址/公钥；不接收助记词/私钥/用户密码）

**请求体**:
```json
{
  "wallets": [
    {
      "chain": "ethereum",
      "address": "0xYourDerivedAddress",
      "public_key": "0xYourDerivedPublicKey",
      "name": "My ETH Wallet"
    },
    {
      "chain": "bsc",
      "address": "0xYourDerivedAddressOnBsc",
      "public_key": "0xYourDerivedPublicKeyOnBsc"
    }
  ]
}
```

**响应 200（data）**:
```json
{
  "success": true,
  "wallets": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "chain": "ethereum",
      "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "status": "created",
      "created_at": "2025-11-24T10:00:00Z"
    }
  ],
  "failed": []
}
```

> 说明：旧的“后端生成/导入助记词创建钱包”接口在 IronCore-V2 中不再推荐/不再提供。

**请求体**:
```json
{
  "chains": ["ethereum", "bsc", "polygon"],
  "name_prefix": "My Wallet",
  "mnemonic": "word1 word2 ... word12",
  "account_index": 0
}
```

**响应 200**:
```json
{
  "wallets": [
    {
      "wallet_id": "550e8400-e29b-41d4-a716-446655440000",
      "chain_id": 1,
      "chain_symbol": "ETH",
      "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "name": "My Wallet - ETH"
    },
    {
      "wallet_id": "660e8400-e29b-41d4-a716-446655440001",
      "chain_id": 56,
      "chain_symbol": "BNB",
      "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "name": "My Wallet - BNB"
    },
    {
      "wallet_id": "770e8400-e29b-41d4-a716-446655440002",
      "chain_id": 137,
      "chain_symbol": "MATIC",
      "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "name": "My Wallet - MATIC"
    }
  ],
  "total": 3
}
```

---

### 查询钱包列表

**端点**: `GET /api/v1/wallets`  
**认证**: 需要JWT Token  
**描述**: 获取当前用户的所有钱包

**查询参数**:
- `chain_id` (可选): 按链ID筛选
- `curve_type` (可选): 按曲线类型筛选（Secp256k1, Ed25519, Sr25519）
- `page` (可选): 页码，默认1
- `page_size` (可选): 每页数量，默认20

**请求示例**:
```
GET /api/v1/wallets?chain_id=1&page=1&page_size=10
```

**响应 200**:
```json
{
  "wallets": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "chain_id": 1,
      "chain_symbol": "ETH",
      "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "name": "My ETH Wallet",
      "curve_type": "Secp256k1",
      "created_at": "2025-11-24T10:00:00Z"
    }
  ],
  "total": 1,
  "page": 1,
  "page_size": 10
}
```

---

### 查询单个钱包

**端点**: `GET /api/v1/wallets/:id`  
**认证**: 需要JWT Token  
**描述**: 获取指定钱包的详细信息

**路径参数**:
- `id`: 钱包UUID

**响应 200**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "660e8400-e29b-41d4-a716-446655440001",
  "chain_id": 1,
  "chain_symbol": "ETH",
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "name": "My ETH Wallet",
  "curve_type": "Secp256k1",
  "derivation_path": "m/44'/60'/0'/0/0",
  "account_index": 0,
  "address_index": 0,
  "created_at": "2025-11-24T10:00:00Z"
}
```

**错误码**:
- `404` - 钱包不存在
- `403` - 无权访问此钱包

---

### 获取支持的链列表

**端点**: `GET /api/v1/chains`  
**认证**: 不需要  
**描述**: 获取所有支持的区块链信息

**响应 200**:
```json
{
  "chains": [
    {
      "chain_id": 1,
      "symbol": "ETH",
      "name": "Ethereum Mainnet",
      "curve_type": "Secp256k1",
      "derivation_path": "m/44'/60'/0'/0/0",
      "is_testnet": false
    },
    {
      "chain_id": 56,
      "symbol": "BNB",
      "name": "BNB Smart Chain",
      "curve_type": "Secp256k1",
      "derivation_path": "m/44'/60'/0'/0/0",
      "is_testnet": false
    },
    {
      "chain_id": 11155111,
      "symbol": "ETH",
      "name": "Ethereum Sepolia",
      "curve_type": "Secp256k1",
      "derivation_path": "m/44'/60'/0'/0/0",
      "is_testnet": true
    }
  ],
  "total": 6
}
```

---

### 按曲线分组链信息

**端点**: `GET /api/v1/chains/by-curve`  
**认证**: 不需要  
**描述**: 按加密曲线类型分组返回链信息

**响应 200**:
```json
{
  "Secp256k1": [
    {
      "chain_id": 1,
      "symbol": "ETH",
      "name": "Ethereum Mainnet"
    },
    {
      "chain_id": 56,
      "symbol": "BNB",
      "name": "BNB Smart Chain"
    }
  ],
  "Ed25519": [
    {
      "chain_id": 501,
      "symbol": "SOL",
      "name": "Solana Mainnet"
    }
  ],
  "Sr25519": []
}
```

---

### 地址最小验证（推荐）

IronCore-V2 不提供单独的 `validate-address` 端点。建议使用余额查询进行最小验证：

**端点**: `GET /api/v1/balance`  
**认证**: 不需要  
**描述**: 对地址做最小校验（无效地址通常会返回 400）

**请求示例**:
```
GET /api/v1/balance?chain=ethereum&address=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb
```

---

## 💸 交易 API

### 获取账户Nonce

**端点**: `GET /api/v1/transactions/nonce`  
**认证**: 不需要（公开访问）  
**描述**: 获取Ethereum账户的当前nonce值（用于构建交易）

**查询参数**:
- `address` (必需): 账户地址（0x开头）
- `chain_id` (必需): 链ID（1=ETH, 56=BSC, 137=Polygon）

**请求示例**:
```
GET /api/v1/transactions/nonce?address=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb&chain_id=1
```

**响应 200**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "nonce": 42
  }
}
```

**错误码**:
- `400` - 无效参数
- `500` - RPC错误或服务不可用

---

### 获取交易历史

**端点**: `GET /api/v1/transactions/history`  
**认证**: 不需要（公开访问）  
**描述**: 获取交易历史记录

**查询参数**:
- `page` (可选): 页码，默认1
- `page_size` (可选): 每页数量，默认20

**请求示例**:
```
GET /api/v1/transactions/history?page=1&page_size=20
```

**响应 200**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "transactions": [],
    "total": 0,
    "page": 1,
    "page_size": 20
  }
}
```

---

### 获取Solana最近区块哈希

**端点**: `GET /api/v1/solana/recent-blockhash`  
**认证**: 不需要（公开访问）  
**描述**: 获取Solana网络的最近区块哈希（用于构建交易）

**请求示例**:
```
GET /api/v1/solana/recent-blockhash
```

**响应 200**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "blockhash": "11111111111111111111111111111111"
  }
}
```

**错误码**:
- `500` - RPC错误或服务不可用

---

### 获取TON账户序列号

**端点**: `GET /api/v1/ton/seqno`  
**认证**: 不需要（公开访问）  
**描述**: 获取TON账户的序列号（用于构建交易）

**查询参数**:
- `address` (必需): TON账户地址（EQ开头）

**请求示例**:
```
GET /api/v1/ton/seqno?address=EQD0vdSA_NedR9uvbgN9EikRX-suesDxGeFgBxEO30vqC2KN
```

**响应 200**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "seqno": 0
  }
}
```

**错误码**:
- `400` - 无效参数
- `500` - RPC错误或服务不可用

---

### 发送交易

**端点**: `POST /api/v1/transactions`  
**认证**: 需要JWT Token  
**描述**: 发送区块链交易（需前端签名）

**请求体**:
```json
{
  "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "to": "0x853d35Cc6634C0532925a3b844Bc9e7595f0bEc",
  "amount": "0.1",
  "chain": "ethereum",
  "signed_tx": "0xf86c..."
}
```

**响应 200**:
```json
{
  "tx_hash": "0xabc123...",
  "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "to": "0x853d35Cc6634C0532925a3b844Bc9e7595f0bEc",
  "amount": "0.1",
  "chain": "ethereum",
  "status": "broadcasted",
  "timestamp": "2025-11-24T10:00:00Z"
}
```

---

### 查询交易列表

**端点**: `GET /api/v1/transactions`  
**认证**: 需要JWT Token  
**描述**: 获取当前用户的交易历史

**查询参数**:
- `wallet_id` (可选): 按钱包筛选
- `status` (可选): 按状态筛选（pending, confirmed, failed）
- `page` (可选): 页码
- `page_size` (可选): 每页数量

**响应 200**:
```json
{
  "transactions": [
    {
      "id": "770e8400-e29b-41d4-a716-446655440002",
      "wallet_id": "550e8400-e29b-41d4-a716-446655440000",
      "tx_hash": "0xabc123...",
      "from": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
      "to": "0x853d35Cc6634C0532925a3b844Bc9e7595f0bEc",
      "amount": "0.1",
      "status": "confirmed",
      "created_at": "2025-11-24T10:00:00Z",
      "confirmed_at": "2025-11-24T10:01:00Z"
    }
  ],
  "total": 1
}
```

---

### 广播交易

**端点**: `POST /api/v1/transactions/broadcast`  
**认证**: 不需要（公开访问）  
**描述**: 广播已签名的交易到区块链网络

**请求体**:
```json
{
  "chain": "ethereum",
  "signed_tx": "0xf86c..."
}
```

**响应 200**:
```json
{
  "tx_hash": "0xabc123...",
  "status": "broadcasted"
}
```

---

### 查询交易状态

**端点**: `GET /api/v1/transactions/{hash}/status`  
**认证**: 不需要（公开访问）  
**描述**: 查询交易状态和确认数

**路径参数**:
- `hash` (必需): 交易哈希

**查询参数**:
- `chain` (可选): 链标识，如 `ethereum`, `bsc`, `polygon`

**请求示例**:
```
GET /api/v1/transactions/0xabc123.../status?chain=ethereum
```

**响应 200**:
```json
{
  "tx_hash": "0xabc123...",
  "status": "confirmed",
  "confirmations": 12,
  "last_seen": 1234567890
}
```

---

## ⛽ Gas 估算 API

### 单速度Gas估算

**端点**: `GET /api/v1/gas/estimate`  
**认证**: 不需要  
**描述**: 获取指定链和速度档位的Gas费用估算

**查询参数**:
- `chain` (必需): 链标识（ethereum, bsc, polygon）
- `speed` (可选): 速度档位（slow, normal, fast），默认normal

**请求示例**:
```
GET /api/v1/gas/estimate?chain=ethereum&speed=fast
```

**响应 200**:
```json
{
  "chain": "ethereum",
  "speed": "fast",
  "base_fee": "0x12a05f200",
  "max_priority_fee": "0x3b9aca00",
  "max_fee_per_gas": "0x165a0bc00",
  "estimated_time_seconds": 30,
  "base_fee_gwei": "5.0",
  "max_priority_fee_gwei": "1.0",
  "max_fee_per_gas_gwei": "6.0",
  "cached": false,
  "timestamp": "2025-11-24T10:00:00Z"
}
```

**字段说明**:
- `base_fee`: 基础费用（Wei，十六进制）
- `max_priority_fee`: 优先费用/小费（Wei，十六进制）
- `max_fee_per_gas`: 最大费用（Wei，十六进制）
- `estimated_time_seconds`: 预计确认时间（秒）
- `*_gwei`: Gwei格式（便于显示）
- `cached`: 是否从缓存返回

---

### 所有速度档位Gas估算（推荐）⭐

**端点**: `GET /api/v1/gas/estimate-all`  
**认证**: 不需要  
**描述**: 获取指定链的所有速度档位（slow, normal, fast）的Gas费用估算

**查询参数**:
- `chain` (必需): 链标识（ethereum, bsc, polygon）

**请求示例**:
```
GET /api/v1/gas/estimate-all?chain=ethereum
```

**响应 200**:
```json
{
  "chain": "ethereum",
  "slow": {
    "max_fee_per_gas": "0x12a05f200",
    "max_priority_fee": "0x1dcd6500",
    "max_fee_per_gas_gwei": "5.0",
    "estimated_time_seconds": 300
  },
  "normal": {
    "max_fee_per_gas": "0x165a0bc00",
    "max_priority_fee": "0x3b9aca00",
    "max_fee_per_gas_gwei": "6.0",
    "estimated_time_seconds": 60
  },
  "fast": {
    "max_fee_per_gas": "0x1a13b8600",
    "max_priority_fee": "0x5d21dba00",
    "max_fee_per_gas_gwei": "7.0",
    "estimated_time_seconds": 30
  },
  "timestamp": "2025-11-24T10:00:00Z"
}
```

**⚠️ 注意**: 旧版的 gas suggest 端点已废弃，请使用 `/api/v1/gas/estimate-all`。

---

### 批量Gas估算

当前不提供 `estimate-batch` 批量接口；如需批量获取，请客户端并发调用：

- `GET /api/v1/gas/estimate-all?chain={chain}`（推荐）
- `GET /api/v1/gas/estimate?chain={chain}&speed={speed}`（单档位）

---

## 👨‍💼 管理员 API

> **注意**: 以下API需要管理员权限（role=admin）

### 创建费率规则

**端点**: `POST /api/v1/admin/fee-rules`  
**认证**: 需要JWT Token (Admin)  
**描述**: 创建新的费率规则

**请求体**:
```json
{
  "name": "VIP User Fee",
  "chain_id": 1,
  "fee_type": "percentage",
  "fee_value": "0.001",
  "min_fee": "0.0001",
  "max_fee": "0.1",
  "priority": 10
}
```

**响应 200**:
```json
{
  "rule_id": "880e8400-e29b-41d4-a716-446655440003",
  "name": "VIP User Fee",
  "created_at": "2025-11-24T10:00:00Z"
}
```

---

### 更新费率规则

**端点**: `PUT /api/v1/admin/fee-rules/:id`  
**认证**: 需要JWT Token (Admin)  
**描述**: 更新现有费率规则

---

### 查询所有费率规则

**端点**: `GET /api/v1/admin/fee-rules`  
**认证**: 需要JWT Token (Admin)  
**描述**: 获取所有费率规则列表

**查询参数**:
- `chain_id` (可选): 按链ID筛选
- `active` (可选): 按激活状态筛选

**响应 200**:
```json
{
  "rules": [
    {
      "id": "880e8400-e29b-41d4-a716-446655440003",
      "name": "VIP User Fee",
      "chain_id": 1,
      "fee_type": "percentage",
      "fee_value": "0.001",
      "active": true
    }
  ],
  "total": 1
}
```

---

### 添加RPC端点

**端点**: `POST /api/v1/admin/rpc-endpoints`  
**认证**: 需要JWT Token (Admin)  
**描述**: 添加新的RPC端点

**请求体**:
```json
{
  "chain_id": 1,
  "url": "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY",
  "priority": 1,
  "max_retries": 3,
  "timeout_ms": 5000
}
```

**响应 200**:
```json
{
  "endpoint_id": "990e8400-e29b-41d4-a716-446655440004",
  "url": "https://eth-mainnet.g.alchemy.com/v2/***",
  "status": "active"
}
```

---

### 更新RPC端点状态

**端点**: `PUT /api/v1/admin/rpc-endpoints/:id`  
**认证**: 需要JWT Token (Admin)  
**描述**: 更新RPC端点配置或状态

---

### 删除RPC端点

**端点**: `DELETE /api/v1/admin/rpc-endpoints/:id`  
**认证**: 需要JWT Token (Admin)  
**描述**: 删除指定RPC端点

---

## ❤️ 健康检查 API

### API健康状态

**端点**: `GET /api/health`  
**认证**: 不需要  
**描述**: 检查API服务状态

**响应 200**:
```json
{
  "status": "healthy",
  "version": "0.4.0",
  "timestamp": "2025-11-24T10:00:00Z",
  "services": {
    "database": "ok",
    "redis": "ok",
    "immudb": "ok"
  }
}
```

**响应 503（服务不可用）**:
```json
{
  "status": "unhealthy",
  "version": "0.4.0",
  "timestamp": "2025-11-24T10:00:00Z",
  "services": {
    "database": "error",
    "redis": "ok",
    "immudb": "ok"
  },
  "error": "Database connection failed"
}
```

---

### Kubernetes探针

**端点**: `GET /healthz`  
**认证**: 不需要  
**描述**: 简化的健康检查（用于K8s liveness/readiness probe）

**响应 200**: 空响应体
**响应 503**: 服务不可用

---

### Prometheus指标

**端点**: `GET /metrics`  
**认证**: 不需要  
**描述**: Prometheus格式的监控指标

**响应示例**:
```
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",endpoint="/api/v1/wallets",status="200"} 1234

# HELP http_request_duration_seconds HTTP request duration in seconds
http_requests_total{method="GET",endpoint="/api/v1/wallets",status="200"} 1234

# HELP http_request_duration_seconds HTTP request duration in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{le="0.1"} 1000
```

---

## 📖 文档 API

### OpenAPI规范

**端点**: `GET /openapi.yaml`  
**认证**: 不需要  
**描述**: 获取OpenAPI 3.0规范文档（YAML格式）

---

### Swagger UI

**端点**: `GET /docs`  
**认证**: 不需要  
**描述**: 交互式API文档（Swagger UI界面）

在浏览器访问: `http://localhost:8088/docs`

---

## ⚠️ 错误码说明

### HTTP状态码

| 状态码 | 说明 | 示例 |
|-------|------|------|
| 200 | 成功 | 请求成功处理 |
| 201 | 已创建 | 资源创建成功 |
| 400 | 请求错误 | 参数验证失败 |
| 401 | 未认证 | Token无效或过期 |
| 403 | 无权限 | 没有访问权限 |
| 404 | 未找到 | 资源不存在 |
| 409 | 冲突 | 资源已存在 |
| 429 | 请求过多 | 触发限流 |
| 500 | 服务器错误 | 内部错误 |
| 503 | 服务不可用 | 服务暂时不可用 |

### 错误响应格式

所有错误响应遵循统一格式：

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid email format",
    "details": {
      "field": "email",
      "value": "invalid-email"
    },
    "request_id": "req-abc123",
    "timestamp": "2025-11-24T10:00:00Z"
  }
}
```

### 常见错误码

| 错误码 | HTTP状态 | 说明 |
|-------|---------|------|
| `VALIDATION_ERROR` | 400 | 参数验证失败 |
| `INVALID_CREDENTIALS` | 401 | 用户名或密码错误 |
| `TOKEN_EXPIRED` | 401 | JWT Token已过期 |
| `TOKEN_INVALID` | 401 | JWT Token无效 |
| `INSUFFICIENT_PERMISSIONS` | 403 | 权限不足 |
| `RESOURCE_NOT_FOUND` | 404 | 资源不存在 |
| `WALLET_NOT_FOUND` | 404 | 钱包不存在 |
| `DUPLICATE_EMAIL` | 409 | 邮箱已被注册 |
| `RATE_LIMIT_EXCEEDED` | 429 | 请求频率过高 |
| `INTERNAL_ERROR` | 500 | 内部服务器错误 |
| `DATABASE_ERROR` | 500 | 数据库错误 |
| `RPC_ERROR` | 500 | 区块链RPC调用失败 |

---

## 📝 请求/响应示例

### 完整示例：创建钱包到发送交易

#### 1. 登录获取Token

```bash
curl -X POST http://localhost:8088/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'
```

**响应**:
```json
{
  "access_token": "eyJhbGc...",
  "token_type": "Bearer"
}
```

#### 2. 创建以太坊钱包

```bash
curl -X POST http://localhost:8088/api/v1/wallets/batch \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "wallets": [
      {
        "chain": "ethereum",
        "address": "0xYourDerivedAddress",
        "public_key": "0xYourDerivedPublicKey",
        "name": "My ETH Wallet"
      }
    ]
  }'
```

**响应**:
```json
{
  "wallet_id": "550e8400-e29b-41d4-a716-446655440000",
  "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb",
  "chain_symbol": "ETH"
}
```

#### 3. 查询Gas费用

```bash
curl "http://localhost:8088/api/v1/gas/estimate?chain=ethereum&speed=fast"
```

**响应**:
```json
{
  "max_fee_per_gas_gwei": "6.0",
  "estimated_time_seconds": 30
}
```

#### 4. 发送交易

```bash
curl -X POST http://localhost:8088/api/v1/transactions \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0xYourDerivedAddress",
    "to": "0x853d35Cc6634C0532925a3b844Bc9e7595f0bEc",
    "amount": "0.1",
    "chain": "ethereum",
    "signed_tx": "0xf86c..."
  }'
```

---

## 🔗 相关文档

- [API使用教程](./API_TUTORIAL.md) - 带完整代码示例
- [API路由映射](../01-architecture/API_ROUTES_MAP.md) - 所有路由一览
- [业务逻辑详解](../01-architecture/BUSINESS_LOGIC.md) - 深入理解
- [错误处理指南](../08-error-handling/ERROR_HANDLING.md) - 错误处理最佳实践

---

## 📞 支持

- **API问题**: 查看 [故障排查手册](../00-quickstart/TROUBLESHOOTING.md)
- **新手指南**: 查看 [零基础快速上手](../00-quickstart/README.md)
- **FAQ**: 查看 [常见问题解答](../00-quickstart/FAQ.md)

---

**最后更新**: 2025-11-24  
**API版本**: v0.4.0  
**维护者**: Backend Team
