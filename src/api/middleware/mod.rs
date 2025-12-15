pub mod auth;
pub mod csrf;
pub mod idempotency;
pub mod jwt_extractor;
pub mod rate_limit;
pub mod rbac;
pub mod resource_ownership;
pub mod trace_id;

// ✅ 企业级新增中间件
pub mod log_sanitizer_simple;
pub mod risk_control;
pub mod method_whitelist; // ✅ P0 Security: HTTP方法白名单

// 别名
pub use auth::{auth_middleware, extract_auth_info, AuthInfo};
pub use csrf::{csrf_middleware, csrf_middleware_with_state, generate_csrf_token, CsrfManager};
pub use idempotency::{clear_idempotency_key, idempotency_middleware};
pub use jwt_extractor::{jwt_extractor_middleware, JwtAuthContext};
pub use log_sanitizer_simple as log_sanitizer;
pub use method_whitelist::method_whitelist_middleware; // ✅ P0 Security
pub use rate_limit::{custom_rate_limit, rate_limit_middleware, RateLimitConfig};
pub use rbac::{require_admin, require_any_role, require_operator_or_admin, require_role, roles};
pub use trace_id::{extract_trace_id, trace_id_middleware, TraceIdGenerator};
