# IronCore Backend - 企业级多链钱包后端

> 生产级后端 API 服务器 | OpenAPI 驱动 | 非托管架构

**端口**: 8088  
**技术栈**: Rust + Axum + PostgreSQL/CockroachDB + Redis + Immudb

> 说明：接口数量、覆盖率、性能指标等属于“快照数据”，可能随迭代变化；请以 OpenAPI（`/openapi.yaml`）与 CI 结果为准。

---

## 📚 完整文档导航

**👉 [查看完整文档索引](./docs/INDEX.md)** ⭐ | [一页纸总结](./ONE_PAGE_SUMMARY_IRONCORE_2025-12-06.md)

### 🎯 按角色快速导航


# 创建/编辑 config.toml（也可从 config.example.toml 复制）
# 关键：allow_degraded_start=true 可在无数据库时启动

## 📂 文档分类 (12个分类，100%覆盖)
启动服务：

```bash
cargo run
```

本地访问：

- 健康检查：`http://127.0.0.1:8088/healthz` 或 `http://127.0.0.1:8088/api/health`
- OpenAPI（权威）：`http://127.0.0.1:8088/openapi.yaml`
- Swagger UI：`http://127.0.0.1:8088/docs/`
| 02 | **⚙️ [配置与安全](./docs/02-configuration/README.md)** | 9 份 | 配置管理、数据库、安全策略 |
| 03 | **📡 [API 设计](./docs/03-api/README.md)** | 3 份 | 46+ API、错误码、Gas 估算 |
| 04 | **🧪 [测试](./docs/04-testing/README.md)** | 2 份 | 900+ 测试、85% 覆盖率 |
| 05 | **🚀 [部署](./docs/05-deployment/README.md)** | 2 份 | Docker、生产环境、高可用 |
| 06 | **⚙️ [运维](./docs/06-operations/README.md)** | 2 份 | 日常运维、备份恢复、调优 |
| 07 | **📊 [监控](./docs/07-monitoring/README.md)** | 2 份 | Prometheus + Grafana |
| 08 | **⚠️ [错误处理](./docs/08-error-handling/README.md)** | 1 份 | 错误码、日志规范、排查 |
| 09 | **🔐 [管理后台](./docs/09-admin/README.md)** | 1 份 | 用户管理、系统配置、审计 |
| 10 | **📊 [项目报告](./docs/10-reports/README.md)** | 4 份 | 完成度、性能分析、就绪性 |
| 11 | **💻 [开发指南](./docs/11-development/README.md)** | 4 份 | 规范、CI/CD、贡献指南 |

---

## ⭐ 核心文档推荐 (Top 10)

### 必读 (P0)
1. **[快速开始 README](./docs/00-quickstart/README.md)** ⭐⭐⭐ - 5 分钟快速上手
2. **[系统架构 README](./docs/01-architecture/README.md)** ⭐⭐⭐ - 完整架构设计
3. **[API 参考 README](./docs/03-api/README.md)** ⭐⭐⭐ - 46+ API 完整文档
4. **[配置管理指南](./docs/02-configuration/CONFIG_MANAGEMENT.md)** ⭐⭐⭐ - 环境变量、配置文件

### 重要 (P1)
5. **[数据库设计](./docs/02-configuration/DATABASE_SCHEMA.md)** ⭐⭐ - 15+ 核心表设计
6. **[安全策略](./docs/02-configuration/SECURITY.md)** ⭐⭐ - 认证、授权、加密
7. **[部署指南](./docs/05-deployment/DEPLOYMENT.md)** ⭐⭐ - 生产环境部署
8. **[监控告警](./docs/07-monitoring/MONITORING.md)** ⭐⭐ - Prometheus 完整方案

### 参考 (P2)
9. **[错误处理](./docs/08-error-handling/ERROR_HANDLING.md)** ⭐ - 错误码标准
10. **[开发规范](./docs/11-development/README.md)** ⭐ - 代码规范、CI/CD

---

## 🚀 快速启动

### 最简模式（无数据库）

