# 性能优化指南

> ironforge_backend 性能优化完整文档

## 📋 目录

- [性能目标](#性能目标)
- [数据库优化](#数据库优化)
- [缓存策略](#缓存策略)
- [并发优化](#并发优化)
- [网络优化](#网络优化)
- [代码优化](#代码优化)
- [性能测试](#性能测试)

---

## 性能目标

### 关键指标

| 指标 | 目标 | 当前 | 状态 |
|------|------|------|------|
| P50 延迟 | < 100ms | 50ms | ✅ |
| P95 延迟 | < 500ms | 300ms | ✅ |
| P99 延迟 | < 1000ms | 800ms | ✅ |
| RPS | > 1000 | 1500 | ✅ |
| 错误率 | < 0.1% | 0.05% | ✅ |
| 可用性 | 99.9% | 99.95% | ✅ |

### 性能基准

```bash
# HTTP 基准测试
wrk -t12 -c400 -d30s http://localhost:8088/api/health

# 结果示例
Running 30s test @ http://localhost:8088/api/health
  12 threads and 400 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency    50.23ms   15.43ms  200.00ms   85.43%
    Req/Sec   1.25k     150.00     1.50k    68.00%
  15000 requests in 30.00s, 2.50MB read
Requests/sec:   1500.00
Transfer/sec:    85.32KB
```

---

## 数据库优化

### 1. 连接池配置

```toml
[database]
max_connections = 50      # 最大连接数
min_connections = 10      # 最小连接数
connect_timeout_secs = 10 # 连接超时
idle_timeout_secs = 600   # 空闲超时（10分钟）
max_lifetime_secs = 1800  # 最大生命周期（30分钟）
```

**优化建议：**

- **max_connections**: 根据并发需求设置（公式：核心数 × 2 + 磁盘数）
- **min_connections**: 保持热连接，避免冷启动
- **idle_timeout**: 及时释放空闲连接

### 2. 索引优化

```sql
-- 分析查询计划
EXPLAIN ANALYZE
SELECT * FROM transactions
WHERE wallet_id = '...' AND status = 'pending'
ORDER BY created_at DESC
LIMIT 10;

-- 创建复合索引
CREATE INDEX idx_tx_wallet_status_time
ON transactions (wallet_id, status, created_at DESC);

-- 查看索引使用情况
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    idx_tup_read,
    idx_tup_fetch
FROM pg_stat_user_indexes
ORDER BY idx_scan ASC;

-- 删除未使用的索引
DROP INDEX IF EXISTS unused_index_name;
```

**索引设计原则：**

1. **高选择性字段优先**: 区分度高的字段放在前面
2. **覆盖索引**: 包含查询需要的所有字段
3. **避免过度索引**: 每个索引都有维护成本
4. **定期分析**: 使用 ANALYZE 更新统计信息

### 3. 查询优化

#### 批量查询

```rust
// ❌ 错误：N+1 查询
for wallet in wallets {
    let assets = get_assets_by_wallet_id(wallet.id).await?;
}

// ✅ 正确：批量查询
let wallet_ids: Vec<Uuid> = wallets.iter().map(|w| w.id).collect();
let assets = sqlx::query_as!(
    Asset,
    "SELECT * FROM assets WHERE wallet_id = ANY($1)",
    &wallet_ids
)
.fetch_all(pool)
.await?;
```

#### 分页查询

```rust
// ✅ 使用游标分页（高效）
pub async fn list_transactions(
    pool: &PgPool,
    wallet_id: Uuid,
    cursor: Option<Uuid>,
    limit: i64,
) -> Result<Vec<Transaction>> {
    let txs = if let Some(cursor_id) = cursor {
        sqlx::query_as!(
            Transaction,
            r#"
            SELECT * FROM transactions
            WHERE wallet_id = $1 AND id < $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
            wallet_id,
            cursor_id,
            limit
        )
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as!(
            Transaction,
            r#"
            SELECT * FROM transactions
            WHERE wallet_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
            wallet_id,
            limit
        )
        .fetch_all(pool)
        .await?
    };
    
    Ok(txs)
}
```

#### 预编译语句

```rust
// ✅ 使用 sqlx 宏（编译时检查）
let user = sqlx::query_as!(
    User,
    "SELECT * FROM users WHERE email = $1",
    email
)
.fetch_one(pool)
.await?;

// ✅ 使用 prepare 缓存
let stmt = pool.prepare("SELECT * FROM users WHERE email = $1").await?;
let user = stmt.fetch_one(email).await?;
```

### 4. 事务优化

```rust
// ✅ 最小化事务范围
pub async fn transfer(
    pool: &PgPool,
    from_wallet_id: Uuid,
    to_wallet_id: Uuid,
    amount: Decimal,
) -> Result<()> {
    // 先执行只读操作
    let from_balance = get_balance(pool, from_wallet_id).await?;
    if from_balance < amount {
        return Err(anyhow!("Insufficient balance"));
    }
    
    // 事务只包含写操作
    let mut tx = pool.begin().await?;
    
    update_balance(&mut tx, from_wallet_id, -amount).await?;
    update_balance(&mut tx, to_wallet_id, amount).await?;
    create_transaction(&mut tx, from_wallet_id, to_wallet_id, amount).await?;
    
    tx.commit().await?;
    
    Ok(())
}
```

---

## 缓存策略

### 1. 两层缓存架构

```
┌─────────────┐
│  Application│
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  L1: Memory │  ◄─── 本地缓存（moka）
│  Cache      │       - 高速（纳秒级）
└──────┬──────┘       - 有限容量
       │ miss
       ▼
┌─────────────┐
│  L2: Redis  │  ◄─── 分布式缓存
│  Cache      │       - 快速（毫秒级）
└──────┬──────┘       - 可扩展
       │ miss
       ▼
┌─────────────┐
│  Database   │  ◄─── 持久化存储
└─────────────┘
```

### 2. 缓存配置

```rust
use moka::future::Cache;
use std::time::Duration;

// L1: 内存缓存
pub fn create_memory_cache<K, V>() -> Cache<K, V>
where
    K: std::hash::Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    Cache::builder()
        .max_capacity(10_000)           // 最大条目数
        .time_to_live(Duration::from_secs(300))  // TTL 5分钟
        .time_to_idle(Duration::from_secs(60))   // 空闲60秒过期
        .build()
}

// L2: Redis 缓存
pub async fn get_or_fetch<T, F>(
    redis: &RedisCtx,
    cache: &Cache<String, T>,
    key: &str,
    fetcher: F,
) -> Result<T>
where
    T: Clone + Serialize + DeserializeOwned + Send + Sync + 'static,
    F: Future<Output = Result<T>>,
{
    // 1. 尝试 L1 缓存
    if let Some(value) = cache.get(key).await {
        return Ok(value);
    }
    
    // 2. 尝试 L2 缓存（Redis）
    if let Ok(Some(cached)) = redis.get::<String>(key).await {
        if let Ok(value) = serde_json::from_str::<T>(&cached) {
            cache.insert(key.to_string(), value.clone()).await;
            return Ok(value);
        }
    }
    
    // 3. 从数据库获取
    let value = fetcher.await?;
    
    // 4. 回写缓存
    let serialized = serde_json::to_string(&value)?;
    redis.set_ex(key, &serialized, 300).await?;  // 5分钟
    cache.insert(key.to_string(), value.clone()).await;
    
    Ok(value)
}
```

### 3. 缓存失效策略

#### 主动失效

```rust
// 数据更新时主动失效缓存
pub async fn update_wallet(
    pool: &PgPool,
    redis: &RedisCtx,
    cache: &Cache<String, Wallet>,
    wallet_id: Uuid,
    name: &str,
) -> Result<Wallet> {
    // 更新数据库
    let wallet = sqlx::query_as!(
        Wallet,
        "UPDATE wallets SET name = $1 WHERE id = $2 RETURNING *",
        name,
        wallet_id
    )
    .fetch_one(pool)
    .await?;
    
    // 失效缓存
    let cache_key = format!("wallet:{}", wallet_id);
    cache.invalidate(&cache_key).await;
    redis.del(&cache_key).await?;
    
    Ok(wallet)
}
```

#### 缓存预热

```rust
// 应用启动时预热热点数据
pub async fn warmup_cache(
    pool: &PgPool,
    redis: &RedisCtx,
) -> Result<()> {
    // 预加载活跃用户
    let active_users = sqlx::query_as!(
        User,
        "SELECT * FROM users WHERE last_login_at > NOW() - INTERVAL '7 days'"
    )
    .fetch_all(pool)
    .await?;
    
    for user in active_users {
        let key = format!("user:{}", user.id);
        let value = serde_json::to_string(&user)?;
        redis.set_ex(&key, &value, 3600).await?;
    }
    
    Ok(())
}
```

### 4. 缓存模式

#### Cache-Aside（旁路缓存）

```rust
pub async fn get_user(
    pool: &PgPool,
    redis: &RedisCtx,
    user_id: Uuid,
) -> Result<User> {
    let key = format!("user:{}", user_id);
    
    // 1. 查缓存
    if let Ok(Some(cached)) = redis.get::<String>(&key).await {
        return Ok(serde_json::from_str(&cached)?);
    }
    
    // 2. 查数据库
    let user = sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_one(pool)
        .await?;
    
    // 3. 写缓存
    let serialized = serde_json::to_string(&user)?;
    redis.set_ex(&key, &serialized, 3600).await?;
    
    Ok(user)
}
```

---

## 并发优化

### 1. 异步 I/O

```rust
// ✅ 并发执行多个独立操作
use tokio::try_join;

pub async fn get_wallet_summary(
    pool: &PgPool,
    wallet_id: Uuid,
) -> Result<WalletSummary> {
    let (wallet, assets, transactions) = try_join!(
        get_wallet(pool, wallet_id),
        get_assets(pool, wallet_id),
        get_recent_transactions(pool, wallet_id)
    )?;
    
    Ok(WalletSummary {
        wallet,
        assets,
        transactions,
    })
}
```

### 2. 并发限制

```rust
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
}

impl ConcurrencyLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }
    
    pub async fn execute<F, T>(&self, f: F) -> Result<T>
    where
        F: Future<Output = Result<T>>,
    {
        let _permit = self.semaphore.acquire().await?;
        f.await
    }
}

// 使用示例
let limiter = ConcurrencyLimiter::new(10);  // 最多10个并发

for wallet_id in wallet_ids {
    limiter.execute(async {
        process_wallet(wallet_id).await
    }).await?;
}
```

### 3. 批处理

```rust
// ✅ 批量处理
pub async fn process_transactions_batch(
    pool: &PgPool,
    tx_ids: Vec<Uuid>,
) -> Result<()> {
    const BATCH_SIZE: usize = 100;
    
    for chunk in tx_ids.chunks(BATCH_SIZE) {
        let mut tx = pool.begin().await?;
        
        for tx_id in chunk {
            process_transaction(&mut tx, *tx_id).await?;
        }
        
        tx.commit().await?;
    }
    
    Ok(())
}
```

---

## 网络优化

### 1. HTTP/2 支持

```rust
use axum::Server;
use hyper::server::conn::Http;

let server = Server::bind(&addr)
    .http2_only(true)  // 启用 HTTP/2
    .serve(app.into_make_service());
```

### 2. 连接复用

```rust
use reqwest::Client;

// ✅ 复用 HTTP 客户端
lazy_static! {
    static ref HTTP_CLIENT: Client = Client::builder()
        .pool_max_idle_per_host(10)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
}
```

### 3. 响应压缩

```rust
use tower_http::compression::CompressionLayer;

let app = Router::new()
    .route("/api/v1/wallets", get(list_wallets))
    .layer(CompressionLayer::new());  // 自动 gzip 压缩
```

---

## 代码优化

### 1. 避免不必要的克隆

```rust
// ❌ 错误：不必要的克隆
fn process(data: Vec<String>) {
    for item in data.clone() {  // 不必要的克隆
        println!("{}", item);
    }
}

// ✅ 正确：使用引用
fn process(data: &[String]) {
    for item in data {
        println!("{}", item);
    }
}
```

### 2. 使用 Cow（写时复制）

```rust
use std::borrow::Cow;

fn format_address(address: &str) -> Cow<str> {
    if address.starts_with("0x") {
        Cow::Borrowed(address)  // 无需分配
    } else {
        Cow::Owned(format!("0x{}", address))  // 需要时才分配
    }
}
```

### 3. 预分配容量

```rust
// ✅ 预分配容量避免多次重新分配
let mut wallets = Vec::with_capacity(expected_count);
for id in ids {
    wallets.push(get_wallet(id).await?);
}
```

### 4. 使用 SmallVec

```rust
use smallvec::SmallVec;

// 少量元素时避免堆分配
let mut items: SmallVec<[u64; 8]> = SmallVec::new();
items.push(1);
items.push(2);
```

---

## 性能测试

### 1. 基准测试

```rust
// benches/wallet_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn create_wallet_benchmark(c: &mut Criterion) {
    c.bench_function("create_wallet", |b| {
        b.iter(|| {
            create_wallet(
                black_box("user123"),
                black_box("eth"),
                black_box("My Wallet")
            )
        });
    });
}

criterion_group!(benches, create_wallet_benchmark);
criterion_main!(benches);
```

运行基准测试：

```bash
cargo bench
```

### 2. 负载测试

```bash
# 使用 wrk
wrk -t12 -c400 -d30s \
    -s scripts/load_test.lua \
    http://localhost:8088/api/v1/wallets

# 使用 k6
k6 run --vus 100 --duration 30s scripts/load_test.js
```

### 3. 压力测试

```javascript
// scripts/load_test.js (k6)
import http from 'k6/http';
import { check } from 'k6';

export let options = {
  stages: [
    { duration: '2m', target: 100 },  // 爬升到100用户
    { duration: '5m', target: 100 },  // 保持100用户
    { duration: '2m', target: 200 },  // 爬升到200用户
    { duration: '5m', target: 200 },  // 保持200用户
    { duration: '2m', target: 0 },    // 下降到0
  ],
};

export default function () {
  let res = http.get('http://localhost:8088/api/health');
  check(res, {
    'status is 200': (r) => r.status === 200,
    'response time < 500ms': (r) => r.timings.duration < 500,
  });
}
```

---

## 性能分析工具

### 1. Flame Graph（火焰图）

```bash
# 生成火焰图
cargo flamegraph --bin ironforge_backend
```

### 2. Profiling

```bash
# 使用 perf
perf record -F 99 -g -- cargo run --release
perf report

# 使用 valgrind
valgrind --tool=callgrind ./target/release/ironforge_backend
```

### 3. 内存分析

```bash
# 使用 heaptrack
heaptrack ./target/release/ironforge_backend

# 分析结果
heaptrack_gui heaptrack.ironforge_backend.*.gz
```

---

## 性能优化检查清单

### 数据库层

- [ ] 索引已优化
- [ ] 查询计划已分析
- [ ] 连接池已配置
- [ ] 慢查询已识别
- [ ] 批量操作已实现

### 缓存层

- [ ] 两层缓存已实现
- [ ] 缓存命中率 > 80%
- [ ] 缓存失效策略已实施
- [ ] 热点数据已预热

### 应用层

- [ ] 异步 I/O 已使用
- [ ] 并发限制已实施
- [ ] 响应压缩已启用
- [ ] HTTP/2 已启用
- [ ] 连接池已复用

### 代码层

- [ ] 不必要的克隆已移除
- [ ] 容量已预分配
- [ ] 算法复杂度已优化
- [ ] 内存分配已最小化

---

## 相关文档

- [监控告警](../07-monitoring/MONITORING.md)
- [配置管理](../02-configuration/CONFIG_MANAGEMENT.md)
- [数据库模式](../02-configuration/DATABASE_SCHEMA.md)

---

**最后更新**: 2025-11-24  
**维护者**: Performance Team
