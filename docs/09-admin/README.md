# 管理后台 (Admin Control Plane)

> 🔐 管理员功能、系统配置、用户管理、审计日志

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [ADMIN_GUIDE.md](../../ADMIN_CONTROL_PLANE_GUIDE.md) | 管理后台完整指南 | ✅ 核心 |

---

## 🎯 快速导航

### 系统管理员
- 🔐 **[管理后台指南](../../ADMIN_CONTROL_PLANE_GUIDE.md)** - 完整管理功能

---

## 🏗️ 管理后台架构

### 管理功能模块

```
┌─────────────────────────────────────────────┐
│       管理后台 (Admin Control Plane)         │
├─────────────────────────────────────────────┤
│                                              │
│  👤 用户管理 (User Management)              │
│     ├─ 用户列表 (分页、搜索、过滤)          │
│     ├─ 用户详情 (钱包、交易、统计)          │
│     ├─ 用户状态管理 (启用、禁用、删除)      │
│     └─ 用户权限管理 (角色、权限)            │
│                                              │
│  👛 钱包管理 (Wallet Management)            │
│     ├─ 钱包列表 (按用户、按链)              │
│     ├─ 钱包详情 (余额、代币、NFT)           │
│     ├─ 钱包监控 (异常活动、大额交易)        │
│     └─ 钱包统计 (按链、按类型)              │
│                                              │
│  💸 交易管理 (Transaction Management)       │
│     ├─ 交易列表 (实时监控)                  │
│     ├─ 交易详情 (链上验证)                  │
│     ├─ 异常交易标记 (失败、高费用)          │
│     └─ 交易统计 (成功率、金额)              │
│                                              │
│  🪙 代币管理 (Token Management)             │
│     ├─ 代币列表 (支持的代币)                │
│     ├─ 代币价格管理 (价格源配置)            │
│     ├─ 新增代币 (审核、上架)                │
│     └─ 代币统计 (持有者、交易量)            │
│                                              │
│  📊 系统监控 (System Monitoring)            │
│     ├─ 实时指标 (CPU、内存、请求数)         │
│     ├─ 健康检查 (服务状态)                  │
│     ├─ 性能监控 (响应时间、吞吐量)          │
│     └─ 告警管理 (告警历史、告警规则)        │
│                                              │
│  📝 审计日志 (Audit Logs)                   │
│     ├─ 操作日志 (谁、何时、做了什么)        │
│     ├─ 登录日志 (成功、失败、IP)            │
│     ├─ 敏感操作日志 (删除、修改权限)        │
│     └─ 日志导出 (CSV、JSON)                 │
│                                              │
│  ⚙️ 系统配置 (System Configuration)         │
│     ├─ 全局配置 (费率、限流)                │
│     ├─ 链配置 (RPC、Gas)                    │
│     ├─ 第三方配置 (MoonPay、价格 API)       │
│     └─ 功能开关 (Feature Flags)             │
│                                              │
│  🔐 权限管理 (Permission Management)        │
│     ├─ 角色管理 (Admin、Operator、Viewer)   │
│     ├─ 权限分配 (RBAC)                      │
│     ├─ API 密钥管理 (生成、撤销)            │
│     └─ IP 白名单 (访问控制)                 │
│                                              │
└─────────────────────────────────────────────┘
```

---

## 📚 管理后台文档详解

### 1️⃣ [管理后台完整指南](../../ADMIN_CONTROL_PLANE_GUIDE.md) ⭐
**适合**: 系统管理员、运营人员、安全团队

**核心内容**:
- 👤 **用户管理** - 用户 CRUD、状态管理
- 👛 **钱包管理** - 钱包监控、异常检测
- 💸 **交易管理** - 交易监控、统计分析
- 📊 **系统监控** - 实时监控、告警管理
- 📝 **审计日志** - 操作审计、合规性
- ⚙️ **系统配置** - 参数配置、功能开关

**用户管理 API**:
```bash
# 获取用户列表（分页）
GET /api/admin/users?page=1&page_size=20&search=email@example.com
Authorization: Bearer <admin_token>

# Response
{
  "success": true,
  "data": {
    "items": [
      {
        "id": "123",
        "email": "user@example.com",
        "status": "active",
        "wallets_count": 5,
        "created_at": "2025-01-01T00:00:00Z"
      }
    ],
    "pagination": {
      "page": 1,
      "page_size": 20,
      "total_items": 1000,
      "total_pages": 50
    }
  }
}

# 获取用户详情
GET /api/admin/users/:id
Authorization: Bearer <admin_token>

# Response
{
  "success": true,
  "data": {
    "id": "123",
    "email": "user@example.com",
    "status": "active",
    "wallets": [...],
    "transactions": [...],
    "stats": {
      "total_wallets": 5,
      "total_transactions": 120,
      "total_volume_usd": 15000.50
    }
  }
}

# 禁用用户
PUT /api/admin/users/:id/disable
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "reason": "Suspicious activity detected"
}
```

**交易监控 API**:
```bash
# 获取实时交易
GET /api/admin/transactions?status=pending&sort=created_at:desc
Authorization: Bearer <admin_token>

# 标记异常交易
POST /api/admin/transactions/:id/flag
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "reason": "High gas fee",
  "severity": "medium"
}
```

