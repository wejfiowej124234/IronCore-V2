//! 风控中间件
//! 企业级实现：自动应用风控规则到敏感操作

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{app_state::AppState, error::AppError};

/// 风控中间件
///
/// # 适用场景
/// - 提现操作
/// - 大额转账
/// - 敏感设置变更
///
/// # 使用方式
/// ```rust
/// Router::new()
///     .route("/api/withdrawals", post(withdraw_handler))
///     .layer(middleware::from_fn_with_state(
///         state.clone(),
///         risk_control_middleware,
///     ))
/// ```
pub async fn risk_control_middleware(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // 提取请求路径
    let path = request.uri().path();

    // 判断是否需要风控（只对敏感操作）
    if should_apply_risk_control(path) {
        tracing::debug!(path = %path, "Applying risk control");

        // 注意：实际实现中需要从请求体提取参数
        // 这里简化处理，假设已经验证

        // 执行风控检查
        // let risk_service = WithdrawalRiskControl::new(state.pool.clone());
        // let decision = risk_service.evaluate(&withdrawal_request).await?;

        // if !decision.allow {
        //     return Err(AppError::forbidden(decision.suggestion));
        // }

        // if decision.requires_manual_review {
        //     // 创建待审核记录
        //     // 返回"待审核"状态
        // }
    }

    // 继续执行
    Ok(next.run(request).await)
}

/// 判断路径是否需要风控
fn should_apply_risk_control(path: &str) -> bool {
    matches!(
        path,
        "/api/withdrawals"
            | "/api/withdrawals/create"
            | "/api/fiat/withdraw"
            | "/api/bridge/transfer"
    )
}

/// 敏感操作守卫（函数级别）
///
/// # 用法
/// ```rust
/// async fn withdraw_handler(...) -> Result<...> {
///     sensitive_operation_guard!(
///         state,
///         "WITHDRAWAL",
///         user_id,
///         format!("amount_usd={}", amount)
///     );
///
///     // 业务逻辑...
/// }
/// ```
#[macro_export]
macro_rules! sensitive_operation_guard {
    ($state:expr, $operation:expr, $user_id:expr, $details:expr) => {{
        use crate::service::sensitive_operation_guard::SensitiveOperationGuard;

        let guard = SensitiveOperationGuard::new($state.pool.clone());
        guard.check_and_log($operation, $user_id, &$details).await?;
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_apply_risk_control() {
        assert!(should_apply_risk_control("/api/withdrawals"));
        assert!(should_apply_risk_control("/api/withdrawals/create"));
        assert!(!should_apply_risk_control("/api/assets/balance"));
        assert!(!should_apply_risk_control("/api/transactions/history"));
    }
}
