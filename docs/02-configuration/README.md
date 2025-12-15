# 配置与安全 (Configuration & Security)

> ⚙️ 配置管理、数据库设计、安全策略、企业就绪性

---

## 📂 本分类文档

| 文档 | 描述 | 行数 | 状态 |
|------|------|------|------|
| [CONFIG_MANAGEMENT.md](./CONFIG_MANAGEMENT.md) | 配置管理完整指南 | 850+ | ✅ 核心 |
| [DATABASE_SCHEMA.md](./DATABASE_SCHEMA.md) | 数据库模式设计 | 1,200+ | ✅ 核心 |
| [SECURITY.md](./SECURITY.md) | 安全策略与实践 | 950+ | ✅ 核心 |
| [DATABASE_AUDIT_REPORT.md](./DATABASE_AUDIT_REPORT.md) | 数据库审计报告 | 450 | ✅ 完成 |
| [DATABASE_INTEGRITY_CHECK.md](./DATABASE_INTEGRITY_CHECK.md) | 数据库完整性检查 | 320 | ✅ 完成 |
| [ENTERPRISE_READINESS_REPORT.md](./ENTERPRISE_READINESS_REPORT.md) | 企业就绪性报告 | 680 | ✅ 完成 |
| [FEE_SYSTEM_COMPLETE.md](./FEE_SYSTEM_COMPLETE.md) | 手续费系统设计 | 280 | ✅ 完成 |
| [SECURITY_VERIFICATION.md](./SECURITY_VERIFICATION.md) | 安全验证报告 | 420 | ✅ 完成 |
| [API_VERSIONING_POLICY.md](./API_VERSIONING_POLICY.md) | API 版本策略 | 220 | ✅ 完成 |

---

## 🎯 快速导航

### 配置管理
- 📘 **[配置管理指南](./CONFIG_MANAGEMENT.md)** - 环境变量、配置文件、最佳实践
- 🔧 **[API 版本策略](./API_VERSIONING_POLICY.md)** - API 版本管理

### 数据库设计
- 📊 **[数据库模式](./DATABASE_SCHEMA.md)** - 15+ 核心表设计
- 🔍 **[数据库审计](./DATABASE_AUDIT_REPORT.md)** - 审计报告
- ✅ **[完整性检查](./DATABASE_INTEGRITY_CHECK.md)** - 数据完整性验证

### 安全策略
- 🔐 **[安全策略](./SECURITY.md)** - 认证、授权、加密、审计
- ✅ **[安全验证](./SECURITY_VERIFICATION.md)** - 安全验证报告

### 系统设计
- 💰 **[手续费系统](./FEE_SYSTEM_COMPLETE.md)** - Gas 估算、手续费计算
- 🏢 **[企业就绪性](./ENTERPRISE_READINESS_REPORT.md)** - 生产环境检查清单

---

## ⚙️ 配置管理架构

### 配置加载优先级

```
1. 命令行参数（最高优先级）
   ↓
2. 环境变量
   ↓
3. config.toml 配置文件
   ↓
4. 默认值（最低优先级）
```

### 配置文件结构

```toml
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = false  # 生产环境必须 false

[database]
url = "postgres://root@localhost:26257/ironcore?sslmode=disable"
max_connections = 20
connect_timeout = 30
idle_timeout = 600

[redis]
url = "redis://localhost:6379"
pool_size = 10

[immudb]
addr = "localhost:3322"
user = "immudb"
password = "immudb"
database = "ironcore_audit"

[jwt]
secret = "your-production-secret-min-32-chars-long"
token_expiry_secs = 3600  # 1 hour

[logging]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json or pretty

[monitoring]
prometheus_port = 9090
health_check_interval = 30
```

---

## 📚 配置文档详解

### 1️⃣ [配置管理指南](./CONFIG_MANAGEMENT.md) ⭐
**适合**: DevOps, SRE, 后端工程师

