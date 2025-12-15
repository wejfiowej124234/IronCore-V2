# API版本控制策略

**创建日期**: 2025-01-27  
**策略版本**: v1.0  
**适用范围**: 所有IronCore API端点

---

## 版本控制原则

### 1. 版本前缀规则

**当前策略**:
- `/api/` - 无版本前缀，新系统标准端点（推荐使用）
- `/api/v1/` - 企业版端点（保留用于向后兼容）
- `/api/v2/` - 前端兼容接口（已废弃，迁移到 `/api/`）

**未来策略**:
- 新功能统一使用 `/api/` 前缀
- 重大变更时引入新版本（如 `/api/v2/`）
- 旧版本保留至少6个月后移除

### 2. 端点命名规范

**标准格式**:
```
/api/{resource}/{action}
```

**示例**:
- ✅ `/api/wallets/unified-create` - 钱包创建
- ✅ `/api/wallets/:id` - 钱包详情
- ✅ `/api/swap/quote` - 交换报价（同链）
- ✅ `/api/swap/cross-chain-quote` - 跨链兑换报价

### 3. 废弃端点处理

**废弃流程**:
1. 标记为废弃（添加警告日志）
2. 返回410 Gone或保留功能但添加警告
3. 在文档中标记废弃日期
4. 6个月后移除

**当前废弃端点**:
- ⚠️ `POST /api/v1/wallets` - 已废弃，使用 `POST /api/wallets/unified-create`
- ⚠️ `POST /api/swap/quote` (POST) - 已废弃，使用 `POST /api/swap/cross-chain-quote`
- ⚠️ `GET /api/swap/simple-quote` - 已废弃，使用 `GET /api/swap/quote`
- ⚠️ `GET /api/wallet/:address/balance` - 已废弃，使用 `GET /api/wallets/:address/balance`

---

## 参数命名规范

### 链标识参数

**标准**: 统一使用 `chain` 参数，支持多种格式

**支持格式**:
- 链名称: `"ethereum"`, `"bsc"`, `"solana"`
- 链符号: `"ETH"`, `"BNB"`, `"SOL"`
- Chain ID: `"1"`, `"56"`, `"501"`

**示例**:
```json
{
  "chain": "ethereum"  // 或 "ETH" 或 "1"
}
```

### 其他参数

- ✅ `id` - 统一使用id作为标识符
- ✅ `public_key` - 统一使用public_key（而非pubkey）
- ✅ `address` - 统一使用address

---

## 响应格式规范

### 统一响应格式

**成功响应**:
```json
{
  "code": 0,
  "message": "success",
  "data": { ... }
}
```

**错误响应**:
```json
{
  "code": "error_code",
  "message": "error_message",
  "trace_id": "optional_trace_id"
}
```

### 序列化配置

**标准**: 所有Option字段使用 `skip_serializing_if`

```rust
#[derive(Debug, Serialize)]
pub struct Response {
    pub field: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_field: Option<String>,
}
```

---

## 迁移指南

### 钱包创建端点迁移

**旧端点** (已废弃):
```http
POST /api/v1/wallets
```

**新端点** (推荐):
```http
POST /api/wallets/unified-create
```

**迁移步骤**:
1. 更新前端代码使用新端点
2. 验证功能正常
3. 移除旧端点调用

### Swap端点迁移

**同链交换**:
- 旧: `GET /api/swap/simple-quote`
- 新: `GET /api/swap/quote`

**跨链兑换**:
- 旧: `POST /api/swap/quote`
- 新: `POST /api/swap/cross-chain-quote`

---

## 版本演进计划

### 当前版本 (v1)
- 基础功能完整
- 端点统一进行中
- 向后兼容保证

### 未来版本 (v2)
- 完全统一的端点结构
- 移除所有废弃端点
- 增强功能

---

**最后更新**: 2025-01-27  
**维护人员**: 开发团队

