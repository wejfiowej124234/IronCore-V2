# 数据库迁移文件说明

## 📋 迁移文件结构

所有迁移文件已按照**数据库标准最佳实践**重新组织：

### 执行顺序

1. **0001_schemas.sql** - 创建所有 Schema
   - `gas`, `admin`, `notify`, `tokens`, `events`, `fiat`

2. **0002_core_tables.sql** - 创建核心业务表（不含外键）
   - `tenants`, `users`, `policies`, `wallets`, `approvals`
   - `api_keys`, `tx_requests`, `tx_broadcasts`, `audit_index`
   - `transactions`, `swap_transactions`, `nonce_tracking`

3. **0003_gas_tables.sql** - 创建费用系统表
   - `gas.platform_fee_rules`
   - `gas.fee_collector_addresses`
   - `gas.fee_audit`

4. **0004_admin_tables.sql** - 创建管理员和RPC表
   - `admin.rpc_endpoints`
   - `admin.admin_operation_log`

5. **0005_notify_tables.sql** - 创建通知系统表
   - `notify.templates`, `notify.user_preferences`
   - `notify.notifications`, `notify.deliveries`
   - `notify.endpoints`, `notify.campaigns`, `notify.notification_history`

6. **0006_asset_tables.sql** - 创建资产聚合表
   - `prices`, `asset_snapshots`, `cross_chain_swaps`

7. **0007_tokens_tables.sql** - 创建代币注册表
   - `tokens.registry`

8. **0008_events_tables.sql** - 创建事件总线表
   - `events.domain_events`
   - `events.event_subscriptions`
   - `events.failed_events`

9. **0009_fiat_tables.sql** - 创建法币系统表
   - `fiat.providers`, `fiat.orders`, `fiat.transactions`
   - `fiat.audit_logs`, `fiat.reconciliation_records`
   - `fiat.alerts`, `fiat.provider_country_support`

10. **0010_constraints.sql** - 添加外键和唯一约束
    - 所有唯一约束
    - 所有外键约束（按依赖顺序）

11. **0011_indexes.sql** - 创建所有索引
    - 核心表索引
    - 费用系统索引
    - 管理员系统索引
    - 通知系统索引
    - 资产系统索引
    - 代币系统索引
    - 事件系统索引
    - 法币系统索引

12. **0012_check_constraints.sql** - 添加检查约束
    - Swap交易状态检查
    - 代币注册数据验证
    - 法币系统数据验证

13. **0013_initial_data.sql** - 插入初始数据
    - 初始价格数据
    - 代币注册数据（多链支持）

---

## 🎯 设计原则

### 1. 分离关注点
- **Schema** → **表结构** → **约束** → **索引** → **数据**
- 每个阶段独立，便于维护和调试

### 2. 依赖顺序
- 先创建被依赖的表，再创建依赖表
- 先创建表，再添加外键约束
- 先添加约束，再创建索引

### 3. 幂等性
- 所有操作使用 `IF NOT EXISTS`
- 约束使用 `DROP IF EXISTS` 然后 `ADD`
- 数据插入使用 `ON CONFLICT DO NOTHING`

### 4. CockroachDB 兼容
- 使用 `DECIMAL` 而非 `NUMERIC`
- 使用 `TIMESTAMPTZ` 而非 `TIMESTAMP`
- 使用 `CURRENT_TIMESTAMP` 而非 `now()`
- 不支持触发器，`updated_at` 在应用层更新

---

## 🚀 使用方法

### 自动迁移（推荐）
启动应用时自动执行：
```bash
cd IronCore
cargo run
```

### 手动迁移
```bash
# Windows
scripts\run-migrations-cockroachdb.bat

# Linux/Mac/Git Bash
./scripts/run-migrations-cockroachdb.sh
```

### 重置数据库
```bash
# 完全重置（删除所有数据）
./scripts/reset-database.sh --force

# 然后启动应用，迁移会自动执行
cargo run
```

---

## 📊 数据库结构概览

### 核心业务表（public schema）
- `tenants` - 租户
- `users` - 用户
- `wallets` - 钱包（支持多链）
- `policies` - 策略
- `approvals` - 审批
- `api_keys` - API密钥
- `tx_requests` - 交易请求
- `tx_broadcasts` - 交易广播
- `audit_index` - 审计索引
- `transactions` - 交易记录
- `swap_transactions` - Swap交易
- `nonce_tracking` - Nonce追踪

### 费用系统（gas schema）
- `platform_fee_rules` - 平台费用规则
- `fee_collector_addresses` - 费用归集地址
- `fee_audit` - 费用审计记录

### 管理员系统（admin schema）
- `rpc_endpoints` - RPC端点
- `admin_operation_log` - 管理员操作日志

### 通知系统（notify schema）
- `templates` - 通知模板
- `user_preferences` - 用户偏好
- `notifications` - 通知实例
- `deliveries` - 投递记录
- `endpoints` - 用户端点
- `campaigns` - 活动批次
- `notification_history` - 通知历史

### 资产系统（public schema）
- `prices` - 价格缓存
- `asset_snapshots` - 资产快照
- `cross_chain_swaps` - 跨链交易

### 代币系统（tokens schema）
- `registry` - 代币注册表

### 事件系统（events schema）
- `domain_events` - 领域事件
- `event_subscriptions` - 事件订阅
- `failed_events` - 失败事件

### 法币系统（fiat schema）
- `providers` - 服务商配置
- `orders` - 法币订单
- `transactions` - 交易历史
- `audit_logs` - 审计日志
- `reconciliation_records` - 对账记录
- `alerts` - 异常告警
- `provider_country_support` - 国家支持映射

---

## ✅ 优势

1. **标准化**：遵循数据库最佳实践
2. **可维护**：清晰的分离和组织
3. **可扩展**：易于添加新的迁移文件
4. **可靠性**：幂等性保证，可重复执行
5. **兼容性**：完全兼容 CockroachDB

---

## 📝 注意事项

1. **不要修改已执行的迁移文件**：如果需要修改，创建新的迁移文件
2. **迁移文件按顺序执行**：确保版本号连续
3. **生产环境谨慎**：在生产环境执行迁移前，请先备份数据
4. **测试环境**：可以在测试环境使用 `RESET_DB=true` 重置数据库

---

## 🔗 相关文档

- [数据库重置指南](../scripts/RESET_DATABASE_GUIDE.md)
- [迁移脚本说明](../scripts/README.md)

