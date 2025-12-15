// 管理员 API - 生产级实现
// 包含费率规则、归集地址、RPC端点的完整CRUD操作

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{
        middleware::{auth::AuthInfoExtractor, rbac::require_admin},
        response::success_response,
    },
    app_state::AppState,
    error::AppError,
};

// ============ 费率规则 CRUD ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateFeeRuleReq {
    pub chain: String,
    pub operation: String,
    pub fee_type: String, // flat, percent, mixed
    pub flat_amount: Option<f64>,
    pub percent_bp: Option<i32>, // 基点 (1bp = 0.01%)
    pub min_fee: Option<f64>,
    pub max_fee: Option<f64>,
    pub priority: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateFeeRuleReq {
    pub flat_amount: Option<f64>,
    pub percent_bp: Option<i32>,
    pub min_fee: Option<f64>,
    pub max_fee: Option<f64>,
    pub priority: Option<i32>,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeeRuleResp {
    pub id: Uuid,
    pub chain: String,
    pub operation: String,
    pub fee_type: String,
    pub flat_amount: Option<f64>,
    pub percent_bp: Option<i32>,
    pub min_fee: Option<f64>,
    pub max_fee: Option<f64>,
    pub priority: i32,
    pub rule_version: i32,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建费率规则
#[utoipa::path(
    post,
    path = "/api/admin/fee-rules",
    request_body = CreateFeeRuleReq,
    responses(
        (status = 201, description = "Rule created", body = FeeRuleResp),
        (status = 403, description = "Forbidden"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_fee_rule(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<CreateFeeRuleReq>,
) -> Result<Json<crate::api::response::ApiResponse<FeeRuleResp>>, AppError> {
    require_admin(&auth)?;

    // 获取最新版本号
    let max_version: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(rule_version) FROM gas.platform_fee_rules WHERE chain = $1 AND operation = $2",
    )
    .bind(&req.chain)
    .bind(&req.operation)
    .fetch_optional(&st.pool)
    .await?;

    let new_version = max_version.unwrap_or(0) + 1;

    let rule_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO gas.platform_fee_rules 
         (id, chain, operation, fee_type, flat_amount, percent_bp, min_fee, max_fee, priority, rule_version, active)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true)"
    )
    .bind(rule_id)
    .bind(&req.chain)
    .bind(&req.operation)
    .bind(&req.fee_type)
    .bind(req.flat_amount)
    .bind(req.percent_bp)
    .bind(req.min_fee)
    .bind(req.max_fee)
    .bind(req.priority)
    .bind(new_version)
    .execute(&st.pool)
    .await?;

    // 记录管理操作审计
    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "create_fee_rule",
        &rule_id.to_string(),
        &format!(
            "chain={},operation={},version={}",
            req.chain, req.operation, new_version
        ),
    )
    .await?;

    // 企业级实现：清除相关缓存，确保新规则立即生效
    st.fee_service
        .invalidate_cache(&req.chain, &req.operation)
        .await;
    tracing::info!(
        "费用规则已创建并清除缓存: chain={}, operation={}, rule_id={}",
        req.chain,
        req.operation,
        rule_id
    );

    // 返回创建的规则
    let rule = get_fee_rule_by_id(&st.pool, rule_id).await?;
    success_response(rule)
}