**核心内容**:
- ⚙️ **配置加载机制** - 优先级、覆盖规则
- 🔐 **敏感信息管理** - JWT secret, 数据库密码
- 🌍 **多环境配置** - dev, staging, production
- 🔄 **热重载** - 无需重启更新配置
- 📝 **配置验证** - 启动时检查配置有效性

**环境变量映射**:
| 环境变量 | 配置项 | 默认值 |
|----------|--------|--------|
| `DATABASE_URL` | database.url | - |
| `REDIS_URL` | redis.url | redis://localhost:6379 |
| `JWT_SECRET` | jwt.secret | - (必填) |
| `LOG_LEVEL` | logging.level | info |
| `BIND_ADDR` | server.bind_addr | 127.0.0.1:8088 |

**最佳实践**:
```bash
# ✅ 生产环境：使用环境变量
export DATABASE_URL="postgres://..."
export JWT_SECRET="$(openssl rand -base64 32)"

# ✅ 开发环境：使用 config.toml
cp config.example.toml config.toml
vim config.toml

# ❌ 不要：硬编码敏感信息
const JWT_SECRET = "hardcoded-secret";  // 危险！
```

**阅读时长**: 15 分钟

---

### 2️⃣ [数据库模式设计](./DATABASE_SCHEMA.md) ⭐
**适合**: 数据库工程师、后端工程师

**核心内容**:
- 📋 **15+ 核心表设计** - users, wallets, transactions, tokens...
- 🔗 **外键关系** - 表间关系图
- 🔍 **索引策略** - 单列索引、复合索引、覆盖索引
- 🗄️ **分区方案** - 时间分区、哈希分区
- 📊 **查询优化** - N+1 问题、批量查询

**核心表**:
| 表名 | 描述 | 行数估计 |
|------|------|----------|
| `users` | 用户账户 | 100 万+ |
| `wallets` | 钱包元数据 | 500 万+ |
| `transactions` | 交易历史 | 1 亿+ |
| `tokens` | 代币信息 | 1 万+ |
| `nft_assets` | NFT 资产 | 50 万+ |
| `swap_quotes` | Swap 报价 | 500 万+ |
| `payment_orders` | 支付订单 | 100 万+ |

**DDL 示例**:
```sql
CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    address VARCHAR(255) NOT NULL,
    chain VARCHAR(50) NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now(),
    
    CONSTRAINT unique_user_wallet UNIQUE(user_id, address, chain)
);

-- 索引优化
CREATE INDEX idx_wallets_user_id ON wallets(user_id);
CREATE INDEX idx_wallets_address ON wallets(address);
CREATE INDEX idx_wallets_chain ON wallets(chain);
CREATE INDEX idx_wallets_created_at ON wallets(created_at DESC);
```

**阅读时长**: 30 分钟

---

### 3️⃣ [安全策略与实践](./SECURITY.md) ⭐
**适合**: 安全工程师、架构师、合规人员

**核心内容**:
- 🔐 **认证机制** - JWT Token, Refresh Token
- 🔑 **授权策略** - RBAC (Role-Based Access Control)
- 🔒 **加密方案** - AES-256-GCM, Argon2id
- 📝 **审计日志** - Immudb 不可变审计
- 🛡️ **攻击防护** - SQL 注入、XSS、CSRF、Rate Limiting

**认证流程**:
```
1. 用户登录 POST /api/auth/login
   ↓
2. 验证 email + password（Argon2id）
   ↓
3. 生成 JWT Token（1 小时过期）
   ↓
4. 生成 Refresh Token（7 天过期）
   ↓
5. 返回两个 Token
   ↓
6. 前端存储 Token（HttpOnly Cookie 或 LocalStorage）
   ↓
7. 每次 API 请求携带: Authorization: Bearer <token>
   ↓
8. Token 过期前调用 /api/auth/refresh 刷新
```

