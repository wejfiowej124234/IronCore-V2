# 业务逻辑详解

> ironforge_backend 核心业务逻辑完整文档

## 📋 目录

- [多链钱包系统](#多链钱包系统)
- [交易流程](#交易流程)
- [费用计算](#费用计算)
- [资产聚合](#资产聚合)
- [跨链兑换](#跨链兑换)
- [通知系统](#通知系统)
- [审批流程](#审批流程)

---

## 多链钱包系统

### 架构概览

```
┌─────────────────────────────────────────┐
│         Multi-Chain Wallet              │
├─────────────────────────────────────────┤
│                                         │
│  BIP39 Mnemonic (12/24 words)          │
│         ↓                               │
│  BIP32 Seed                             │
│         ↓                               │
│  BIP44 Derivation Paths                │
│         ↓                               │
│  ┌─────────┬─────────┬─────────┐      │
│  │   ETH   │   BTC   │ Solana  │ ...  │
│  │secp256k1│secp256k1│ ed25519 │      │
│  └─────────┴─────────┴─────────┘      │
│                                         │
└─────────────────────────────────────────┘
```

### 支持的链

| 链 | 曲线 | 派生路径 | 地址格式 |
|---|------|---------|---------|
| **Ethereum** | secp256k1 | m/44'/60'/0'/0/0 | 0x... (20字节) |
| **BSC** | secp256k1 | m/44'/60'/0'/0/0 | 0x... (20字节) |
| **Polygon** | secp256k1 | m/44'/60'/0'/0/0 | 0x... (20字节) |
| **Bitcoin** | secp256k1 | m/84'/0'/0'/0/0 | bc1... (bech32) |
| **Solana** | ed25519 | m/44'/501'/0'/0' | Base58 (32字节) |
| **TON** | ed25519 | Custom | UQ... (Base64) |

### 钱包创建流程

#### 1. 纯派生模式（不存储私钥）

```rust
POST /api/wallets/create
{
  "mnemonic": "witch collapse practice...",
  "chains": ["ethereum", "bitcoin", "solana"]
}

// 响应
{
  "wallets": [
    {
      "chain": "ethereum",
      "address": "0x1234...",
      "derivation_path": "m/44'/60'/0'/0/0",
      "public_key": "0x04..."
    }
  ]
}
```

**特点**:
- ✅ 完全非托管（后端不存储任何密钥）
- ✅ 客户端自行管理私钥
- ✅ 适用于演示和测试

#### 2. 统一创建模式（存储元数据）

```rust
POST /api/wallets/unified-create
Authorization: Bearer <jwt>
{
  "name": "My Main Wallet",
  "mnemonic": "witch collapse practice...",
  "chains": ["ethereum", "bsc", "polygon"]
}

// 响应
{
  "wallet_id": "550e8400-...",
  "name": "My Main Wallet",
  "chains": [
    {
      "chain": "ethereum",
      "address": "0x1234...",
      "wallet_record_id": "660e8400-..."
    }
  ]
}
```

**特点**:
- ✅ 后端存储：钱包名称、地址、派生路径
- ❌ 后端不存储：私钥、助记词
- ✅ 支持跨设备同步钱包列表
- ✅ 适用于生产环境

### 地址验证

```rust
POST /api/wallets/validate-address
{
  "chain": "ethereum",
  "address": "0x1234567890123456789012345678901234567890"
}

// 响应
{
  "valid": true,
  "normalized": "0x1234567890123456789012345678901234567890"
}
```

### 链信息查询

```rust
GET /api/chains

// 响应
[
  {
    "name": "Ethereum",
    "key": "ethereum",
    "curve": "secp256k1",
    "derivation_path": "m/44'/60'/0'/0/0",
    "chain_id": 1,
    "native_token": "ETH"
  }
]
```

---

## 交易流程

### 完整交易流程

```
1. 用户发起交易
   ↓
2. 前端签名交易（客户端私钥）
   ↓
3. 提交到后端
   ↓
4. 后端验证签名
   ↓
5. 计算平台费用
   ↓
6. 检查审批策略
   ↓ (如需审批)
7. 等待审批
   ↓
8. 广播到区块链
   ↓
9. 监控交易状态
   ↓
10. 更新数据库
    ↓
11. 发送通知
```

### 交易创建

```rust
POST /api/transactions/send
Authorization: Bearer <jwt>
{
  "from_address": "0xABCD...",
  "to_address": "0x1234...",
  "value": "1.0",
  "chain": "ethereum",
  "signed_tx": "0x..." // 客户端签名的交易
}

// 响应
{
  "tx_id": "770e8400-...",
  "tx_hash": "0xabcdef...",
  "status": "pending",
  "estimated_gas": 21000,
  "gas_price": "20 gwei"
}
```

### 交易状态

| 状态 | 说明 |
|-----|------|
| **pending** | 等待广播 |
| **broadcasted** | 已广播到网络 |
| **confirming** | 确认中 |
| **confirmed** | 已确认 |
| **failed** | 失败 |
| **dropped** | 被网络丢弃 |

### 交易监控

系统自动监控交易状态：

```rust
// 每30秒检查一次
async fn monitor_transactions() {
    let pending_txs = get_pending_transactions().await?;
    
    for tx in pending_txs {
        match get_transaction_receipt(tx.tx_hash).await {
            Ok(receipt) if receipt.confirmed => {
                update_tx_status(tx.id, "confirmed").await?;
                send_notification(tx.user_id, "confirmed").await?;
            }
            Ok(receipt) if receipt.block_number > 0 => {
                update_tx_status(tx.id, "confirming").await?;
            }
            Err(_) => {
                // 检查是否超时
                if tx.created_at + 30.minutes < now() {
                    update_tx_status(tx.id, "dropped").await?;
                }
            }
        }
    }
}
```

### 交易重试机制

```rust
// 自动重试失败交易
async fn retry_failed_transaction(tx_id: Uuid) -> Result<()> {
    let tx = get_transaction(tx_id).await?;
    
    // 增加 gas price (bumping)
    let new_gas_price = tx.gas_price * 1.2;
    
    // 重新签名（需要客户端配合）
    let new_signed_tx = request_resign_transaction(tx, new_gas_price).await?;
    
    // 重新广播
    let tx_hash = broadcast_transaction(new_signed_tx).await?;
    
    // 更新记录
    update_transaction(tx_id, tx_hash, new_gas_price).await?;
    
    Ok(())
}
```

---

## 费用计算

### 费用类型

1. **网络费用（Gas Fee）**
   - 由区块链网络收取
   - 支付给矿工/验证者
   - 前端估算，用户承担

2. **平台费用（Platform Fee）**
   - 由平台收取
   - 可配置费率规则
   - 从交易金额中扣除

### 平台费用计算

#### 固定费用

```rust
// 规则配置
{
  "fee_type": "flat",
  "flat_amount": 0.001
}

// 计算
fn calculate_fee(amount: Decimal) -> Decimal {
    Decimal::from_str("0.001").unwrap()
}

// 示例
transfer_amount = 1.0 ETH
platform_fee = 0.001 ETH
actual_transfer = 0.999 ETH
```

#### 百分比费用

```rust
// 规则配置
{
  "fee_type": "percent",
  "percent_bp": 10  // 0.1% (10 基点)
}

// 计算
fn calculate_fee(amount: Decimal) -> Decimal {
    amount * Decimal::from(10) / Decimal::from(10000)
}

// 示例
transfer_amount = 1.0 ETH
platform_fee = 0.001 ETH (0.1%)
actual_transfer = 0.999 ETH
```

#### 混合费用

```rust
// 规则配置
{
  "fee_type": "mixed",
  "flat_amount": 0.0005,
  "percent_bp": 10,
  "min_fee": 0.0003,
  "max_fee": 0.01
}

// 计算
fn calculate_fee(amount: Decimal) -> Decimal {
    let flat = Decimal::from_str("0.0005").unwrap();
    let percent = amount * Decimal::from(10) / Decimal::from(10000);
    let total = flat + percent;
    
    // 应用最小/最大限制
    let min = Decimal::from_str("0.0003").unwrap();
    let max = Decimal::from_str("0.01").unwrap();
    
    total.max(min).min(max)
}
```

### 费用查询API

```rust
GET /api/fees?chain=ethereum&amount=1.0

// 响应
{
  "chain": "ethereum",
  "amount": "1.0",
  "platform_fee": {
    "amount": "0.001",
    "usd_value": "2.50"
  },
  "network_fee": {
    "slow": { "gwei": 10, "eth": 0.00021, "usd": 0.50 },
    "normal": { "gwei": 20, "eth": 0.00042, "usd": 1.00 },
    "fast": { "gwei": 50, "eth": 0.00105, "usd": 2.50 }
  },
  "total": {
    "slow": "1.00121 ETH",
    "normal": "1.00142 ETH", 
    "fast": "1.00205 ETH"
  }
}
```

---

## 资产聚合

### 用户总资产

```rust
GET /api/wallets/assets
Authorization: Bearer <jwt>

// 响应
{
  "total_value_usd": 12500.50,
  "by_chain": [
    {
      "chain": "ethereum",
      "wallets": 3,
      "value_usd": 8500.00,
      "assets": [
        {
          "symbol": "ETH",
          "balance": "5.0",
          "value_usd": 8000.00
        },
        {
          "symbol": "USDT",
          "balance": "500.0",
          "value_usd": 500.00
        }
      ]
    }
  ]
}
```

### 单个钱包资产

```rust
GET /api/wallets/{wallet_id}/assets
Authorization: Bearer <jwt>

// 响应
{
  "wallet_id": "550e8400-...",
  "chain": "ethereum",
  "address": "0x1234...",
  "total_value_usd": 8500.00,
  "assets": [
    {
      "type": "native",
      "symbol": "ETH",
      "balance": "5.0",
      "decimals": 18,
      "value_usd": 8000.00,
      "price_usd": 1600.00
    },
    {
      "type": "erc20",
      "symbol": "USDT",
      "contract_address": "0xdac17...",
      "balance": "500.0",
      "decimals": 6,
      "value_usd": 500.00,
      "price_usd": 1.00
    }
  ]
}
```

### 价格数据源

系统支持多个价格数据源：

1. **CoinGecko API** (默认)
2. **Binance API** (备用)
3. **本地缓存** (15分钟TTL)

```rust
async fn get_token_price(symbol: &str) -> Result<Decimal> {
    // 1. 尝试缓存
    if let Some(cached) = cache.get(&format!("price:{}", symbol)).await {
        return Ok(cached);
    }
    
    // 2. 尝试 CoinGecko
    match fetch_coingecko_price(symbol).await {
        Ok(price) => {
            cache.set(&format!("price:{}", symbol), price, 900).await?;
            return Ok(price);
        }
        Err(e) => warn!("CoinGecko failed: {}", e),
    }
    
    // 3. 降级到 Binance
    let price = fetch_binance_price(symbol).await?;
    cache.set(&format!("price:{}", symbol), price, 900).await?;
    
    Ok(price)
}
```

---

## 跨链兑换

### 兑换流程

```
1. 用户请求报价
   ↓
2. 调用跨链桥 SDK
   ↓
3. 返回最优路径
   ↓
4. 用户确认兑换
   ↓
5. 执行跨链交易
   ↓
6. 监控兑换状态
   ↓
7. 通知用户完成
```

### 获取兑换报价

```rust
POST /api/swap/quote
{
  "from_chain": "ethereum",
  "to_chain": "bsc",
  "from_token": "ETH",
  "to_token": "BNB",
  "amount": "1.0"
}

// 响应
{
  "quote_id": "880e8400-...",
  "from_amount": "1.0 ETH",
  "to_amount": "15.5 BNB",
  "exchange_rate": 15.5,
  "bridge_fee": "0.001 ETH",
  "estimated_time": "10-15 minutes",
  "expires_at": "2025-11-24T10:35:00Z",
  "route": [
    {
      "action": "swap",
      "protocol": "Uniswap",
      "from": "ETH",
      "to": "USDT"
    },
    {
      "action": "bridge",
      "protocol": "Celer cBridge",
      "from_chain": "ethereum",
      "to_chain": "bsc"
    },
    {
      "action": "swap",
      "protocol": "PancakeSwap",
      "from": "USDT",
      "to": "BNB"
    }
  ]
}
```

### 执行跨链兑换

```rust
POST /api/swap/cross-chain
Authorization: Bearer <jwt>
{
  "quote_id": "880e8400-...",
  "signed_tx": "0x..."  // 客户端签名
}

// 响应
{
  "swap_id": "990e8400-...",
  "status": "pending",
  "tx_hashes": {
    "source_chain": "0xabcd...",
    "dest_chain": null  // 待完成
  }
}
```

### 查询兑换状态

```rust
GET /api/swap/{swap_id}
Authorization: Bearer <jwt>

// 响应
{
  "swap_id": "990e8400-...",
  "status": "completed",
  "from_chain": "ethereum",
  "to_chain": "bsc",
  "from_amount": "1.0 ETH",
  "to_amount": "15.5 BNB",
  "tx_hashes": {
    "source_chain": "0xabcd...",
    "dest_chain": "0xef123..."
  },
  "completed_at": "2025-11-24T10:15:00Z"
}
```

---

## 通知系统

### 通知类型

| 类型 | 触发条件 | 示例 |
|-----|---------|------|
| **transaction_confirmed** | 交易确认 | "您的 1.0 ETH 转账已确认" |
| **transaction_failed** | 交易失败 | "交易失败：Gas 不足" |
| **wallet_created** | 钱包创建 | "新钱包已创建" |
| **approval_required** | 需要审批 | "交易需要审批：2.5 ETH" |
| **approval_approved** | 审批通过 | "您的交易已获批" |
| **price_alert** | 价格提醒 | "ETH 价格突破 $2000" |

### 发送通知

```rust
POST /api/notify/publish
Authorization: Bearer <jwt>
{
  "user_id": "550e8400-...",
  "type": "transaction_confirmed",
  "title": "交易已确认",
  "body": "您的 1.0 ETH 转账已成功确认",
  "data": {
    "tx_hash": "0xabcd...",
    "amount": "1.0",
    "chain": "ethereum"
  }
}
```

### 获取通知列表

```rust
GET /api/notify/feed?page=1&limit=20
Authorization: Bearer <jwt>

// 响应
{
  "notifications": [
    {
      "id": "aa0e8400-...",
      "type": "transaction_confirmed",
      "title": "交易已确认",
      "body": "您的 1.0 ETH 转账已成功确认",
      "read": false,
      "created_at": "2025-11-24T10:15:00Z"
    }
  ],
  "unread_count": 5,
  "total": 50
}
```

### 通知偏好设置

```rust
PUT /api/notify/preferences
Authorization: Bearer <jwt>
{
  "email_enabled": true,
  "push_enabled": true,
  "preferences": {
    "transaction_confirmed": {
      "email": true,
      "push": true
    },
    "transaction_failed": {
      "email": true,
      "push": true
    },
    "price_alert": {
      "email": false,
      "push": true
    }
  }
}
```

---

## 审批流程

### 审批策略

管理员可配置审批规则：

```json
{
  "policy_id": "policy-001",
  "name": "大额转账审批",
  "type": "approval",
  "conditions": {
    "operation": "send",
    "min_amount_usd": 1000.00
  },
  "approvers": [
    "user-admin-01",
    "user-admin-02"
  ],
  "required_approvals": 1  // 至少1人批准
}
```

### 审批流程

```
1. 用户发起交易
   ↓
2. 系统检查策略
   ↓ (匹配审批策略)
3. 创建审批请求
   ↓
4. 通知审批者
   ↓
5. 审批者审核
   ↓ (批准)
6. 执行交易
   ↓ (拒绝)
7. 通知用户拒绝原因
```

### 创建审批请求

```rust
POST /api/v1/approvals
Authorization: Bearer <jwt>
{
  "transaction_id": "tx-001",
  "policy_id": "policy-001",
  "reason": "大额转账需要审批"
}
```

### 审批操作

```rust
PUT /api/v1/approvals/{approval_id}/status
Authorization: Bearer <jwt_approver>
{
  "status": "approved",  // 或 "rejected"
  "reason": "已确认交易有效性"
}
```

### 查询待审批列表

```rust
GET /api/v1/approvals?status=pending
Authorization: Bearer <jwt>

// 响应
{
  "approvals": [
    {
      "id": "approval-001",
      "transaction_id": "tx-001",
      "requester": "user-001",
      "amount": "2.5 ETH",
      "status": "pending",
      "created_at": "2025-11-24T10:00:00Z"
    }
  ]
}
```

---

## 事件总线

系统使用事件驱动架构：

```rust
// 事件类型
enum Event {
    WalletCreated { wallet_id: Uuid },
    TransactionBroadcasted { tx_id: Uuid, tx_hash: String },
    TransactionConfirmed { tx_id: Uuid },
    TransactionFailed { tx_id: Uuid, reason: String },
    ApprovalRequired { approval_id: Uuid },
    ApprovalProcessed { approval_id: Uuid, approved: bool },
}

// 发布事件
event_bus.publish(Event::TransactionConfirmed {
    tx_id: tx.id
}).await?;

// 订阅事件
event_bus.subscribe(|event: Event| async move {
    match event {
        Event::TransactionConfirmed { tx_id } => {
            send_notification(tx_id, "confirmed").await?;
        }
        _ => {}
    }
}).await;
```

---

## 相关文档

- [多链钱包架构](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)
- [API 路由映射](../01-architecture/API_ROUTES_MAP.md)
- [管理员指南](../09-admin/ADMIN_GUIDE.md)
- [错误处理](../08-error-handling/ERROR_HANDLING.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