/// 更新费率规则
#[utoipa::path(
    put,
    path = "/api/admin/fee-rules/{id}",
    request_body = UpdateFeeRuleReq,
    responses(
        (status = 200, description = "Rule updated", body = FeeRuleResp),
        (status = 404, description = "Rule not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_fee_rule(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFeeRuleReq>,
) -> Result<Json<crate::api::response::ApiResponse<FeeRuleResp>>, AppError> {
    require_admin(&auth)?;

    // 构建动态更新语句
    let mut updates = Vec::new();
    let mut bind_count = 1;

    let mut query_str = "UPDATE gas.platform_fee_rules SET ".to_string();

    if req.flat_amount.is_some() {
        updates.push(format!("flat_amount = ${}", bind_count));
        bind_count += 1;
    }
    if req.percent_bp.is_some() {
        updates.push(format!("percent_bp = ${}", bind_count));
        bind_count += 1;
    }
    if req.min_fee.is_some() {
        updates.push(format!("min_fee = ${}", bind_count));
        bind_count += 1;
    }
    if req.max_fee.is_some() {
        updates.push(format!("max_fee = ${}", bind_count));
        bind_count += 1;
    }
    if req.priority.is_some() {
        updates.push(format!("priority = ${}", bind_count));
        bind_count += 1;
    }
    if req.active.is_some() {
        updates.push(format!("active = ${}", bind_count));
        bind_count += 1;
    }

    updates.push("updated_at = CURRENT_TIMESTAMP".to_string());

    query_str.push_str(&updates.join(", "));
    query_str.push_str(&format!(" WHERE id = ${}", bind_count));

    let mut query = sqlx::query(&query_str);

    if let Some(v) = req.flat_amount {
        query = query.bind(v);
    }
    if let Some(v) = req.percent_bp {
        query = query.bind(v);
    }
    if let Some(v) = req.min_fee {
        query = query.bind(v);
    }
    if let Some(v) = req.max_fee {
        query = query.bind(v);
    }
    if let Some(v) = req.priority {
        query = query.bind(v);
    }
    if let Some(v) = req.active {
        query = query.bind(v);
    }

    query = query.bind(id);

    let result = query.execute(&st.pool).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Fee rule not found"));
    }

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "update_fee_rule",
        &id.to_string(),
        &serde_json::to_string(&req).unwrap_or_default(),
    )
    .await?;

    // 企业级实现：获取链和操作类型，然后清除相关缓存
    let rule = get_fee_rule_by_id(&st.pool, id).await?;
    st.fee_service
        .invalidate_cache(&rule.chain, &rule.operation)
        .await;
    tracing::info!(
        "费用规则已更新并清除缓存: chain={}, operation={}, rule_id={}",
        rule.chain,
        rule.operation,
        id
    );

    success_response(rule)
}

/// 查询所有费率规则
#[utoipa::path(
    get,
    path = "/api/admin/fee-rules",
    responses(
        (status = 200, description = "Rules list", body = Vec<FeeRuleResp>),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_fee_rules(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
) -> Result<Json<crate::api::response::ApiResponse<Vec<FeeRuleResp>>>, AppError> {
    require_admin(&auth)?;

    let rules = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            Option<f64>,
            Option<i32>,
            Option<f64>,
            Option<f64>,
            i32,
            i32,
            bool,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, chain, operation, fee_type, flat_amount, percent_bp, min_fee, max_fee, 
                priority, rule_version, active, created_at, updated_at
         FROM gas.platform_fee_rules
         ORDER BY chain, operation, priority DESC",
    )
    .fetch_all(&st.pool)
    .await?;

    let resp = rules
        .into_iter()
        .map(
            |(
                id,
                chain,
                operation,
                fee_type,
                flat_amount,
                percent_bp,
                min_fee,
                max_fee,
                priority,
                rule_version,
                active,
                created_at,
                updated_at,
            )| {
                FeeRuleResp {
                    id,
                    chain,
                    operation,
                    fee_type,
                    flat_amount,
                    percent_bp,
                    min_fee,
                    max_fee,
                    priority,
                    rule_version,
                    active,
                    created_at: created_at.to_rfc3339(),
                    updated_at: updated_at.to_rfc3339(),
                }
            },
        )
        .collect();

    success_response(resp)
}

