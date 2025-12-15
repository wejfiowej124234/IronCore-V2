//! Trace ID 中间件
//! 为每个请求生成唯一的 trace_id，用于全链路追踪

use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};
use uuid::Uuid;

/// Trace ID 生成器
pub struct TraceIdGenerator;

impl TraceIdGenerator {
    /// 生成新的 trace_id
    pub fn generate() -> String {
        Uuid::new_v4().to_string()
    }

    /// 从请求头中提取 trace_id，如果没有则生成新的
    pub fn get_or_generate(req: &Request) -> String {
        // 优先从请求头获取 trace_id
        if let Some(trace_id_header) = req.headers().get("X-Trace-Id") {
            if let Ok(trace_id) = trace_id_header.to_str() {
                if !trace_id.is_empty() {
                    return trace_id.to_string();
                }
            }
        }

        // 如果没有，生成新的 trace_id
        Self::generate()
    }
}

/// Trace ID 中间件
/// 为每个请求生成或提取 trace_id，并添加到请求扩展和响应头中
pub async fn trace_id_middleware(mut req: Request, next: Next) -> Response {
    // 获取或生成 trace_id
    let trace_id = TraceIdGenerator::get_or_generate(&req);

    // 将 trace_id 添加到请求扩展中，供后续处理使用
    req.extensions_mut().insert(trace_id.clone());

    // 处理请求
    let mut response = next.run(req).await;

    // 将 trace_id 添加到响应头中
    if let Ok(header_value) = HeaderValue::from_str(&trace_id) {
        response.headers_mut().insert("X-Trace-Id", header_value);
    }

    response
}

/// 从请求扩展中提取 trace_id
pub fn extract_trace_id(req: &Request) -> Option<String> {
    req.extensions().get::<String>().cloned()
}
