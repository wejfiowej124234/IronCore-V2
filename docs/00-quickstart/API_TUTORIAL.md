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
curl -X POST http://localhost:8088/api/wallets/create \
  -H "Content-Type: application/json" \
  -d '{"mnemonic":"...","chains":["ethereum"]}'
```

**方案2: Postman（图形界面）**
```
1. 下载 Postman: https://www.postman.com/downloads/
2. 导入 OpenAPI: http://localhost:8088/api-docs/openapi.yaml
3. 可视化测试所有API
```

**方案3: JavaScript（前端集成）**
```javascript
const response = await fetch('http://localhost:8088/api/wallets/create', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    mnemonic: '...',
    chains: ['ethereum']
  })
});
const data = await response.json();
```

---

## 认证与授权

### 1. 注册用户

```bash
POST /api/auth/register

curl -X POST http://localhost:8088/api/auth/register \
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
POST /api/auth/login

curl -X POST http://localhost:8088/api/auth/login \
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
curl http://localhost:8088/api/wallets \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
```

**提示**: Token有效期1小时，过期需要重新登录

---

## 钱包管理

### 1. 创建钱包（纯派生，无需认证）

```bash
POST /api/wallets/create

curl -X POST http://localhost:8088/api/wallets/create \
  -H "Content-Type: application/json" \
  -d '{
    "mnemonic": "witch collapse practice feed shame open despair creek road again ice least",
    "chains": ["ethereum", "bitcoin", "solana"]
  }'
```

**响应**:
```json
{
  "wallets": [
    {
      "chain": "ethereum",
      "address": "0x9858EfFD232B4033E47d90003D41EC34EcaEda94",
      "derivation_path": "m/44'/60'/0'/0/0",
      "public_key": "0x04..."
    },
    {
      "chain": "bitcoin",
      "address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
      "derivation_path": "m/84'/0'/0'/0/0",
      "public_key": "02..."
    },
    {
      "chain": "solana",
      "address": "DYw8jCTfwHNRJhhmFcbXvVDTqWMEVFBX6ZKUmG5CNSKK",
      "derivation_path": "m/44'/501'/0'/0'",
      "public_key": "..."
    }
  ]
}
```

**注意**: 
- 这个API不存储任何数据到后端
- 适合快速测试和演示

### 2. 创建钱包（存储元数据）

```bash
POST /api/wallets/unified-create
Authorization: Bearer <token>

curl -X POST http://localhost:8088/api/wallets/unified-create \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Main Wallet",
    "mnemonic": "witch collapse practice...",
    "chains": ["ethereum", "bsc", "polygon"]
  }'
```

**响应**:
```json
{
  "wallet_id": "660e8400-e29b-41d4-a716-446655440001",
  "name": "My Main Wallet",
  "chains": [
    {
      "chain": "ethereum",
      "address": "0x9858...",
      "wallet_record_id": "770e8400-..."
    },
    {
      "chain": "bsc",
      "address": "0x9858...",
      "wallet_record_id": "880e8400-..."
    }
  ],
  "created_at": "2025-11-24T10:00:00Z"
}
```

**优点**:
- 后端存储钱包名称、地址
- 支持跨设备同步
- 适合生产环境

### 3. 查询我的钱包列表

```bash
GET /api/wallets
Authorization: Bearer <token>

curl http://localhost:8088/api/wallets \
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

### 4. 验证地址

```bash
POST /api/wallets/validate-address

curl -X POST http://localhost:8088/api/wallets/validate-address \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "ethereum",
    "address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1"
  }'
```

**响应**:
```json
{
  "valid": true,
  "normalized": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
  "checksum": true
}
```

### 5. 获取支持的链列表

```bash
GET /api/chains

curl http://localhost:8088/api/chains
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
POST /api/transactions/send
Authorization: Bearer <token>

curl -X POST http://localhost:8088/api/transactions/send \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "from_address": "0x9858EfFD232B4033E47d90003D41EC34EcaEda94",
    "to_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
    "value": "0.1",
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
GET /api/transactions/{tx_id}
Authorization: Bearer <token>

curl http://localhost:8088/api/transactions/990e8400-e29b-41d4-a716-446655440003 \
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
GET /api/transactions?address=0x9858...&chain=ethereum&page=1&limit=20
Authorization: Bearer <token>

curl "http://localhost:8088/api/transactions?address=0x9858EfFD232B4033E47d90003D41EC34EcaEda94&chain=ethereum&page=1&limit=20" \
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
GET /api/asset/balance?chain=ethereum&address=0x9858...

curl "http://localhost:8088/api/asset/balance?chain=ethereum&address=0x9858EfFD232B4033E47d90003D41EC34EcaEda94"
```

