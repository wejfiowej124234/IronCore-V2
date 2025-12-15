use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum AppErrorCode {
    // HTTP 基础错误码
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    RateLimit,
    Timeout,
    Network,
    Internal,

    // 业务错误码
    WalletNotFound,
    WalletAlreadyExists,
    InsufficientBalance,
    InvalidAddress,
    InvalidAmount,
    TransactionFailed,
    NonceConflict,
    GasEstimationFailed,
    RpcError,
    InvalidMnemonic,
    EncryptionFailed,
    DecryptionFailed,
    InvalidSignature,
    ChainNotSupported,
    // 扩展业务错误码
    UserNotFound,
    UserAlreadyExists,
    InvalidCredentials,
    TokenExpired,
    TokenInvalid,
    RateLimitExceeded,
    ResourceNotFound,
    PermissionDenied,
    InvalidParameter,
    ServiceUnavailable,
    DatabaseError,
    CacheError,
    ExternalServiceError,
    ValidationFailed,
    DuplicateRequest,
}

#[derive(Debug, Clone)]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
    pub status: StatusCode,
    pub trace_id: Option<String>,
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    code: &'a str,
    message: &'a str,
    trace_id: Option<&'a str>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code_str = match self.code {
            // HTTP 基础错误码
            AppErrorCode::BadRequest => "bad_request",
            AppErrorCode::Unauthorized => "unauthorized",
            AppErrorCode::Forbidden => "forbidden",
            AppErrorCode::NotFound => "not_found",
            AppErrorCode::RateLimit => "rate_limit",
            AppErrorCode::Timeout => "timeout",
            AppErrorCode::Network => "network",
            AppErrorCode::Internal => "internal",

            // 业务错误码
            AppErrorCode::WalletNotFound => "wallet_not_found",
            AppErrorCode::WalletAlreadyExists => "wallet_already_exists",
            AppErrorCode::InsufficientBalance => "insufficient_balance",
            AppErrorCode::InvalidAddress => "invalid_address",
            AppErrorCode::InvalidAmount => "invalid_amount",
            AppErrorCode::TransactionFailed => "transaction_failed",
            AppErrorCode::NonceConflict => "nonce_conflict",
            AppErrorCode::GasEstimationFailed => "gas_estimation_failed",
            AppErrorCode::RpcError => "rpc_error",
            AppErrorCode::InvalidMnemonic => "invalid_mnemonic",
            AppErrorCode::EncryptionFailed => "encryption_failed",
            AppErrorCode::DecryptionFailed => "decryption_failed",
            AppErrorCode::InvalidSignature => "invalid_signature",
            AppErrorCode::ChainNotSupported => "chain_not_supported",
            // 扩展业务错误码
            AppErrorCode::UserNotFound => "user_not_found",
            AppErrorCode::UserAlreadyExists => "user_already_exists",
            AppErrorCode::InvalidCredentials => "invalid_credentials",
            AppErrorCode::TokenExpired => "token_expired",
            AppErrorCode::TokenInvalid => "token_invalid",
            AppErrorCode::RateLimitExceeded => "rate_limit_exceeded",
            AppErrorCode::ResourceNotFound => "resource_not_found",
            AppErrorCode::PermissionDenied => "permission_denied",
            AppErrorCode::InvalidParameter => "invalid_parameter",
            AppErrorCode::ServiceUnavailable => "service_unavailable",
            AppErrorCode::DatabaseError => "database_error",
            AppErrorCode::CacheError => "cache_error",
            AppErrorCode::ExternalServiceError => "external_service_error",
            AppErrorCode::ValidationFailed => "validation_failed",
            AppErrorCode::DuplicateRequest => "duplicate_request",
        };
        let body = ErrorBody {
            code: code_str,
            message: &self.message,
            trace_id: self.trace_id.as_deref(),
        };
        (self.status, Json(body)).into_response()
    }
}

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::BadRequest,
            message: Self::user_friendly_message(msg.into()),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    /// 将技术错误消息转换为用户友好的消息
    fn user_friendly_message(msg: String) -> String {
        // 检测常见的技术错误并转换为友好提示
        if msg.contains("database") || msg.contains("Database") {
            return "系统暂时不可用，请稍后重试".to_string();
        }
        if msg.contains("timeout") || msg.contains("Timeout") {
            return "请求超时，请检查网络连接后重试".to_string();
        }
        if msg.contains("network") || msg.contains("Network") {
            return "网络错误，请检查网络连接后重试".to_string();
        }
        if msg.contains("RPC") || msg.contains("rpc") {
            return "区块链网络暂时不可用，请稍后重试".to_string();
        }
        // 邮箱已注册错误 - 保持原消息，因为已经是用户友好的
        if msg.contains("Email already registered") || msg.contains("email already") {
            return "该邮箱已被注册，请使用其他邮箱或直接登录".to_string();
        }
        // 如果已经是友好消息，直接返回
        msg
    }

    pub fn bad_request_with_trace(msg: impl Into<String>, trace_id: String) -> Self {
        Self {
            code: AppErrorCode::BadRequest,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: Some(trace_id),
        }
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::NotFound,
            message: msg.into(),
            status: StatusCode::NOT_FOUND,
            trace_id: None,
        }
    }

    pub fn not_found_with_trace(msg: impl Into<String>, trace_id: String) -> Self {
        Self {
            code: AppErrorCode::NotFound,
            message: msg.into(),
            status: StatusCode::NOT_FOUND,
            trace_id: Some(trace_id),
        }
    }

    /// 设置追踪ID
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// 从请求扩展中获取trace_id并设置
    pub fn with_trace_id_from_request(mut self, req: &axum::extract::Request) -> Self {
        if let Some(trace_id) = req.extensions().get::<String>() {
            self.trace_id = Some(trace_id.clone());
        }
        self
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::BadRequest,
            message: msg.into(),
            status: StatusCode::CONFLICT,
            trace_id: None,
        }
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::Internal,
            message: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            trace_id: None,
        }
    }

    /// 别名：内部错误（兼容旧代码）
    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::internal(msg)
    }

    pub fn internal_with_trace(msg: impl Into<String>, trace_id: String) -> Self {
        Self {
            code: AppErrorCode::Internal,
            message: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            trace_id: Some(trace_id),
        }
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::Unauthorized,
            message: msg.into(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: None,
        }
    }

    pub fn unauthorized_with_trace(msg: impl Into<String>, trace_id: String) -> Self {
        Self {
            code: AppErrorCode::Unauthorized,
            message: msg.into(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: Some(trace_id),
        }
    }

    // 业务错误辅助函数
    pub fn wallet_not_found(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::WalletNotFound,
            message: msg.into(),
            status: StatusCode::NOT_FOUND,
            trace_id: None,
        }
    }

    pub fn wallet_already_exists(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::WalletAlreadyExists,
            message: msg.into(),
            status: StatusCode::CONFLICT,
            trace_id: None,
        }
    }

    pub fn insufficient_balance(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InsufficientBalance,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn invalid_address(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidAddress,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn invalid_amount(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidAmount,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn transaction_failed(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::TransactionFailed,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn nonce_conflict(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::NonceConflict,
            message: msg.into(),
            status: StatusCode::CONFLICT,
            trace_id: None,
        }
    }

    pub fn gas_estimation_failed(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::GasEstimationFailed,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn rpc_error(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::RpcError,
            message: msg.into(),
            status: StatusCode::BAD_GATEWAY,
            trace_id: None,
        }
    }

    pub fn invalid_mnemonic(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidMnemonic,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn encryption_failed(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::EncryptionFailed,
            message: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            trace_id: None,
        }
    }

    pub fn decryption_failed(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::DecryptionFailed,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn invalid_signature(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidSignature,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn chain_not_supported(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::ChainNotSupported,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    // 扩展业务错误辅助函数
    pub fn user_not_found(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::UserNotFound,
            message: msg.into(),
            status: StatusCode::NOT_FOUND,
            trace_id: None,
        }
    }

    pub fn user_already_exists(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::UserAlreadyExists,
            message: msg.into(),
            status: StatusCode::CONFLICT,
            trace_id: None,
        }
    }

    pub fn invalid_credentials(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidCredentials,
            message: msg.into(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: None,
        }
    }

    pub fn token_expired(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::TokenExpired,
            message: msg.into(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: None,
        }
    }

    pub fn token_invalid(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::TokenInvalid,
            message: msg.into(),
            status: StatusCode::UNAUTHORIZED,
            trace_id: None,
        }
    }

    pub fn rate_limit_exceeded(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::RateLimitExceeded,
            message: msg.into(),
            status: StatusCode::TOO_MANY_REQUESTS,
            trace_id: None,
        }
    }

    pub fn resource_not_found(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::ResourceNotFound,
            message: msg.into(),
            status: StatusCode::NOT_FOUND,
            trace_id: None,
        }
    }

    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::PermissionDenied,
            message: msg.into(),
            status: StatusCode::FORBIDDEN,
            trace_id: None,
        }
    }

    /// 别名：禁止访问（兼容旧代码）
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::permission_denied(msg)
    }

    pub fn invalid_parameter(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidParameter,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::ServiceUnavailable,
            message: msg.into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
            trace_id: None,
        }
    }

    pub fn database_error(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::DatabaseError,
            message: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            trace_id: None,
        }
    }

    pub fn cache_error(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::CacheError,
            message: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
            trace_id: None,
        }
    }

    pub fn external_service_error(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::ExternalServiceError,
            message: msg.into(),
            status: StatusCode::BAD_GATEWAY,
            trace_id: None,
        }
    }

    pub fn validation_failed(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::ValidationFailed,
            message: msg.into(),
            status: StatusCode::BAD_REQUEST,
            trace_id: None,
        }
    }

    pub fn duplicate_request(msg: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::DuplicateRequest,
            message: msg.into(),
            status: StatusCode::CONFLICT,
            trace_id: None,
        }
    }
}

// 从 serde_json 错误转换
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::bad_request(format!("JSON serialization error: {}", err))
    }
}

// 从 SQLx 错误转换
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::not_found("Resource not found"),
            sqlx::Error::Database(ref db_err) => {
                // 检查是否是约束违规（如唯一性冲突）
                if let Some(code) = db_err.code() {
                    if code == "23505" {
                        // PostgreSQL unique_violation
                        return Self::bad_request("Resource already exists");
                    }
                    if code == "23503" {
                        // PostgreSQL foreign_key_violation
                        return Self::bad_request("Foreign key constraint violation");
                    }
                }
                Self::internal(format!("Database error: {}", db_err))
            }
            _ => Self::internal(format!("Database operation failed: {}", err)),
        }
    }
}

// 从 UUID 错误转换
impl From<uuid::Error> for AppError {
    fn from(err: uuid::Error) -> Self {
        Self::bad_request(format!("Invalid UUID: {}", err))
    }
}

// 从 anyhow 错误转换
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::internal(format!("{}", err))
    }
}
