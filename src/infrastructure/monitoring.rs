//! 监控和告警模块
//! 提供Prometheus metrics导出和告警规则配置

use crate::metrics;
use axum::{response::IntoResponse, routing::get, Router};

/// 创建Prometheus metrics路由
pub fn create_metrics_router() -> Router {
    Router::new().route("/metrics", get(metrics_handler))
}

/// Prometheus metrics处理器
async fn metrics_handler() -> impl IntoResponse {
    let prometheus_output = metrics::render_prometheus();
    (
        axum::http::StatusCode::OK,
        [("Content-Type", "text/plain; version=0.0.4")],
        prometheus_output,
    )
}

/// 告警规则配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub condition: String,
    pub threshold: f64,
    pub severity: AlertSeverity,
    pub enabled: bool,
}

/// 告警严重程度
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

/// 告警管理器
pub struct AlertManager {
    rules: Vec<AlertRule>,
}

impl AlertManager {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    pub fn check_alerts(&self, metrics: &str) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            // 简化的告警检查逻辑
            // 实际应该解析Prometheus metrics并评估条件
            if self.evaluate_condition(&rule.condition, metrics, rule.threshold) {
                alerts.push(Alert {
                    rule_name: rule.name.clone(),
                    severity: rule.severity.clone(),
                    message: format!("Alert triggered: {}", rule.condition),
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        alerts
    }

    fn evaluate_condition(&self, condition: &str, metrics: &str, threshold: f64) -> bool {
        // 解析Prometheus metrics格式并评估条件
        // Prometheus metrics格式示例：
        // ironforge_errors_total 10
        // ironforge_requests_total 100

        if condition.contains("error_rate") {
            // 计算错误率：errors / requests
            let errors = self.extract_metric_value(metrics, "ironforge_errors_total");
            let requests = self.extract_metric_value(metrics, "ironforge_requests_total");

            if requests > 0.0 {
                let error_rate = errors / requests;
                return error_rate > threshold;
            }
        } else if condition.contains("latency") || condition.contains("p99") {
            // 生产级实现：从 histogram 提取 p99 分位数
            // Prometheus histogram 格式：metric_name_bucket{le="x.x"} count
            if condition.contains("p99") {
                // 尝试提取 p99 分位数（需要解析 histogram buckets）
                let p99_latency =
                    self.extract_histogram_quantile(metrics, "ironforge_upstream_latency_ms", 0.99);
                return p99_latency > threshold;
            } else {
                // 回退到平均延迟
                let latency_sum =
                    self.extract_metric_value(metrics, "ironforge_upstream_latency_ms_sum");
                let request_count =
                    self.extract_metric_value(metrics, "ironforge_upstream_requests_total");

                if request_count > 0.0 {
                    let avg_latency = latency_sum / request_count;
                    return avg_latency > threshold;
                }
            }
        } else if condition.contains("database_health") {
            // 检查数据库健康状态
            let health = self.extract_metric_value(metrics, "ironforge_database_health");
            return health <= threshold;
        }

        false
    }

    /// 从Prometheus metrics文本中提取指标值
    fn extract_metric_value(&self, metrics: &str, metric_name: &str) -> f64 {
        // 查找指标行：metric_name value
        for line in metrics.lines() {
            if line.trim_start().starts_with(metric_name) {
                // 解析值（可能是带标签的：metric_name{label="value"} number）
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Some(last_part) = parts.last() {
                    if let Ok(value) = last_part.parse::<f64>() {
                        return value;
                    }
                }
            }
        }
        0.0
    }

    /// 从 Prometheus histogram 中提取分位数（生产级实现）
    /// 解析 histogram buckets 并计算指定分位数的近似值
    fn extract_histogram_quantile(&self, metrics: &str, metric_name: &str, quantile: f64) -> f64 {
        // 收集所有 bucket 数据：histogram_name_bucket{le="upper_bound"} count
        let mut buckets: Vec<(f64, f64)> = Vec::new(); // (upper_bound, cumulative_count)
        let bucket_pattern = format!("{}_bucket", metric_name);

        for line in metrics.lines() {
            if line.trim_start().starts_with(&bucket_pattern) {
                // 解析格式：metric_name_bucket{le="100.0"} 123
                if let Some(le_start) = line.find("le=\"") {
                    let le_str = &line[le_start + 4..];
                    if let Some(le_end) = le_str.find('"') {
                        if let Ok(upper_bound) = le_str[..le_end].parse::<f64>() {
                            // 提取计数值
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if let Some(count_str) = parts.last() {
                                if let Ok(count) = count_str.parse::<f64>() {
                                    buckets.push((upper_bound, count));
                                }
                            }
                        }
                    }
                }
            }
        }

        if buckets.is_empty() {
            return 0.0;
        }

        // 排序 buckets（按上界升序）
        buckets.sort_by(|a, b| {
            a.0.partial_cmp(&b.0).unwrap_or_else(|| {
                // 如果无法比较（NaN），保持顺序
                std::cmp::Ordering::Equal
            })
        });

        // 获取总计数（最后一个bucket 或 _count 指标）
        let total_count = buckets.last().map(|(_, count)| *count).unwrap_or(0.0);
        if total_count == 0.0 {
            return 0.0;
        }

        // 计算目标分位数对应的计数阈值
        let target_count = total_count * quantile;

        // 线性插值找到对应的延迟值
        let mut prev_bound = 0.0;
        let mut prev_count = 0.0;

        for (upper_bound, cumulative_count) in &buckets {
            if *cumulative_count >= target_count {
                // 找到了目标bucket，进行线性插值
                if *cumulative_count == prev_count {
                    return *upper_bound;
                }
                let ratio = (target_count - prev_count) / (*cumulative_count - prev_count);
                return prev_bound + ratio * (*upper_bound - prev_bound);
            }
            prev_bound = *upper_bound;
            prev_count = *cumulative_count;
        }

        // 如果所有bucket都小于目标，返回最大上界
        buckets.last().map(|(bound, _)| *bound).unwrap_or(0.0)
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 告警信息
#[derive(Debug, Clone)]
pub struct Alert {
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 创建默认告警规则
pub fn create_default_alert_rules() -> Vec<AlertRule> {
    vec![
        AlertRule {
            name: "high_error_rate".to_string(),
            condition: "error_rate > 0.1".to_string(),
            threshold: 0.1,
            severity: AlertSeverity::Critical,
            enabled: true,
        },
        AlertRule {
            name: "high_latency".to_string(),
            condition: "p99_latency > 1000".to_string(),
            threshold: 1000.0,
            severity: AlertSeverity::Warning,
            enabled: true,
        },
        AlertRule {
            name: "database_connection_failure".to_string(),
            condition: "database_health == 0".to_string(),
            threshold: 0.0,
            severity: AlertSeverity::Critical,
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_manager() {
        let mut manager = AlertManager::new();
        let rule = AlertRule {
            name: "test_rule".to_string(),
            condition: "error_rate > 0.1".to_string(),
            threshold: 0.1,
            severity: AlertSeverity::Warning,
            enabled: true,
        };
        manager.add_rule(rule);
        assert_eq!(manager.rules.len(), 1);
    }

    #[test]
    fn test_default_alert_rules() {
        let rules = create_default_alert_rules();
        assert!(!rules.is_empty());
    }
}
