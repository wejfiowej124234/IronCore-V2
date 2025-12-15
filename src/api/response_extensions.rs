//! 响应扩展（兼容性）
//!
//! 提供error_response兼容性函数

use axum::Json;

use crate::{api::response::ApiResponse, error::AppError};

/// 错误响应（兼容旧代码）
pub fn error_response<T>(message: String) -> Result<Json<ApiResponse<T>>, AppError>
where
    T: serde::Serialize,
{
    Err(AppError::bad_request(message))
}