/// 删除费率规则（软删除）
#[utoipa::path(
    delete,
    path = "/api/admin/fee-rules/{id}",
    responses(
        (status = 204, description = "Rule deleted"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_fee_rule(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    require_admin(&auth)?;

    let result = sqlx::query(
        "UPDATE gas.platform_fee_rules SET active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1"
    )
    .bind(id)
    .execute(&st.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Fee rule not found"));
    }

    // 企业级实现：获取链和操作类型，然后清除相关缓存
    // 注意：需要在删除前获取规则信息，因为删除后无法获取
    let rule_opt = get_fee_rule_by_id(&st.pool, id).await.ok();
    if let Some(rule) = rule_opt {
        st.fee_service
            .invalidate_cache(&rule.chain, &rule.operation)
            .await;
        tracing::info!(
            "费用规则已删除并清除缓存: chain={}, operation={}, rule_id={}",
            rule.chain,
            rule.operation,
            id
        );
    }

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "delete_fee_rule",
        &id.to_string(),
        "soft_delete",
    )
    .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============ 归集地址管理 ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCollectorAddressReq {
    pub chain: String,
    pub address: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CollectorAddressResp {
    pub id: Uuid,
    pub chain: String,
    pub address: String,
    pub active: bool,
    pub created_at: String,
}

/// 添加归集地址
#[utoipa::path(
    post,
    path = "/api/admin/collector-addresses",
    request_body = CreateCollectorAddressReq,
    responses(
        (status = 201, description = "Address created", body = CollectorAddressResp),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_collector_address(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<CreateCollectorAddressReq>,
) -> Result<Json<crate::api::response::ApiResponse<CollectorAddressResp>>, AppError> {
    require_admin(&auth)?;

    let addr_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO gas.fee_collector_addresses (id, chain, address, active)
         VALUES ($1, $2, $3, true)",
    )
    .bind(addr_id)
    .bind(&req.chain)
    .bind(&req.address)
    .execute(&st.pool)
    .await?;

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "create_collector_address",
        &addr_id.to_string(),
        &format!("chain={},address={}", req.chain, req.address),
    )
    .await?;

    let addr = get_collector_address_by_id(&st.pool, addr_id).await?;
    success_response(addr)
}

/// 激活/停用归集地址
#[utoipa::path(
    put,
    path = "/api/admin/collector-addresses/{id}/activate",
    responses(
        (status = 200, description = "Address updated", body = CollectorAddressResp),
    ),
    security(("bearer_auth" = []))
)]
pub async fn toggle_collector_address(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<crate::api::response::ApiResponse<CollectorAddressResp>>, AppError> {
    require_admin(&auth)?;

    sqlx::query(
        "UPDATE gas.fee_collector_addresses 
         SET active = NOT active, updated_at = CURRENT_TIMESTAMP 
         WHERE id = $1",
    )
    .bind(id)
    .execute(&st.pool)
    .await?;

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "toggle_collector_address",
        &id.to_string(),
        "toggle_active",
    )
    .await?;

    let addr = get_collector_address_by_id(&st.pool, id).await?;
    success_response(addr)
}

// ============ RPC 端点管理 ============

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRpcEndpointReq {
    pub chain: String,
    pub url: String,
    pub priority: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRpcEndpointReq {
    pub priority: Option<i32>,
    pub healthy: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RpcEndpointResp {
    pub id: Uuid,
    pub chain: String,
    pub url: String,
    pub priority: i32,
    pub healthy: bool,
    pub circuit_state: String,
    pub created_at: String,
}

/// 创建 RPC 端点
#[utoipa::path(
    post,
    path = "/api/admin/rpc-endpoints",
    request_body = CreateRpcEndpointReq,
    responses(
        (status = 201, description = "Endpoint created", body = RpcEndpointResp),
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_rpc_endpoint(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<CreateRpcEndpointReq>,
) -> Result<Json<crate::api::response::ApiResponse<RpcEndpointResp>>, AppError> {
    require_admin(&auth)?;

    let endpoint_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO admin.rpc_endpoints (id, chain, url, priority, healthy, circuit_state)
         VALUES ($1, $2, $3, $4, true, 'closed')",
    )
    .bind(endpoint_id)
    .bind(&req.chain)
    .bind(&req.url)
    .bind(req.priority)
    .execute(&st.pool)
    .await?;

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "create_rpc_endpoint",
        &endpoint_id.to_string(),
        &format!("chain={},url={}", req.chain, req.url),
    )
    .await?;

    let endpoint = get_rpc_endpoint_by_id(&st.pool, endpoint_id).await?;
    success_response(endpoint)
}

/// 更新 RPC 端点
#[utoipa::path(
    put,
    path = "/api/admin/rpc-endpoints/{id}",
    request_body = UpdateRpcEndpointReq,
    responses(
        (status = 200, description = "Endpoint updated", body = RpcEndpointResp),
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_rpc_endpoint(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateRpcEndpointReq>,
) -> Result<Json<crate::api::response::ApiResponse<RpcEndpointResp>>, AppError> {
    require_admin(&auth)?;

    // 构建动态更新语句
    let mut updates = Vec::new();
    let mut bind_idx = 1;

    if req.priority.is_some() {
        updates.push(format!("priority = ${}", bind_idx));
        bind_idx += 1;
    }
    if req.healthy.is_some() {
        updates.push(format!("healthy = ${}", bind_idx));
        bind_idx += 1;
    }

    updates.push("updated_at = CURRENT_TIMESTAMP".to_string());

    let query_str = format!(
        "UPDATE admin.rpc_endpoints SET {} WHERE id = ${}",
        updates.join(", "),
        bind_idx
    );

    let mut query = sqlx::query(&query_str);

    if let Some(v) = req.priority {
        query = query.bind(v);
    }
    if let Some(v) = req.healthy {
        query = query.bind(v);
    }

    query = query.bind(id);
    let result = query.execute(&st.pool).await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("RPC endpoint not found"));
    }

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "update_rpc_endpoint",
        &id.to_string(),
        &serde_json::to_string(&req).unwrap_or_default(),
    )
    .await?;

    let endpoint = get_rpc_endpoint_by_id(&st.pool, id).await?;
    success_response(endpoint)
}