**系统配置 API**:
```bash
# 获取全局配置
GET /api/admin/config
Authorization: Bearer <admin_token>

# Response
{
  "success": true,
  "data": {
    "rate_limit": {
      "default": 100,
      "authenticated": 500
    },
    "fee_rates": {
      "swap": 0.003,
      "payment": 0.01
    },
    "feature_flags": {
      "swap_enabled": true,
      "nft_enabled": false
    }
  }
}

# 更新配置
PUT /api/admin/config
Authorization: Bearer <admin_token>
Content-Type: application/json

{
  "rate_limit": {
    "default": 200
  }
}
```

**阅读时长**: 45 分钟

---

## 🔐 权限管理

### RBAC 角色定义

```rust
pub enum Role {
    // 超级管理员 (所有权限)
    SuperAdmin,
    
    // 管理员 (用户管理、配置管理)
    Admin,
    
    // 运维人员 (系统监控、日志查看)
    Operator,
    
    // 查看者 (只读权限)
    Viewer,
}

pub struct Permission {
    pub resource: String,  // users, wallets, transactions, config
    pub action: Action,    // read, write, delete
}

pub enum Action {
    Read,
    Write,
    Delete,
}
```

### 权限矩阵

| 角色 | 用户管理 | 钱包管理 | 交易管理 | 系统配置 | 审计日志 |
|------|---------|---------|---------|---------|---------|
| SuperAdmin | ✅ 读写删 | ✅ 读写删 | ✅ 读写删 | ✅ 读写 | ✅ 读写 |
| Admin | ✅ 读写 | ✅ 读写 | ✅ 读 | ✅ 读写 | ✅ 读 |
| Operator | ✅ 读 | ✅ 读 | ✅ 读 | ✅ 读 | ✅ 读 |
| Viewer | ✅ 读 | ✅ 读 | ✅ 读 | ❌ | ✅ 读 |

---

## 📊 管理后台统计

### 仪表盘指标

```
实时指标 (Real-time Metrics)
  ├─ 在线用户数: 1,234
  ├─ 活跃钱包数: 5,678
  ├─ 今日交易数: 12,345
  └─ 今日交易额: $1,234,567

趋势分析 (Trend Analysis)
  ├─ 用户增长率: +15% (vs 上周)
  ├─ 交易成功率: 99.5%
  ├─ 平均响应时间: 85ms
  └─ 错误率: 0.05%

热门资产 (Top Assets)
  ├─ ETH: 45% 持有量
  ├─ USDT: 30% 持有量
  ├─ USDC: 15% 持有量
  └─ BTC: 10% 持有量

异常告警 (Anomalies)
  ├─ 0 条严重告警
  ├─ 3 条警告告警
  └─ 15 条信息告警
```

---

## 📝 审计日志示例

### 操作日志格式

```json
{
  "id": "audit-123",
  "timestamp": "2025-12-06T12:00:00Z",
  "actor": {
    "id": "admin-456",
    "email": "admin@example.com",
    "role": "Admin",
    "ip": "192.168.1.100"
  },
  "action": "USER_DISABLED",
  "resource": {
    "type": "user",
    "id": "user-789",
    "email": "target@example.com"
  },
  "details": {
    "reason": "Suspicious activity detected",
    "previous_state": "active",
    "new_state": "disabled"
  },
  "metadata": {
    "user_agent": "Mozilla/5.0...",
    "trace_id": "abc123xyz"
  }
}
```

### 敏感操作类型

| 操作 | 描述 | 审计级别 |
|------|------|----------|
| `USER_CREATED` | 创建用户 | INFO |
| `USER_DISABLED` | 禁用用户 | WARNING |
| `USER_DELETED` | 删除用户 | CRITICAL |
| `CONFIG_UPDATED` | 更新配置 | WARNING |
| `ROLE_ASSIGNED` | 分配角色 | WARNING |
| `API_KEY_CREATED` | 创建 API 密钥 | INFO |
| `API_KEY_REVOKED` | 撤销 API 密钥 | WARNING |

---

## 🔍 监控与告警

### 异常检测规则

```yaml
# 异常用户检测
- name: SuspiciousUser
  conditions:
    - login_failures > 10 in 1h
    - multiple_ips in 1h
    - high_transaction_volume > $100k in 1h
  action:
    - flag_user
    - notify_admin
    - require_verification

# 异常交易检测
- name: SuspiciousTransaction
  conditions:
    - amount > $50k
    - gas_fee > 0.1 ETH
    - to_address in blacklist
  action:
    - flag_transaction
    - notify_compliance_team
    - delay_confirmation

# 系统异常检测
- name: SystemAnomaly
  conditions:
    - error_rate > 1%
    - response_time > 500ms
    - cpu_usage > 90%
  action:
    - alert_sre_team
    - auto_scale
    - health_check
```

---

## 🔗 相关文档

- **配置管理**: [02-configuration/CONFIG_MANAGEMENT.md](../02-configuration/CONFIG_MANAGEMENT.md)
- **安全策略**: [02-configuration/SECURITY.md](../02-configuration/SECURITY.md)
- **监控告警**: [07-monitoring/MONITORING.md](../07-monitoring/MONITORING.md)
- **运维手册**: [06-operations/OPERATIONS.md](../06-operations/OPERATIONS.md)

---

**最后更新**: 2025-12-06  
**维护者**: Admin Platform Team  
**审查者**: Security Lead, Compliance Officer
