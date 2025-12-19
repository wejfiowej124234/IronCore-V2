# 🧪 性能测试指南

> 完整的性能基准测试和压力测试指南

## 📋 目录

- [性能基准测试](#性能基准测试)
- [压力测试](#压力测试)
- [性能指标](#性能指标)
- [优化建议](#优化建议)

---

## 性能基准测试

### 当前基准测试

项目包含3个性能基准测试：

```
backend/benches/
├── fee_service_bench.rs       # 费用服务性能测试
├── rpc_selector_bench.rs      # RPC选择器性能测试
└── performance_bench.rs       # 通用性能测试
```

### 运行基准测试

#### 运行所有基准测试

```bash
cd backend
cargo bench
```

**输出示例**:
```
test fee_calculation          ... bench:   1,234 ns/iter (+/- 56)
test rpc_selection            ... bench:     987 ns/iter (+/- 42)
test cache_lookup             ... bench:     123 ns/iter (+/- 12)
```

#### 运行特定基准测试

```bash
# 只测试费用服务
cargo bench --bench fee_service_bench

# 只测试RPC选择器
cargo bench --bench rpc_selector_bench

# 只测试通用性能
cargo bench --bench performance_bench
```

#### 生成HTML报告

```bash
# 安装criterion（如果还没安装）
cargo install cargo-criterion

# 生成详细报告
cargo criterion

# 查看报告
# 报告位置: target/criterion/report/index.html
```

在浏览器打开: `file:///path/to/backend/target/criterion/report/index.html`

### 基准测试详解

#### 1. 费用服务基准测试 (`fee_service_bench.rs`)

**测试场景**:
- 费用计算性能
- 费率规则查询
- 费用审计记录

**关键指标**:
- 费用计算: <2ms
- 规则查询: <5ms
- 审计记录: <10ms

**运行**:
```bash
cargo bench --bench fee_service_bench -- --verbose
```

#### 2. RPC选择器基准测试 (`rpc_selector_bench.rs`)

**测试场景**:
- RPC端点选择算法
- 健康检查性能
- 故障转移速度

**关键指标**:
- 端点选择: <1ms
- 健康检查: <100ms
- 故障转移: <500ms

**运行**:
```bash
cargo bench --bench rpc_selector_bench -- --verbose
```

#### 3. 通用性能基准测试 (`performance_bench.rs`)

**测试场景**:
- 数据库查询性能
- 缓存命中率
- JSON序列化/反序列化

**关键指标**:
- DB查询: <50ms (p95)
- 缓存命中: <1ms
- JSON处理: <5ms

**运行**:
```bash
cargo bench --bench performance_bench -- --verbose
```

---

## 压力测试

### 使用 Apache Bench (ab)

#### 安装

```bash
# Ubuntu/Debian
sudo apt-get install apache2-utils

# macOS
brew install httpd

# Windows (使用WSL或下载二进制)
```

#### 基础API压力测试

```bash
# 健康检查端点（1000请求，10并发）
ab -n 1000 -c 10 http://localhost:8088/api/health

# 输出示例:
# Requests per second:    2543.21 [#/sec] (mean)
# Time per request:       3.932 [ms] (mean)
# Time per request:       0.393 [ms] (mean, across all concurrent requests)
```

#### 带认证的API测试

```bash
# 创建测试用token
export TEST_TOKEN="your_jwt_token_here"

# 测试钱包API（500请求，20并发）
ab -n 500 -c 20 \
   -H "Authorization: Bearer $TEST_TOKEN" \
  http://localhost:8088/api/v1/wallets
```

#### POST请求测试

```bash
# 创建测试数据文件
cat > test_payload.json <<EOF
{
  "wallets": [
    {
      "chain": "ETH",
      "address": "0x0000000000000000000000000000000000000001",
      "public_key": "04aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }
  ]
}
EOF

# 测试创建钱包（100请求，5并发）
ab -n 100 -c 5 \
   -p test_payload.json \
   -T application/json \
   -H "Authorization: Bearer $TEST_TOKEN" \
  http://localhost:8088/api/v1/wallets/batch
```

### 使用 wrk (推荐)

#### 安装

```bash
# Ubuntu/Debian
sudo apt-get install wrk

# macOS
brew install wrk

# 从源码编译
git clone https://github.com/wg/wrk.git
cd wrk && make
```

#### 基础压力测试

```bash
# 10秒测试，10个线程，100个连接
wrk -t10 -c100 -d10s http://localhost:8088/api/health

# 输出示例:
# Running 10s test @ http://localhost:8088/api/health
#   10 threads and 100 connections
#   Thread Stats   Avg      Stdev     Max   +/- Stdev
#     Latency     5.23ms    2.15ms  50.12ms   89.45%
#     Req/Sec     1.92k   234.17     2.50k    75.00%
#   191234 requests in 10.01s, 28.54MB read
# Requests/sec:  19105.23
# Transfer/sec:      2.85MB
```

#### 带脚本的复杂测试

创建 `scripts/load-test.lua`:

```lua
-- Lua脚本用于复杂场景测试
wrk.method = "POST"
wrk.body   = '{"wallets": [{"chain": "ETH", "address": "0x0000000000000000000000000000000000000001", "public_key": "04aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]}'
wrk.headers["Content-Type"] = "application/json"
wrk.headers["Authorization"] = "Bearer YOUR_TOKEN"

function response(status, headers, body)
  if status ~= 200 then
    print("Error: " .. status)
  end
end
```

运行:
```bash
wrk -t10 -c100 -d30s -s scripts/load-test.lua \
  http://localhost:8088/api/v1/wallets/batch
```

### 使用 k6 (现代化工具)

#### 安装

```bash
# Linux
sudo apt-key adv --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
echo "deb https://dl.k6.io/deb stable main" | sudo tee /etc/apt/sources.list.d/k6.list
sudo apt-get update
sudo apt-get install k6

# macOS
brew install k6
```

#### 创建测试脚本

`scripts/k6-load-test.js`:

```javascript
import http from 'k6/http';
import { check, sleep } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 50 },  // 爬坡到50用户
    { duration: '1m', target: 100 },  // 保持100用户
    { duration: '30s', target: 0 },   // 降到0
  ],
  thresholds: {
    http_req_duration: ['p(95)<500'], // 95%请求<500ms
    http_req_failed: ['rate<0.01'],   // 错误率<1%
  },
};

export default function () {
  // 测试健康检查
  const healthRes = http.get('http://localhost:8088/api/health');
  check(healthRes, {
    'health check status is 200': (r) => r.status === 200,
  });

  sleep(1);

  // 测试钱包列表（需要token）
  const params = {
    headers: {
      'Authorization': 'Bearer YOUR_TOKEN',
    },
  };
  const walletsRes = http.get('http://localhost:8088/api/v1/wallets', params);
  check(walletsRes, {
    'wallets status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });

  sleep(1);
}
```

#### 运行测试

```bash
k6 run scripts/k6-load-test.js

# 输出示例:
# ✓ health check status is 200
# ✓ wallets status is 200
# ✓ response time < 500ms
#
# checks.........................: 100.00% ✓ 30000      ✗ 0
# data_received..................: 4.5 MB  75 kB/s
# http_req_duration..............: avg=123.45ms min=45.12ms med=98.23ms max=987.65ms p(95)=345.67ms
```

---

## 性能指标

### 目标指标

| 端点类型 | p50 | p95 | p99 | 吞吐量 |
|---------|-----|-----|-----|--------|
| 健康检查 | <5ms | <10ms | <20ms | >10k req/s |
| 简单查询 | <50ms | <100ms | <200ms | >1k req/s |
| 复杂查询 | <200ms | <500ms | <1s | >500 req/s |
| 写操作 | <100ms | <300ms | <500ms | >500 req/s |

### 监控指标

使用Prometheus监控:

```bash
# 查询请求延迟
http_request_duration_seconds{endpoint="/api/v1/wallets",quantile="0.95"}

# 查询请求速率
rate(http_requests_total[5m])

# 查询错误率
rate(http_requests_failed_total[5m]) / rate(http_requests_total[5m])
```

---

## 性能分析

### CPU Profiling

```bash
# 安装profiling工具
cargo install cargo-flamegraph

# 生成火焰图
cargo flamegraph --bench performance_bench

# 查看火焰图
# 生成文件: flamegraph.svg
```

### 内存分析

```bash
# 使用heaptrack
heaptrack cargo bench

# 使用valgrind
valgrind --tool=massif cargo bench
```

### 数据库查询分析

在PostgreSQL/CockroachDB中：

```sql
-- 开启查询日志
SET log_min_duration_statement = 100;  -- 记录>100ms的查询

-- 分析慢查询
EXPLAIN ANALYZE SELECT * FROM wallets WHERE user_id = 'xxx';

-- 查看表统计信息
SELECT * FROM pg_stat_user_tables WHERE relname = 'wallets';

-- 查看索引使用情况
SELECT * FROM pg_stat_user_indexes WHERE relname = 'wallets';
```

---

## 优化建议

### 1. 数据库优化

```sql
-- 添加缺失的索引
CREATE INDEX CONCURRENTLY idx_wallets_created_at ON wallets(created_at);

-- 定期VACUUM
VACUUM ANALYZE wallets;

-- 更新统计信息
ANALYZE wallets;
```

### 2. 缓存优化

```rust
// 增加缓存TTL
let cache_config = CacheConfig {
    ttl: Duration::from_secs(300),  // 5分钟
    max_size: 10000,
};

// 使用批量查询减少往返
let wallets = wallet_repo.get_by_ids(&ids).await?;
```

### 3. 并发优化

```rust
// 使用tokio::spawn并行处理
let futures: Vec<_> = chains
    .iter()
    .map(|chain| tokio::spawn(fetch_balance(chain)))
    .collect();

let results = join_all(futures).await;
```

### 4. 连接池优化

```toml
# config.toml
[database]
max_connections = 20        # 增加连接池大小
min_connections = 5
connection_timeout = 30
idle_timeout = 600
```

---

## 性能测试检查清单

执行压力测试前检查：

- [ ] 关闭DEBUG日志（使用INFO或WARN）
- [ ] 确保数据库有适当的索引
- [ ] 确保缓存已启用
- [ ] 使用Release模式编译（`cargo build --release`）
- [ ] 监控系统资源（CPU、内存、磁盘I/O）
- [ ] 准备足够的测试数据
- [ ] 设置合理的超时时间
- [ ] 记录测试环境配置

---

## 相关文档

- [性能优化指南](../07-monitoring/PERFORMANCE.md)
- [监控告警指南](../07-monitoring/MONITORING.md)
- [数据库设计](../02-configuration/DATABASE_SCHEMA.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