/// 删除 RPC 端点
#[utoipa::path(
    delete,
    path = "/api/admin/rpc-endpoints/{id}",
    responses(
        (status = 204, description = "Endpoint deleted"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_rpc_endpoint(
    State(st): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    require_admin(&auth)?;

    let result = sqlx::query("DELETE FROM admin.rpc_endpoints WHERE id = $1")
        .bind(id)
        .execute(&st.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("RPC endpoint not found"));
    }

    record_admin_operation(
        &st.pool,
        auth.user_id,
        &auth.role,
        "delete_rpc_endpoint",
        &id.to_string(),
        "hard_delete",
    )
    .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

// ============ 辅助函数 ============

async fn get_fee_rule_by_id(pool: &sqlx::PgPool, id: Uuid) -> Result<FeeRuleResp, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            String,
            Option<f64>,
            Option<i32>,
            Option<f64>,
            Option<f64>,
            i32,
            i32,
            bool,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, chain, operation, fee_type, flat_amount, percent_bp, min_fee, max_fee,
                priority, rule_version, active, created_at, updated_at
         FROM gas.platform_fee_rules WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::not_found("Fee rule not found"))?;

    Ok(FeeRuleResp {
        id: row.0,
        chain: row.1,
        operation: row.2,
        fee_type: row.3,
        flat_amount: row.4,
        percent_bp: row.5,
        min_fee: row.6,
        max_fee: row.7,
        priority: row.8,
        rule_version: row.9,
        active: row.10,
        created_at: row.11.to_rfc3339(),
        updated_at: row.12.to_rfc3339(),
    })
}

async fn get_collector_address_by_id(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Result<CollectorAddressResp, AppError> {
    let row = sqlx::query_as::<_, (Uuid, String, String, bool, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, chain, address, active, created_at
         FROM gas.fee_collector_addresses WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::not_found("Collector address not found"))?;

    Ok(CollectorAddressResp {
        id: row.0,
        chain: row.1,
        address: row.2,
        active: row.3,
        created_at: row.4.to_rfc3339(),
    })
}

async fn get_rpc_endpoint_by_id(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Result<RpcEndpointResp, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            Uuid,
            String,
            String,
            i32,
            bool,
            String,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        "SELECT id, chain, url, priority, healthy, circuit_state, created_at
         FROM admin.rpc_endpoints WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::not_found("RPC endpoint not found"))?;

    Ok(RpcEndpointResp {
        id: row.0,
        chain: row.1,
        url: row.2,
        priority: row.3,
        healthy: row.4,
        circuit_state: row.5,
        created_at: row.6.to_rfc3339(),
    })
}

async fn record_admin_operation(
    pool: &sqlx::PgPool,
    operator_user_id: Uuid,
    role: &str,
    action: &str,
    target_ref: &str,
    payload_hash: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO admin.admin_operation_log (operator_user_id, role, action, target_ref, payload_hash)
         VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(operator_user_id)
    .bind(role)
    .bind(action)
    .bind(target_ref)
    .bind(payload_hash)
    .execute(pool)
    .await?;

    Ok(())
}

// ============ 路由配置（✅ 企业级标准 V1）============

pub fn create_admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        // ✅ V1: 费率规则
        .route(
            "/api/v1/admin/fee-rules",
            post(create_fee_rule).get(list_fee_rules),
        )
        .route(
            "/api/v1/admin/fee-rules/:id",
            put(update_fee_rule).delete(delete_fee_rule),
        )
        // ✅ V1: 归集地址
        .route(
            "/api/v1/admin/collector-addresses",
            post(create_collector_address),
        )
        .route(
            "/api/v1/admin/collector-addresses/:id/activate",
            put(toggle_collector_address),
        )
        // ✅ V1: RPC 端点
        .route("/api/v1/admin/rpc-endpoints", post(create_rpc_endpoint))
        .route(
            "/api/v1/admin/rpc-endpoints/:id",
            put(update_rpc_endpoint).delete(delete_rpc_endpoint),
        )
}

// Alias for consistency
pub fn routes() -> Router<Arc<AppState>> {
    create_admin_routes()
}
