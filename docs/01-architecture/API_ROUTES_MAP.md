# IronCore-V2 Backend API 路由图

## 📍 完整 API 端点清单（Port 8088）

> ✅ 权威来源：`/openapi.yaml`、`/docs` 以及 `IronCore-V2/src/api/mod.rs`（路由注册）。
>
> 约定：除健康检查外，业务 API 统一使用 `/api/v1/...` 前缀。

### 🌐 公开路由（无需认证）

```
┌─────────────────────────────────────────────────────────────┐
│                        公开 API                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  🔐 认证                                                    │
│  ├─ POST   /api/v1/auth/register     用户注册               │
│  ├─ POST   /api/v1/auth/login        用户登录               │
│  └─ POST   /api/v1/auth/refresh      刷新Token              │
│                                                             │
│  🌐 公共查询                                                 │
│  ├─ GET    /api/v1/chains            链信息列表              │
│  ├─ GET    /api/v1/chains/by-curve   按曲线分组              │
│  ├─ GET    /api/v1/gas/estimate      Gas 估算（单档位）       │
│  └─ GET    /api/v1/gas/estimate-all  Gas 估算（所有档位）     │
│                                                             │
│  ❤️ 健康检查                                                 │
│  ├─ GET    /api/health               API健康状态            │
│  ├─ GET    /healthz                  K8s探针                │
│  └─ GET    /metrics                  Prometheus指标         │
│                                                             │
│  📖 文档                                                     │
│  ├─ GET    /openapi.yaml             OpenAPI规范            │
│  └─ GET    /docs                     Swagger UI             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

### 🔒 受保护路由（需要 JWT 认证）

```
┌─────────────────────────────────────────────────────────────┐
│                      受保护 API                              │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  🔐 认证管理                                                 │
│  ├─ POST   /api/v1/auth/logout       登出                   │
│  ├─ GET    /api/v1/auth/me           当前用户信息           │
│  ├─ POST   /api/v1/auth/set-password 设置密码               │
│  ├─ POST   /api/v1/auth/reset-password 重置密码             │
│  └─ GET    /api/v1/auth/login-history 登录历史              │
│                                                             │
│  👛 钱包（非托管）                                            │
│  ├─ POST   /api/v1/wallets/batch     批量登记钱包（地址/公钥）│
│  ├─ GET    /api/v1/wallets           钱包列表                │
│  ├─ GET    /api/v1/wallets/:id       钱包详情                │
│  ├─ DELETE /api/v1/wallets/:id       删除钱包                │
│  ├─ POST   /api/v1/wallets/unlock    钱包解锁（双锁机制）     │
│  ├─ POST   /api/v1/wallets/lock      钱包锁定                │
│  ├─ GET    /api/v1/wallets/:wallet_id/unlock-status 解锁状态│
│  ├─ GET    /api/v1/wallets/assets    用户资产聚合            │
│  └─ GET    /api/v1/wallets/:id/assets 单钱包资产             │
│                                                             │
│  💸 交易                                                     │
│  ├─ POST   /api/v1/transactions      发送交易（需要客户端签名）│
│  ├─ GET    /api/v1/transactions      交易列表                │
│  ├─ GET    /api/v1/transactions/:hash/status 交易状态         │
│  ├─ GET    /api/v1/transactions/nonce 获取 nonce             │
│  ├─ GET    /api/v1/transactions/history 历史                 │
│  ├─ POST   /api/v1/transactions/broadcast 广播原始交易        │
│  ├─ POST   /api/v1/tx                企业交易记录（兼容）     │
│  ├─ GET    /api/v1/tx                企业交易列表（兼容）     │
│  └─ PUT    /api/v1/tx/:id/status     更新交易状态（兼容）     │
│                                                             │
│  🏢 租户管理                                                 │
│  ├─ POST   /api/v1/tenants           创建租户                │
│  ├─ GET    /api/v1/tenants           租户列表                │
│  ├─ GET    /api/v1/tenants/:id       租户详情                │
│  ├─ PUT    /api/v1/tenants/:id       更新租户                │
│  └─ DELETE /api/v1/tenants/:id       删除租户                │
│                                                             │
│  👤 用户管理                                                 │
│  ├─ POST   /api/v1/users             创建用户                │
│  ├─ GET    /api/v1/users             用户列表                │
│  ├─ GET    /api/v1/users/:id         用户详情                │
│  ├─ PUT    /api/v1/users/:id         更新用户                │
│  └─ DELETE /api/v1/users/:id         删除用户                │
│                                                             │
│  📋 策略管理                                                 │
│  ├─ POST   /api/v1/policies          创建策略                │
│  ├─ GET    /api/v1/policies          策略列表                │
│  ├─ GET    /api/v1/policies/:id      策略详情                │
│  ├─ PUT    /api/v1/policies/:id      更新策略                │
│  └─ DELETE /api/v1/policies/:id      删除策略                │
│                                                             │
│  ✅ 审批管理                                                 │
│  ├─ POST   /api/v1/approvals         创建审批                │
│  ├─ GET    /api/v1/approvals         审批列表                │
│  ├─ GET    /api/v1/approvals/:id     审批详情                │
│  ├─ PUT    /api/v1/approvals/:id/status  更新审批状态       │
│  └─ DELETE /api/v1/approvals/:id     删除审批                │
│                                                             │
│  🔑 API 密钥管理                                             │
│  ├─ POST   /api/v1/api-keys          创建API密钥             │
│  ├─ GET    /api/v1/api-keys          API密钥列表             │
│  ├─ GET    /api/v1/api-keys/:id      API密钥详情             │
│  ├─ PUT    /api/v1/api-keys/:id/status  更新密钥状态        │
│  └─ DELETE /api/v1/api-keys/:id      删除API密钥             │
│                                                             │
│  📡 交易广播                                                 │
│  ├─ POST   /api/v1/tx-broadcasts     创建交易广播            │
│  ├─ GET    /api/v1/tx-broadcasts     广播列表                │
│  ├─ GET    /api/v1/tx-broadcasts/:id 广播详情                │
│  ├─ PUT    /api/v1/tx-broadcasts/:id 更新广播                │
│  └─ GET    /api/v1/tx-broadcasts/by-tx-hash/:hash 按哈希查询│
│                                                             │
│  ⛽ 区块链查询                                               │
│  ├─ POST   /api/v1/fees/calculate    平台服务费计算          │
│  ├─ GET    /api/v1/gas/estimate-all  Gas估算（所有档位）      │
│  └─ GET    /api/v1/balance            余额查询               │
│                                                             │
│  🔄 Bridge（跨链）                                            │
│  ├─ POST   /api/v1/bridge/quote      跨链报价                │
│  ├─ POST   /api/v1/bridge/execute    执行跨链（需要签名/授权） │
│  ├─ GET    /api/v1/bridge/:id/status 执行状态                │
│  └─ GET    /api/v1/bridge/history    历史记录                │
│                                                             │
│  🪙 Tokens                                                   │
│  ├─ GET    /api/v1/tokens/list       Token 列表              │
│  ├─ GET    /api/v1/tokens/search     Token 搜索              │
│  ├─ GET    /api/v1/tokens/popular    热门 Token              │
│  ├─ GET    /api/v1/tokens/metadata   Token 元数据            │
│  └─ GET    /api/v1/tokens/:token_address/balance 余额         │
│                                                             │
│  🔁 Swap                                                     │
│  ├─ GET    /api/v1/swap/quote        报价                    │
│  ├─ POST   /api/v1/swap/execute      执行                    │
│  ├─ GET    /api/v1/swap/history      历史                    │
│  └─ GET    /api/v1/swap/:id/status   状态                    │
│                                                             │
│  🛠️ Admin                                                    │
│  ├─ GET    /api/v1/admin/fee-rules   平台费规则              │
│  ├─ POST   /api/v1/admin/fee-rules   创建规则                │
│  └─ ...（更多请以 OpenAPI 为准）                             │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 🎯 API 使用建议

