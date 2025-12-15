use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

static METRICS: OnceLock<Mutex<MetricsState>> = OnceLock::new();

struct MetricsState {
    total: u64,
    errors: u64,
    per_endpoint: HashMap<&'static str, u64>,
    per_endpoint_err: HashMap<&'static str, u64>,
    // 上游成功/失败与时延统计（毫秒）
    upstream_ok: u64,
    upstream_err: u64,
    upstream_latency_sum_ms: u128,
    // 简易直方图分桶（毫秒）：<50, <100, <250, <500, <1000, >=1000
    upstream_hist_buckets: [u64; 6],
    // 新增：平台费相关
    fee_calculation_total: u64,
    fee_audit_write_fail: u64,
    fee_total_amount: f64,
    // RPC 端点相关
    rpc_selection_total: u64,
    rpc_selection_fallback: u64,
    rpc_circuit_open_total: u64,
}

fn state() -> &'static Mutex<MetricsState> {
    METRICS.get_or_init(|| {
        Mutex::new(MetricsState {
            total: 0,
            errors: 0,
            per_endpoint: HashMap::new(),
            per_endpoint_err: HashMap::new(),
            upstream_ok: 0,
            upstream_err: 0,
            upstream_latency_sum_ms: 0,
            upstream_hist_buckets: [0; 6],
            fee_calculation_total: 0,
            fee_audit_write_fail: 0,
            fee_total_amount: 0.0,
            rpc_selection_total: 0,
            rpc_selection_fallback: 0,
            rpc_circuit_open_total: 0,
        })
    })
}

pub fn count_ok(endpoint: &'static str) {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(), // 避免因锁污染导致 panic
    };
    s.total += 1;
    *s.per_endpoint.entry(endpoint).or_insert(0) += 1;
}

pub fn count_err(endpoint: &'static str) {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.total += 1;
    s.errors += 1;
    *s.per_endpoint.entry(endpoint).or_insert(0) += 1;
    *s.per_endpoint_err.entry(endpoint).or_insert(0) += 1;
}

pub fn render_prometheus() -> String {
    let s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let mut out = String::new();
    out.push_str("# HELP ironforge_requests_total Total requests\n");
    out.push_str("# TYPE ironforge_requests_total counter\n");
    out.push_str(&format!("ironforge_requests_total {}\n", s.total));

    out.push_str("# HELP ironforge_errors_total Total error responses\n");
    out.push_str("# TYPE ironforge_errors_total counter\n");
    out.push_str(&format!("ironforge_errors_total {}\n", s.errors));

    out.push_str("# HELP ironforge_endpoint_requests_total Requests per endpoint\n");
    out.push_str("# TYPE ironforge_endpoint_requests_total counter\n");
    for (k, v) in s.per_endpoint.iter() {
        out.push_str(&format!(
            "ironforge_endpoint_requests_total{{endpoint=\"{}\"}} {}\n",
            k, v
        ));
    }

    out.push_str("# HELP ironforge_endpoint_errors_total Errors per endpoint\n");
    out.push_str("# TYPE ironforge_endpoint_errors_total counter\n");
    for (k, v) in s.per_endpoint_err.iter() {
        out.push_str(&format!(
            "ironforge_endpoint_errors_total{{endpoint=\"{}\"}} {}\n",
            k, v
        ));
    }

    // 上游统计
    out.push_str("# HELP ironforge_upstream_requests_total Upstream requests\n");
    out.push_str("# TYPE ironforge_upstream_requests_total counter\n");
    out.push_str(&format!(
        "ironforge_upstream_requests_total{{result=\"ok\"}} {}\n",
        s.upstream_ok
    ));
    out.push_str(&format!(
        "ironforge_upstream_requests_total{{result=\"err\"}} {}\n",
        s.upstream_err
    ));

    out.push_str("# HELP ironforge_upstream_latency_ms_sum Sum of upstream latency in ms\n");
    out.push_str("# TYPE ironforge_upstream_latency_ms_sum counter\n");
    out.push_str(&format!(
        "ironforge_upstream_latency_ms_sum {}\n",
        s.upstream_latency_sum_ms
    ));

    out.push_str(
        "# HELP ironforge_upstream_latency_ms_bucket Upstream latency histogram buckets\n",
    );
    out.push_str("# TYPE ironforge_upstream_latency_ms_bucket histogram\n");
    let bounds = [50, 100, 250, 500, 1000];
    for (i, bound) in bounds.iter().enumerate() {
        out.push_str(&format!(
            "ironforge_upstream_latency_ms_bucket{{le=\"{}\"}} {}\n",
            bound, s.upstream_hist_buckets[i]
        ));
    }
    // +Inf 桶
    out.push_str(&format!(
        "ironforge_upstream_latency_ms_bucket{{le=\"+Inf\"}} {}\n",
        s.upstream_hist_buckets.iter().sum::<u64>()
    ));

    // 费用系统指标
    out.push_str("# HELP ironforge_fee_calculation_total Total fee calculations\n");
    out.push_str("# TYPE ironforge_fee_calculation_total counter\n");
    out.push_str(&format!(
        "ironforge_fee_calculation_total {}\n",
        s.fee_calculation_total
    ));

    out.push_str("# HELP ironforge_fee_audit_write_fail_total Failed fee audit writes\n");
    out.push_str("# TYPE ironforge_fee_audit_write_fail_total counter\n");
    out.push_str(&format!(
        "ironforge_fee_audit_write_fail_total {}\n",
        s.fee_audit_write_fail
    ));

    out.push_str("# HELP ironforge_fee_total_amount_collected Total platform fees collected\n");
    out.push_str("# TYPE ironforge_fee_total_amount_collected counter\n");
    out.push_str(&format!(
        "ironforge_fee_total_amount_collected {}\n",
        s.fee_total_amount
    ));

    // RPC 选择指标
    out.push_str("# HELP ironforge_rpc_selection_total Total RPC endpoint selections\n");
    out.push_str("# TYPE ironforge_rpc_selection_total counter\n");
    out.push_str(&format!(
        "ironforge_rpc_selection_total {}\n",
        s.rpc_selection_total
    ));

    out.push_str("# HELP ironforge_rpc_selection_fallback_total RPC selections using fallback\n");
    out.push_str("# TYPE ironforge_rpc_selection_fallback_total counter\n");
    out.push_str(&format!(
        "ironforge_rpc_selection_fallback_total {}\n",
        s.rpc_selection_fallback
    ));

    out.push_str("# HELP ironforge_rpc_circuit_open_total RPC endpoints in circuit open state\n");
    out.push_str("# TYPE ironforge_rpc_circuit_open_total counter\n");
    out.push_str(&format!(
        "ironforge_rpc_circuit_open_total {}\n",
        s.rpc_circuit_open_total
    ));

    out
}

