# 系统架构 (Architecture)

> 🏗️ IronCore 后端的技术架构设计、多链钱包架构、业务逻辑

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [MULTI_CHAIN_WALLET_ARCHITECTURE.md](./MULTI_CHAIN_WALLET_ARCHITECTURE.md) | 多链钱包完整架构设计 | ✅ 核心 |
| [API_ROUTES_MAP.md](./API_ROUTES_MAP.md) | 46+ API 路由映射表 | ✅ 完成 |
| [BUSINESS_LOGIC.md](./BUSINESS_LOGIC.md) | 业务逻辑详细设计 | ✅ 完成 |

---

## 🎯 快速导航

### 架构师必读
1. **[多链钱包架构](./MULTI_CHAIN_WALLET_ARCHITECTURE.md)** - 完整架构设计
2. **[业务逻辑](./BUSINESS_LOGIC.md)** - 核心业务流程

### API 开发者
1. **[API 路由映射](./API_ROUTES_MAP.md)** - 快速查找 API 路由

---

## 🏗️ 架构概览

### 整体架构

```
┌────────────────────────────────────────────────────────┐
│              IronCore Backend (Port 8088)              │
├────────────────────────────────────────────────────────┤
│                                                          │
│  ┌──────────────────────────────────────────────────┐  │
│  │           API Layer (Axum Router)                │  │
│  │  ┌────────────────────────────────────────────┐ │  │
│  │  │  46+ REST API Endpoints                    │ │  │
│  │  │  - Auth: /api/auth/*                       │ │  │
│  │  │  - Wallet: /api/wallets/*                  │ │  │
│  │  │  - Transaction: /api/transactions/*        │ │  │
│  │  │  - Token: /api/tokens/*                    │ │  │
│  │  │  - Swap: /api/swap/*                       │ │  │
│  │  │  - Payment: /api/payments/*                │ │  │
│  │  └────────────────────────────────────────────┘ │  │
│  └──────────────────┬───────────────────────────────┘  │
│                     │                                   │
│  ┌──────────────────▼───────────────────────────────┐  │
│  │        Middleware Layer                          │  │
│  │  - Authentication (JWT)                          │  │
│  │  - Rate Limiting (100 req/min)                   │  │
│  │  - CSRF Protection                               │  │
│  │  - Idempotency (防重复提交)                      │  │
│  │  - Request/Response Logging                      │  │
│  └──────────────────┬───────────────────────────────┘  │
│                     │                                   │
│  ┌──────────────────▼───────────────────────────────┐  │
│  │         Service Layer (Business Logic)           │  │
│  │  - WalletService (钱包管理)                      │  │
│  │  - TransactionService (交易处理)                 │  │
│  │  - TokenService (代币管理)                       │  │
│  │  - SwapService (代币兑换)                        │  │
│  │  - PaymentService (支付处理)                     │  │
│  └──────────────────┬───────────────────────────────┘  │
│                     │                                   │
│  ┌──────────────────▼───────────────────────────────┐  │
│  │      Repository Layer (Data Access)              │  │
│  │  - WalletRepository                              │  │
│  │  - TransactionRepository                         │  │
│  │  - TokenRepository                               │  │
│  │  - UserRepository                                │  │
│  └──────────────────┬───────────────────────────────┘  │
│                     │                                   │
│  ┌──────────────────▼───────────────────────────────┐  │
│  │    Infrastructure Layer (External Services)      │  │
│  │  - Database (CockroachDB/PostgreSQL)            │  │
│  │  - Cache (Redis)                                 │  │
│  │  - Audit Log (Immudb)                            │  │
│  │  - Monitoring (Prometheus)                       │  │
│  │  - Blockchain Clients (ETH, BSC, Polygon, BTC)  │  │
│  └──────────────────────────────────────────────────┘  │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### 多链支持架构

```
Backend API
    ↓
Chain Abstraction Layer
    ├─ Ethereum Client (Web3, chain_id: 1)
    ├─ BSC Client (Web3, chain_id: 56)
    ├─ Polygon Client (Web3, chain_id: 137)
    ├─ Bitcoin Client (RPC, Mainnet/Testnet)
    └─ [Future] Solana, Cosmos, Cardano...
