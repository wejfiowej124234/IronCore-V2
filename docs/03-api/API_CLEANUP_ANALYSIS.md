# API 端点清理分析报告

## 📋 当前 API 端点清单

### ✅ 公开路由（无需认证）

#### 1. 认证相关
- `POST /api/auth/register` - 用户注册
- `POST /api/auth/login` - 用户登录
- `POST /api/auth/refresh` - 刷新Token

#### 2. 健康检查
- `GET /api/health` - API健康检查
- `GET /healthz` - Kubernetes健康探针
- `GET /api/errors` - 错误信息查询
- `GET /metrics` - Prometheus指标

#### 3. 文档
- `GET /openapi.yaml` - OpenAPI规范
- `GET /docs` - Swagger UI文档

#### 4. **多链钱包 API（新增）**
- `POST /api/wallets/create` - 纯派生钱包（不存储）✨
- `POST /api/wallets/create-multi` - 批量多链派生 ✨
- `POST /api/wallets/unified-create` - **统一创建（派生+存储）** ✨ **推荐**
- `POST /api/v2/wallets/create` - 前端兼容 API ✨
- `GET /api/chains` - 链信息列表 ✨
- `GET /api/chains/by-curve` - 按曲线分组 ✨
- `POST /api/wallets/validate-address` - 地址验证 ✨

---

### 🔒 受保护路由（需要认证）

#### 1. 认证管理
- `POST /api/auth/logout` - 登出
- `GET /api/auth/me` - 获取当前用户信息
- `POST /api/auth/set-password` - 设置密码
- `POST /api/auth/reset-password` - 重置密码
- `GET /api/auth/login-history` - 登录历史

#### 2. **简化钱包 API（IronForge 兼容）** ⚠️
- `POST /api/wallets` - 创建钱包（简化版）
- `GET /api/wallets` - 钱包列表
- `GET /api/wallets/:id` - 钱包详情
- `DELETE /api/wallets/:id` - 删除钱包

#### 3. **企业级钱包 API（v1）** ⚠️
- `POST /api/v1/wallets` - 创建钱包（企业版）
- `GET /api/v1/wallets` - 钱包列表
- `GET /api/v1/wallets/:id` - 钱包详情
- `DELETE /api/v1/wallets/:id` - 删除钱包

#### 4. 交易 API
- `POST /api/transactions/send` - 发送交易（简化）
- `GET /api/transactions` - 交易列表（简化）
- `POST /api/v1/tx` - 创建交易
- `GET /api/v1/tx` - 交易列表
- `GET /api/v1/tx/:id` - 交易详情
- `PUT /api/v1/tx/:id/status` - 更新交易状态

#### 5. 租户管理
- `POST /api/v1/tenants` - 创建租户
- `GET /api/v1/tenants` - 租户列表
- `GET /api/v1/tenants/:id` - 租户详情
- `PUT /api/v1/tenants/:id` - 更新租户
- `DELETE /api/v1/tenants/:id` - 删除租户

#### 6. 用户管理
- `POST /api/v1/users` - 创建用户
- `GET /api/v1/users` - 用户列表
- `GET /api/v1/users/:id` - 用户详情
- `PUT /api/v1/users/:id` - 更新用户
- `DELETE /api/v1/users/:id` - 删除用户

#### 7. 策略管理
- `POST /api/v1/policies` - 创建策略
- `GET /api/v1/policies` - 策略列表
- `GET /api/v1/policies/:id` - 策略详情
- `PUT /api/v1/policies/:id` - 更新策略
- `DELETE /api/v1/policies/:id` - 删除策略

#### 8. 审批管理
- `POST /api/v1/approvals` - 创建审批
- `GET /api/v1/approvals` - 审批列表
- `GET /api/v1/approvals/:id` - 审批详情
- `PUT /api/v1/approvals/:id/status` - 更新审批状态
- `DELETE /api/v1/approvals/:id` - 删除审批

