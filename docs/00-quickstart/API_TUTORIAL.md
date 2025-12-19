# 📖 API 使用教程

> 从零开始学会调用后端所有API

## 🎯 学习目标

完成本教程后，你将学会：
- ✅ 使用curl/Postman调用API
- ✅ 理解认证流程（JWT）
- ✅ 创建和管理钱包
- ✅ 发送交易
- ✅ 查询余额和交易历史
- ✅ 处理错误

---

## 📚 目录

1. [准备工作](#准备工作)
2. [认证与授权](#认证与授权)
3. [钱包管理](#钱包管理)
4. [交易操作](#交易操作)
5. [资产查询](#资产查询)
6. [通知管理](#通知管理)
7. [管理员操作](#管理员操作)
8. [错误处理](#错误处理)

---

## 准备工作

> ✅ 路由权威说明：除健康检查外，IronCore-V2 的现行 API 统一使用 `/api/v1/...` 前缀。
> 
> ✅ 非托管原则：**不要把私钥/助记词/密码发送到后端**；后端只接收地址、公钥等公开信息。

### 环境检查

```bash
# 1. 确认服务已启动
curl http://localhost:8088/api/health

# 返回: {"status":"ok"} 表示正常
```

### 工具选择

**方案1: curl（命令行）**
```bash
# 适合：脚本自动化、快速测试
curl http://localhost:8088/api/v1/chains
```

**方案2: Postman（图形界面）**
```
1. 下载 Postman: https://www.postman.com/downloads/
2. 导入 OpenAPI: http://localhost:8088/openapi.yaml
3. 可视化测试所有API
```

**方案3: JavaScript（前端集成）**
```javascript
const response = await fetch('http://localhost:8088/api/v1/chains');
const data = await response.json();
```

---

## 认证与授权

### 1. 注册用户

```bash
POST /api/v1/auth/register

curl -X POST http://localhost:8088/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "email": "alice@example.com",
    "password": "SecurePass123!"
  }'
```

**响应**:
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "alice",
  "email": "alice@example.com",
  "created_at": "2025-11-24T10:00:00Z"
}
```

### 2. 登录获取Token

```bash
POST /api/v1/auth/login

curl -X POST http://localhost:8088/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "SecurePass123!"
  }'
```

**响应**:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2025-11-24T11:00:00Z",
  "user": {
    "id": "550e8400-...",
    "username": "alice",
    "role": "user"
  }
}
```

**保存Token**: 后续请求都需要这个token！

### 3. 使用Token访问受保护API

```bash
# 在请求头加上 Authorization
curl http://localhost:8088/api/v1/wallets \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**提示**: Token有效期1小时，过期需要重新登录

---

## 钱包管理

### 1. 批量登记钱包（非托管，需认证）

```bash
POST /api/v1/wallets/batch
Authorization: Bearer <token>

curl -X POST http://localhost:8088/api/v1/wallets/batch \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "wallets": [
      {
        "chain": "ETH",
        "address": "0x9858EfFD232B4033E47d90003D41EC34EcaEda94",
        "public_key": "0x04...",
        "derivation_path": "m/44\u0027/60\u0027/0\u0027/0/0",
        "curve_type": "secp256k1",
        "name": "My Main Wallet"
      }
    ]
  }'
```

**响应**:
```json
{
  "success": true,
  "wallets": [
    {
      "id": "...",
      "chain": "ETH",
      "address": "0x9858EfFD232B4033E47d90003D41EC34EcaEda94",
      "created_at": "2025-11-24T10:00:00Z",
      "status": "created"
    }
  ],
  "failed": []
}
```

**注意**:
- ✅ 必须由客户端先派生地址/公钥，再调用本接口登记
- ❌ 不要把助记词/私钥发给后端

### 3. 查询我的钱包列表

```bash
GET /api/v1/wallets
Authorization: Bearer <token>

curl http://localhost:8088/api/v1/wallets \
  -H "Authorization: Bearer eyJhbGc..."
```

**响应**:
```json
{
  "wallets": [
    {
      "id": "660e8400-...",
      "name": "My Main Wallet",
      "address": "0x9858...",
      "chain": "ethereum",
      "created_at": "2025-11-24T10:00:00Z"
    },
    {
      "id": "770e8400-...",
      "name": "Trading Wallet",
      "address": "0x1234...",
      "chain": "bsc",
      "created_at": "2025-11-24T10:05:00Z"
    }
  ],
  "total": 2
}
```

### 4. 最小验证：查询余额（用于验证地址可用）

> IronCore-V2 当前不提供独立的“validate-address”接口；
> 推荐使用余额查询作为最小验证（格式错误会返回 400）。

```bash
GET /api/v1/balance?chain=ethereum&address=0x742d...

curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1"
```

### 5. 获取支持的链列表

```bash
GET /api/v1/chains

curl http://localhost:8088/api/v1/chains
```

**响应**:
```json
[
  {
    "name": "Ethereum",
    "key": "ethereum",
    "chain_id": 1,
    "curve": "secp256k1",
    "derivation_path": "m/44'/60'/0'/0/0",
    "native_token": "ETH",
    "testnet": false
  },
  {
    "name": "BSC",
    "key": "bsc",
    "chain_id": 56,
    "curve": "secp256k1",
    "derivation_path": "m/44'/60'/0'/0/0",
    "native_token": "BNB",
    "testnet": false
  }
]
```

---

## 交易操作

### 1. 发送交易

```bash
POST /api/v1/transactions
Authorization: Bearer <token>

curl -X POST http://localhost:8088/api/v1/transactions \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "from": "0x9858EfFD232B4033E47d90003D41EC34EcaEda94",
    "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
    "amount": "0.1",
    "chain": "ethereum",
    "signed_tx": "0xf86c808504a817c800825208947..." 
  }'
```

**注意**: `signed_tx` 必须在客户端签名！

**响应**:
```json
{
  "tx_id": "990e8400-e29b-41d4-a716-446655440003",
  "tx_hash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "status": "broadcasted",
  "chain": "ethereum",
  "from": "0x9858...",
  "to": "0x742d...",
  "value": "0.1",
  "gas_price": "20 gwei",
  "estimated_confirmation": "3-5 minutes"
}
```

### 2. 查询交易状态

```bash
GET /api/v1/transactions/:hash/status
Authorization: Bearer <token>

curl http://localhost:8088/api/v1/transactions/0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890/status \
  -H "Authorization: Bearer eyJhbGc..."
```

**响应**:
```json
{
  "tx_id": "990e8400-...",
  "tx_hash": "0xabcdef...",
  "status": "confirmed",
  "confirmations": 12,
  "block_number": 18500000,
  "timestamp": "2025-11-24T10:15:00Z",
  "gas_used": 21000,
  "actual_fee": "0.00042 ETH"
}
```

**状态说明**:
- `pending` - 等待广播
- `broadcasted` - 已广播
- `confirming` - 确认中
- `confirmed` - 已确认
- `failed` - 失败

### 3. 查询交易历史

```bash
GET /api/v1/transactions
Authorization: Bearer <token>

curl "http://localhost:8088/api/v1/transactions" \
  -H "Authorization: Bearer eyJhbGc..."
```

**响应**:
```json
{
  "transactions": [
    {
      "tx_hash": "0xabcdef...",
      "from": "0x9858...",
      "to": "0x742d...",
      "value": "0.1 ETH",
      "status": "confirmed",
      "timestamp": "2025-11-24T10:15:00Z"
    }
  ],
  "total": 5,
  "page": 1,
  "limit": 20
}
```

---

## 资产查询

### 1. 查询钱包余额

```bash
GET /api/v1/balance?chain=ethereum&address=0x9858...

curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x9858EfFD232B4033E47d90003D41EC34EcaEda94"
```

**响应**:
```json
{
  "balance": "0",
  "chain_id": 1,
  "confirmed": true
}
```

### 2. 查询所有资产（含代币）

```bash
GET /api/v1/wallets/{wallet_id}/assets
Authorization: Bearer <token>

curl http://localhost:8088/api/v1/wallets/660e8400-.../assets \
  -H "Authorization: Bearer eyJhbGc..."
```

**响应**:
```json
{
  "wallet_id": "660e8400-...",
  "chain": "ethereum",
  "address": "0x9858...",
  "total_value_usd": "2900.00",
  "assets": [
    {
      "type": "native",
      "symbol": "ETH",
      "balance": "1.5",
      "decimals": 18,
      "usd_value": "2400.00",
      "price_usd": "1600.00"
    },
    {
      "type": "erc20",
      "symbol": "USDT",
      "contract_address": "0xdac17f958d2ee523a2206206994597c13d831ec7",
      "balance": "500.0",
      "decimals": 6,
      "usd_value": "500.00",
      "price_usd": "1.00"
    }
  ]
}
```

### 3. 估算Gas费（单档）

```bash
GET /api/v1/gas/estimate?chain=ethereum&speed=normal

curl "http://localhost:8088/api/v1/gas/estimate?chain=ethereum&speed=normal"
```

**响应（data 示例）**:
```json
{
  "base_fee": "0x12a05f200",
  "max_priority_fee": "0x1dcd6500",
  "max_fee_per_gas": "0x165a0bc00",
  "estimated_time_seconds": 180,
  "base_fee_gwei": 5.0,
  "max_priority_fee_gwei": 0.5,
  "max_fee_per_gas_gwei": 5.5
}
```

### 4. 估算Gas费

```bash
GET /api/v1/gas/estimate-all?chain=ethereum

curl "http://localhost:8088/api/v1/gas/estimate-all?chain=ethereum"
```

**响应**:
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

---

## 通知管理

### 1. 发布通知

```bash
POST /api/v1/notifications/publish
Authorization: Bearer <admin_token>

curl -X POST http://localhost:8088/api/v1/notifications/publish \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "交易已确认",
    "body": "您的 0.1 ETH 转账已成功确认",
    "category": "transaction",
    "severity": "info",
    "scope": "global"
  }'
```

### 2. 获取通知列表

```bash
GET /api/v1/notifications/feed
Authorization: Bearer <token>

curl "http://localhost:8088/api/v1/notifications/feed" \
  -H "Authorization: Bearer eyJhbGc..."
```

**响应**:
```json
{
  "items": []
}
```

---

## 管理员操作

### 1. 创建费用规则（需要Admin角色）

```bash
POST /api/v1/admin/fee-rules
Authorization: Bearer <admin_token>

curl -X POST http://localhost:8088/api/v1/admin/fee-rules \
  -H "Authorization: Bearer <admin_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "ethereum",
    "fee_type": "percent",
    "percent_bp": 10,
    "min_fee": "0.0001",
    "max_fee": "0.01"
  }'
```

**详见**: [管理员操作手册](../09-admin/ADMIN_GUIDE.md)

---

## 错误处理

### 常见错误响应

**400 Bad Request**:
```json
{
  "error": "InvalidRequest",
  "message": "Missing required field",
  "details": {
    "field": "wallets",
    "reason": "required"
  }
}
```

**401 Unauthorized**:
```json
{
  "error": "Unauthorized",
  "message": "Invalid or expired token"
}
```

**429 Too Many Requests**:
```json
{
  "error": "RateLimitExceeded",
  "message": "Rate limit exceeded: 100 requests per minute",
  "retry_after": 60
}
```

### 错误处理最佳实践

```javascript
// JavaScript示例
async function callAPI() {
  try {
    const response = await fetch('http://localhost:8088/api/v1/wallets');
    
    if (!response.ok) {
      const error = await response.json();
      
      // 根据错误码处理
      switch (response.status) {
        case 401:
          // Token过期，重新登录
          await refreshToken();
          return callAPI(); // 重试
        
        case 429:
          // 限流，等待后重试
          await sleep(error.retry_after * 1000);
          return callAPI();
        
        case 500:
          // 服务器错误，提示用户
          alert('服务暂时不可用，请稍后重试');
          break;
        
        default:
          console.error(error);
      }
    }
    
    return await response.json();
  } catch (e) {
    console.error('Network error:', e);
  }
}
```

---

## 🎓 完整示例：从创建钱包到转账

```bash
# 1. 登录获取token
TOKEN=$(curl -s -X POST http://localhost:8088/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"alice@example.com","password":"SecurePass123!"}' \
  | jq -r '.data.access_token')

# 2. 登记钱包（非托管：只提交地址/公钥；助记词/私钥永远不上传后端）
WALLET_BATCH=$(curl -s -X POST http://localhost:8088/api/v1/wallets/batch \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "wallets": [
      {
        "name": "My Wallet",
        "chain": "ethereum",
        "address": "0xYourDerivedAddress",
        "public_key": "0xYourDerivedPublicKey"
      }
    ]
  }')

ADDRESS=$(echo $WALLET_BATCH | jq -r '.data.wallets[0].address')

# 3. 查询余额
curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=$ADDRESS"

# 4. 发送交易（需要客户端签名）
curl -X POST http://localhost:8088/api/v1/transactions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from": "'$ADDRESS'",
    "to": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
    "amount": "0.1",
    "chain": "ethereum",
    "signed_tx": "0x..."
  }'
```

---

## 📚 下一步

- 查看 [业务逻辑详解](../01-architecture/BUSINESS_LOGIC.md) 理解底层原理
- 查看 [常见问题FAQ](./FAQ.md) 解决常见疑惑
- 查看 [错误处理指南](../08-error-handling/ERROR_HANDLING.md) 学习错误处理

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
