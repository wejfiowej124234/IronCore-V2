//! 生产级性能基准测试 - RPC Selector
//! 
//! 测试场景:
//! 1. 单链端点选择性能（p50/p95/p99延迟）
//! 2. 多链轮询性能（模拟真实负载）
//! 3. 高并发场景（100+ QPS）
//! 4. 缓存命中率影响
//! 5. 故障切换延迟
//!
//! 性能目标:
//! - 单次查询: < 5ms (p95)
//! - 并发100: < 20ms (p95)
//! - 缓存命中: < 1ms

use std::env;
use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ironcore::infrastructure::rpc_selector::RpcSelector;
use sqlx::PgPool;
use tokio::runtime::Runtime;

// ============ 生产环境配置 ============

const CHAINS: &[&str] = &["ethereum", "bsc", "polygon", "arbitrum", "optimism"];
const CONCURRENT_LOW: usize = 10;
const CONCURRENT_MED: usize = 50;
const CONCURRENT_HIGH: usize = 100;

// ============ 测试辅助函数 ============

async fn setup_test_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/ironcore?sslmode=disable".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// 预热缓存 - 确保基准测试的公平性
async fn warmup_cache(selector: &RpcSelector) {
    for chain in CHAINS {
        let _ = selector.select(chain).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
}

// ============ 基准测试函数 ============

/// Benchmark 1: 单链端点选择 - 基础性能测试
fn bench_single_chain_selection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let selector = Arc::new(RpcSelector::new(pool));
    rt.block_on(warmup_cache(&selector));

    let mut group = c.benchmark_group("single_chain_selection");
    group.throughput(Throughput::Elements(1));
    group.measurement_time(Duration::from_secs(10));

    for chain in CHAINS {
        group.bench_with_input(
            BenchmarkId::new("select", chain),
            chain,
            |b, &chain| {
                b.iter(|| {
                    rt.block_on(async {
                        let result = selector.select(black_box(chain)).await;
                        black_box(result);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark 2: 多链轮询 - 模拟真实API负载
fn bench_multi_chain_round_robin(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let selector = Arc::new(RpcSelector::new(pool));
    rt.block_on(warmup_cache(&selector));

    let mut group = c.benchmark_group("multi_chain_round_robin");
    group.throughput(Throughput::Elements(100));

    group.bench_function("100_requests_5_chains", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 0..100 {
                    let chain = CHAINS[i % CHAINS.len()];
                    let result = selector.select(black_box(chain)).await;
                    black_box(result);
                }
            })
        });
    });
    group.finish();
}

/// Benchmark 3: 并发测试 - 低/中/高负载
fn bench_concurrent_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let selector = Arc::new(RpcSelector::new(pool));
    rt.block_on(warmup_cache(&selector));

    let mut group = c.benchmark_group("concurrent_load");
    group.measurement_time(Duration::from_secs(15));

    // 低负载: 10 并发
    group.throughput(Throughput::Elements(CONCURRENT_LOW as u64));
    group.bench_function("concurrent_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..CONCURRENT_LOW {
                    let s = selector.clone();
                    let chain = CHAINS[i % CHAINS.len()];
                    handles.push(tokio::spawn(async move {
                        s.select(chain).await
                    }));
                }
                let results = futures::future::join_all(handles).await;
                black_box(results);
            })
        });
    });

    // 中负载: 50 并发
    group.throughput(Throughput::Elements(CONCURRENT_MED as u64));
    group.bench_function("concurrent_50", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..CONCURRENT_MED {
                    let s = selector.clone();
                    let chain = CHAINS[i % CHAINS.len()];
                    handles.push(tokio::spawn(async move {
                        s.select(chain).await
                    }));
                }
                let results = futures::future::join_all(handles).await;
                black_box(results);
            })
        });
    });

    // 高负载: 100 并发
    group.throughput(Throughput::Elements(CONCURRENT_HIGH as u64));
    group.bench_function("concurrent_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..CONCURRENT_HIGH {
                    let s = selector.clone();
                    let chain = CHAINS[i % CHAINS.len()];
                    handles.push(tokio::spawn(async move {
                        s.select(chain).await
                    }));
                }
                let results = futures::future::join_all(handles).await;
                black_box(results);
            })
        });
    });

    group.finish();
}

/// Benchmark 4: 缓存性能 - 冷/热缓存对比
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());

    let mut group = c.benchmark_group("cache_performance");
    group.measurement_time(Duration::from_secs(10));

    // 冷缓存 - 每次创建新 selector
    group.bench_function("cold_cache", |b| {
        b.iter(|| {
            let selector = RpcSelector::new(pool.clone());
            rt.block_on(async {
                let result = selector.select("ethereum").await;
                black_box(result);
            })
        });
    });

    // 热缓存 - 重复使用同一个 selector
    let selector = Arc::new(RpcSelector::new(pool));
    rt.block_on(warmup_cache(&selector));
    
    group.throughput(Throughput::Elements(1000));
    group.bench_function("hot_cache_1000_hits", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..1000 {
                    let result = selector.select("ethereum").await;
                    black_box(result);
                }
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_single_chain_selection,
    bench_multi_chain_round_robin,
    bench_concurrent_load,
    bench_cache_performance
);

criterion_main!(benches);