```

---

## 📚 架构文档详解

### 1️⃣ [多链钱包架构](./MULTI_CHAIN_WALLET_ARCHITECTURE.md) ⭐
**适合**: 架构师、技术负责人、高级工程师

**核心内容**:
- 🏗️ **整体架构设计** - 四层架构（API → Service → Repository → Infrastructure）
- 🔗 **多链支持策略** - 4+ 区块链集成（ETH, BSC, Polygon, BTC）
- 🔒 **非托管模型** - 后端只存储元数据，前端管理私钥
- 📊 **数据分离** - 前端（私钥）+ 后端（地址、交易历史）
- 🚀 **可扩展性** - 插件化架构，易于添加新链

**关键概念**:
- **Chain Abstraction** - 统一接口抽象不同区块链
- **Repository Pattern** - 数据访问层抽象
- **Service Layer** - 业务逻辑封装
- **Middleware Stack** - 认证、限流、日志、CSRF

**阅读时长**: 25 分钟

---

### 2️⃣ [API 路由映射](./API_ROUTES_MAP.md)
**适合**: 前端工程师、API 集成人员

**核心内容**:
- 📋 **46+ API 路由表** - 完整路由列表
- 🔐 **认证要求** - 哪些 API 需要 JWT Token
- 📝 **请求/响应格式** - 标准 JSON 结构
- ⚠️ **错误码映射** - 所有错误码说明

**路由分类**:
| 分类 | 数量 | 示例 |
|------|------|------|
| Auth | 3 | `/api/auth/register`, `/api/auth/login` |
| Wallet | 8 | `/api/wallets`, `/api/wallets/:id` |
| Transaction | 6 | `/api/transactions`, `/api/transactions/:id` |
| Token | 5 | `/api/tokens/balance`, `/api/tokens/price` |
| Swap | 4 | `/api/swap/quote`, `/api/swap/execute` |
| Payment | 3 | `/api/payments/moonpay/url` |
| User | 4 | `/api/users/profile` |
| Notification | 3 | `/api/notifications` |
| Stats | 5 | `/api/stats/dashboard` |
| System | 5 | `/api/health`, `/api/version` |

**阅读时长**: 10 分钟

---

### 3️⃣ [业务逻辑](./BUSINESS_LOGIC.md)
**适合**: 后端工程师、产品经理

**核心内容**:
- 💼 **业务流程** - 钱包创建、交易发送、Swap 执行
- 🔄 **状态机** - 交易状态转换
- ⚖️ **业务规则** - 余额检查、手续费计算
- 📊 **数据一致性** - 事务处理、回滚策略

**业务流程示例 - 钱包创建**:
```
1. 前端生成私钥 + 助记词
   ↓
2. 前端派生地址（BIP44）
   ↓
3. 前端调用 POST /api/wallets
   ↓
4. 后端验证 JWT Token
   ↓
5. 后端存储钱包元数据（name, address, chain）
   ↓
6. 后端返回钱包 ID
   ↓
7. 前端加密存储私钥到 IndexedDB
```

**阅读时长**: 20 分钟

---

## 🔍 架构决策记录 (ADR)

### 为什么选择四层架构？
- ✅ **职责清晰** - 每层单一职责
- ✅ **易于测试** - 每层独立测试
- ✅ **易于扩展** - 插件化设计
- ✅ **易于维护** - 低耦合高内聚

### 为什么使用 Axum？
- ✅ **性能优秀** - 基于 Tokio 异步运行时
- ✅ **类型安全** - 编译时检查
- ✅ **生态成熟** - Tower middleware 生态
- ✅ **可组合性** - 易于扩展

### 为什么选择 CockroachDB？
- ✅ **分布式** - 水平扩展
- ✅ **强一致性** - ACID 事务
- ✅ **PostgreSQL 兼容** - 易于迁移
- ✅ **高可用** - 自动故障转移

### 为什么使用 Repository Pattern？
- ✅ **数据库无关** - 易于切换数据库
- ✅ **易于测试** - Mock Repository
- ✅ **业务逻辑分离** - Service 层不关心存储细节

---

## 📊 架构指标

| 指标 | 目标 | 当前状态 |
|------|------|----------|
| **API 响应时间 (p95)** | < 100ms | 80ms ✅ |
| **并发支持** | 10,000 req/s | 8,500 req/s 🔄 |
| **数据库连接池** | 20-50 | 20 ✅ |
| **缓存命中率** | > 80% | 85% ✅ |
| **错误率** | < 0.1% | 0.05% ✅ |
| **可用性** | 99.9% | 99.95% ✅ |

---

## 🛠️ 架构实施指南

### 添加新 API 端点
1. 在 `src/api/handlers/` 添加 handler
2. 在 `src/api/mod.rs` 注册路由
3. 在 Service 层添加业务逻辑
4. 在 Repository 层添加数据访问
5. 添加单元测试和集成测试
6. 更新 OpenAPI 文档

### 添加新区块链支持
1. 在 `src/blockchain/` 实现 `ChainClient` trait
2. 添加 chain_id 配置
3. 实现地址验证、余额查询、交易发送
4. 添加集成测试
5. 更新文档

---

## 🔗 相关文档

- **配置管理**: [02-configuration/CONFIG_MANAGEMENT.md](../02-configuration/CONFIG_MANAGEMENT.md)
- **数据库设计**: [02-configuration/DATABASE_SCHEMA.md](../02-configuration/DATABASE_SCHEMA.md)
- **API 参考**: [03-api/API_REFERENCE.md](../03-api/API_REFERENCE.md)
- **安全策略**: [02-configuration/SECURITY.md](../02-configuration/SECURITY.md)
- **部署指南**: [05-deployment/DEPLOYMENT.md](../05-deployment/DEPLOYMENT.md)

---

**最后更新**: 2025-12-06  
**维护者**: Backend Architecture Team  
**审查者**: CTO, Lead Engineers
