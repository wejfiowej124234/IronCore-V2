# 监控与告警 (Monitoring & Alerting)

> 📊 Prometheus 监控、Grafana 可视化、告警规则

---

## 📂 本分类文档

| 文档 | 描述 | 状态 |
|------|------|------|
| [MONITORING.md](./MONITORING.md) | 监控系统完整指南 | ✅ 核心 |
| [PERFORMANCE.md](./PERFORMANCE.md) | 性能监控与优化 | ✅ 核心 |

---

## 🎯 快速导航

### SRE 工程师
- 📊 **[监控系统](./MONITORING.md)** - Prometheus + Grafana
- ⚡ **[性能监控](./PERFORMANCE.md)** - 性能分析与优化

---

## 📊 监控架构

### 监控体系

```
┌─────────────────────────────────────────────┐
│         监控体系 (Monitoring Stack)          │
├─────────────────────────────────────────────┤
│                                              │
│  📊 Metrics (指标监控)                      │
│     ├─ Prometheus (时序数据库)              │
│     ├─ Node Exporter (系统指标)             │
│     ├─ PostgreSQL Exporter (数据库指标)    │
│     └─ Custom Metrics (应用指标)            │
│                                              │
│  📝 Logs (日志监控)                         │
│     ├─ Loki (日志聚合)                      │
│     ├─ Promtail (日志收集)                  │
│     └─ LogQL (日志查询)                     │
│                                              │
│  🔍 Traces (链路追踪)                       │
│     ├─ Jaeger (分布式追踪)                  │
│     └─ OpenTelemetry (追踪协议)             │
│                                              │
│  📈 Visualization (可视化)                  │
│     ├─ Grafana (Dashboard)                 │
│     └─ Alertmanager (告警管理)              │
│                                              │
│  🚨 Alerting (告警)                         │
│     ├─ AlertManager (告警路由)              │
│     ├─ Slack/Email/SMS (通知渠道)          │
│     └─ PagerDuty (值班管理)                │
│                                              │
└─────────────────────────────────────────────┘
```

### 监控指标分类

```
系统指标 (System Metrics)
  ├─ CPU 使用率 (%)
  ├─ 内存使用率 (%)
  ├─ 磁盘使用率 (%)
  ├─ 磁盘 I/O (IOPS)
  ├─ 网络流量 (MB/s)
  └─ 网络连接数

应用指标 (Application Metrics)
  ├─ API 请求数 (req/s)
  ├─ API 响应时间 (ms)
  ├─ API 错误率 (%)
  ├─ 活跃用户数
  ├─ 钱包创建数
  └─ 交易发送数

数据库指标 (Database Metrics)
  ├─ 连接池使用率 (%)
  ├─ 查询响应时间 (ms)
  ├─ 慢查询数量
  ├─ 死锁数量
  ├─ 表大小 (GB)
  └─ 索引命中率 (%)

缓存指标 (Cache Metrics)
  ├─ 缓存命中率 (%)
  ├─ 缓存大小 (MB)
  ├─ 缓存驱逐数
  └─ 缓存响应时间 (ms)

业务指标 (Business Metrics)
  ├─ 新用户注册数
  ├─ 活跃钱包数
  ├─ 交易成功率 (%)
  ├─ Swap 成功率 (%)
  └─ 支付成功率 (%)
```

---

## 📚 监控文档详解

### 1️⃣ [监控系统指南](./MONITORING.md) ⭐
**适合**: SRE, DevOps, 后端工程师

**核心内容**:
- 📊 **Prometheus 配置** - 指标采集
- 📈 **Grafana Dashboard** - 可视化仪表盘
- 🚨 **告警规则** - 告警阈值与通知
- 📝 **日志监控** - Loki 日志查询

**Prometheus 配置示例**:
```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # IronCore Backend
  - job_name: 'ironcore'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: '/metrics'
  
  # CockroachDB
  - job_name: 'cockroachdb'
    static_configs:
      - targets: ['localhost:8090']
  
  # Node Exporter (系统指标)
  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']
  
  # Redis Exporter
  - job_name: 'redis'
    static_configs:
      - targets: ['localhost:9121']
```

**Grafana Dashboard 配置**:
```json
{
  "dashboard": {
    "title": "IronCore Backend Monitoring",
    "panels": [
      {
        "title": "API Request Rate",
        "targets": [{
          "expr": "rate(http_requests_total[5m])"
        }]
      },
      {
        "title": "API Response Time (p95)",
        "targets": [{
          "expr": "histogram_quantile(0.95, http_request_duration_seconds_bucket)"
        }]
      },
      {
        "title": "Error Rate",
        "targets": [{
          "expr": "rate(http_requests_total{status=~\"5..\"}[5m])"
        }]
      }
    ]
  }
}
```

**告警规则**:
```yaml
# alerts.yml
groups:
  - name: ironcore_alerts
    interval: 30s
    rules:
      # API 错误率过高
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.01
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          description: "Error rate is {{ $value }} req/s"
      
      # API 响应时间过慢
      - alert: SlowAPIResponse
        expr: histogram_quantile(0.95, http_request_duration_seconds_bucket) > 0.5
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "API response time is slow"
          description: "p95 latency is {{ $value }}s"
      
      # CPU 使用率过高
      - alert: HighCPUUsage
        expr: 100 - (avg by(instance) (irate(node_cpu_seconds_total{mode="idle"}[5m])) * 100) > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage detected"
          description: "CPU usage is {{ $value }}%"
      
      # 内存使用率过高
      - alert: HighMemoryUsage
        expr: (1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100 > 90
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High memory usage detected"
          description: "Memory usage is {{ $value }}%"
```