#### 9. API密钥管理
- `POST /api/v1/api-keys` - 创建API密钥
- `GET /api/v1/api-keys` - API密钥列表
- `GET /api/v1/api-keys/:id` - API密钥详情
- `PUT /api/v1/api-keys/:id/status` - 更新密钥状态
- `DELETE /api/v1/api-keys/:id` - 删除密钥

#### 10. 交易广播
- `POST /api/v1/tx-broadcasts` - 创建交易广播
- `GET /api/v1/tx-broadcasts` - 广播列表
- `GET /api/v1/tx-broadcasts/:id` - 广播详情
- `PUT /api/v1/tx-broadcasts/:id` - 更新广播
- `GET /api/v1/tx-broadcasts/by-tx-hash/:hash` - 按哈希查询

#### 11. 区块链查询
- `GET /api/fees` - Gas费用查询
- `GET /api/gas/suggest` - Gas建议
- `GET /api/network/status` - 网络状态
- `GET /balance` - 余额查询

---

## ⚠️ 潜在重复/冲突分析

### 问题 1: 钱包创建 API 重复（3 套系统）

#### 系统 A: 新多链钱包 API（推荐使用）✨
```
POST /api/wallets/unified-create       # 统一创建（派生+存储）⭐ 推荐
POST /api/wallets/create                # 纯派生（不存储）
POST /api/wallets/create-multi          # 批量多链
POST /api/v2/wallets/create             # 前端兼容
```
**特点**:
- ✅ 支持 8 条链（ETH, BSC, Polygon, BTC, SOL, ADA, DOT）
- ✅ 自动派生地址
- ✅ 数据库存储元数据
- ✅ 响应时间 22-33ms
- ✅ 包含完整链信息（curve_type, derivation_path）

#### 系统 B: 简化钱包 API（IronForge 前端使用）⚠️
```
POST /api/wallets                       # 创建钱包
GET /api/wallets                        # 钱包列表
GET /api/wallets/:id                    # 钱包详情
DELETE /api/wallets/:id                 # 删除钱包
```
**特点**:
- ⚠️ 只存储地址，不派生
- ⚠️ 需要从 JWT 提取 tenant_id/user_id
- ⚠️ 缺少多链字段（derivation_path, curve_type）
- ⚠️ 映射链名称到 chain_id（仅支持 ETH, BSC, Polygon）

**在 `handlers.rs` 中实现**: `simple_create_wallet()` (line 2172)

#### 系统 C: 企业级钱包 API（v1 版本）⚠️
```
POST /api/v1/wallets                    # 创建钱包
GET /api/v1/wallets                     # 钱包列表
GET /api/v1/wallets/:id                 # 钱包详情
DELETE /api/v1/wallets/:id              # 删除钱包
```
**特点**:
- ⚠️ 企业级功能（需要 tenant_id, policy_id）
- ⚠️ 只存储地址，不派生
- ⚠️ 不支持多链字段
- ⚠️ 需要完整的 tenant/user 上下文

**在 `handlers.rs` 中实现**: `create_wallet()` (line 48)

---

### 问题 2: 路径冲突风险

#### 冲突点 1: `/api/wallets` 
- **公开路由**: `POST /api/wallets/unified-create` (多链)
- **公开路由**: `POST /api/wallets/create` (多链)
- **受保护路由**: `POST /api/wallets` (简化版) ⚠️

**潜在问题**: 路径前缀匹配可能导致路由混乱

#### 冲突点 2: 功能重叠
- `/api/wallets/unified-create` 做的事 = `/api/wallets` 想做的事
- 两者都是"创建钱包并存储"，但实现方式不同

---

## 🎯 清理建议

### 方案 1: 渐进式迁移（推荐）⭐

