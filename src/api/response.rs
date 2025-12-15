//! 统一 API 响应格式
//!
//! 所有 API 接口应使用统一的响应格式：{ code, message, data }

use axum::Json;
use serde::Serialize;

use crate::error::AppError;

pub mod pagination;

/// 统一成功响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    pub data: T,
}

/// 统一错误响应格式（已在 AppError 中实现）
/// 错误响应格式：{ code: "error_code", message: "error_message", trace_id?: "trace_id" }

impl<T> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            message: "success".to_string(),
            data,
        }
    }

    /// 创建成功响应（带自定义消息）
    pub fn success_with_message(data: T, message: String) -> Self {
        Self {
            code: 0,
            message,
            data,
        }
    }
}

/// 将 ApiResponse 转换为 Axum Json 响应
impl<T: Serialize> From<ApiResponse<T>> for Result<Json<ApiResponse<T>>, AppError> {
    fn from(response: ApiResponse<T>) -> Self {
        Ok(Json(response))
    }
}

/// 辅助函数：将数据包装为统一响应格式
pub fn success_response<T: Serialize>(data: T) -> Result<Json<ApiResponse<T>>, AppError> {
    Ok(Json(ApiResponse::success(data)))
}

/// 辅助函数：将数据包装为统一响应格式（带自定义消息）
pub fn success_response_with_message<T: Serialize>(
    data: T,
    message: String,
) -> Result<Json<ApiResponse<T>>, AppError> {
    Ok(Json(ApiResponse::success_with_message(data, message)))
}

/// 将 (StatusCode, String) 错误转换为 AppError
/// 用于迁移使用旧错误格式的接口
pub fn convert_error(status: axum::http::StatusCode, msg: String) -> AppError {
    use axum::http::StatusCode;

    use crate::error::AppErrorCode;

    let code = match status {
        StatusCode::BAD_REQUEST => AppErrorCode::BadRequest,
        StatusCode::UNAUTHORIZED => AppErrorCode::Unauthorized,
        StatusCode::FORBIDDEN => AppErrorCode::Forbidden,
        StatusCode::NOT_FOUND => AppErrorCode::NotFound,
        StatusCode::TOO_MANY_REQUESTS => AppErrorCode::RateLimit,
        StatusCode::REQUEST_TIMEOUT | StatusCode::GATEWAY_TIMEOUT => AppErrorCode::Timeout,
        _ => AppErrorCode::Internal,
    };

    AppError {
        code,
        message: msg,
        status,
        trace_id: None,
    }
}

/// 创建错误响应（兼容旧代码）
pub fn error_response(status: axum::http::StatusCode, msg: String) -> AppError {
    convert_error(status, msg)
}
