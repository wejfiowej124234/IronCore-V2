use std::{sync::Arc, time::Instant};

use axum::{
    extract::Request,
    http::{
        header::{
            ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
            ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE,
            CACHE_CONTROL, CONTENT_SECURITY_POLICY, PRAGMA, REFERRER_POLICY,
            X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
        },
        HeaderValue, StatusCode,
    },
    middleware::{from_fn, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Router,
};
use tower::ServiceBuilder;
use tracing::Level;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{
    api::{
        handlers::{
            api_errors, api_fees, api_health, api_network_status, balance,
            broadcast_raw_transaction, calculate_platform_fee, create_api_key, create_approval,
            create_policy, create_tenant, create_tx, create_tx_broadcast, create_user,
            delete_api_key, delete_approval, delete_policy, delete_tenant, delete_user,
            delete_wallet, get_api_key, get_approval, get_login_history, get_me, get_nonce,
            get_policy, get_solana_recent_blockhash, get_tenant, get_ton_seqno, get_tx,
            get_tx_broadcast, get_tx_broadcast_by_tx_hash, get_tx_history, get_user, get_wallet,
            healthz, list_api_keys, list_approvals, list_policies, list_tenants, list_tx,
            list_tx_broadcasts, list_users, list_wallets, login, logout, openapi_yaml,
            refresh_token, register, reset_password, set_password, simple_list_transactions,
            simple_send_transaction, tx_status, update_api_key_status, update_approval_status,
            update_policy, update_tenant, update_tx_broadcast, update_tx_status, update_user,
        },
        middleware::{rate_limit_middleware, trace_id_middleware},
    },
    app_state::AppState,
};

pub mod admin_api;
pub mod asset_api;
pub mod audit_api;
pub mod auth_api; // ✅ 认证 API（注册、登录、登出）
pub mod bitcoin_api; // ✅ Bitcoin特定功能API
pub mod bridge_api; // NEW: 跨链桥 API
pub mod bridge_enhanced_api; // ✅ 增强版跨链桥API
pub mod config_api; // ✅ 公共配置API（前端获取token配置）
pub mod country_support_api; // ✅ 国家支持查询API
pub mod cross_chain_enhanced_api; // ✅ G项深度优化: 跨链桥状态机+双锁验证
pub mod feature_api; // ✅ 功能开关API
pub mod fee_config_api; // ✅ P0-5: 统一费率配置API
pub mod fiat_api;
pub mod fiat_api_cancel_retry;
pub mod fiat_api_orders;
pub mod fiat_offramp_enhanced; // ✅ H项深度优化: 法币提现签名验证+风控
pub mod fiat_onramp_non_custodial; // ✅ P2: 法币充值非托管API
pub mod gas_api;
pub mod gas_estimation_api; // ✅ 增强版Gas估算API
pub mod handlers;
pub mod history_api;
pub mod limit_order_api;
pub mod middleware;
pub mod multi_chain_api;
pub mod network_config_api;
pub mod nonce_management_api;
pub mod notification_api;
pub mod notification_settings; // NEW: 通知偏好设置 API
pub mod provider_api;
pub mod reconciliation_api;
pub mod response; // 统一响应格式
pub mod response_extensions; // 响应扩展（兼容性）
pub mod router_integration; // ✅ 路由集成模块
pub mod swap_api;
pub mod token_api;
pub mod token_detection_api; // NEW: 代币检测 API
pub mod transaction_accelerate_api; // ✅ M项优化: RBF交易加速API
pub mod transaction_sign_required_middleware; // ✅ P1: 交易签名强制中间件
pub mod user_api; // ✅ 用户信息与KYC状态API
pub mod wallet_batch_create_api; // ✅ 非托管批量创建钱包API
pub mod wallet_unlock_api; // ✅ P0: 钱包解锁API（双锁机制）
pub mod wallet_unlock_verify_api; // ✅ B项增强: 双锁机制后端验证
pub mod webhook_api;
pub mod withdrawal_api; // ✅ P0-4: 提现API // ✅ F项补充: Nonce查询API

#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::create_tenant,
        handlers::list_tenants,
        handlers::get_tenant,
        handlers::update_tenant,
        handlers::delete_tenant,
        handlers::create_user,
        handlers::list_users,
        handlers::get_user,
        handlers::update_user,
        handlers::delete_user,
        handlers::create_policy,
        handlers::list_policies,
        handlers::get_policy,
        handlers::update_policy,
        handlers::delete_policy,
        handlers::create_approval,
        handlers::list_approvals,
        handlers::get_approval,
        handlers::update_approval_status,
        handlers::delete_approval,
        handlers::create_api_key,
        handlers::list_api_keys,
        handlers::get_api_key,
        handlers::update_api_key_status,
        handlers::delete_api_key,
        handlers::create_tx_broadcast,
        handlers::list_tx_broadcasts,
        handlers::get_tx_broadcast,
        handlers::update_tx_broadcast,
        handlers::get_tx_broadcast_by_tx_hash,
        handlers::register,
        handlers::login,
        handlers::logout,
        handlers::get_me,
        handlers::set_password,
        handlers::refresh_token,
        handlers::reset_password,
        handlers::get_login_history,
        handlers::list_wallets,
        handlers::get_wallet,
        handlers::delete_wallet,
        handlers::simple_delete_wallet,
        multi_chain_api::create_multi_chain_wallets,
        multi_chain_api::validate_address,
        handlers::create_tx,
        handlers::list_tx,
        handlers::get_tx,
        handlers::update_tx_status,
        handlers::api_health,
        handlers::healthz,
        handlers::api_errors,
        handlers::api_fees,
        handlers::api_network_status,
        handlers::balance,
        handlers::calculate_platform_fee,
        gas_api::estimate_gas,
        gas_api::estimate_all_speeds,
        multi_chain_api::create_multi_chain_wallets,
        multi_chain_api::list_chains,
        multi_chain_api::list_chains_by_curve,
        multi_chain_api::validate_address
    ),
    components(
        schemas(
            handlers::CreateTenantReq,
            handlers::TenantResp,
            handlers::ListTenantsQuery,
            handlers::ListTenantsResp,
            handlers::UpdateTenantReq,
            handlers::CreateUserReq,
            handlers::UserResp,
            handlers::ListUsersQuery,
            handlers::ListUsersResp,
            handlers::UpdateUserReq,
            handlers::CreatePolicyReq,
            handlers::PolicyResp,
            handlers::ListPoliciesQuery,
            handlers::ListPoliciesResp,
            handlers::UpdatePolicyReq,
            handlers::CreateApprovalReq,
            handlers::ApprovalResp,
            handlers::ListApprovalsQuery,
            handlers::ListApprovalsResp,
            handlers::UpdateApprovalStatusReq,
            handlers::CreateApiKeyReq,
            handlers::ApiKeyResp,
            handlers::ListApiKeysQuery,
            handlers::ListApiKeysResp,
            handlers::UpdateApiKeyStatusReq,
            handlers::CreateTxBroadcastReq,
            handlers::TxBroadcastResp,
            handlers::ListTxBroadcastsQuery,
            handlers::ListTxBroadcastsResp,
            handlers::UpdateTxBroadcastReq,
            handlers::GetTxBroadcastByHashQuery,
            handlers::WalletResp,
            handlers::ListWalletsQuery,
            handlers::ListWalletsResp,
            handlers::GetWalletParams,
            handlers::HealthResponse,
            handlers::Healthz,
            handlers::FeesQuery,
            gas_api::EstimateGasQuery,
            gas_api::EstimateAllQuery,
            crate::service::gas_estimator::GasEstimate,
            crate::service::gas_estimator::GasEstimateResponse,
            crate::service::gas_estimator::GasSpeed,
            handlers::FeesResponse,
            handlers::NetworkStatusQuery,
            handlers::NetworkStatusResponse,
            handlers::BalanceQuery,
            handlers::BalanceResponse,
            handlers::CreateTxReq,
            handlers::TxResp,
            handlers::ListTxQuery,
            handlers::ListTxResp,
            handlers::GetTxParams,
            handlers::TxDetailResp,
        handlers::UpdateTxStatusReq,
        handlers::RegisterReq,
        handlers::RegisterResp,
        handlers::LoginReq,
        handlers::LoginResp,
        handlers::LogoutReq,
        handlers::SetPasswordReq,
        handlers::RefreshTokenReq,
        handlers::RefreshTokenResp,
        handlers::ResetPasswordReq,
        handlers::GetLoginHistoryQuery,
        handlers::LoginHistoryResp,
        multi_chain_api::CreateMultiChainWalletsRequest,
        multi_chain_api::CreateWalletApiResponse,
        multi_chain_api::ValidateAddressRequest,
        multi_chain_api::ValidateAddressResponse,
        crate::error_body::ErrorBodyDoc
        )
    ),
    tags(
        (name = "IronForge API", description = "Auto-generated OpenAPI via utoipa")
    )
)]
struct ApiDoc;