#### 阶段 1: 标记废弃（当前）
在旧 API 响应中添加 `Deprecated` 头：
```rust
// handlers.rs - simple_create_wallet()
resp.headers_mut().insert(
    "X-Api-Status", 
    HeaderValue::from_static("deprecated")
);
resp.headers_mut().insert(
    "X-Api-Migration", 
    HeaderValue::from_static("Use POST /api/wallets/unified-create")
);
```

#### 阶段 2: 前端迁移（1-2周）
1. 更新 IronForge 前端调用：
   ```typescript
   // 旧方式
   POST /api/wallets { name, address, chain }
   
   // 新方式
   POST /api/wallets/unified-create { name, chain }
   ```

2. 验证功能正常

#### 阶段 3: 删除旧端点（2周后）
移除以下端点：
- ❌ `POST /api/wallets` (简化版)
- ❌ `POST /api/v1/wallets` (企业版)

保留查询端点（向后兼容）：
- ✅ `GET /api/wallets` (列表)
- ✅ `GET /api/wallets/:id` (详情)
- ✅ `DELETE /api/wallets/:id` (删除)

---

### 方案 2: 立即重构（激进）

#### 统一 API 路径结构
```
# 多链钱包（新系统）
POST   /api/v2/wallets              # 统一创建（合并 unified-create）
POST   /api/v2/wallets/batch        # 批量创建（重命名 create-multi）
POST   /api/v2/wallets/derive       # 纯派生（重命名 create）
GET    /api/v2/wallets              # 钱包列表
GET    /api/v2/wallets/:id          # 钱包详情
DELETE /api/v2/wallets/:id          # 删除钱包
POST   /api/v2/wallets/validate     # 地址验证

# 链信息查询
GET    /api/v2/chains               # 链列表
GET    /api/v2/chains/by-curve      # 按曲线分组

# 废弃旧端点
❌ /api/wallets/*                    # 简化版（废弃）
❌ /api/v1/wallets/*                 # 企业版（废弃）
```

**优点**:
- ✅ 路径清晰，无冲突
- ✅ 版本隔离（v2）
- ✅ 统一命名规范

**缺点**:
- ⚠️ 需要立即更新前端
- ⚠️ 破坏现有集成
- ⚠️ 测试工作量大

---

### 方案 3: 代理迁移（兼容性最佳）

#### 让旧端点调用新系统
```rust
// handlers.rs - simple_create_wallet()
pub async fn simple_create_wallet(
    State(st): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<SimpleCreateWalletReq>,
) -> Result<Json<SimpleWalletResp>, AppError> {
    // 1. 从 JWT 提取用户信息
    let (tenant_id, user_id) = extract_user_from_jwt(&headers)?;
    
    // 2. 调用新的多链 API
    let unified_req = UnifiedCreateWalletRequest {
        name: req.name.clone(),
        chain: req.chain.clone(),
        mnemonic: None,  // 前端已派生
        word_count: None,
        account: None,
        index: None,
        tenant_id: Some(tenant_id.to_string()),
        user_id: Some(user_id.to_string()),
    };
    
    // 3. 委托给 unified_create_wallet
    let result = crate::api::multi_chain_api::unified_create_wallet(
        State(st),
        Json(unified_req)
    ).await?;
    
    // 4. 转换响应格式
    Ok(Json(SimpleWalletResp {
        id: result.wallet.id,
        name: req.name,
        address: result.wallet.address,
        chain: req.chain,
        balance: "0".to_string(),
        created_at: result.wallet.created_at,
    }))
}
```

**优点**:
- ✅ 前端无感知迁移
- ✅ 逐步替换旧实现
- ✅ 保持API兼容性

**缺点**:
- ⚠️ 增加一层间接调用
- ⚠️ 响应格式转换开销

---

## 📊 端点使用统计（需要）

### 推荐添加监控
在 `metrics.rs` 中添加端点调用计数：
```rust
pub fn count_endpoint(path: &str) {
    metrics::increment_counter!("api_endpoint_calls", "path" => path);
}
```