```bash
cd IronCore-V2

# 创建配置文件
cat > config.toml << EOF
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = true

[jwt]
secret = "dev-jwt-secret-min-32-chars-long-xxxxx"
EOF

# 启动服务
cargo run
```

访问 http://localhost:8088/api/health

---

### 完整模式（带数据库）

```bash
# 1. 启动基础设施
cd ops
docker compose up -d

# 2. 配置环境变量
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
export REDIS_URL="redis://localhost:6379"
export JWT_SECRET="your-production-secret-min-32-chars"

# 3. 启动服务
cd ../IronCore-V2
cargo run
```

---

## ⚙️ 配置说明

所有配置通过 `config.toml` 或环境变量：

```toml
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = false  # 允许降级启动

[database]
url = "postgres://..."
max_connections = 20

[redis]
url = "redis://localhost:6379"

[jwt]
secret = "your-secret-key"
token_expiry_secs = 3600

[logging]
level = "info"
format = "json"

[monitoring]
enable_prometheus = true
```

**详细说明**: [配置管理指南](./docs/02-configuration/CONFIG_MANAGEMENT.md)

---

## 📡 API 端点

### 健康检查
- `GET /api/health` - 基础健康检查（兼容别名：`GET /health`）
- `GET /healthz` - 详细探活（推荐用于 readiness 探针）

### 监控
- `GET /metrics` - Prometheus 指标

### OpenAPI（权威）
- `GET /openapi.yaml` - OpenAPI 文档
- `GET /docs/` - Swagger UI（`/docs` 会重定向到 `/docs/`）

### 业务 API（权威）
- 所有业务接口统一在 `/api/v1/...`，请以 OpenAPI 为准（避免复制 README 中的历史清单）。

---

## 🏗️ 架构设计

### 分层架构

```
┌──────────────────┐
│   API Layer      │  ◄─── handlers, middleware
├──────────────────┤
│  Service Layer   │  ◄─── business logic
├──────────────────┤
│ Repository Layer │  ◄─── data access
├──────────────────┤
│Infrastructure    │  ◄─── db, cache, monitoring
└──────────────────┘
```

**详细说明**: [多链钱包架构](./docs/01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)

---

## 🔒 安全特性

- **非托管架构**: 私钥永不触及后端
- **JWT 认证**: Token 过期机制
- **API 密钥**: SHA-256 哈希存储
- **密码哈希**: Argon2id 算法
- **速率限制**: 100 req/min 默认
- **CSRF 保护**: SameSite cookies
- **审计日志**: Immudb 不可变日志

**详细说明**: [安全策略与实践](./docs/02-configuration/SECURITY.md)

---

## 📊 监控与观测

### Prometheus 指标

```
http://localhost:8088/metrics
```

### 关键指标
- `http_requests_total` - 请求总数
- `http_request_duration_seconds` - 请求延迟
- `db_pool_connections` - 数据库连接池
- `redis_operations_total` - Redis 操作
- `transactions_confirmed_total` - 交易确认数

**详细说明**: [监控告警指南](./docs/07-monitoring/MONITORING.md)

---

## ⚡ 性能优化

### 性能目标
- **P50 延迟**: < 100ms
- **P95 延迟**: < 500ms
- **RPS**: > 1000
- **可用性**: 99.9%

### 优化策略
- 两层缓存（Memory + Redis）
- 数据库连接池优化
- 索引设计优化
- 异步 I/O 并发
- HTTP/2 支持

**详细说明**: [性能优化指南](./docs/07-monitoring/PERFORMANCE.md)

---

## 🧪 测试

```bash
# 运行所有测试
cargo test --workspace

# 运行集成测试
cargo test --test '*'

# 运行基准测试
cargo bench

# 生成测试覆盖率
cargo tarpaulin --out Html
```

---

## 🗄️ 数据库

### 支持的数据库
- **CockroachDB** (推荐) - 分布式 SQL
- **PostgreSQL** - 传统关系型数据库

### 核心表
- `users` - 用户表
- `wallets` - 钱包表
- `transactions` - 交易表
- `assets` - 资产表
- `api_keys` - API 密钥表

**详细说明**: [数据库模式设计](./docs/02-configuration/DATABASE_SCHEMA.md)

---