**安全最佳实践清单**:
- [x] JWT Secret 至少 32 字节
- [x] Token 1 小时过期
- [x] HTTPS Only（生产环境）
- [x] Rate Limiting（100 req/min）
- [x] SQL 参数化查询（防注入）
- [x] 输入验证（所有 API）
- [x] 输出编码（防 XSS）
- [x] CSRF Token（状态改变 API）
- [x] 审计日志（所有关键操作）
- [x] 定期安全扫描（cargo audit）

**阅读时长**: 25 分钟

---

### 4️⃣ [手续费系统](./FEE_SYSTEM_COMPLETE.md)
**适合**: 后端工程师、产品经理

**核心内容**:
- ⛽ **Gas 估算** - 动态 Gas Price 获取
- 💰 **手续费计算** - Gas Limit × Gas Price
- 🔄 **费用优化** - EIP-1559, Layer 2
- 📊 **费用监控** - 实时费用追踪

**Gas 估算流程**:
```rust
// 1. 获取当前 Gas Price
let gas_price = provider.get_gas_price().await?;

// 2. 估算 Gas Limit
let gas_limit = provider.estimate_gas(&tx).await?;

// 3. 计算总手续费
let total_fee = gas_price * gas_limit;

// 4. 返回给前端
Ok(FeeEstimate {
    gas_price: gas_price.as_u64(),
    gas_limit: gas_limit.as_u64(),
    total_fee: total_fee.as_u64(),
    estimated_usd: total_fee_in_usd,
})
```

**阅读时长**: 10 分钟

---

## 🔍 安全架构

### 多层防护

```
┌────────────────────────────────────────┐
│  Layer 1: Network (边界防护)           │
│  - Firewall                            │
│  - DDoS Protection                     │
│  - WAF (Web Application Firewall)     │
└────────────────┬───────────────────────┘
                 │
┌────────────────▼───────────────────────┐
│  Layer 2: API Gateway (流量控制)       │
│  - Rate Limiting (100 req/min)        │
│  - IP Whitelist/Blacklist             │
│  - Request Size Limit                 │
└────────────────┬───────────────────────┘
                 │
┌────────────────▼───────────────────────┐
│  Layer 3: Application (应用层)         │
│  - JWT Authentication                 │
│  - RBAC Authorization                 │
│  - Input Validation                   │
│  - Output Encoding                    │
│  - CSRF Protection                    │
└────────────────┬───────────────────────┘
                 │
┌────────────────▼───────────────────────┐
│  Layer 4: Data (数据层)                │
│  - Parameterized Queries (防注入)     │
│  - Encryption at Rest                 │
│  - Encryption in Transit (TLS 1.3)    │
│  - Audit Logging (Immudb)             │
└────────────────────────────────────────┘
```

---

## 📊 配置与安全指标

| 指标 | 目标 | 当前状态 |
|------|------|----------|
| **JWT Token 长度** | ≥ 32 bytes | 32 bytes ✅ |
| **Token 过期时间** | 1 hour | 1 hour ✅ |
| **Rate Limit** | 100 req/min | 100 req/min ✅ |
| **TLS 版本** | TLS 1.3 | TLS 1.3 ✅ |
| **数据库加密** | AES-256 | AES-256 ✅ |
| **审计日志覆盖率** | 100% 关键操作 | 100% ✅ |
| **安全漏洞** | 0 高危 | 0 高危 ✅ |

---

## 🔗 相关文档

- **系统架构**: [01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)
- **API 参考**: [03-api/API_REFERENCE.md](../03-api/API_REFERENCE.md)
- **部署指南**: [05-deployment/DEPLOYMENT.md](../05-deployment/DEPLOYMENT.md)
- **监控告警**: [07-monitoring/MONITORING.md](../07-monitoring/MONITORING.md)
- **错误处理**: [08-error-handling/ERROR_HANDLING.md](../08-error-handling/ERROR_HANDLING.md)

---

**最后更新**: 2025-12-06  
**维护者**: Security & Infrastructure Team  
**审查者**: CISO, CTO, Lead Engineers
