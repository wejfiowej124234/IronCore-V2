//! 生产级性能基准测试 - FeeService
//!
//! 测试场景:
//! 1. 不同金额级别的费用计算（小/中/大）
//! 2. 多链费用计算性能对比
//! 3. 缓存命中/未命中性能
//! 4. 高并发费用计算
//! 5. 批量费用查询性能
//!
//! 性能目标:
//! - 单次计算: < 10ms (p95)
//! - 缓存命中: < 2ms (p95)
//! - 并发50: < 30ms (p95)
//! - 吞吐量: > 100 QPS

use std::{env, sync::Arc, time::Duration};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ironcore::service::fee_service::FeeService;
use sqlx::PgPool;
use tokio::runtime::Runtime;

// ============ 生产环境配置 ============

const CHAINS: &[&str] = &["ethereum", "bsc", "polygon", "arbitrum", "optimism"];
const TX_TYPES: &[&str] = &["transfer", "contract_call", "swap"];

#[derive(Debug, Clone, Copy)]
enum AmountLevel {
    Small,     // < 1 ETH
    Medium,    // 1-100 ETH
    Large,     // 100-10000 ETH
    VeryLarge, // > 10000 ETH
}

impl AmountLevel {
    fn value(&self) -> f64 {
        match self {
            Self::Small => 0.1,
            Self::Medium => 10.0,
            Self::Large => 1000.0,
            Self::VeryLarge => 50000.0,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Small => "small_0.1",
            Self::Medium => "medium_10",
            Self::Large => "large_1000",
            Self::VeryLarge => "very_large_50k",
        }
    }
}

// ============ 测试辅助函数 ============

