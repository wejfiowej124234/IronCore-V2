# 配置管理指南

> ironforge_backend 配置管理完整指南

## 📋 目录

- [配置架构](#配置架构)
- [配置文件](#配置文件)
- [环境变量](#环境变量)
- [配置优先级](#配置优先级)
- [配置验证](#配置验证)
- [最佳实践](#最佳实践)

---

## 配置架构

### 配置结构体

```rust
pub struct Config {
    pub server: ServerConfig,      // 服务器配置
    pub database: DatabaseConfig,  // 数据库配置
    pub redis: RedisConfig,         // Redis 配置
    pub immudb: ImmudbConfig,       // Immudb 配置
    pub jwt: JwtConfig,             // JWT 配置
    pub logging: LoggingConfig,     // 日志配置
    pub monitoring: MonitoringConfig, // 监控配置
}
```

### 配置加载流程

```
CONFIG_PATH 环境变量
    ↓
config.toml 文件（如果存在）
    ↓
环境变量覆盖
    ↓
验证配置
    ↓
应用配置
```

---

## 配置文件

### config.toml 完整示例

```toml
# 服务器配置
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = false  # 是否允许降级启动（跳过数据库检查）

# 数据库配置
[database]
url = "postgres://root@localhost:26257/ironcore?sslmode=disable"
max_connections = 20
min_connections = 5
connect_timeout_secs = 30
idle_timeout_secs = 600
max_lifetime_secs = 1800

# Redis 配置
[redis]
url = "redis://localhost:6379"
pool_size = 10
connection_timeout_secs = 5

# Immudb 配置
[immudb]
addr = "localhost:3322"
user = "immudb"
password = "immudb"
database = "defaultdb"

# JWT 配置
[jwt]
secret = "your-secure-jwt-secret-key-change-in-production"
token_expiry_secs = 3600  # 1小时

# 日志配置
[logging]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json, pretty, compact
# 可选：日志文件
file_path = "logs/backend.log"
max_file_size_mb = 100
max_backup_files = 10

# 监控配置
[monitoring]
enable_prometheus = true
prometheus_addr = "127.0.0.1:9090"
enable_health_checks = true
health_check_interval_secs = 30
```

---

## 环境变量

### 必需的环境变量

```bash
# 数据库（如果未在 config.toml 中配置）
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"

# JWT 密钥（生产环境必须设置）
export JWT_SECRET="your-production-jwt-secret-key-min-32-chars"

# Redis（可选，默认 redis://localhost:6379）
export REDIS_URL="redis://localhost:6379"
```

### 可选的环境变量

```bash
# 配置文件路径
export CONFIG_PATH="/path/to/config.toml"

# 服务器地址
export SERVER_BIND_ADDR="0.0.0.0:8088"

# 日志级别
export LOG_LEVEL="debug"

# 允许降级启动（开发环境）
export ALLOW_DEGRADED_START="true"

# Immudb 配置
export IMMUDB_ADDR="localhost:3322"
export IMMUDB_USER="immudb"
export IMMUDB_PASSWORD="immudb"
export IMMUDB_DATABASE="defaultdb"
```

---

## 配置优先级

配置值的优先级（从高到低）：

1. **环境变量** - 最高优先级
2. **config.toml 文件** - 中等优先级
3. **默认值** - 最低优先级

### 示例

```toml
# config.toml
[server]
bind_addr = "127.0.0.1:8088"
```

```bash
# 环境变量会覆盖 config.toml
export SERVER_BIND_ADDR="0.0.0.0:9000"

# 最终使用: 0.0.0.0:9000
```

---

## 配置验证

### 自动验证

配置加载后会自动进行验证：

```rust
// 在 main.rs 中
let config = Config::from_env_and_file(config_path)?;
config.validate()?;  // 自动验证所有配置项
```

### 验证规则

1. **JWT Secret**
   - 生产环境必须设置
   - 最小长度 32 字符
   - 不能使用默认测试密钥

2. **数据库 URL**
   - 必须是有效的 PostgreSQL/CockroachDB 连接串
   - 连接池大小合理（5-100）

3. **服务器地址**
   - 必须是有效的 IP:端口格式
   - 端口范围 1024-65535

4. **日志级别**
   - 必须是 trace/debug/info/warn/error 之一

---

## 最佳实践

### 开发环境

```toml
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = true  # 允许无数据库启动

[logging]
level = "debug"
format = "pretty"

[jwt]
secret = "dev-jwt-secret-only-for-local-testing"
token_expiry_secs = 86400  # 24小时
```

### 生产环境

```toml
[server]
bind_addr = "0.0.0.0:8088"
allow_degraded_start = false  # 禁止降级启动

[database]
url = "${DATABASE_URL}"  # 从环境变量读取
max_connections = 50
connect_timeout_secs = 10

[logging]
level = "info"
format = "json"
file_path = "/var/log/ironforge/backend.log"

[jwt]
secret = "${JWT_SECRET}"  # 从环境变量读取
token_expiry_secs = 3600

[monitoring]
enable_prometheus = true
prometheus_addr = "0.0.0.0:9090"
```

### 敏感信息保护

**❌ 不要这样做：**

```toml
[jwt]
secret = "hardcoded-secret-in-file"  # 危险！

[database]
url = "postgres://root:password@localhost:26257/db"  # 危险！
```

**✅ 应该这样做：**

```bash
# 使用环境变量
export JWT_SECRET="$(openssl rand -base64 32)"
export DATABASE_URL="postgres://user:pass@host:port/db"
```

```toml
# config.toml 使用占位符
[jwt]
secret = "${JWT_SECRET}"

[database]
url = "${DATABASE_URL}"
```

### 配置文件管理

```bash
# 生产环境配置
config.production.toml

# 预发布环境配置
config.staging.toml

# 开发环境配置
config.development.toml

# 示例配置（提交到 Git）
config.example.toml
```

**Git 管理：**

```gitignore
# .gitignore
config.toml
config.*.toml
!config.example.toml
.env
.env.*
!.env.example
```

---

## 配置热重载

当前版本不支持配置热重载，需要重启服务：

```bash
# 修改配置后
kill -SIGTERM <pid>  # 优雅关闭
cargo run            # 重新启动
```

**未来计划：**

- 支持 SIGHUP 信号热重载
- 配置文件监听自动重载
- 动态调整日志级别

---

## 故障排查

### 配置未生效

1. 检查配置文件路径
```bash
echo $CONFIG_PATH
ls -l config.toml
```

2. 检查环境变量
```bash
env | grep -E "(DATABASE|REDIS|JWT)"
```

3. 启用详细日志
```bash
export LOG_LEVEL=debug
cargo run
```

### 数据库连接失败

```bash
# 测试数据库连接
psql $DATABASE_URL -c "SELECT 1"

# 检查数据库服务
docker ps | grep cockroach
```

### JWT 验证失败

```bash
# 检查 JWT_SECRET 是否设置
echo $JWT_SECRET | wc -c  # 应该 >= 32

# 检查配置文件
grep "jwt" config.toml
```

---

## 配置模板

### 最小配置（开发环境）

```toml
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = true

[jwt]
secret = "dev-jwt-secret-min-32-chars-long-xxxxx"
```

### 完整配置（生产环境）

参见 `config.example.toml`

---

## 相关文档

- [部署指南](../05-deployment/DEPLOYMENT.md)
- [架构概览](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)
- [安全指南](./SECURITY.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