**阅读时长**: 40 分钟

---

### 2️⃣ [性能监控与优化](./PERFORMANCE.md) ⭐
**适合**: 性能工程师, 后端工程师

**核心内容**:
- ⚡ **性能基准测试** - 基线性能指标
- 🔍 **性能分析工具** - Flamegraph, cargo bench
- 📊 **性能优化案例** - 真实优化案例
- 📈 **容量规划** - 扩容建议

**性能基准测试**:
```rust
// benches/api_benchmark.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_create_wallet(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    c.bench_function("create_wallet", |b| {
        b.to_async(&rt).iter(|| async {
            let service = WalletService::new();
            service.create_wallet(black_box(request)).await
        });
    });
}

criterion_group!(benches, benchmark_create_wallet);
criterion_main!(benches);
```

**性能指标目标**:
| 操作 | p50 | p95 | p99 |
|------|-----|-----|-----|
| GET /api/v1/wallets | 15ms | 50ms | 80ms |
| POST /api/v1/wallets/batch | 30ms | 100ms | 150ms |
| GET /api/v1/tx | 20ms | 80ms | 120ms |
| GET /api/v1/gas/estimate | 50ms | 200ms | 300ms |
| GET /api/v1/swap/quote | 100ms | 500ms | 800ms |

**阅读时长**: 35 分钟

---

## 📈 核心监控指标

### Golden Signals (四大黄金指标)

```
1️⃣ Latency (延迟)
   - API 响应时间 (p50, p95, p99)
   - 数据库查询时间
   - 缓存命中时间

2️⃣ Traffic (流量)
   - 请求数 (req/s)
   - 活跃用户数
   - 数据传输量 (MB/s)

3️⃣ Errors (错误)
   - 错误率 (%)
   - 5xx 错误数
   - 超时错误数

4️⃣ Saturation (饱和度)
   - CPU 使用率 (%)
   - 内存使用率 (%)
   - 数据库连接池使用率 (%)
```

### RED 指标

```
Rate (请求速率)
  - 每秒请求数 (req/s)

Errors (错误率)
  - 错误请求比例 (%)

Duration (持续时间)
  - 请求处理时间 (ms)
```

---

## 🚨 告警策略

### 告警级别

| 级别 | 描述 | 响应时间 | 通知渠道 |
|------|------|----------|----------|
| **P0 - Critical** | 服务完全中断 | 5 分钟 | Slack + SMS + 电话 |
| **P1 - High** | 核心功能受影响 | 15 分钟 | Slack + Email |
| **P2 - Medium** | 部分功能受影响 | 1 小时 | Slack |
| **P3 - Low** | 性能下降 | 4 小时 | Email |

### 告警降噪

```
1. 告警聚合 (5 分钟窗口)
   - 相同告警只发送一次
   
2. 告警抑制
   - 从服务器宕机 → 抑制该服务器的所有告警
   
3. 告警路由
   - P0 级告警 → 所有人
   - P1 级告警 → 值班人员
   - P2/P3 级告警 → 相关负责人
```

---

## 🔍 监控工具命令

### Prometheus 查询
```promql
# API 请求速率（每秒）
rate(http_requests_total[5m])

# API 错误率
rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m])

# API 响应时间 (p95)
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))

# CPU 使用率
100 - (avg by(instance) (irate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)

# 内存使用率
(1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100

# 数据库连接池使用率
(pg_stat_database_numbackends / pg_settings_max_connections) * 100
```

### Loki 日志查询
```logql
# 查看错误日志
{job="ironcore"} |= "ERROR"

# 查看特定用户日志
{job="ironcore"} |= "user_id=123"

# 统计错误数量
count_over_time({job="ironcore"} |= "ERROR" [5m])

# 慢查询日志
{job="ironcore"} |= "slow_query" | json | duration > 1000
```

---

## 📊 监控仪表盘

### 推荐 Grafana Dashboard

1. **[IronCore Backend Dashboard](https://grafana.com/grafana/dashboards/15000)**
   - API 请求速率、响应时间、错误率
   - CPU、内存、磁盘、网络监控

2. **[PostgreSQL Dashboard](https://grafana.com/grafana/dashboards/9628)**
   - 数据库连接数、查询性能、锁等待

3. **[Redis Dashboard](https://grafana.com/grafana/dashboards/11835)**
   - 缓存命中率、内存使用、键数量

4. **[Node Exporter Dashboard](https://grafana.com/grafana/dashboards/1860)**
   - 系统级监控（CPU、内存、磁盘、网络）

---

## 🔗 相关文档

- **运维手册**: [06-operations/OPERATIONS.md](../06-operations/OPERATIONS.md)
- **部署指南**: [05-deployment/DEPLOYMENT.md](../05-deployment/DEPLOYMENT.md)
- **性能测试**: [04-testing/TESTING_FRAMEWORK.md](../04-testing/TESTING_FRAMEWORK.md)
- **错误处理**: [08-error-handling/ERROR_HANDLING.md](../08-error-handling/ERROR_HANDLING.md)

---

**最后更新**: 2025-12-06  
**维护者**: SRE & Monitoring Team  
**审查者**: SRE Lead, Infrastructure Manager
