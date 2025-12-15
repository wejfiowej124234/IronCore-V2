//! 性能基准测试
//! 使用criterion进行性能测试

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ironcore::metrics;

fn bench_metrics_rendering(c: &mut Criterion) {
    c.bench_function("render_prometheus_metrics", |b| {
        b.iter(|| {
            // 模拟一些指标
            metrics::count_ok("bench_endpoint");
            metrics::count_err("bench_endpoint");
            black_box(metrics::render_prometheus())
        })
    });
}

fn bench_metrics_counting(c: &mut Criterion) {
    c.bench_function("count_metrics", |b| {
        b.iter(|| {
            metrics::count_ok(black_box("test_endpoint"));
            metrics::count_err(black_box("test_endpoint"));
        })
    });
}

criterion_group!(benches, bench_metrics_rendering, bench_metrics_counting);
criterion_main!(benches);
