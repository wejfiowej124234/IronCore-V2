# 管理员操作指南

> ironforge_backend 管理员完整操作手册

## 📋 目录

- [管理员权限](#管理员权限)
- [费率规则管理](#费率规则管理)
- [归集地址管理](#归集地址管理)
- [RPC端点管理](#rpc端点管理)
- [用户管理](#用户管理)
- [系统监控](#系统监控)
- [安全审计](#安全审计)
- [故障处理](#故障处理)

---

## 管理员权限

### 权限级别

IronForge 后端采用基于角色的访问控制（RBAC）：

```rust
pub enum Role {
    Admin,      // 管理员 - 完整系统控制权限
    User,       // 普通用户 - 基本钱包操作
    Approver,   // 审批者 - 交易审批权限
    Viewer,     // 只读用户 - 查看权限
}
```

### 管理员功能

管理员拥有以下特权：

- ✅ 费率规则配置（CRUD）
- ✅ 归集地址管理
- ✅ RPC 端点配置
- ✅ 用户权限管理
- ✅ 系统配置修改
- ✅ 审计日志查看
- ✅ 监控数据访问

### 获取管理员权限

```sql
-- 提升用户为管理员
UPDATE users 
SET role = 'Admin' 
WHERE id = '<user_id>';

-- 查看当前管理员列表
SELECT id, username, email, role, created_at 
FROM users 
WHERE role = 'Admin';
```

---

## 费率规则管理

### 概述

费率规则用于配置平台手续费，支持：
- **固定费用**: 固定金额（如 0.001 ETH）
- **百分比费用**: 按交易金额百分比（如 0.1%）
- **混合费用**: 固定 + 百分比
- **区间限制**: 最小/最大费用

### API 端点

> 响应统一使用 `{ code, message, data }` 包装格式；下文示例响应默认展示 `data` 字段内容。

#### 1. 创建费率规则

```bash
POST /api/v1/admin/fee-rules
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "chain": "ethereum",
  "operation": "send",
  "fee_type": "mixed",
  "flat_amount": 0.001,      # 固定 0.001 ETH
  "percent_bp": 10,          # 0.1% (10 基点)
  "min_fee": 0.0005,         # 最小 0.0005 ETH
  "max_fee": 0.01,           # 最大 0.01 ETH
  "priority": 100            # 优先级（数字越大越优先）
}
```

**响应**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "chain": "ethereum",
  "operation": "send",
  "fee_type": "mixed",
  "flat_amount": 0.001,
  "percent_bp": 10,
  "min_fee": 0.0005,
  "max_fee": 0.01,
  "priority": 100,
  "rule_version": 1,
  "active": true,
  "created_at": "2025-11-24T10:00:00Z",
  "updated_at": "2025-11-24T10:00:00Z"
}
```

#### 2. 查询所有规则

```bash
GET /api/v1/admin/fee-rules
Authorization: Bearer <admin_jwt>
```

**响应**:
```json
[
  {
    "id": "...",
    "chain": "ethereum",
    "operation": "send",
    "fee_type": "mixed",
    "flat_amount": 0.001,
    "percent_bp": 10,
    "min_fee": 0.0005,
    "max_fee": 0.01,
    "priority": 100,
    "rule_version": 1,
    "active": true,
    "created_at": "2025-11-24T10:00:00Z",
    "updated_at": "2025-11-24T10:00:00Z"
  }
]
```

#### 3. 更新费率规则

```bash
PUT /api/v1/admin/fee-rules/{id}
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "flat_amount": 0.002,
  "percent_bp": 15,
  "priority": 110
}
```

#### 4. 删除费率规则（软删除）

```bash
DELETE /api/v1/admin/fee-rules/{id}
Authorization: Bearer <admin_jwt>
```

### 费率类型说明

#### flat（固定费用）
```json
{
  "fee_type": "flat",
  "flat_amount": 0.001,
  "percent_bp": null
}
```
计算公式: `fee = 0.001 ETH`

#### percent（百分比费用）
```json
{
  "fee_type": "percent",
  "flat_amount": null,
  "percent_bp": 10  // 0.1%
}
```
计算公式: `fee = amount × 0.001`

#### mixed（混合费用）
```json
{
  "fee_type": "mixed",
  "flat_amount": 0.001,
  "percent_bp": 10
}
```
计算公式: `fee = 0.001 + (amount × 0.001)`

### 优先级规则

当多个规则匹配时，系统按以下顺序选择：

1. **优先级数字**: 数字越大优先级越高
2. **版本号**: 同优先级选择最新版本
3. **激活状态**: 只选择 `active = true` 的规则

### 最佳实践

1. **测试规则**: 在测试网先验证规则正确性
2. **版本控制**: 修改规则会自动创建新版本
3. **监控费用**: 定期检查费用审计日志
4. **逐步调整**: 小幅度调整费率，观察影响

---

## 归集地址管理

### 概述

归集地址用于收集平台手续费，每条链可配置多个归集地址。

### API 端点

#### 1. 添加归集地址

```bash
POST /api/v1/admin/collector-addresses
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "chain": "ethereum",
  "address": "0x1234567890123456789012345678901234567890"
}
```

**响应**:
```json
{
  "id": "...",
  "chain": "ethereum",
  "address": "0x1234567890123456789012345678901234567890",
  "active": true,
  "created_at": "2025-11-24T10:00:00Z"
}
```

#### 2. 激活/停用归集地址

```bash
PUT /api/v1/admin/collector-addresses/{id}/activate
Authorization: Bearer <admin_jwt>
```

> 说明：当前版本未提供 `GET /api/v1/admin/collector-addresses` 列表查询 API；如需盘点请以 OpenAPI 与数据库为准。

### 安全建议

1. **冷钱包**: 使用硬件钱包管理归集地址私钥
2. **多签验证**: 大额提现使用多签钱包
3. **定期审计**: 检查归集地址余额和交易记录
4. **地址验证**: 添加前仔细验证地址正确性

---

## RPC端点管理

### 概述

RPC 端点用于连接区块链节点，系统支持：
- **优先级配置**: 按优先级选择节点
- **健康检查**: 自动检测节点可用性
- **熔断保护**: 故障节点自动切换
- **负载均衡**: 分散请求压力

### API 端点

#### 1. 添加 RPC 端点

```bash
POST /api/v1/admin/rpc-endpoints
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "chain": "ethereum",
  "url": "https://eth-mainnet.alchemyapi.io/v2/YOUR_API_KEY",
  "priority": 100
}
```

**响应**:
```json
{
  "id": "...",
  "chain": "ethereum",
  "url": "https://eth-mainnet.alchemyapi.io/v2/YOUR_API_KEY",
  "priority": 100,
  "healthy": true,
  "circuit_state": "closed",
  "created_at": "2025-11-24T10:00:00Z"
}
```

#### 2. 更新端点

```bash
PUT /api/v1/admin/rpc-endpoints/{id}
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "priority": 110,
  "healthy": true
}
```

#### 3. 删除端点

```bash
DELETE /api/v1/admin/rpc-endpoints/{id}
Authorization: Bearer <admin_jwt>
```

> 说明：当前版本未提供 `GET /api/v1/admin/rpc-endpoints` 列表查询 API；如需盘点请以 OpenAPI 与数据库为准。

### 熔断器状态

- **closed**: 正常工作状态
- **open**: 故障打开状态（暂停使用）
- **half_open**: 半开状态（尝试恢复）

### RPC 提供商推荐

#### Ethereum
- **Alchemy**: https://eth-mainnet.alchemyapi.io/v2/
- **Infura**: https://mainnet.infura.io/v3/
- **QuickNode**: https://YOUR_ENDPOINT.quiknode.pro/

#### BSC
- **BSC Official**: https://bsc-dataseed.binance.org/
- **NodeReal**: https://bsc-mainnet.nodereal.io/v1/

#### Polygon
- **Alchemy**: https://polygon-mainnet.g.alchemy.com/v2/
- **QuickNode**: https://YOUR_ENDPOINT.matic.quiknode.pro/

### 监控指标

定期检查以下指标：

```bash
# 当前版本未提供 RPC 统计查询 API。
# 建议：
# - 通过 Prometheus 指标查看（/metrics）
# - 或在日志/监控系统中聚合 RPC 错误与延迟
```

---

## 用户管理

### 用户操作

#### 1. 查看用户列表

```bash
GET /api/v1/users?page=1&limit=20
Authorization: Bearer <admin_jwt>
```

#### 2. 查看用户详情

```bash
GET /api/v1/users/{user_id}
Authorization: Bearer <admin_jwt>
```

#### 3. 更新用户角色

```bash
PUT /api/v1/users/{user_id}
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "role": "Approver",
  "is_active": true
}
```

#### 4. 禁用用户

```bash
PUT /api/v1/users/{user_id}
Authorization: Bearer <admin_jwt>
Content-Type: application/json

{
  "is_active": false
}
```

#### 5. 删除用户

```bash
DELETE /api/v1/users/{user_id}
Authorization: Bearer <admin_jwt>
```

### 用户统计

```sql
-- 用户总数
SELECT COUNT(*) FROM users;

-- 活跃用户数
SELECT COUNT(*) FROM users WHERE is_active = true;

-- 按角色统计
SELECT role, COUNT(*) 
FROM users 
GROUP BY role;

-- 最近注册用户
SELECT username, email, created_at 
FROM users 
ORDER BY created_at DESC 
LIMIT 10;
```

---

## 系统监控

### Prometheus 指标

访问: `http://localhost:8088/metrics`

#### 关键指标

```promql
# HTTP 请求总数
http_requests_total

# 请求延迟 P95
histogram_quantile(0.95, http_request_duration_seconds_bucket)

# 数据库连接池
db_pool_connections{state="active"}
db_pool_connections{state="idle"}

# Redis 操作
redis_operations_total{operation="get"}
redis_operations_total{operation="set"}

# 交易统计
transactions_confirmed_total{chain="ethereum"}
transactions_failed_total{chain="ethereum"}

# 费用统计
platform_fees_collected_total{chain="ethereum"}
```

### Grafana 仪表盘

推荐配置以下仪表盘：

1. **系统概览**
   - 请求速率 (RPS)
   - 错误率
   - P50/P95/P99 延迟
   - 活跃用户数

2. **数据库监控**
   - 连接池使用率
   - 查询延迟
   - 慢查询列表
   - 事务成功率

3. **业务指标**
   - 新注册用户
   - 钱包创建趋势
   - 交易成功率
   - 平台手续费收入

4. **RPC 监控**
   - 节点健康状态
   - 请求分布
   - 平均延迟
   - 错误率

### 告警配置

#### 关键告警

```yaml
# prometheus/alerts.yml
groups:
  - name: ironforge_critical
    rules:
      # 高错误率
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "错误率超过 5%"
      
      # 数据库连接池耗尽
      - alert: DatabasePoolExhausted
        expr: db_pool_connections{state="idle"} < 5
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "数据库连接池即将耗尽"
      
      # RPC 节点不可用
      - alert: RpcEndpointDown
        expr: rpc_endpoint_healthy == 0
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "RPC 节点不可用"
```

---

## 安全审计

### 审计日志

所有管理员操作都会记录到审计日志：

```sql
-- 查看管理员操作日志
SELECT 
    admin_user_id,
    admin_role,
    operation_type,
    resource_type,
    resource_id,
    details,
    created_at
FROM admin.operation_log
ORDER BY created_at DESC
LIMIT 100;

-- 查看特定管理员的操作
SELECT * 
FROM admin.operation_log
WHERE admin_user_id = '<user_id>'
ORDER BY created_at DESC;

-- 查看费率规则修改历史
SELECT * 
FROM admin.operation_log
WHERE operation_type IN ('create_fee_rule', 'update_fee_rule', 'delete_fee_rule')
ORDER BY created_at DESC;
```

### Immudb 不可变日志

重要操作会写入 Immudb：

```bash
# 查询审计事件
curl -X POST http://localhost:3322/api/scan \
  -d '{
    "prefix": "YXVkaXQ6",
    "limit": 100
  }'
```

### 安全检查清单

#### 每日检查
- [ ] 查看系统健康状态
- [ ] 检查错误日志
- [ ] 确认备份完成
- [ ] 检查异常登录

#### 每周检查
- [ ] 审查管理员操作日志
- [ ] 检查 RPC 端点健康
- [ ] 查看费用收集统计
- [ ] 分析用户增长趋势

#### 每月检查
- [ ] 全面安全审计
- [ ] 数据库性能分析
- [ ] 费率规则优化
- [ ] 系统容量规划

---

## 故障处理

### 常见问题

#### 1. 数据库连接失败

**症状**: `Database error: connection failed`

**排查步骤**:
```bash
# 检查数据库服务
docker ps | grep cockroach

# 测试连接
psql $DATABASE_URL -c "SELECT 1"

# 查看连接池状态
curl http://localhost:8088/metrics | grep db_pool
```

**解决方案**:
```bash
# 重启数据库
docker restart cockroach

# 调整连接池配置
# 编辑 config.toml
[database]
max_connections = 50
connect_timeout_secs = 10
```

#### 2. RPC 节点不可用

**症状**: `RPC error: connection timeout`

**排查步骤**:
```bash
# 当前版本未提供 GET /api/v1/admin/rpc-endpoints 列表查询 API。
# 建议：通过 OpenAPI(/docs) 核对可用管理端点，或直接检查数据库/配置。

# 手动测试 RPC
curl -X POST https://eth-mainnet.alchemyapi.io/v2/YOUR_KEY \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

**解决方案**:
1. 切换到备用 RPC 端点
2. 增加超时配置
3. 联系 RPC 提供商

#### 3. 高延迟

**症状**: P95 延迟 > 1秒

**排查步骤**:
```bash
# 检查慢查询
SELECT 
    query,
    mean_exec_time,
    calls
FROM pg_stat_statements
WHERE mean_exec_time > 1000
ORDER BY mean_exec_time DESC
LIMIT 10;

# 检查缓存命中率
curl http://localhost:8088/metrics | grep redis_cache
```

**解决方案**:
1. 优化慢查询（添加索引）
2. 启用查询缓存
3. 增加数据库连接池
4. 扩容服务器资源

#### 4. 内存泄漏

**症状**: 内存使用持续增长

**排查步骤**:
```bash
# 检查内存使用
ps aux | grep ironforge_backend

# 查看 Prometheus 指标
process_resident_memory_bytes
```

**解决方案**:
1. 重启服务（临时）
2. 使用内存分析工具
3. 修复内存泄漏代码
4. 增加内存限制

---

## 紧急联系人

### 技术支持

- **后端团队**: backend@ironforge.io
- **运维团队**: ops@ironforge.io
- **安全团队**: security@ironforge.io

### 升级流程

```
1. L1 运维值班 → 基础故障处理
   ↓ (无法解决，15分钟内)
2. L2 后端工程师 → 代码级问题
   ↓ (重大故障，立即)
3. L3 架构师 → 架构级问题
   ↓ (灾难性故障，立即)
4. CTO → 决策与协调
```

---

## 附录

### A. 管理员 API 完整列表

#### 费率规则
- `POST /api/v1/admin/fee-rules` - 创建规则
- `GET /api/v1/admin/fee-rules` - 查询规则
- `PUT /api/v1/admin/fee-rules/{id}` - 更新规则
- `DELETE /api/v1/admin/fee-rules/{id}` - 删除规则

#### 归集地址
- `POST /api/v1/admin/collector-addresses` - 添加地址
- `PUT /api/v1/admin/collector-addresses/{id}/activate` - 激活/停用

#### RPC 端点
- `POST /api/v1/admin/rpc-endpoints` - 创建端点
- `PUT /api/v1/admin/rpc-endpoints/{id}` - 更新端点
- `DELETE /api/v1/admin/rpc-endpoints/{id}` - 删除端点

#### 用户管理
- `GET /api/v1/users` - 用户列表
- `GET /api/v1/users/{id}` - 用户详情
- `PUT /api/v1/users/{id}` - 更新用户
- `DELETE /api/v1/users/{id}` - 删除用户

### B. 数据库管理

#### 备份
```bash
# 全量备份
cockroach dump ironcore --url=$DATABASE_URL > backup.sql

# 恢复
cockroach sql --url=$DATABASE_URL < backup.sql
```

#### 性能分析
```sql
-- 查看表大小
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname NOT IN ('pg_catalog', 'information_schema')
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- 查看索引使用情况
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read
FROM pg_stat_user_indexes
ORDER BY idx_scan ASC;
```

### C. 相关文档

- [配置管理](../02-configuration/CONFIG_MANAGEMENT.md)
- [安全策略](../02-configuration/SECURITY.md)
- [监控告警](../07-monitoring/MONITORING.md)
- [错误处理](../08-error-handling/ERROR_HANDLING.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team & Operations Team  
**紧急联系**: ops@ironforge.io
