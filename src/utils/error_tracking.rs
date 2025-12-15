//! 错误追踪模块
//! 提供错误追踪ID生成和管理

use uuid::Uuid;

/// 生成错误追踪ID
pub fn generate_error_id() -> String {
    format!("err_{}", Uuid::new_v4().simple())
}

/// 从请求中提取或生成追踪ID
pub fn get_or_generate_trace_id(request_id: Option<&str>) -> String {
    request_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("trace_{}", Uuid::new_v4().simple()))
}

/// 格式化错误信息（包含追踪ID）
pub fn format_error_with_trace(error: &str, trace_id: &str) -> String {
    format!("[{}] {}", trace_id, error)
}