**响应**:
```json
{
  "chain": "ethereum",
  "address": "0x9858...",
  "balance": "1.5",
  "symbol": "ETH",
  "usd_value": "2400.00",
  "price_per_unit": "1600.00"
}
```

### 2. 查询所有资产（含代币）

```bash
GET /api/wallets/{wallet_id}/assets
Authorization: Bearer <token>

curl http://localhost:8088/api/wallets/660e8400-.../assets \
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

### 3. 查询Gas价格

```bash
GET /api/gas/price?chain=ethereum

curl "http://localhost:8088/api/gas/price?chain=ethereum"
```

**响应**:
```json
{
  "chain": "ethereum",
  "timestamp": "2025-11-24T10:00:00Z",
  "prices": {
    "slow": {
      "gwei": 10,
      "eth": 0.00021,
      "usd": 0.34,
      "estimated_time": "10-30 minutes"
    },
    "normal": {
      "gwei": 20,
      "eth": 0.00042,
      "usd": 0.67,
      "estimated_time": "3-5 minutes"
    },
    "fast": {
      "gwei": 50,
      "eth": 0.00105,
      "usd": 1.68,
      "estimated_time": "30 seconds"
    }
  }
}
```

### 4. 估算Gas费

```bash
POST /api/gas/estimate

curl -X POST http://localhost:8088/api/gas/estimate \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "ethereum",
    "from": "0x9858...",
    "to": "0x742d...",
    "value": "0.1",
    "data": ""
  }'
```

**响应**:
```json
{
  "gas_limit": 21000,
  "gas_price": {
    "slow": 10,
    "normal": 20,
    "fast": 50
  },
  "total_cost": {
    "slow": "0.00021 ETH",
    "normal": "0.00042 ETH",
    "fast": "0.00105 ETH"
  },
  "usd_value": {
    "slow": 0.34,
    "normal": 0.67,
    "fast": 1.68
  }
}
```

---

## 通知管理

### 1. 发布通知

```bash
POST /api/notify/publish
Authorization: Bearer <token>

curl -X POST http://localhost:8088/api/notify/publish \
  -H "Authorization: Bearer eyJhbGc..." \
  -H "Content-Type: application/json" \
  -d '{
    "type": "transaction_confirmed",
    "title": "交易已确认",
    "body": "您的 0.1 ETH 转账已成功确认",
    "data": {
      "tx_hash": "0xabcdef...",
      "amount": "0.1",
      "chain": "ethereum"
    }
  }'
```

### 2. 获取通知列表

```bash
GET /api/notify/feed?page=1&limit=20
Authorization: Bearer <token>

curl "http://localhost:8088/api/notify/feed?page=1&limit=20" \
  -H "Authorization: Bearer eyJhbGc..."
```

**响应**:
```json
{
  "notifications": [
    {
      "id": "aa0e8400-...",
      "type": "transaction_confirmed",
      "title": "交易已确认",
      "body": "您的 0.1 ETH 转账已成功确认",
      "read": false,
      "created_at": "2025-11-24T10:15:00Z"
    }
  ],
  "unread_count": 5,
  "total": 50
}
```

### 3. 标记为已读

```bash
PUT /api/notify/{notification_id}/read
Authorization: Bearer <token>

curl -X PUT http://localhost:8088/api/notify/aa0e8400-.../read \
  -H "Authorization: Bearer eyJhbGc..."
```

---

## 管理员操作

### 1. 创建费用规则（需要Admin角色）

```bash
POST /api/admin/fee-rules
Authorization: Bearer <admin_token>

curl -X POST http://localhost:8088/api/admin/fee-rules \
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
  "message": "Missing required field: mnemonic",
  "details": {
    "field": "mnemonic",
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
    const response = await fetch('http://localhost:8088/api/wallets');
    
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
TOKEN=$(curl -s -X POST http://localhost:8088/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","password":"SecurePass123!"}' \
  | jq -r '.token')

# 2. 创建钱包
WALLET=$(curl -s -X POST http://localhost:8088/api/wallets/unified-create \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Wallet",
    "mnemonic": "witch collapse practice...",
    "chains": ["ethereum"]
  }')

ADDRESS=$(echo $WALLET | jq -r '.chains[0].address')

# 3. 查询余额
curl "http://localhost:8088/api/asset/balance?chain=ethereum&address=$ADDRESS"

# 4. 发送交易（需要客户端签名）
curl -X POST http://localhost:8088/api/transactions/send \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "from_address": "'$ADDRESS'",
    "to_address": "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1",
    "value": "0.1",
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