pub fn routes(state: Arc<AppState>) -> Router {
    // 公开路由（不需要认证）
    let public_routes = Router::new()
        // ✅ 企业级标准 V1：认证 API
        .route(
            "/api/v1/auth/register",
            post(register).options(preflight_ok),
        )
        .route("/api/v1/auth/login", post(login).options(preflight_ok))
        .route(
            "/api/v1/auth/refresh",
            post(refresh_token).options(preflight_ok),
        )
        .route("/api/v1/errors", get(api_errors))
        .route("/openapi.yaml", get(openapi_yaml))
        .merge(utoipa_swagger_ui::SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .route(
            "/metrics",
            get(|| async { crate::metrics::render_prometheus().into_response() }),
        )
        // ✅ 企业级标准 V1：多链钱包 API
        .route("/api/v1/chains", get(multi_chain_api::list_chains))
        .route(
            "/api/v1/chains/by-curve",
            get(multi_chain_api::list_chains_by_curve),
        )
        // ✅ 企业级标准 V1：价格查询 API
        .route(
            "/api/v1/prices",
            get(asset_api::get_prices).options(preflight_ok),
        )
        // ✅ 企业级标准：Swap API v1版本（同链交换不同代币）
        .route(
            "/api/v1/swap/quote",
            axum::routing::get(swap_api::get_simple_swap_quote).options(preflight_ok),
        )
        .route(
            "/api/v1/swap/execute",
            axum::routing::post(swap_api::execute_simple_swap).options(preflight_ok),
        )
        .route(
            "/api/v1/swap/:id/status",
            axum::routing::get(swap_api::get_swap_status).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：Gas 费预估 API
        .route(
            "/api/v1/gas/estimate",
            get(gas_api::estimate_gas).options(preflight_ok),
        )
        .route(
            "/api/v1/gas/estimate-all",
            get(gas_api::estimate_all_speeds).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：平台服务费计算 API（公开，无需认证）
        .route(
            "/api/v1/fees/calculate",
            post(calculate_platform_fee).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：网络配置 API
        .route(
            "/api/v1/network-config",
            get(network_config_api::get_network_config).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：Bitcoin 特定功能 API
        .nest("/api/v1/bitcoin", bitcoin_api::routes())
        // ✅ 企业级标准 V1：功能开关 API
        .nest("/api/v1/features", feature_api::routes())
        // ✅ 企业级标准 V1：公共配置 API（前端获取token配置，无需认证）
        .route(
            "/api/v1/config/public",
            axum::routing::get(config_api::get_public_config).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：国家支持 API
        .nest("/api/v1/country-support", country_support_api::routes())
        // ✅ 企业级标准 V1：Webhook回调 API
        .route(
            "/api/v1/fiat/webhook/:provider",
            axum::routing::post(webhook_api::handle_webhook).options(preflight_ok),
        )
        .route(
            "/api/v1/webhook/:provider",
            axum::routing::post(webhook_api::handle_webhook).options(preflight_ok),
        )
        .route(
            "/api/v1/webhook/:provider/verify",
            axum::routing::get(webhook_api::handle_webhook).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：服务商 API
        .route(
            "/api/v1/providers",
            axum::routing::get(provider_api::get_providers).options(preflight_ok),
        )
        .route(
            "/api/v1/providers/country-support",
            axum::routing::get(provider_api::check_country_support).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：代币信息 API
        .route(
            "/api/v1/tokens/list",
            axum::routing::get(token_api::get_token_list).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/:address/info",
            axum::routing::get(token_api::get_token_info_by_address).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/:token_address/balance",
            axum::routing::get(token_api::get_token_balance).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/:symbol/address",
            axum::routing::get(token_api::get_token_address_by_symbol).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/detect",
            axum::routing::get(token_detection_api::detect_tokens).options(preflight_ok),
        )
        // ✅ 企业级标准：法币报价 API（公开，不需要认证）
        .route(
            "/api/v1/fiat/onramp/quote",
            axum::routing::get(fiat_api::get_fiat_quote).options(preflight_ok),
        )
        .route(
            "/api/v1/fiat/offramp/quote",
            axum::routing::get(fiat_api::get_withdraw_quote).options(preflight_ok),
        )
        // ✅ 企业级标准：法币服务商列表 API（公开）
        .route(
            "/api/v1/fiat/providers",
            axum::routing::get(provider_api::get_providers).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/metadata",
            axum::routing::get(token_detection_api::get_token_metadata).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/search",
            axum::routing::get(token_detection_api::search_tokens).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/popular",
            axum::routing::get(token_detection_api::get_popular_tokens).options(preflight_ok),
        )
        .route(
            "/api/v1/tokens/balances",
            axum::routing::post(token_detection_api::get_token_balances).options(preflight_ok),
        )
        // ✅ 企业级标准 V1：钱包余额和交易 API
        .route(
            "/api/v1/wallets/:address/balance",
            axum::routing::get(handlers::wallet_balance_public).options(preflight_ok),
        )
        .route(
            "/api/v1/wallets/:address/transactions",
            axum::routing::get(handlers::wallet_transactions_public).options(preflight_ok),
        )
        // ✅ 系统健康检查（必须在middleware之前定义，才能被middleware包裹）
        .route("/health", get(api_health)) // 简短别名，兼容测试脚本
        .route("/api/health", get(api_health))
        .route("/healthz", get(healthz))
        .layer(
            ServiceBuilder::new()
                .layer(from_fn(middleware::method_whitelist_middleware)) // ✅ P0 Security: 最先应用
                .layer(from_fn(trace_id_middleware))
                .layer(from_fn(add_cors_headers))
                .layer(from_fn(add_security_headers)) // ✅ P1修复：添加安全头
                .layer(from_fn(add_response_time_header))
                .layer(from_fn(trace_log))
                .layer(from_fn(set_request_id)),
        );

    // 需要认证的路由
    let protected_routes = Router::new()
        // ✅ 企业级标准 V1：认证 API (受保护)
        .route("/api/v1/auth/logout", post(logout))
        .route("/api/v1/auth/me", get(get_me))
        .route("/api/v1/auth/set-password", post(set_password))
        .route("/api/v1/auth/reset-password", post(reset_password))
        .route("/api/v1/auth/login-history", get(get_login_history))
        // ✅ 企业级标准 V1：Swap 历史记录（需要认证）
        .route(
            "/api/v1/swap/history",
            axum::routing::get(history_api::get_transaction_history).options(preflight_ok),
        )
        .route(
            "/api/v1/swap/history/:id",
            axum::routing::get(history_api::get_transaction_detail).options(preflight_ok),
        )
        // ✅ 企业级标准：交易API v1版本（需要认证）
        .route(
            "/api/v1/transactions",
            post(simple_send_transaction).get(simple_list_transactions),
        )
        .route(
            "/api/v1/transactions/broadcast",
            post(broadcast_raw_transaction).options(preflight_ok),
        )
        .route(
            "/api/v1/transactions/:hash/status",
            get(tx_status).options(preflight_ok),
        )
        .route(
            "/api/v1/transactions/nonce",
            get(get_nonce).options(preflight_ok),
        )
        .route(
            "/api/v1/transactions/history",
            get(get_tx_history).options(preflight_ok),
        )
        // ✅ 链特定端点 v1版本
        .route(
            "/api/v1/solana/recent-blockhash",
            get(get_solana_recent_blockhash).options(preflight_ok),
        )
        .route(
            "/api/v1/ton/seqno",
            get(get_ton_seqno).options(preflight_ok),
        )
        // ✅ 企业级标准：钱包批量创建端点（带CORS预检）
        .route(
            "/api/v1/wallets/batch",
            post(wallet_batch_create_api::batch_create_wallets).options(preflight_ok),
        )
        // ✅ Wallet Unlock API（钱包解锁双锁机制 - V1标准）
        .route(
            "/api/v1/wallets/unlock",
            post(wallet_unlock_api::unlock_wallet).options(preflight_ok),
        )
        .route(
            "/api/v1/wallets/lock",
            post(wallet_unlock_api::lock_wallet).options(preflight_ok),
        )
        .route(
            "/api/v1/wallets/:wallet_id/unlock-status",
            get(wallet_unlock_api::get_unlock_status).options(preflight_ok),
        )
        // Tenants API
        .route("/api/v1/tenants", post(create_tenant).get(list_tenants))
        .route(
            "/api/v1/tenants/:id",
            get(get_tenant).put(update_tenant).delete(delete_tenant),
        )
        // Users API
        .route("/api/v1/users", post(create_user).get(list_users))
        .route(
            "/api/v1/users/:id",
            get(get_user).put(update_user).delete(delete_user),
        )
        // ✅ 用户KYC状态查询API（需要认证）
        .route(
            "/api/v1/users/kyc/status",
            get(user_api::get_kyc_status).options(preflight_ok),
        )
        .route(
            "/api/v1/users/me",
            get(user_api::get_user_info).options(preflight_ok),
        )
        // Policies API
        .route("/api/v1/policies", post(create_policy).get(list_policies))
        .route(
            "/api/v1/policies/:id",
            get(get_policy).put(update_policy).delete(delete_policy),
        )
        // Approvals API
        .route(
            "/api/v1/approvals",
            post(create_approval).get(list_approvals),
        )
        .route(
            "/api/v1/approvals/:id",
            get(get_approval).delete(delete_approval),
        )
        .route("/api/v1/approvals/:id/status", put(update_approval_status))
        // API Keys API
        .route("/api/v1/api-keys", post(create_api_key).get(list_api_keys))
        .route(
            "/api/v1/api-keys/:id",
            get(get_api_key).delete(delete_api_key),
        )
        .route("/api/v1/api-keys/:id/status", put(update_api_key_status))
        // Tx Broadcasts API
        .route(
            "/api/v1/tx-broadcasts",
            post(create_tx_broadcast).get(list_tx_broadcasts),
        )
        .route(
            "/api/v1/tx-broadcasts/:id",
            get(get_tx_broadcast).put(update_tx_broadcast),
        )
        .route(
            "/api/v1/tx-broadcasts/by-tx-hash/:hash",
            get(get_tx_broadcast_by_tx_hash),
        )
        // Wallets API - 企业级标准：POST端点已完全移除，统一使用 /api/wallets/unified-create
        .route(
            "/api/v1/wallets",
            // ✅ 企业级标准：仅保留GET端点用于列表查询
            // POST端点已完全移除，请使用: POST /api/wallets/unified-create
            get(list_wallets).options(preflight_ok),
        )
        .route(
            "/api/v1/wallets/:id",
            get(get_wallet).delete(delete_wallet).options(preflight_ok),
        )
        // Transactions API
        .route("/api/v1/tx", post(create_tx).get(list_tx))
        .route("/api/v1/tx/:id", get(get_tx))
        .route(
            "/api/v1/tx/:id/status",
            axum::routing::put(update_tx_status),
        )
        .route("/api/v1/fees", get(api_fees).options(preflight_ok))
        .route(
            "/api/v1/network/status",
            get(api_network_status).options(preflight_ok),
        )
        .route("/api/v1/balance", get(balance).options(preflight_ok))
        // ✅ 企业级标准：资产聚合 API v1版本（需要认证）
        .route(
            "/api/v1/wallets/assets",
            get(asset_api::get_user_total_assets).options(preflight_ok),
        )
        .route(
            "/api/v1/wallets/:id/assets",
            get(asset_api::get_wallet_asset).options(preflight_ok),
        )
        // 通知系统 API（需要认证）
        .merge(notification_api::create_notification_routes())
        // ✅ 企业级标准：Bridge API v1版本（跨链转移相同代币到不同链）
        .route(
            "/api/v1/bridge/quote",
            axum::routing::get(bridge_api::get_bridge_quote).options(preflight_ok),
        )
        .route(
            "/api/v1/bridge/execute",
            axum::routing::post(bridge_enhanced_api::execute_bridge).options(preflight_ok),
        )
        .route(
            "/api/v1/bridge/:id/status",
            axum::routing::get(bridge_enhanced_api::get_bridge_status).options(preflight_ok),
        )
        // 管理员 API（需要管理员权限）
        .merge(admin_api::create_admin_routes())
        // ✅ 企业级标准：限价单 API v1版本
        .route(
            "/api/v1/limit-orders",
            axum::routing::get(limit_order_api::get_limit_order_list)
                .post(limit_order_api::create_limit_order)
                .options(preflight_ok),
        )
        .route(
            "/api/v1/limit-orders/:id",
            axum::routing::get(limit_order_api::get_limit_order)
                .delete(limit_order_api::cancel_limit_order)
                .options(preflight_ok),
        )
        // ✅ 企业级标准：法币订单API（需要认证）
        .route(
            "/api/v1/fiat/onramp/orders",
            axum::routing::get(fiat_api::list_onramp_orders)
                .post(fiat_api::create_fiat_order)
                .options(preflight_ok),
        )
        .route(
            "/api/v1/fiat/onramp/orders/:order_id",
            axum::routing::get(fiat_api::get_fiat_order_status).options(preflight_ok),
        )
        .route(
            "/api/v1/fiat/offramp/orders",
            axum::routing::get(fiat_api::list_offramp_orders)
                .post(fiat_api::create_withdraw_order)
                .options(preflight_ok),
        )
        .route(
            "/api/v1/fiat/offramp/orders/:order_id",
            axum::routing::get(fiat_api::get_withdraw_order_status).options(preflight_ok),
        )
        // 服务商管理 API（需要认证）
        .route(
            "/api/providers/:provider/countries",
            axum::routing::get(provider_api::get_provider_countries),
        )
        // 对账和监控 API（需要认证）
        .route(
            "/api/v1/reconciliation/daily",
            axum::routing::post(reconciliation_api::run_daily_reconciliation),
        )
        .route(
            "/api/v1/reconciliation/sync",
            axum::routing::post(reconciliation_api::sync_order_status),
        )
        .route(
            "/api/v1/reconciliation/history",
            axum::routing::get(reconciliation_api::get_alerts),
        )
        .route(
            "/api/v1/reconciliation/monitoring",
            axum::routing::get(reconciliation_api::get_alerts),
        )
        // 审计日志 API（需要认证）
        .route(
            "/api/v1/audit/logs",
            axum::routing::get(audit_api::get_audit_logs),
        )
        .route(
            "/api/v1/audit/compliance/report",
            axum::routing::get(audit_api::generate_compliance_report),
        )
        // ✅ 企业级标准 V1：提现 API（需要认证）
        .nest("/api/v1/withdrawals", withdrawal_api::routes())
        .nest("/api/v1/withdrawal/review", withdrawal_api::routes())
        // ✅ 企业级标准：订单管理API扩展（RESTful风格）
        .route(
            "/api/v1/fiat/onramp/orders/:order_id/cancel",
            axum::routing::post(fiat_api_cancel_retry::cancel_fiat_order),
        )
        .route(
            "/api/v1/fiat/onramp/orders/:order_id/retry",
            axum::routing::post(fiat_api_cancel_retry::retry_fiat_order),
        )
        .route(
            "/api/v1/fiat/offramp/orders/:order_id/cancel",
            axum::routing::post(fiat_api_cancel_retry::cancel_offramp_order),
        )
        .route(
            "/api/v1/fiat/offramp/orders/:order_id/retry",
            axum::routing::post(fiat_api_cancel_retry::retry_offramp_order),
        )
        // 注意：Gas API 已在 public_routes 中注册，避免重复
        // 注意：通配符 /*path 会干扰其他路由，已移除
        // CORS 预检由各路由的 .options() 处理
        // 应用幂等性检查中间件（在速率限制之前）
        // TODO: 修复幂等性中间件的类型问题
        // .layer(from_fn_with_state(state.clone(), idempotency_middleware))
        // 应用速率限制中间件
        .layer(from_fn_with_state(state.clone(), rate_limit_middleware))
        // 应用CSRF防护中间件（可选，如果需要CSRF保护）
        // 注意：Bearer Token已提供足够保护，CSRF是可选的
        // .layer(from_fn_with_state(state.clone(), csrf_middleware_with_state))
        // 应用认证中间件
        .layer(from_fn_with_state(
            state.clone(),
            middleware::jwt_extractor::jwt_extractor_middleware,
        ))
        // 安全与观测中间件
        .layer(
            ServiceBuilder::new()
                .layer(from_fn(middleware::method_whitelist_middleware)) // ✅ P0 Security: 最先应用
                .layer(from_fn(trace_id_middleware))
                .layer(from_fn(add_security_headers))
                .layer(from_fn(add_api_version_header))
                .layer(from_fn(add_cors_headers))
                .layer(from_fn(add_response_time_header))
                .layer(from_fn(trace_log))
                .layer(from_fn(set_request_id)),
        )
        .with_state(state.clone());

    // ✅ 合并所有路由：public_routes（含健康检查）+ protected_routes
    public_routes.merge(protected_routes).with_state(state)
}

async fn preflight_ok(headers: axum::http::HeaderMap) -> Response {
    // IMPORTANT: 浏览器会先发 OPTIONS 预检。
    // 旧实现固定返回 allow-origins 的第一个值（默认 localhost），会导致生产前端跨域预检失败。

    let origin = headers
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_default();

    let requested_headers = headers
        .get("access-control-request-headers")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    // 🔧 开发环境：允许localhost和127.0.0.1；生产环境：建议显式配置 CORS_ALLOW_ORIGINS
    let allow_origins = std::env::var("CORS_ALLOW_ORIGINS").unwrap_or_else(|_| {
        "http://localhost:8080,http://127.0.0.1:8080,http://localhost:8081,http://127.0.0.1:8081".into()
    });

    let allowed_origin = if allow_origins.trim() == "*" {
        "*".to_string()
    } else if !origin.is_empty()
        && allow_origins
            .split(',')
            .any(|allowed| allowed.trim() == origin)
    {
        origin
    } else if !origin.is_empty() {
        // 兼容当前策略：存在显式 Origin 时，先放行（避免运营误配导致全站不可用）
        // 如需严格限制，把此分支移除即可。
        origin
    } else {
        allow_origins
            .split(',')
            .next()
            .unwrap_or("*")
            .trim()
            .to_string()
    };

    let mut resp = StatusCode::OK.into_response();
    let resp_headers = resp.headers_mut();

    if let Ok(val) = HeaderValue::from_str(&allowed_origin) {
        resp_headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, val);
    } else {
        resp_headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    }

    resp_headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET,POST,PUT,DELETE,OPTIONS"),
    );

    if let Some(req_hdrs) = requested_headers {
        if let Ok(val) = HeaderValue::from_str(&req_hdrs) {
            resp_headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, val);
        } else {
            resp_headers.insert(
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static(
                    "Content-Type, Authorization, Idempotency-Key, X-Request-Id, X-Platform, x-platform, X-Client-Version, Accept-Language",
                ),
            );
        }
    } else {
        resp_headers.insert(
            ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static(
                "Content-Type, Authorization, Idempotency-Key, X-Request-Id, X-Platform, x-platform, X-Client-Version, Accept-Language",
            ),
        );
    }

    resp_headers.insert(
        ACCESS_CONTROL_ALLOW_CREDENTIALS,
        HeaderValue::from_static("false"),
    );

    resp_headers.insert(ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("600"));
    resp
}

async fn cors_preflight_middleware(req: Request, next: axum::middleware::Next) -> Response {
    if req.method() == axum::http::Method::OPTIONS {
        let origin = req
            .headers()
            .get("origin")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();

        let requested_headers = req
            .headers()
            .get("access-control-request-headers")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let allow_origins = std::env::var("CORS_ALLOW_ORIGINS").unwrap_or_else(|_| {
            "http://localhost:8080,http://127.0.0.1:8080,http://localhost:8081,http://127.0.0.1:8081".into()
        });

        let allowed_origin = if allow_origins == "*" {
            "*".to_string()
        } else if !origin.is_empty()
            && allow_origins
                .split(',')
                .any(|allowed| allowed.trim() == origin)
        {
            origin.clone()
        } else if !origin.is_empty() {
            // Keep permissive behavior for now: allow any explicit Origin.
            origin.clone()
        } else {
            allow_origins.split(',').next().unwrap_or("*").to_string()
        };

        let mut resp = StatusCode::OK.into_response();
        let headers = resp.headers_mut();

        if let Ok(val) = HeaderValue::from_str(&allowed_origin) {
            headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, val);
        } else {
            headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
        }

        headers.insert(
            ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET,POST,PUT,DELETE,OPTIONS"),
        );

        if let Some(req_hdrs) = requested_headers {
            if let Ok(val) = HeaderValue::from_str(&req_hdrs) {
                headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, val);
            } else {
                headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type, Authorization, Idempotency-Key, X-Request-Id, X-Platform, x-platform, X-Client-Version, Accept-Language"));
            }
        } else {
            headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type, Authorization, Idempotency-Key, X-Request-Id, X-Platform, x-platform, X-Client-Version, Accept-Language"));
        }

        headers.insert(
            ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("false"),
        );

        return resp;
    }

    next.run(req).await
}

async fn add_cors_headers(req: Request, next: axum::middleware::Next) -> Response {
    // 🔧 获取请求来源，动态返回对应的CORS头（需要clone，因为req会被移动）
    let origin = req
        .headers()
        .get("origin")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_default();

    tracing::info!(
        "🌐 CORS middleware - origin: '{}', path: {}",
        origin,
        req.uri().path()
    );

    // Capture requested headers for preflight reflection
    let requested_headers = req
        .headers()
        .get("access-control-request-headers")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();

    // 🔧 允许的origins列表（开发环境默认值）
    let allow_origins = std::env::var("CORS_ALLOW_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:8080,http://127.0.0.1:8080,http://localhost:8081,http://127.0.0.1:8081".into());

    // ✅ 修复：检查请求origin是否在允许列表中，否则使用origin本身（开发环境）
    let allowed_origin = if allow_origins == "*" {
        "*".to_string()
    } else if !origin.is_empty()
        && allow_origins
            .split(',')
            .any(|allowed| allowed.trim() == origin)
    {
        origin.clone() // 返回实际请求的origin
    } else if !origin.is_empty() {
        // ✅ 开发环境：如果origin不在列表但存在，也允许（用于本地开发）
        origin.clone()
    } else {
        // 无origin时使用第一个配置的origin
        allow_origins.split(',').next().unwrap_or("*").to_string()
    };

    // ✅ 修复：确保CORS头正确设置
    if let Ok(val) = HeaderValue::from_str(&allowed_origin) {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, val);
    } else {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    }
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET,POST,PUT,DELETE,OPTIONS"),
    );
    // If browser sent Access-Control-Request-Headers, reflect it back to allow custom headers
    // (e.g., X-Platform)
    if let Some(req_hdrs) = requested_headers {
        if let Ok(val) = HeaderValue::from_str(&req_hdrs) {
            headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, val);
        } else {
            headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type, Authorization, Idempotency-Key, X-Request-Id, X-Platform, x-platform, X-Client-Version, Accept-Language"));
        }
    } else {
        headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Content-Type, Authorization, Idempotency-Key, X-Request-Id, X-Platform, x-platform, X-Client-Version, Accept-Language"));
    }
    headers.insert(
        ACCESS_CONTROL_ALLOW_CREDENTIALS,
        HeaderValue::from_static("false"),
    );
    resp
}

