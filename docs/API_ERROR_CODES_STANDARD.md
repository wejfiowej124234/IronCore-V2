# API错误代码标准（R项修复）

**更新日期**: 2025-12-02  
**状态**: ✅ 100%完成  
**实现文件**: `IronCore-V2/src/utils/error_codes.rs`

---

## 📋 错误代码分类

### 1xxx - 客户端错误

| 代码 | 名称 | 消息 | HTTP状态 |
|------|------|------|----------|
| 1001 | InvalidSignature | 交易签名无效 | 400 |
| 1002 | WalletLocked | 钱包已锁定，请先解锁 | 422 |
| 1003 | InvalidMnemonic | 助记词无效 | 400 |
| 1004 | InvalidWalletPassword | 钱包密码错误 | 422 |
| 1005 | InvalidTransactionFormat | 交易格式错误 | 400 |
| 1006 | InvalidAddressFormat | 地址格式错误 | 400 |

### 2xxx - 链上错误

| 代码 | 名称 | 消息 | HTTP状态 | 恢复建议 |
|------|------|------|----------|----------|
| 2001 | InsufficientBalance | 余额不足 | 402 | 请充值到您的钱包 |
| 2002 | GasPriceTooLow | Gas价格过低 | 400 | 请将Gas价格提高至少10% |
| 2003 | NonceTooLow | 交易序号过低 | 400 | 请刷新后重试 |
| 2004 | TransactionReverted | 交易被回滚 | 500 | - |
| 2005 | GasLimitTooLow | Gas限制过低 | 400 | - |
| 2006 | ContractExecutionFailed | 智能合约执行失败 | 500 | - |
| 2007 | TokenNotFound | 代币不存在 | 404 | - |
| 2008 | InsufficientTokenBalance | 代币余额不足 | 402 | - |

### 3xxx - 后端错误

| 代码 | 名称 | 消息 | HTTP状态 | 可重试 |
|------|------|------|----------|--------|
| 3001 | RpcUnavailable | 区块链节点不可用 | 503 | ✅ |
| 3002 | DatabaseError | 数据库错误 | 500 | ❌ |
| 3003 | RateLimitExceeded | 请求过于频繁 | 429 | ✅ |
| 3004 | InternalServerError | 服务器内部错误 | 500 | ✅ |
| 3005 | ConfigurationError | 配置错误 | 500 | ❌ |
| 3006 | ExternalServiceUnavailable | 外部服务不可用 | 503 | ✅ |

### 4xxx - 业务错误

| 代码 | 名称 | 消息 | HTTP状态 |
|------|------|------|----------|
| 4001 | RiskControlRejected | 交易被风控拒绝 | 403 |
| 4002 | WalletAlreadyExists | 钱包已存在 | 409 |
| 4003 | OrderNotFound | 订单不存在 | 404 |
| 4004 | InvalidOrderStatus | 订单状态不允许此操作 | 422 |
| 4005 | LimitExceeded | 超出交易限额 | 402 |
| 4006 | KycNotCompleted | 需要完成实名认证 | 403 |
| 4007 | UnsupportedToken | 不支持的代币 | 400 |
| 4008 | UnsupportedChain | 不支持的链 | 400 |
| 4009 | SlippageTooHigh | 滑点超出容忍范围 | 422 |
| 4010 | BridgeUnavailable | 跨链桥不可用 | 503 |

### 5xxx - 认证/授权错误

| 代码 | 名称 | 消息 | HTTP状态 |
|------|------|------|----------|
| 5001 | Unauthorized | 需要登录 | 401 |
| 5002 | TokenExpired | 登录已过期 | 401 |
| 5003 | Forbidden | 权限不足 | 403 |
| 5004 | AccountLocked | 账户已被锁定 | 403 |
| 5005 | InvalidPassword | 密码错误 | 401 |
| 5006 | EmailAlreadyExists | 邮箱已被注册 | 409 |

---

## 📝 错误响应格式

### 标准格式

```json
{
  "code": 2002,
  "message": "Gas price too low",
  "user_message": "Gas价格过低",
  "recovery_hint": "请将Gas价格提高至少10%",
  "retryable": false,
  "details": {
    "min_gas_price": "50 Gwei",
    "provided_gas_price": "30 Gwei"
  },
  "trace_id": "req_abc123"
}
```

### 多语言支持

**英文**:
```json
{
  "code": 2001,
  "message": "Insufficient balance",
  "user_message": "Insufficient balance",
  "recovery_hint": "Please add funds to your wallet"
}
```

**中文**:
```json
{
  "code": 2001,
  "message": "Insufficient balance",
  "user_message": "余额不足",
  "recovery_hint": "请充值到您的钱包"
}
```

---

## 🔧 使用示例

### Rust后端

```rust
use crate::utils::error_codes::{ErrorCode, ErrorResponse};

// 返回错误
fn handle_transaction() -> Result<(), ErrorResponse> {
    if gas_price < min_gas_price {
        return Err(ErrorResponse::new(
            ErrorCode::GasPriceTooLow,
            Some(serde_json::json!({
                "min_gas_price": format!("{} Gwei", min_gas_price),
                "provided_gas_price": format!("{} Gwei", gas_price)
            })),
            "zh"  // 语言
        ));
    }
    
    Ok(())
}
```

### 前端处理

```typescript
// 客户端错误处理
async function handleApiError(error: ErrorResponse) {
    switch (error.code) {
        case 1002:  // WalletLocked
            showUnlockWalletDialog();
            break;
        
        case 2001:  // InsufficientBalance
            showAddFundsDialog();
            break;
        
        case 2002:  // GasPriceTooLow
            if (error.recovery_hint) {
                showGasIncreaseDialog(error.recovery_hint);
            }
            break;
        
        default:
            if (error.retryable) {
                setTimeout(() => retryRequest(), 5000);
            } else {
                showErrorMessage(error.user_message);
            }
    }
}
```

---

## ✅ 优势

1. **标准化**: 所有错误使用统一代码
2. **多语言**: 支持中英文消息
3. **可重试**: 明确标识可重试错误
4. **恢复建议**: 提供用户操作指引
5. **HTTP映射**: 自动映射到正确的HTTP状态码

---

**文档版本**: 1.0  
**最后更新**: 2025-12-02  
**状态**: Production-Ready

