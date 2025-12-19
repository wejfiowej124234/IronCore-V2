# API版本控制策略

**创建日期**: 2025-01-27  
**策略版本**: v1.0  
**适用范围**: 所有IronCore API端点

---

## 版本控制原则

### 1. 版本前缀规则

**当前策略**:
- `/api/v1/` - 业务 API 标准前缀（推荐使用）
- `/api/health` - 健康检查（历史原因保留为无版本前缀）
- 其他无版本前缀的 API 端点视为历史/兼容，不应在新集成中使用

**未来策略**:
- 新功能统一使用 `/api/v1/` 前缀
- 重大变更时引入新版本（如 v2）
- 旧版本保留周期以发布说明为准

### 2. 端点命名规范

**标准格式**:
```
/api/v1/{resource}/{action}
```

**示例**:
- ✅ `/api/v1/wallets/batch` - 钱包登记（非托管：地址/公钥）
- ✅ `/api/v1/wallets/:id` - 钱包详情
- ✅ `/api/v1/swap/quote` - Swap 报价
- ✅ `/api/v1/bridge/quote` - 跨链报价

### 3. 废弃端点处理

**废弃流程**:
1. 标记为废弃（添加警告日志）
2. 返回410 Gone或保留功能但添加警告
3. 在文档中标记废弃日期
4. 6个月后移除

**当前废弃端点**:
- ⚠️ 旧版无版本前缀端点：已停止推荐/逐步移除
- ⚠️ 助记词/私钥上送后端创建钱包类端点：不符合非托管原则，不再提供

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
- 历史版本曾提供“unified-create”类无版本前缀钱包创建接口；该类接口不符合非托管原则，已停止推荐/逐步移除。

**新端点** (推荐):
```http
POST /api/v1/wallets/batch
```

**迁移步骤**:
1. 客户端本地生成/管理助记词与私钥（非托管）
2. 仅将派生出的地址/公钥通过 `POST /api/v1/wallets/batch` 登记到后端
3. 更新所有调用从旧无版本前缀端点迁移到 `/api/v1/...`

### Swap端点迁移

**Swap 报价/执行**:
- 使用 `/api/v1/swap/quote`、`/api/v1/swap/execute`、`/api/v1/swap/history`
- 如遇历史文档中的旧 swap 无版本前缀端点，统一迁移到 `/api/v1/swap/...`

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