### ✅ 推荐使用

#### 钱包登记（非托管）
```bash
# 后端只接收公开信息（地址/公钥），助记词/私钥永远不上传
POST /api/v1/wallets/batch
Authorization: Bearer <token>
Content-Type: application/json

{
  "wallets": [
    {
      "chain": "ethereum",
      "address": "0xYourDerivedAddress",
      "public_key": "0xYourDerivedPublicKey",
      "name": "My Wallet"
    }
  ]
}
```

#### 钱包查询
```bash
# 列表
GET /api/v1/wallets
Authorization: Bearer <token>

# 详情
GET /api/v1/wallets/{id}
Authorization: Bearer <token>
```

> 完整端点列表与认证要求请以 `/openapi.yaml` 与 Swagger UI(`/docs`) 为准。

## 🔄 中间件栈

### 公开路由中间件
```
Request
  ↓
1. set_request_id           # 生成请求ID
  ↓
2. trace_log                # 日志追踪
  ↓
3. add_response_time_header # 响应时间
  ↓
4. add_cors_headers         # CORS支持
  ↓
Handler
  ↓
Response
```

### 受保护路由中间件
```
Request
  ↓
1. set_request_id           # 生成请求ID
  ↓
2. trace_log                # 日志追踪
  ↓
3. add_response_time_header # 响应时间
  ↓
4. add_cors_headers         # CORS支持
  ↓
5. add_api_version_header   # API版本
  ↓
6. add_security_headers     # 安全头
  ↓
7. auth_middleware          # JWT验证 ⚠️
  ↓
8. rate_limit_middleware    # 速率限制
  ↓
9. idempotency_middleware   # 幂等性检查
  ↓
Handler
  ↓
Response
```