async fn setup_test_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://root@localhost:26257/ironcore?sslmode=disable".to_string());

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// 预热缓存
async fn warmup_cache(service: &FeeService) {
    for chain in CHAINS {
        for tx_type in TX_TYPES {
            let _ = service.calculate_fee(chain, tx_type, 1.0).await;
        }
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
}

// ============ 基准测试函数 ============

/// Benchmark 1: 不同金额级别的费用计算
fn bench_fee_by_amount_levels(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let service = Arc::new(FeeService::new(pool));
    rt.block_on(warmup_cache(&service));

    let mut group = c.benchmark_group("fee_by_amount");
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    let levels = [
        AmountLevel::Small,
        AmountLevel::Medium,
        AmountLevel::Large,
        AmountLevel::VeryLarge,
    ];

    for level in &levels {
        group.bench_with_input(
            BenchmarkId::new("ethereum_transfer", level.name()),
            level,
            |b, level| {
                b.iter(|| {
                    rt.block_on(async {
                        let result = service
                            .calculate_fee(
                                black_box("ethereum"),
                                black_box("transfer"),
                                black_box(level.value()),
                            )
                            .await;
                        black_box(result);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark 2: 多链费用计算性能对比
fn bench_fee_multi_chain(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let service = Arc::new(FeeService::new(pool));
    rt.block_on(warmup_cache(&service));

    let mut group = c.benchmark_group("fee_multi_chain");
    group.measurement_time(Duration::from_secs(10));
    group.throughput(Throughput::Elements(1));

    for chain in CHAINS {
        group.bench_with_input(
            BenchmarkId::new("transfer_1eth", chain),
            chain,
            |b, &chain| {
                b.iter(|| {
                    rt.block_on(async {
                        let result = service
                            .calculate_fee(black_box(chain), black_box("transfer"), black_box(1.0))
                            .await;
                        black_box(result);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark 3: 不同交易类型性能
fn bench_fee_by_tx_type(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let service = Arc::new(FeeService::new(pool));
    rt.block_on(warmup_cache(&service));

    let mut group = c.benchmark_group("fee_by_tx_type");
    group.measurement_time(Duration::from_secs(8));

    for tx_type in TX_TYPES {
        group.bench_with_input(
            BenchmarkId::new("ethereum_1eth", tx_type),
            tx_type,
            |b, &tx_type| {
                b.iter(|| {
                    rt.block_on(async {
                        let result = service
                            .calculate_fee(
                                black_box("ethereum"),
                                black_box(tx_type),
                                black_box(1.0),
                            )
                            .await;
                        black_box(result);
                    })
                });
            },
        );
    }
    group.finish();
}

/// Benchmark 4: 缓存性能测试
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let pool_clone = pool.clone();
    let service = Arc::new(FeeService::new(pool));

    let mut group = c.benchmark_group("cache_performance");
    group.measurement_time(Duration::from_secs(12));

    // 冷缓存 - 首次查询
    group.bench_function("cold_cache_first_query", |b| {
        b.iter(|| {
            let fresh_service = FeeService::new(pool_clone.clone());
            rt.block_on(async {
                let result = fresh_service
                    .calculate_fee("ethereum", "transfer", 1.0)
                    .await;
                black_box(result);
            })
        });
    });

    // 热缓存 - 重复查询同一配置
    rt.block_on(warmup_cache(&service));
    group.throughput(Throughput::Elements(100));
    group.bench_function("hot_cache_100_hits", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    let result = service.calculate_fee("ethereum", "transfer", 1.0).await;
                    black_box(result);
                }
            })
        });
    });

    // 缓存未命中 - 轮换不同链
    group.throughput(Throughput::Elements(50));
    group.bench_function("cache_miss_chain_rotation", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 0..50 {
                    let chain = CHAINS[i % CHAINS.len()];
                    let result = service.calculate_fee(chain, "transfer", 1.0).await;
                    black_box(result);
                }
            })
        });
    });

    group.finish();
}

/// Benchmark 5: 高并发费用计算
fn bench_concurrent_calculation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let service = Arc::new(FeeService::new(pool));
    rt.block_on(warmup_cache(&service));

    let mut group = c.benchmark_group("concurrent_calculation");
    group.measurement_time(Duration::from_secs(15));

    // 并发10
    group.throughput(Throughput::Elements(10));
    group.bench_function("concurrent_10", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..10 {
                    let s = service.clone();
                    let chain = CHAINS[i % CHAINS.len()];
                    handles.push(tokio::spawn(async move {
                        s.calculate_fee(chain, "transfer", 1.0).await
                    }));
                }
                let results = futures::future::join_all(handles).await;
                black_box(results);
            })
        });
    });

    // 并发50
    group.throughput(Throughput::Elements(50));
    group.bench_function("concurrent_50", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..50 {
                    let s = service.clone();
                    let chain = CHAINS[i % CHAINS.len()];
                    let tx_type = TX_TYPES[i % TX_TYPES.len()];
                    handles.push(tokio::spawn(async move {
                        s.calculate_fee(chain, tx_type, 1.0).await
                    }));
                }
                let results = futures::future::join_all(handles).await;
                black_box(results);
            })
        });
    });

    // 并发100 - 压力测试
    group.throughput(Throughput::Elements(100));
    group.bench_function("concurrent_100_stress", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = vec![];
                for i in 0..100 {
                    let s = service.clone();
                    let chain = CHAINS[i % CHAINS.len()];
                    let tx_type = TX_TYPES[i % TX_TYPES.len()];
                    let amount = if i % 2 == 0 { 0.1 } else { 1000.0 };
                    handles.push(tokio::spawn(async move {
                        s.calculate_fee(chain, tx_type, amount).await
                    }));
                }
                let results = futures::future::join_all(handles).await;
                black_box(results);
            })
        });
    });

    group.finish();
}

/// Benchmark 6: 批量计算吞吐量测试
fn bench_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let pool = rt.block_on(setup_test_pool());
    let service = Arc::new(FeeService::new(pool));
    rt.block_on(warmup_cache(&service));

    let mut group = c.benchmark_group("throughput");
    group.measurement_time(Duration::from_secs(20));
    group.throughput(Throughput::Elements(1000));

    group.bench_function("sequential_1000_requests", |b| {
        b.iter(|| {
            rt.block_on(async {
                for i in 0..1000 {
                    let chain = CHAINS[i % CHAINS.len()];
                    let result = service.calculate_fee(chain, "transfer", 1.0).await;
                    black_box(result);
                }
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_fee_by_amount_levels,
    bench_fee_multi_chain,
    bench_fee_by_tx_type,
    bench_cache_performance,
    bench_concurrent_calculation,
    bench_throughput
);

criterion_main!(benches);
