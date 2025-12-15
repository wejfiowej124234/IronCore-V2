# IronCore 生产级性能基准测试

## 概述

本目录包含企业级性能基准测试，用于持续监控关键服务的性能表现。所有测试使用 [Criterion.rs](https://github.com/bheisler/criterion.rs) 框架，提供统计学严谨的性能分析。

## 测试套件

### 1. RPC Selector 性能测试 (`rpc_selector_bench.rs`)

**测试场景**：
- ✅ 单链端点选择（5条主链）
- ✅ 多链轮询负载（100次请求）
- ✅ 并发负载测试（10/50/100并发）
- ✅ 冷/热缓存性能对比

**性能目标**：
| 指标 | 目标值 | 关键性 |
|------|--------|--------|
| 单次查询 (p95) | < 5ms | 🔴 Critical |
| 并发100 (p95) | < 20ms | 🟡 Important |
| 缓存命中 | < 1ms | 🟢 Nice to have |
| 吞吐量 | > 200 QPS | 🟡 Important |

**运行方式**：
```bash
# 运行所有RPC测试
cargo bench --bench rpc_selector_bench

# 运行特定测试
cargo bench --bench rpc_selector_bench -- single_chain
cargo bench --bench rpc_selector_bench -- concurrent

# 生成详细报告
cargo bench --bench rpc_selector_bench -- --verbose
```

**结果分析**：
- `target/criterion/rpc_select_ethereum/` - 详细统计报告
- 关注 p95 延迟（95%请求的响应时间）
- 监控缓存命中率对性能的影响

---

### 2. Fee Service 性能测试 (`fee_service_bench.rs`)

**测试场景**：
- ✅ 不同金额级别（0.1 ETH ~ 50k ETH）
- ✅ 多链费用计算（5条链）
- ✅ 交易类型对比（transfer/contract_call/swap）
- ✅ 缓存性能（冷/热缓存）
- ✅ 高并发计算（10/50/100并发）
- ✅ 吞吐量压测（1000次连续请求）

**性能目标**：
| 指标 | 目标值 | 关键性 |
|------|--------|--------|
| 单次计算 (p95) | < 10ms | 🔴 Critical |
| 缓存命中 (p95) | < 2ms | 🟡 Important |
| 并发50 (p95) | < 30ms | 🟡 Important |
| 吞吐量 | > 100 QPS | 🟢 Nice to have |

**运行方式**：
```bash
# 运行所有费用测试
cargo bench --bench fee_service_bench

# 按金额级别测试
cargo bench --bench fee_service_bench -- fee_by_amount

# 多链对比
cargo bench --bench fee_service_bench -- fee_multi_chain

# 并发压测
cargo bench --bench fee_service_bench -- concurrent

# 吞吐量测试
cargo bench --bench fee_service_bench -- throughput
```

**结果分析**：
- `target/criterion/fee_by_amount/` - 金额级别影响
- `target/criterion/cache_performance/` - 缓存效率
- `target/criterion/concurrent_calculation/` - 并发能力

---

## 环境要求

### 1. 数据库连接
测试需要连接真实数据库。设置环境变量：

```bash
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
```

或使用默认配置（CockroachDB本地实例）。

### 2. 数据准备
确保数据库包含测试数据：
```sql
-- RPC端点配置
INSERT INTO rpc_endpoints (chain, url, priority, is_active) VALUES ...;

-- 费用配置
INSERT INTO fee_configs (chain, tx_type, base_fee, percentage_fee) VALUES ...;
```

使用种子脚本：
```bash
cd IronCore
cargo run --bin seed-test-data
```

### 3. 系统要求
- **CPU**: 4核+（并发测试需要）
- **内存**: 4GB+
- **网络**: 稳定的数据库连接（本地 < 1ms延迟）

---

## CI/CD 集成

### GitHub Actions 配置

```yaml
name: Performance Benchmarks

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: cockroachdb/cockroach:latest
        options: >-
          --health-cmd "curl http://localhost:8080/health"
          --health-interval 10s
    
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          
      - name: Run Benchmarks
        run: |
          cargo bench --bench rpc_selector_bench -- --save-baseline main
          cargo bench --bench fee_service_bench -- --save-baseline main
          
      - name: Upload Results
        uses: actions/upload-artifact@v3
        with:
          name: benchmark-results
          path: target/criterion/
```

### 性能回归检测

```bash
# 1. 建立基准线（main分支）
git checkout main
cargo bench -- --save-baseline main

# 2. 测试新代码（feature分支）
git checkout feature/optimization
cargo bench -- --baseline main

# 3. 查看对比报告
# Criterion会自动显示性能变化百分比
```

**回归阈值**：
- 🟢 **提升 > 5%**: 优秀，可合并
- 🟡 **变化 ±5%**: 可接受
- 🔴 **降低 > 10%**: 需要优化或说明原因

---

## 报告解读

### Criterion 输出示例

```
rpc_select_ethereum time:   [4.2341 ms 4.2890 ms 4.3521 ms]
                    change: [-5.2301% -3.1245% -1.0234%] (p = 0.00 < 0.05)
                    Performance has improved.
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high mild
```

**关键指标**：
- **time**: [最小值 中位数 最大值]
- **change**: 与上次基准线的性能变化
- **p值**: 统计显著性（< 0.05 表示变化显著）
- **outliers**: 异常值数量（应 < 5%）

### 性能趋势图

Criterion 自动生成HTML报告：
```bash
open target/criterion/report/index.html
```

包含：
- 📊 延迟分布图（PDF/CDF）
- 📈 性能趋势线
- 🔍 回归分析
- 📉 吞吐量对比

---

## 最佳实践

### 1. 测试隔离
```rust
// ✅ 每个测试独立创建资源
fn bench_test(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    // ...
}

// ❌ 避免共享可变状态
static mut SHARED_POOL: Option<PgPool> = None; // 不要这样做
```

### 2. 预热缓存
```rust
// ✅ 热缓存测试前预热
async fn warmup_cache(service: &Service) {
    for _ in 0..10 {
        let _ = service.operation().await;
    }
}

// ❌ 直接测试会包含首次查询开销
```

### 3. 真实负载模拟
```rust
// ✅ 模拟生产环境请求分布
let chains = ["ethereum", "bsc", "polygon"]; // 按实际比例
for i in 0..1000 {
    let chain = chains[i % chains.len()]; // 轮询
    // ...
}

// ❌ 单一场景测试不全面
for _ in 0..1000 {
    test_same_case(); // 过于理想化
}
```

### 4. 错误处理
```rust
// ✅ Benchmark中适当忽略错误（但要记录）
let result = service.operation().await;
if result.is_err() {
    eprintln!("Benchmark error: {:?}", result);
}
black_box(result);

// ❌ 不要让错误中断测试
let result = service.operation().await.unwrap(); // panic会导致测试失败
```

---

## 性能优化检查清单

在提交性能优化代码前，确保：

- [ ] 所有benchmark通过（无panic）
- [ ] 关键指标无回归（< 10%降低）
- [ ] p95延迟满足SLA要求
- [ ] 异常值比例 < 5%
- [ ] 并发测试无死锁/竞态
- [ ] 缓存命中率保持稳定
- [ ] 吞吐量符合容量规划
- [ ] 提交代码包含性能对比报告

---

## 故障排查

### 问题1: "Failed to connect to database"
```bash
# 检查数据库是否运行
docker ps | grep cockroach

# 启动数据库
docker-compose -f ops/docker-compose.yml up -d cockroachdb

# 验证连接
psql postgres://root@localhost:26257/ironcore?sslmode=disable -c "SELECT 1"
```

### 问题2: 性能波动大（outliers > 10%）
- 关闭后台程序（浏览器、IDE等）
- 固定CPU频率：`sudo cpupower frequency-set -g performance`
- 增加测试时间：`group.measurement_time(Duration::from_secs(30));`

### 问题3: 并发测试失败
- 检查连接池大小：`pool.max_connections()`
- 增加数据库超时：`PgPool::connect_with_config(...)`
- 减少并发数：从100降到50测试

---

## 参考资料

- [Criterion.rs 文档](https://bheisler.github.io/criterion.rs/book/)
- [性能测试最佳实践](https://easyperf.net/blog/2018/08/26/Basics-of-performance-testing)
- [CockroachDB性能调优](https://www.cockroachlabs.com/docs/stable/performance-tuning-recipes.html)
- [Rust异步性能指南](https://www.reddit.com/r/rust/comments/jm4g3k/async_performance_guide/)

---

## 联系与反馈

- **性能问题**: 在 GitHub Issues 中标记 `performance` 标签
- **基准测试改进**: 提交 PR 到 `benches/` 目录
- **CI失败**: 联系 DevOps 团队

**维护者**: IronCore Performance Team  
**更新日期**: 2025-12-07