pub fn observe_upstream_latency_ms(latency_ms: u128, ok: bool) {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    if ok {
        s.upstream_ok += 1;
    } else {
        s.upstream_err += 1;
    }
    s.upstream_latency_sum_ms += latency_ms;
    let b = if latency_ms < 50 {
        0
    } else if latency_ms < 100 {
        1
    } else if latency_ms < 250 {
        2
    } else if latency_ms < 500 {
        3
    } else if latency_ms < 1000 {
        4
    } else {
        5
    };
    s.upstream_hist_buckets[b] += 1;
}

pub fn inc_fee_calculation() {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.fee_calculation_total += 1;
}

pub fn inc_fee_audit_fail() {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.fee_audit_write_fail += 1;
}

pub fn add_fee_amount(amount: f64) {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.fee_total_amount += amount;
}

pub fn inc_rpc_selection() {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.rpc_selection_total += 1;
}

pub fn inc_rpc_fallback() {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.rpc_selection_fallback += 1;
}

pub fn inc_rpc_circuit_open() {
    let mut s = match state().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    s.rpc_circuit_open_total += 1;
}

// 区块链广播指标✅多链支持
pub fn inc_blockchain_broadcast_success(chain: &str) {
    let endpoint = match chain {
        "eth" | "ethereum" => "blockchain_broadcast_eth_success",
        "bsc" | "binance" => "blockchain_broadcast_bsc_success",
        "polygon" | "matic" => "blockchain_broadcast_polygon_success",
        "solana" | "sol" => "blockchain_broadcast_solana_success",
        "bitcoin" | "btc" => "blockchain_broadcast_bitcoin_success",
        "ton" => "blockchain_broadcast_ton_success",
        "arbitrum" | "arb" => "blockchain_broadcast_arbitrum_success",
        "optimism" | "op" => "blockchain_broadcast_optimism_success",
        "avalanche" | "avax" => "blockchain_broadcast_avalanche_success",
        _ => "blockchain_broadcast_other_success",
    };
    count_ok(endpoint);
}

pub fn inc_blockchain_broadcast_fail(chain: &str) {
    let endpoint = match chain {
        "eth" | "ethereum" => "blockchain_broadcast_eth_fail",
        "bsc" | "binance" => "blockchain_broadcast_bsc_fail",
        "polygon" | "matic" => "blockchain_broadcast_polygon_fail",
        "solana" | "sol" => "blockchain_broadcast_solana_fail",
        "bitcoin" | "btc" => "blockchain_broadcast_bitcoin_fail",
        "ton" => "blockchain_broadcast_ton_fail",
        "arbitrum" | "arb" => "blockchain_broadcast_arbitrum_fail",
        "optimism" | "op" => "blockchain_broadcast_optimism_fail",
        "avalanche" | "avax" => "blockchain_broadcast_avalanche_fail",
        _ => "blockchain_broadcast_other_fail",
    };
    count_err(endpoint);
}