async fn add_security_headers(req: Request, next: axum::middleware::Next) -> Response {
    let path = req.uri().path().to_string();
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();

    // 调试日志
    tracing::debug!("🔒 add_security_headers called for path: {}", path);

    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(PRAGMA, HeaderValue::from_static("no-cache"));
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'self'"),
    );
    // HSTS 仅在 HTTPS 部署时启用：通过环境变量控制（HSTS_ENABLE=1）
    if std::env::var("HSTS_ENABLE")
        .ok()
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        headers.insert(
            "strict-transport-security",
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    resp
}

async fn set_request_id(mut req: Request, next: axum::middleware::Next) -> Response {
    let req_id = Uuid::new_v4().to_string();
    let trace_id = crate::utils::get_or_generate_trace_id(Some(&req_id));

    // 将追踪ID注入到请求扩展中
    req.extensions_mut().insert(trace_id.clone());

    // ✅ 修复：将X-Request-ID添加到请求头（用于日志追踪）
    req.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&req_id).unwrap_or(HeaderValue::from_static("gen-failed")),
    );
    let mut resp = next.run(req).await;

    // ✅ 修复：确保X-Request-ID返回给客户端
    resp.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&req_id).unwrap_or(HeaderValue::from_static("gen-failed")),
    );
    resp.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&req_id).unwrap_or(HeaderValue::from_static("gen-failed")),
    );
    resp.headers_mut().insert(
        "x-trace-id",
        HeaderValue::from_str(&trace_id).unwrap_or(HeaderValue::from_static("gen-failed")),
    );
    resp
}

async fn add_api_version_header(req: Request, next: axum::middleware::Next) -> Response {
    let mut resp = next.run(req).await;
    resp.headers_mut()
        .insert("x-api-version", HeaderValue::from_static("v1"));
    resp
}

async fn add_response_time_header(req: Request, next: axum::middleware::Next) -> Response {
    let start = Instant::now();
    let mut resp = next.run(req).await;
    let elapsed_ms = start.elapsed().as_millis().to_string();
    resp.headers_mut().insert(
        "x-response-time",
        HeaderValue::from_str(&format!("{}ms", elapsed_ms))
            .unwrap_or(HeaderValue::from_static("0ms")),
    );
    resp
}

async fn trace_log(req: Request, next: axum::middleware::Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();
    let req_id = req
        .headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let resp = next.run(req).await;
    let status = resp.status();
    let elapsed = start.elapsed().as_millis();
    tracing::event!(Level::INFO, request_id=%req_id, method=%method, path=%path, status=%status.as_u16(), elapsed_ms=%elapsed, "http_request");
    resp
}
