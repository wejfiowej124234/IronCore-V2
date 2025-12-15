# 数据库 Schema 完整文档

> 基于标准化迁移文件的完整数据库结构文档

## 📋 目录

- [数据库概览](#数据库概览)
- [Schema 组织](#schema-组织)
- [核心业务表](#核心业务表)
- [费用系统表](#费用系统表)
- [管理员系统表](#管理员系统表)
- [通知系统表](#通知系统表)
- [资产系统表](#资产系统表)
- [代币系统表](#代币系统表)
- [事件系统表](#事件系统表)
- [法币系统表](#法币系统表)
- [表关系图](#表关系图)
- [索引设计](#索引设计)
- [约束完整性](#约束完整性)

---

## 数据库概览

### 技术栈

- **主数据库**: CockroachDB (PostgreSQL 兼容)
- **缓存**: Redis
- **审计日志**: Immudb（不可变日志）

### 数据库特性

- **ACID 事务**: 完整的事务支持
- **水平扩展**: CockroachDB 支持分布式部署
- **高可用**: 多副本自动故障转移
- **地理分布**: 支持多地域部署

### 迁移文件结构

所有迁移文件按照数据库标准最佳实践组织：

1. **0001_schemas.sql** - 创建所有 Schema
2. **0002_core_tables.sql** - 核心业务表（不含外键）
3. **0003_gas_tables.sql** - 费用系统表
4. **0004_admin_tables.sql** - 管理员和RPC表
5. **0005_notify_tables.sql** - 通知系统表
6. **0006_asset_tables.sql** - 资产聚合表
7. **0007_tokens_tables.sql** - 代币注册表
8. **0008_events_tables.sql** - 事件总线表
9. **0009_fiat_tables.sql** - 法币系统表
10. **0010_constraints.sql** - 外键和唯一约束
11. **0011_indexes.sql** - 所有索引
12. **0012_check_constraints.sql** - 检查约束
13. **0013_initial_data.sql** - 初始数据

---

## Schema 组织

### Public Schema（核心业务）

- `tenants` - 租户
- `users` - 用户
- `policies` - 策略
- `wallets` - 钱包（支持多链）
- `approvals` - 审批
- `api_keys` - API密钥
- `tx_requests` - 交易请求
- `tx_broadcasts` - 交易广播
- `audit_index` - 审计索引
- `transactions` - 交易记录
- `swap_transactions` - Swap交易
- `nonce_tracking` - Nonce追踪
- `prices` - 价格缓存
- `asset_snapshots` - 资产快照
- `cross_chain_swaps` - 跨链交易

### Gas Schema（费用系统）

- `platform_fee_rules` - 平台费用规则
- `fee_collector_addresses` - 费用归集地址
- `fee_audit` - 费用审计记录

### Admin Schema（管理员系统）

- `rpc_endpoints` - RPC端点
- `admin_operation_log` - 管理员操作日志

### Notify Schema（通知系统）

- `templates` - 通知模板
- `user_preferences` - 用户偏好
- `notifications` - 通知实例
- `deliveries` - 投递记录
- `endpoints` - 用户端点
- `campaigns` - 活动批次
- `notification_history` - 通知历史

### Tokens Schema（代币系统）

- `registry` - 代币注册表

### Events Schema（事件系统）

- `domain_events` - 领域事件
- `event_subscriptions` - 事件订阅
- `failed_events` - 失败事件

### Fiat Schema（法币系统）

- `providers` - 服务商配置
- `orders` - 法币订单
- `transactions` - 交易历史
- `audit_logs` - 审计日志
- `reconciliation_records` - 对账记录
- `alerts` - 异常告警
- `provider_country_support` - 国家支持映射

---

## 核心业务表

### tenants（租户表）

```sql
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### users（用户表）

```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    email_cipher TEXT NOT NULL,
    email TEXT,  -- 开发环境使用
    phone_cipher TEXT,
    phone TEXT,  -- 开发环境使用
    password_hash TEXT,
    role TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**外键**: `tenant_id` → `tenants(id)`

### wallets（钱包表）

```sql
CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    chain_id INT NOT NULL,
    chain_symbol TEXT,
    address TEXT NOT NULL,
    pubkey TEXT,
    name TEXT,
    derivation_path TEXT,
    curve_type TEXT,
    account_index INT NOT NULL DEFAULT 0,
    address_index INT NOT NULL DEFAULT 0,
    policy_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**外键**: 
- `tenant_id` → `tenants(id)`
- `user_id` → `users(id)`

**唯一约束**: `(tenant_id, chain_id, address)`

### transactions（交易表）

```sql
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    wallet_id UUID NOT NULL,
    chain_type TEXT NOT NULL,
    tx_hash TEXT NOT NULL,
    tx_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    amount TEXT NOT NULL,
    token_symbol TEXT,
    gas_fee TEXT,
    nonce BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    confirmed_at TIMESTAMPTZ
);
```

**外键**: 
- `user_id` → `users(id)`
- `wallet_id` → `wallets(id)`

**唯一约束**: `(tx_hash, chain_type)`

**检查约束**: `status IN ('pending', 'confirmed', 'failed', 'dropped')`

### swap_transactions（Swap交易表）

```sql
CREATE TABLE swap_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    wallet_id UUID NOT NULL,
    network TEXT NOT NULL,
    from_token TEXT NOT NULL,
    to_token TEXT NOT NULL,
    from_amount DECIMAL(30, 18) NOT NULL,
    to_amount DECIMAL(30, 18),
    slippage DECIMAL(5, 2),
    swap_id TEXT NOT NULL,
    tx_hash TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    gas_used TEXT,
    confirmations INT NOT NULL DEFAULT 0,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**外键**: 
- `tenant_id` → `tenants(id)`
- `user_id` → `users(id)`
- `wallet_id` → `wallets(id)`

**唯一约束**: `swap_id`

**检查约束**: 
- `status IN ('pending', 'executing', 'confirmed', 'failed', 'cancelled')`
- `confirmations >= 0`
- `slippage IS NULL OR (slippage >= 0 AND slippage <= 100)`

---

## 费用系统表

### gas.platform_fee_rules（平台费用规则表）

```sql
CREATE TABLE gas.platform_fee_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    operation TEXT NOT NULL,
    fee_type TEXT NOT NULL,
    flat_amount DECIMAL(30, 8) NOT NULL DEFAULT 0,
    percent_bp INT NOT NULL DEFAULT 0,
    min_fee DECIMAL(30, 8) NOT NULL DEFAULT 0,
    max_fee DECIMAL(30, 8),
    priority INT NOT NULL DEFAULT 100,
    rule_version INT NOT NULL DEFAULT 1,
    active BOOL NOT NULL DEFAULT true,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### gas.fee_audit（费用审计记录表）

```sql
CREATE TABLE gas.fee_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tx_id UUID,
    user_id UUID,
    chain TEXT NOT NULL,
    operation TEXT NOT NULL,
    original_amount DECIMAL(30, 8) NOT NULL,
    platform_fee DECIMAL(30, 8) NOT NULL,
    fee_type TEXT NOT NULL,
    applied_rule UUID,
    collector_address TEXT NOT NULL,
    tx_hash TEXT,
    wallet_address TEXT,
    gas_used BIGINT DEFAULT 0,
    gas_fee_native DECIMAL(30, 8) DEFAULT 0,
    quote_source TEXT,
    rule_version INT DEFAULT 1,
    retry_count INT DEFAULT 0,
    last_retry_at TIMESTAMPTZ,
    block_number BIGINT,
    confirmations INT DEFAULT 0,
    tx_status SMALLINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

---

## 管理员系统表

### admin.rpc_endpoints（RPC端点表）

```sql
CREATE TABLE admin.rpc_endpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    url TEXT NOT NULL,
    provider TEXT,
    priority INT NOT NULL DEFAULT 100,
    healthy BOOL NOT NULL DEFAULT true,
    fail_count INT NOT NULL DEFAULT 0,
    last_fail_at TIMESTAMPTZ,
    avg_latency_ms INT NOT NULL DEFAULT 0,
    last_latency_ms INT NOT NULL DEFAULT 0,
    circuit_state TEXT NOT NULL DEFAULT 'closed',
    last_checked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**唯一约束**: `(chain, url)`

---

## 代币系统表

### tokens.registry（代币注册表）

```sql
CREATE TABLE tokens.registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    chain_id INT NOT NULL,
    address TEXT NOT NULL,
    decimals INT NOT NULL,
    is_native BOOL NOT NULL DEFAULT false,
    is_stablecoin BOOL NOT NULL DEFAULT false,
    logo_url TEXT,
    coingecko_id TEXT,
    is_enabled BOOL NOT NULL DEFAULT true,
    priority INT NOT NULL DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**唯一约束**: 
- `(chain_id, symbol)`
- `(chain_id, address)`

**检查约束**: 
- `decimals >= 0 AND decimals <= 18`
- `priority >= 0`

---

## 表关系图

```
tenants
  ├── users
  │     ├── wallets
  │     │     ├── transactions
  │     │     ├── swap_transactions
  │     │     └── tx_requests
  │     │           └── tx_broadcasts
  │     └── approvals
  ├── policies
  │     └── approvals
  └── api_keys

gas.platform_fee_rules
gas.fee_collector_addresses
gas.fee_audit

admin.rpc_endpoints
admin.admin_operation_log

notify.templates
notify.user_preferences
notify.notifications
  └── notify.deliveries
notify.endpoints
notify.campaigns
notify.notification_history

tokens.registry

events.domain_events
events.event_subscriptions
events.failed_events

fiat.providers
  └── fiat.provider_country_support
fiat.orders
  ├── fiat.transactions
  └── fiat.audit_logs
fiat.reconciliation_records
fiat.alerts
```

---

## 索引设计

### 核心业务表索引

- `users`: `(tenant_id)`, `(tenant_id, email_cipher)`, `(email)`
- `wallets`: `(tenant_id, chain_id, address)`, `(user_id, chain_id)`, `(curve_type, chain_id)`
- `transactions`: `(user_id)`, `(wallet_id)`, `(tx_hash)`, `(status)`, `(wallet_id, status)`
- `swap_transactions`: `(tenant_id, user_id)`, `(wallet_id)`, `(swap_id)`, `(status)`

### 费用系统索引

- `gas.platform_fee_rules`: `(chain, operation, active, priority)`
- `gas.fee_audit`: `(user_id, created_at DESC)`, `(chain, created_at DESC)`, `(tx_hash, gas_used, retry_count)`

### 管理员系统索引

- `admin.rpc_endpoints`: `(chain, healthy, priority)`, `(chain, circuit_state)`
- `admin.admin_operation_log`: `(operator_user_id, created_at DESC)`, `(action, created_at DESC)`

---

## 约束完整性

### 外键约束

所有外键约束都使用 `ON DELETE CASCADE` 确保数据一致性：

- `users.tenant_id` → `tenants.id`
- `wallets.tenant_id` → `tenants.id`
- `wallets.user_id` → `users.id`
- `transactions.user_id` → `users.id`
- `transactions.wallet_id` → `wallets.id`
- `swap_transactions.tenant_id` → `tenants.id`
- `swap_transactions.user_id` → `users.id`
- `swap_transactions.wallet_id` → `wallets.id`

### 唯一约束

- `wallets`: `(tenant_id, chain_id, address)`
- `transactions`: `(tx_hash, chain_type)`
- `swap_transactions`: `swap_id`
- `nonce_tracking`: `(chain, address)`
- `prices`: `(symbol, source)`
- `gas.fee_collector_addresses`: `(chain, address)`
- `admin.rpc_endpoints`: `(chain, url)`
- `tokens.registry`: `(chain_id, symbol)`, `(chain_id, address)`

### 检查约束

- `transactions.status`: `IN ('pending', 'confirmed', 'failed', 'dropped')`
- `swap_transactions.status`: `IN ('pending', 'executing', 'confirmed', 'failed', 'cancelled')`
- `swap_transactions.confirmations`: `>= 0`
- `swap_transactions.slippage`: `IS NULL OR (slippage >= 0 AND slippage <= 100)`
- `tokens.registry.decimals`: `>= 0 AND decimals <= 18`
- `tokens.registry.priority`: `>= 0`

---

## 迁移管理

### 执行迁移

```bash
# 自动迁移（启动应用时）
cargo run

# 手动迁移
./scripts/run-migrations-cockroachdb.sh
```

### 重置数据库

```bash
# 完全重置（删除所有数据）
./scripts/reset-database.sh --force
```

### 迁移文件说明

详见 [migrations/README.md](../../migrations/README.md)

---

## 数据完整性保证

1. **外键约束**: 确保引用完整性
2. **唯一约束**: 防止重复数据
3. **检查约束**: 确保数据有效性
4. **索引优化**: 提高查询性能
5. **级联删除**: 自动清理关联数据

---

## 注意事项

1. **CockroachDB 兼容性**: 
   - 使用 `DECIMAL` 而非 `NUMERIC`
   - 使用 `TIMESTAMPTZ` 而非 `TIMESTAMP`
   - 不支持触发器，`updated_at` 在应用层更新

2. **幂等性**: 所有迁移文件可重复执行

3. **生产环境**: 执行迁移前请先备份数据

---

## 相关文档

- [迁移文件说明](../../migrations/README.md)
- [数据库重置指南](../../scripts/RESET_DATABASE_GUIDE.md)
- [费用系统文档](./FEE_SYSTEM_COMPLETE.md)
