# 监控与告警指南

> ironforge_backend 监控系统完整文档

## 📋 目录

- [监控架构](#监控架构)
- [Prometheus 指标](#prometheus-指标)
- [健康检查](#健康检查)
- [日志系统](#日志系统)
- [告警规则](#告警规则)
- [性能监控](#性能监控)
- [仪表盘](#仪表盘)

---

## 监控架构

### 监控技术栈

```
┌──────────────┐
│  Application │
│  (Backend)   │
└──────┬───────┘
       │ metrics
       ▼
┌──────────────┐
│  Prometheus  │ ◄─── 收集指标
└──────┬───────┘
       │ data
       ▼
┌──────────────┐
│   Grafana    │ ◄─── 可视化
└──────────────┘
       │
       ▼
┌──────────────┐
│ AlertManager │ ◄─── 告警
└──────────────┘
```

### 监控配置

```toml
[monitoring]
enable_prometheus = true
prometheus_addr = "127.0.0.1:9090"
enable_health_checks = true
health_check_interval_secs = 30
```

---

## Prometheus 指标

### 指标端点

```
GET http://localhost:8088/metrics
```

### 内置指标

#### 1. HTTP 请求指标

```rust
// 请求总数
http_requests_total{method="POST", path="/api/v1/wallets/batch", status="200"}

// 请求延迟（秒）
http_request_duration_seconds{method="POST", path="/api/v1/wallets/batch", quantile="0.5"}
http_request_duration_seconds{method="POST", path="/api/v1/wallets/batch", quantile="0.95"}
http_request_duration_seconds{method="POST", path="/api/v1/wallets/batch", quantile="0.99"}

// 活跃连接数
http_active_connections
```

#### 2. 数据库指标

```rust
// 连接池使用率
db_pool_connections{state="idle"}
db_pool_connections{state="active"}

// 查询延迟
db_query_duration_seconds{query="select_user", quantile="0.95"}

// 查询错误率
db_query_errors_total{query="select_user"}

// 事务数
db_transactions_total{status="committed"}
db_transactions_total{status="rolled_back"}
```

#### 3. Redis 指标

```rust
// Redis 操作
redis_operations_total{operation="get", status="success"}
redis_operations_total{operation="set", status="success"}

// Redis 连接
redis_connections{state="active"}

// 缓存命中率
redis_cache_hits_total
redis_cache_misses_total
```

#### 4. 业务指标

```rust
// 钱包操作
wallets_created_total
wallets_deleted_total

// 交易指标
transactions_submitted_total{chain="eth"}
transactions_confirmed_total{chain="eth"}
transactions_failed_total{chain="eth"}

// 用户指标
users_registered_total
users_active_total
users_login_attempts_total{status="success"}
users_login_attempts_total{status="failure"}
```

### 自定义指标示例

```rust
use prometheus::{Counter, Histogram, IntGauge, Registry};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();
    
    // 计数器：单调递增
    pub static ref HTTP_REQUESTS: Counter = Counter::new(
        "http_requests_total",
        "Total HTTP requests"
    ).unwrap();
    
    // 直方图：测量分布
    pub static ref HTTP_DURATION: Histogram = Histogram::new(
        "http_request_duration_seconds",
        "HTTP request duration"
    ).unwrap();
    
    // 量表：可增可减
    pub static ref ACTIVE_USERS: IntGauge = IntGauge::new(
        "active_users",
        "Number of active users"
    ).unwrap();
}

// 注册指标
pub fn init_metrics() {
    REGISTRY.register(Box::new(HTTP_REQUESTS.clone())).unwrap();
    REGISTRY.register(Box::new(HTTP_DURATION.clone())).unwrap();
    REGISTRY.register(Box::new(ACTIVE_USERS.clone())).unwrap();
}

// 使用指标
pub async fn handle_request() {
    HTTP_REQUESTS.inc();
    let timer = HTTP_DURATION.start_timer();
    
    // 处理请求...
    
    timer.observe_duration();
}
```

---

## 健康检查

### 健康检查端点

#### 1. 基本健康检查

```
GET /api/health
```

**响应**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "status": "ok"
  }
}
```

> 说明：当前实现还提供两个实用别名/扩展：
> - `GET /health`：简短别名（兼容部分测试脚本）
> - `GET /healthz`：包含 DB/Redis/Immudb/RPC 探活与版本信息

#### 2. 详细健康检查（推荐用于就绪探针）

```
GET /healthz
```

**响应（示例）**:
```json
{
  "code": 0,
  "message": "success",
  "data": {
    "status": "ok",
    "db_ok": true,
    "redis_ok": true,
    "immu_ok": true,
    "rpc_ok": true,
    "version": "0.1.0+dev"
  }
}
```

### 健康检查实现

```rust
use axum::{Json, response::IntoResponse};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

pub async fn readiness_check(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let db_ok = check_database(&state.pool).await;
    let redis_ok = check_redis(&state.redis).await;
    let immu_ok = check_immudb(&state.immu).await;
    
    let status = if db_ok && redis_ok && immu_ok {
        "ready"
    } else {
        "not_ready"
    };
    
    Json(json!({
        "status": status,
        "checks": {
            "database": if db_ok { "ok" } else { "error" },
            "redis": if redis_ok { "ok" } else { "error" },
            "immudb": if immu_ok { "ok" } else { "error" }
        },
        "timestamp": Utc::now().to_rfc3339()
    }))
}

async fn check_database(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await
        .is_ok()
}

async fn check_redis(redis: &RedisCtx) -> bool {
    redis.ping().await.is_ok()
}

async fn check_immudb(immu: &ImmuCtx) -> bool {
    immu.health_check().await.is_ok()
}
```

---

## 日志系统

### 日志配置

```toml
[logging]
level = "info"           # trace, debug, info, warn, error
format = "json"          # json, pretty, compact
file_path = "logs/backend.log"
max_file_size_mb = 100
max_backup_files = 10
```

### 日志级别

| 级别 | 用途 | 示例 |
|------|------|------|
| TRACE | 详细追踪 | 函数参数、返回值 |
| DEBUG | 调试信息 | 中间计算结果 |
| INFO | 常规信息 | 服务启动、请求处理 |
| WARN | 警告信息 | 降级运行、重试操作 |
| ERROR | 错误信息 | 异常、失败操作 |

### 结构化日志

```rust
use tracing::{info, warn, error, instrument};

#[instrument(skip(pool))]
pub async fn create_wallet(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
) -> Result<Wallet> {
    info!(
        user_id = %user_id,
        wallet_name = name,
        "Creating new wallet"
    );
    
    let wallet = sqlx::query_as!(
        Wallet,
        "INSERT INTO wallets (user_id, name) VALUES ($1, $2) RETURNING *",
        user_id,
        name
    )
    .fetch_one(pool)
    .await
    .map_err(|e| {
        error!(
            user_id = %user_id,
            error = %e,
            "Failed to create wallet"
        );
        e
    })?;
    
    info!(
        wallet_id = %wallet.id,
        "Wallet created successfully"
    );
    
    Ok(wallet)
}
```

### 日志格式

#### JSON 格式（生产环境）

```json
{
  "timestamp": "2025-11-24T10:30:00.123Z",
  "level": "INFO",
  "target": "ironforge_backend::service::wallets",
  "message": "Creating new wallet",
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "wallet_name": "My Wallet",
  "span": {
    "name": "create_wallet"
  }
}
```

#### Pretty 格式（开发环境）

```
2025-11-24 10:30:00.123  INFO ironforge_backend::service::wallets: Creating new wallet
  user_id: 550e8400-e29b-41d4-a716-446655440000
  wallet_name: My Wallet
```

---

## 告警规则

### Prometheus 告警规则

```yaml
# prometheus/alerts.yml
groups:
  - name: ironforge_backend
    interval: 30s
    rules:
      # 高错误率告警
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "高错误率检测"
          description: "过去5分钟错误率超过5%"
      
      # 高延迟告警
      - alert: HighLatency
        expr: histogram_quantile(0.95, http_request_duration_seconds_bucket) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "高延迟检测"
          description: "P95延迟超过1秒"
      
      # 数据库连接池耗尽
      - alert: DatabasePoolExhausted
        expr: db_pool_connections{state="idle"} < 5
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: "数据库连接池即将耗尽"
          description: "空闲连接少于5个"
      
      # Redis 连接失败
      - alert: RedisDown
        expr: redis_operations_total{status="error"} > 10
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Redis 连接失败"
          description: "Redis 操作失败超过10次"
      
      # 内存使用率高
      - alert: HighMemoryUsage
        expr: process_resident_memory_bytes > 1e9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "内存使用率高"
          description: "进程内存使用超过1GB"
```

### AlertManager 配置

```yaml
# alertmanager/config.yml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'default'
  
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
    
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'default'
    webhook_configs:
      - url: 'http://localhost:5001/alert'
  
  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/WEBHOOK/URL'
        channel: '#alerts'
        title: 'IronForge Alert'
  
  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_SERVICE_KEY'
```

---

## 性能监控

### 关键性能指标（KPI）

#### 1. 响应时间

- **P50**: 中位数响应时间
- **P95**: 95%请求的响应时间
- **P99**: 99%请求的响应时间

**目标**:
- P50 < 100ms
- P95 < 500ms
- P99 < 1000ms

#### 2. 吞吐量

- **RPS**: 每秒请求数
- **TPS**: 每秒事务数

**目标**:
- RPS > 1000
- TPS > 500

#### 3. 错误率

- **5xx 错误率**: 服务器错误
- **4xx 错误率**: 客户端错误

**目标**:
- 5xx < 0.1%
- 4xx < 1%

#### 4. 可用性

- **Uptime**: 服务正常运行时间

**目标**:
- 99.9% (每月停机 < 43分钟)

### 性能分析工具

#### 1. Flame Graph（火焰图）

```bash
# 使用 perf 生成火焰图
cargo build --release
perf record -F 99 -g -- ./target/release/ironforge_backend
perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg
```

#### 2. Profiling

```rust
// 使用 pprof
use pprof::ProfilerGuard;

let guard = ProfilerGuard::new(100).unwrap();

// 运行需要分析的代码...

let report = guard.report().build().unwrap();
let file = std::fs::File::create("profile.svg").unwrap();
report.flamegraph(file).unwrap();
```

---

## 仪表盘

### Grafana 仪表盘

#### 1. 系统概览仪表盘

```json
{
  "dashboard": {
    "title": "IronForge Backend - Overview",
    "panels": [
      {
        "title": "Request Rate",
        "targets": [
          {
            "expr": "rate(http_requests_total[5m])"
          }
        ]
      },
      {
        "title": "Response Time (P95)",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, http_request_duration_seconds_bucket)"
          }
        ]
      },
      {
        "title": "Error Rate",
        "targets": [
          {
            "expr": "rate(http_requests_total{status=~\"5..\"}[5m])"
          }
        ]
      }
    ]
  }
}
```

#### 2. 数据库仪表盘

- 连接池使用率
- 查询延迟分布
- 慢查询列表
- 事务成功/失败率

#### 3. 业务仪表盘

- 新注册用户数
- 钱包创建趋势
- 交易成功率
- 活跃用户数

### 预定义仪表盘

位置: `backend/ops/grafana/dashboards/`

- `overview.json` - 系统概览
- `database.json` - 数据库监控
- `business.json` - 业务指标
- `security.json` - 安全监控

---

## 日志查询

### 使用 jq 查询日志

```bash
# 查找错误日志
cat logs/backend.log | jq 'select(.level == "ERROR")'

# 统计各个级别的日志数量
cat logs/backend.log | jq -r '.level' | sort | uniq -c

# 查找特定用户的操作
cat logs/backend.log | jq 'select(.user_id == "550e8400-e29b-41d4-a716-446655440000")'

# 查找慢查询（> 1秒）
cat logs/backend.log | jq 'select(.duration_ms > 1000)'
```

### ELK Stack 集成

```yaml
# logstash/config.yml
input {
  file {
    path => "/var/log/ironforge/backend.log"
    codec => json
  }
}

filter {
  if [level] == "ERROR" {
    mutate {
      add_tag => ["error"]
    }
  }
}

output {
  elasticsearch {
    hosts => ["localhost:9200"]
    index => "ironforge-backend-%{+YYYY.MM.dd}"
  }
}
```

---

## 最佳实践

### 监控最佳实践

1. **全面覆盖**: 监控所有关键组件
2. **合理粒度**: 不要过度监控
3. **及时告警**: 问题发生时立即通知
4. **可操作**: 告警信息包含解决方案
5. **定期检查**: 定期审查监控配置

### 告警最佳实践

1. **避免告警疲劳**: 减少误报
2. **分级处理**: 根据严重程度分级
3. **包含上下文**: 告警包含足够信息
4. **可操作性**: 提供解决建议
5. **持续优化**: 根据反馈调整规则

---

## 故障排查

### 常见问题

#### 1. Prometheus 连接失败

```bash
# 检查 Prometheus 是否运行
curl http://localhost:9090/-/healthy

# 检查指标端点
curl http://localhost:8088/metrics
```

#### 2. 日志文件过大

```bash
# 轮转日志
logrotate /etc/logrotate.d/ironforge

# 压缩旧日志
gzip logs/backend.log.1
```

#### 3. 高内存使用

```bash
# 检查内存使用
ps aux | grep ironforge_backend

# 查看堆分配
valgrind --tool=massif ./target/release/ironforge_backend
```

---

## 相关文档

- [配置管理](./CONFIG_MANAGEMENT.md)
- [性能优化](./PERFORMANCE.md)
- [部署指南](../05-deployment/DEPLOYMENT.md)

---

**最后更新**: 2025-11-24  
**维护者**: DevOps Team