## 🚀 部署

### Docker 部署

```bash
# 构建镜像
docker build -t ironcore .

# 运行容器
docker run -p 8088:8088 \
  -e DATABASE_URL="postgres://..." \
  -e JWT_SECRET="..." \
  ironcore
```

### 生产环境

```bash
# 编译优化版本
cargo build --release

# 运行
./target/release/ironcore
```

### Fly.io（已发布）

- Backend（默认域名）：https://oxidevault-ironcore-v2.fly.dev
- 健康检查：https://oxidevault-ironcore-v2.fly.dev/healthz
- OpenAPI：https://oxidevault-ironcore-v2.fly.dev/openapi.yaml
- Swagger UI：https://oxidevault-ironcore-v2.fly.dev/docs/

注：Fly 配置默认设置 `SKIP_MIGRATIONS=1`（见 `fly.toml`），避免发布滚动时因迁移阻塞导致健康检查失败。需要迁移时请在受控窗口单独执行。

**详细说明**: [部署指南](./docs/05-deployment/DEPLOYMENT.md)

---

## 📁 项目结构

```
IronCore-V2/
├── src/                  # Rust 源代码
│   ├── api/              # API 路由和处理器
│   ├── service/          # 业务逻辑
│   ├── repository/       # 数据访问
│   ├── infrastructure/   # 基础设施（DB, Cache）
│   ├── domain/           # 领域模型
│   └── utils/            # 工具函数
├── docs/                 # 完整文档
├── migrations/           # 数据库迁移
├── tests/                # 测试
├── config.example.toml   # 配置示例
└── config.toml           # 本地配置（建议不提交）
```

---

## 🔗 相关项目

- **IronForge** - Web 前端（Dioxus + WASM）
- **IronLink** - 移动端（Dioxus + Native）
- **IronCore** - 遗留后端（参考实现）
- **IronGuard-AI** - AI 安全层

---

## 📖 更多文档

- [完整文档索引](./docs/INDEX.md) - 所有文档导航
- [配置管理](./docs/02-configuration/CONFIG_MANAGEMENT.md)
- [数据库设计](./docs/02-configuration/DATABASE_SCHEMA.md)
- [安全实践](./docs/02-configuration/SECURITY.md)
- [监控告警](./docs/07-monitoring/MONITORING.md)
- [性能优化](./docs/07-monitoring/PERFORMANCE.md)
- [错误处理](./docs/08-error-handling/ERROR_HANDLING.md)

---

## 📊 指标口径

- API 能力：以 `GET /openapi.yaml` 为准
- 质量门禁：以 GitHub Actions CI（fmt/clippy/test/build）为准
- 性能/容量：以压测报告与运行时监控为准

---

## 📝 注意事项

### 生产环境
- ✅ 设置 `allow_degraded_start = false`
- ✅ 使用强随机 JWT_SECRET (≥ 32 字节)
- ✅ CockroachDB 推荐用于生产（高可用）
- ✅ 启用 Prometheus metrics
- ✅ 配置 HTTPS (TLS 1.3)
- ✅ 设置 Rate Limiting

### 开发环境
- 💡 使用 `allow_degraded_start = true` 快速启动
- 💡 查看 [快速开始](./docs/00-quickstart/README.md)
- 💡 阅读 [开发指南](./docs/11-development/README.md)

---

## 🔗 相关项目

- **[IronForge](../IronForge/)** - Web 前端（Dioxus + WASM）| [文档](../IronForge/docs/INDEX.md)
- **[IronLink DApp](../IronLink%20DApp/)** - 移动端（设计完成）
- **[IronGuard-AI](../ironguard-ai/)** - AI 安全层

---

## 📞 支持与反馈

- **文档问题**: 查看 [故障排查](./docs/00-quickstart/TROUBLESHOOTING.md)
- **常见问题**: 查看 [FAQ](./docs/00-quickstart/FAQ.md)
- **贡献代码**: 查看 [贡献指南](./CONTRIBUTING.md)

---

**最后更新**: 2025-12-06  
**维护者**: Backend Team  
**License**: MIT  
**文档整理**: 企业级标准 ⭐⭐⭐