**监控目标**:
- `/api/wallets` (简化版) 调用次数
- `/api/v1/wallets` (企业版) 调用次数
- `/api/wallets/unified-create` (新版) 调用次数

**决策依据**:
- 如果旧端点调用 < 10次/天 → 可以立即废弃
- 如果旧端点调用 > 100次/天 → 需要渐进式迁移

---

## 🔍 未使用功能检查

### 可能未使用的端点（需要验证）

#### 1. 企业级功能（如果是个人钱包项目）
- `/api/v1/tenants/*` - 租户管理
- `/api/v1/policies/*` - 策略管理
- `/api/v1/approvals/*` - 审批管理
- `/api/v1/api-keys/*` - API密钥管理

**建议**: 检查前端是否调用这些端点

#### 2. 区块链查询（可能重复）
- `/api/fees` - Gas费用
- `/api/gas/suggest` - Gas建议
- `/api/network/status` - 网络状态

**建议**: 这些功能是否应该整合到多链 API？

---

## ✅ 推荐执行步骤

### 第1步: 添加废弃警告（立即）
```rust
// handlers.rs
pub async fn simple_create_wallet(...) -> Result<...> {
    tracing::warn!(
        "Deprecated API called: POST /api/wallets. \
         Please migrate to POST /api/wallets/unified-create"
    );
    
    // 现有逻辑...
}
```

### 第2步: 添加监控（本周）
```rust
crate::metrics::count_endpoint("POST /api/wallets");
crate::metrics::count_endpoint("POST /api/wallets/unified-create");
```

### 第3步: 通知前端团队（本周）
创建迁移文档：
- 新旧 API 对比表
- 迁移示例代码
- 兼容性说明
- 废弃时间表

### 第4步: 实施代理迁移（下周）
让旧端点内部调用新系统

### 第5步: 观察 7 天（下周）
监控调用量和错误率

### 第6步: 删除旧代码（2周后）
如果旧端点调用量 < 5%，可以安全删除

---

## 🎯 最终目标 API 结构

### 公开 API
```
# 认证
POST   /api/auth/register
POST   /api/auth/login
POST   /api/auth/refresh

# 多链钱包（统一入口）
POST   /api/wallets/unified-create   ⭐ 主要创建接口
POST   /api/wallets/create           # 纯派生（高级用户）
POST   /api/wallets/create-multi     # 批量创建
GET    /api/chains                   # 链信息
POST   /api/wallets/validate-address # 地址验证

# 监控与文档
GET    /api/health
GET    /healthz
GET    /metrics
GET    /docs
```

### 受保护 API
```
# 钱包管理
GET    /api/wallets                  # 列表
GET    /api/wallets/:id              # 详情
DELETE /api/wallets/:id              # 删除

# 交易
POST   /api/transactions/send
GET    /api/transactions

# 用户管理（可选）
GET    /api/auth/me
POST   /api/auth/logout
```

### 移除的 API
```
❌ POST /api/v1/wallets              # 合并到 unified-create
❌ 所有企业级端点（如果不需要）
```

---

## 📝 总结

### 核心问题
1. **3 套钱包创建系统并存**：多链、简化、企业级
2. **路径潜在冲突**：`/api/wallets` 和 `/api/wallets/*`
3. **功能重复**：都是创建钱包，但实现不同

### 推荐方案
**渐进式迁移 + 代理模式**（方案 1 + 方案 3）
- 周期：2-3周
- 风险：低
- 兼容性：高

### 立即行动
1. ✅ 添加废弃警告日志
2. ✅ 添加端点调用监控
3. ✅ 通知前端团队迁移计划

### 后续行动
4. ⏳ 实施代理迁移（让旧 API 调用新系统）
5. ⏳ 观察 7 天监控数据
6. ⏳ 删除旧端点代码

---

**报告生成时间**: 2025-11-23  
**分析范围**: backend/src/api/*  
**状态**: 🟡 需要清理
