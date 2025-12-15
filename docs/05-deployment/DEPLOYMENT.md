# IronCore 后端部署文档

## 📋 目录

1. [部署前准备](#部署前准备)
2. [本地开发环境](#本地开发环境)
3. [生产环境部署](#生产环境部署)
4. [Docker部署](#docker部署)
5. [Kubernetes部署](#kubernetes部署)
6. [监控和告警](#监控和告警)
7. [故障排查](#故障排查)

---

## 部署前准备

### 系统要求

- **操作系统**: Linux (推荐 Ubuntu 20.04+)
- **内存**: 最低 2GB，推荐 4GB+
- **CPU**: 最低 2核，推荐 4核+
- **磁盘**: 最低 20GB，推荐 50GB+

### 依赖服务

- **CockroachDB**: 版本 23.1+
- **Redis**: 版本 6.0+
- **immudb**: 版本 1.9+

---

## 本地开发环境

### 1. 安装依赖

```bash
# 安装Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装PostgreSQL客户端（用于CockroachDB）
sudo apt-get install postgresql-client

# 安装Redis客户端
sudo apt-get install redis-tools
```

### 2. 启动本地服务

```bash
# 使用Docker Compose启动所有服务
cd backend/ops
docker-compose up -d
```

### 3. 配置环境变量

```bash
# 复制示例配置文件
cp ops/env.prod.sample .env

# 编辑环境变量
vim .env
```

### 4. 运行数据库迁移

```bash
# 设置环境变量
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"

# 运行迁移
cd backend
sqlx migrate run
```

### 5. 启动应用

```bash
cd backend
cargo run --release
```

---

## 生产环境部署

### 1. 构建应用

```bash
# 构建Release版本
cd backend
cargo build --release

# 二进制文件位置
target/release/ironforge_backend
```

### 2. 配置环境变量

创建 `.env` 文件或使用环境变量：

```bash
# 必需的环境变量
DATABASE_URL=postgres://user:password@host:26257/ironcore?sslmode=require
REDIS_URL=redis://host:6379
IMMUDB_ADDR=host:3322
IMMUDB_USER=immudb
IMMUDB_PASS=password
IMMUDB_DB=defaultdb
JWT_SECRET=your-secret-key-must-be-at-least-32-characters-long
WALLET_ENC_KEY=your-encryption-key-32-bytes-or-hex

# 可选配置
BIND_ADDR=0.0.0.0:8088
LOG_LEVEL=info
LOG_FORMAT=json
ENABLE_PROMETHEUS=1
```

### 3. 使用配置文件（推荐）

```bash
# 复制示例配置
cp config.example.toml config.toml

# 编辑配置
vim config.toml

# 设置配置路径
export CONFIG_PATH=./config.toml
```

### 4. 运行应用

```bash
# 直接运行
./target/release/ironforge_backend

# 或使用systemd服务
sudo systemctl start ironforge-backend
```

### 5. systemd服务配置

创建 `/etc/systemd/system/ironforge-backend.service`:

```ini
[Unit]
Description=IronCore Backend Service
After=network.target

[Service]
Type=simple
User=ironforge
WorkingDirectory=/opt/ironforge/backend
Environment="CONFIG_PATH=/opt/ironforge/backend/config.toml"
ExecStart=/opt/ironforge/backend/target/release/ironforge_backend
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

---

## Docker部署

### 1. 构建Docker镜像

```bash
# 创建Dockerfile
cat > Dockerfile <<EOF
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ironforge_backend /usr/local/bin/
COPY --from=builder /app/config.example.toml /etc/ironforge/config.toml
EXPOSE 8088
CMD ["ironforge_backend"]
EOF

# 构建镜像
docker build -t ironforge-backend:latest .
```

### 2. 运行容器

```bash
docker run -d \
  --name ironforge-backend \
  -p 8088:8088 \
  -e DATABASE_URL="postgres://..." \
  -e REDIS_URL="redis://..." \
  -e JWT_SECRET="..." \
  -e WALLET_ENC_KEY="..." \
  ironforge-backend:latest
```

### 3. Docker Compose

参考 `ops/docker-compose.yml`

---

## Kubernetes部署

### 1. 创建ConfigMap

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: ironforge-config
data:
  config.toml: |
    [database]
    url = "postgres://..."
    ...
```

### 2. 创建Secret

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: ironforge-secrets
type: Opaque
stringData:
  JWT_SECRET: "your-secret-key"
  WALLET_ENC_KEY: "your-encryption-key"
```

### 3. 创建Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ironforge-backend
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ironforge-backend
  template:
    metadata:
      labels:
        app: ironforge-backend
    spec:
      containers:
      - name: backend
        image: ironforge-backend:latest
        ports:
        - containerPort: 8088
        env:
        - name: CONFIG_PATH
          value: "/etc/config/config.toml"
        volumeMounts:
        - name: config
          mountPath: /etc/config
        livenessProbe:
          httpGet:
            path: /healthz
            port: 8088
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /api/health
            port: 8088
          initialDelaySeconds: 10
          periodSeconds: 5
      volumes:
      - name: config
        configMap:
          name: ironforge-config
```

---

## 监控和告警

### 1. Prometheus Metrics

应用自动暴露Prometheus metrics在 `/metrics` 端点：

```bash
curl http://localhost:8088/metrics
```

### 2. 健康检查

- **就绪探针**: `GET /api/health`
- **存活探针**: `GET /healthz`

### 3. 日志

- **日志格式**: JSON（生产环境）或文本（开发环境）
- **日志级别**: 通过 `LOG_LEVEL` 环境变量配置
- **日志文件**: 通过 `LOG_FILE_PATH` 配置

### 4. 告警规则

参考 `src/infrastructure/monitoring.rs` 中的告警规则配置

---

## 故障排查

### 常见问题

1. **数据库连接失败**
   - 检查 `DATABASE_URL` 是否正确
   - 检查数据库是否可访问
   - 检查防火墙规则

2. **Redis连接失败**
   - 检查 `REDIS_URL` 是否正确
   - 检查Redis服务是否运行

3. **JWT验证失败**
   - 检查 `JWT_SECRET` 是否设置
   - 检查Token是否过期

4. **迁移失败**
   - 检查数据库权限
   - 检查迁移文件是否正确

### 日志查看

```bash
# 查看应用日志
journalctl -u ironforge-backend -f

# 查看Docker日志
docker logs -f ironforge-backend

# 查看Kubernetes日志
kubectl logs -f deployment/ironforge-backend
```

---

## 性能优化

### 1. 数据库连接池

配置 `DB_MAX_CONNS` 和 `DB_MIN_CONNS` 环境变量

### 2. Redis连接池

Redis客户端自动管理连接池

### 3. 日志级别

生产环境建议使用 `info` 级别

---

## 安全建议

1. **密钥管理**: 使用密钥管理服务（如AWS KMS、HashiCorp Vault）
2. **TLS**: 生产环境启用TLS
3. **防火墙**: 限制数据库和Redis的访问
4. **定期更新**: 定期更新依赖和系统

---

**文档版本**: 1.0  
**最后更新**: 2024年

