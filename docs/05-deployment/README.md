# 部署与运维 (Deployment & Operations)

> 🚀 Docker 部署、生产环境配置、高可用架构

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [DEPLOYMENT.md](./DEPLOYMENT.md) | 完整部署指南 | ✅ 核心 |
| [DOCKER.md](./DOCKER.md) | Docker 容器化 | ✅ 核心 |

---

## 🎯 快速导航

### DevOps 工程师
- 🚀 **[部署指南](./DEPLOYMENT.md)** - 生产环境部署
- 🐳 **[Docker 指南](./DOCKER.md)** - 容器化部署

---

## 🏗️ 部署架构

### 生产环境架构

```
                    Internet
                       ↓
               ┌───────────────┐
               │  Load Balancer │  (Nginx/HAProxy)
               │  (SSL Termination)
               └───────┬───────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
   ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
   │ IronCore│    │ IronCore│    │ IronCore│  (3+ instances)
   │ Instance│    │ Instance│    │ Instance│
   │  :8088  │    │  :8088  │    │  :8088  │
   └────┬────┘    └────┬────┘    └────┬────┘
        │              │              │
        └──────────────┼──────────────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
   ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
   │CockroachDB  │Redis    │ │Immudb   │
   │ Cluster │    │ Cluster │    │ Cluster │
   │(3 nodes)│    │(3 nodes)│    │(1 node) │
   └─────────┘    └─────────┘    └─────────┘
```

### Docker Compose 架构

```yaml
services:
  ironcore:
    image: ironcore:latest
    ports:
      - "8088:8088"
    environment:
      - DATABASE_URL=postgres://...
      - REDIS_URL=redis://...
    depends_on:
      - cockroachdb
      - redis
      - immudb
  
  cockroachdb:
    image: cockroachdb/cockroach:latest
    command: start-single-node --insecure
    ports:
      - "26257:26257"
      - "8090:8080"
  
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
  
  immudb:
    image: codenotary/immudb:latest
    ports:
      - "3322:3322"
```

---

## 📚 部署文档详解

### 1️⃣ [完整部署指南](./DEPLOYMENT.md) ⭐
**适合**: DevOps, SRE, 系统管理员

**核心内容**:
- 🚀 **生产环境部署** - 完整部署流程
- 🔐 **环境变量配置** - 敏感信息管理
- 📊 **资源规划** - CPU/内存/存储
- 🔄 **滚动更新** - 零停机部署
- 📈 **扩缩容** - 水平扩展

**部署流程**:
```bash
# 1. 克隆仓库
git clone https://github.com/your-org/ironcore.git
cd ironcore/IronCore-V2

# 2. 配置环境变量
cp .env.example .env.production
vim .env.production

# 3. 构建 Docker 镜像
docker build -t ironcore:latest .

# 4. 启动服务
docker compose -f docker-compose.prod.yml up -d

# 5. 运行数据库迁移
docker exec ironcore sqlx migrate run

# 6. 验证部署
curl http://localhost:8088/api/health
```

**生产环境检查清单**:
- [x] ✅ TLS/SSL 证书配置
- [x] ✅ 环境变量加密存储
- [x] ✅ 数据库备份策略
- [x] ✅ 日志收集（ELK/Loki）
- [x] ✅ 监控告警（Prometheus + Grafana）
- [x] ✅ 健康检查端点
- [x] ✅ 负载均衡配置
- [x] ✅ 防火墙规则
- [x] ✅ Rate Limiting
- [x] ✅ CORS 配置

**阅读时长**: 35 分钟

---

### 2️⃣ [Docker 容器化](./DOCKER.md)
**适合**: DevOps, 后端工程师

**核心内容**:
- 🐳 **Dockerfile 优化** - 多阶段构建
- 📦 **镜像分层** - 减小镜像大小
- 🔧 **Docker Compose** - 本地开发环境
- 📊 **容器监控** - cAdvisor, Docker stats

**Dockerfile 示例**:
```dockerfile
# Stage 1: Builder
FROM rust:1.75 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

# Stage 2: Runtime
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ironforge_backend /app/
EXPOSE 8088
CMD ["./ironforge_backend"]
```

**阅读时长**: 20 分钟

---

## 🔍 部署策略

### 1. 蓝绿部署
```
Blue (Current)     Green (New)
  v1.0       →       v1.1
    ↓                 ↓
  Traffic          (Testing)
    ↓                 ↓
  (Switch)    →    Traffic
```

### 2. 金丝雀部署
```
Step 1: 5% traffic → v1.1
Step 2: 25% traffic → v1.1
Step 3: 50% traffic → v1.1
Step 4: 100% traffic → v1.1
```

### 3. 滚动更新
```
Instance 1: v1.0 → v1.1 (update)
Wait for health check...
Instance 2: v1.0 → v1.1 (update)
Wait for health check...
Instance 3: v1.0 → v1.1 (update)
```

---

## 📊 资源规划

### 最小配置（单实例）
| 资源 | 最小值 | 推荐值 |
|------|--------|--------|
| CPU | 2 核 | 4 核 |
| 内存 | 4 GB | 8 GB |
| 磁盘 | 20 GB | 50 GB |
| 带宽 | 10 Mbps | 100 Mbps |

### 生产环境（高可用）
| 组件 | 实例数 | CPU | 内存 | 磁盘 |
|------|--------|-----|------|------|
| IronCore Backend | 3+ | 4 核 | 8 GB | 50 GB |
| CockroachDB | 3+ | 4 核 | 16 GB | 200 GB |
| Redis | 3 | 2 核 | 4 GB | 20 GB |
| Immudb | 1 | 2 核 | 4 GB | 100 GB |
| Nginx | 2 | 2 核 | 2 GB | 10 GB |

### 流量估算
| 用户数 | QPS | 实例数 | 配置 |
|--------|-----|--------|------|
| 1K | 100 | 2 | 4 核 8 GB |
| 10K | 1,000 | 4 | 4 核 8 GB |
| 100K | 10,000 | 12 | 8 核 16 GB |
| 1M | 100,000 | 50+ | 16 核 32 GB |

---

## 🔧 运维命令

### Docker 管理
```bash
# 启动所有服务
docker compose up -d

# 查看日志
docker compose logs -f ironcore

# 重启服务
docker compose restart ironcore

# 停止所有服务
docker compose down

# 清理所有数据（危险！）
docker compose down -v
```

### 数据库管理
```bash
# 运行迁移
docker exec ironcore sqlx migrate run

# 数据库备份
docker exec cockroachdb cockroach dump \
  --insecure --host=localhost \
  ironcore > backup.sql

# 数据库还原
docker exec -i cockroachdb cockroach sql \
  --insecure --host=localhost \
  < backup.sql
```

### 健康检查
```bash
# 后端健康检查
curl http://localhost:8088/api/health

# 数据库健康检查
curl http://localhost:8090/health

# Redis 健康检查
redis-cli -h localhost -p 6379 ping
```

---

## 🔗 相关文档

- **配置管理**: [02-configuration/CONFIG_MANAGEMENT.md](../02-configuration/CONFIG_MANAGEMENT.md)
- **监控告警**: [07-monitoring/MONITORING.md](../07-monitoring/MONITORING.md)
- **运维手册**: [06-operations/OPERATIONS.md](../06-operations/OPERATIONS.md)
- **备份恢复**: [06-operations/BACKUP_RECOVERY.md](../06-operations/BACKUP_RECOVERY.md)

---

**最后更新**: 2025-12-06  
**维护者**: DevOps & SRE Team  
**审查者**: Infrastructure Lead, CTO