---

## 🛡️ 安全特性

### 1. 认证
- JWT Token 认证
- Token 过期时间: 1 小时
- Refresh Token 支持

### 2. CORS
- 允许来源: 可配置（默认 `*`）
- 允许方法: GET, POST, PUT, DELETE, OPTIONS
- 允许头: Content-Type, Authorization, Idempotency-Key, X-Request-Id

### 3. 安全头
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Referrer-Policy: no-referrer`
- `Cache-Control: no-store`
- `Content-Security-Policy: default-src 'self'`
- `Strict-Transport-Security: max-age=31536000` (HTTPS)

### 4. 速率限制
- 默认: 100 请求/分钟/IP
- 可通过环境变量配置

### 5. 幂等性
- 支持 `Idempotency-Key` 头
- 防止重复请求

---

## 📈 性能指标

### 响应时间（P95）
| 端点类型 | 响应时间 |
|---------|---------|
| 健康检查 | < 1ms |
| 链列表查询 | < 1ms |
| 钱包派生 | 13ms |
| 统一创建 | 22-33ms |
| 数据库查询 | 5-10ms |

### 吞吐量
- 健康检查: ~10,000 req/s
- 钱包创建: ~500 req/s
- 数据库写入: ~200 req/s

---

## 🔧 配置

### 环境变量
```bash
# 服务器
BIND_ADDR=0.0.0.0:8088

# 数据库
DATABASE_URL=postgres://root@localhost:26257/ironcore

# Redis
REDIS_URL=redis://localhost:6379

# JWT
JWT_SECRET=<your-secret>
TOKEN_EXPIRY_SECS=3600

# CORS
CORS_ALLOW_ORIGINS=*

# 安全
HSTS_ENABLE=1  # 仅在 HTTPS 时启用
```

---

## 📚 相关文档

- [API 清理分析](./API_CLEANUP_ANALYSIS.md)
- [API 清理总结](./API_CLEANUP_SUMMARY.md)
- [多链钱包架构](./MULTI_CHAIN_WALLET_ARCHITECTURE.md)
- [集成完成报告](./INTEGRATION_COMPLETE_REPORT.md)

---

**最后更新**: 2025-11-23  
**API 版本**: v1  
**后端版本**: v0.1.0
