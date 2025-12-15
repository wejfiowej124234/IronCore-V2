//! 时间工具模块
//! 提供时间处理相关的工具函数

use chrono::{DateTime, Utc};

/// 格式化时间戳为RFC3339格式
pub fn format_timestamp(dt: &DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// 获取当前时间戳（秒）
pub fn current_timestamp() -> i64 {
    Utc::now().timestamp()
}

/// 获取当前时间戳（毫秒）
pub fn current_timestamp_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// 格式化持续时间
pub fn format_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
    }
}
