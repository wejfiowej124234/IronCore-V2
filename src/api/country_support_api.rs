//! Country Support API - 国家支持查询
//! 查询不同服务商对各国家的支持情况

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::{
    api::response::success_response,
    app_state::AppState,
    error::AppError,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Request/Response Models
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CountrySupportInfo {
    pub country_code: String,
    pub country_name: String,
    pub supported_providers: Vec<String>,
    pub unsupported_providers: Vec<String>,
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProviderCountrySupport {
    pub provider: String,
    pub supported_countries: Vec<String>,
    pub unsupported_countries: Vec<String>,
    pub last_synced: String,
}

#[derive(Debug, Deserialize)]
pub struct SyncQuery {
    pub provider: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SyncResult {
    pub provider: String,
    pub synced_countries: u64,
    pub updated_countries: u64,
    pub sync_time: String,
    pub errors: Vec<String>,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Routes
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:country_code", get(get_country_support))
        .route("/provider/:provider", get(get_provider_support_by_name))
        .route("/providers", get(list_all_providers))
        .route("/sync", get(sync_country_support))
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handlers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// GET /api/v1/country-support/:country_code
#[utoipa::path(
    get,
    path = "/api/v1/country-support/{country_code}",
    params(
        ("country_code" = String, Path, description = "ISO 3166-1 alpha-2 country code")
    ),
    responses(
        (status = 200, description = "Country support info", body = crate::api::response::ApiResponse<CountrySupportInfo>)
    ),
    tag = "CountrySupport"
)]
pub async fn get_country_support(
    State(_state): State<Arc<AppState>>,
    Path(country_code): Path<String>,
) -> Result<Json<crate::api::response::ApiResponse<CountrySupportInfo>>, AppError> {
    // TODO: 实际实现应从数据库查询
    let info = CountrySupportInfo {
        country_code: country_code.to_uppercase(),
        country_name: "Country Name".to_string(),
        supported_providers: vec!["stripe".to_string(), "paypal".to_string()],
        unsupported_providers: vec![],
        last_updated: chrono::Utc::now().to_rfc3339(),
    };

    success_response(info)
}

/// GET /api/v1/country-support/provider/:provider
#[utoipa::path(
    get,
    path = "/api/v1/country-support/provider/{provider}",
    params(
        ("provider" = String, Path, description = "Provider name")
    ),
    responses(
        (status = 200, description = "Provider country support", body = crate::api::response::ApiResponse<ProviderCountrySupport>)
    ),
    tag = "CountrySupport"
)]
pub async fn get_provider_support_by_name(
    State(_state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Result<Json<crate::api::response::ApiResponse<ProviderCountrySupport>>, AppError> {
    // TODO: 实际实现应从数据库查询
    let support = ProviderCountrySupport {
        provider,
        supported_countries: vec!["US".to_string(), "GB".to_string(), "CN".to_string()],
        unsupported_countries: vec![],
        last_synced: chrono::Utc::now().to_rfc3339(),
    };

    success_response(support)
}

/// GET /api/v1/country-support/providers
#[utoipa::path(
    get,
    path = "/api/v1/country-support/providers",
    responses(
        (status = 200, description = "All providers support", body = crate::api::response::ApiResponse<Vec<ProviderCountrySupport>>)
    ),
    tag = "CountrySupport"
)]
pub async fn list_all_providers(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<crate::api::response::ApiResponse<Vec<ProviderCountrySupport>>>, AppError> {
    // TODO: 实际实现应从数据库查询所有服务商
    let providers = vec![
        ProviderCountrySupport {
            provider: "stripe".to_string(),
            supported_countries: vec!["US".to_string(), "GB".to_string()],
            unsupported_countries: vec![],
            last_synced: chrono::Utc::now().to_rfc3339(),
        },
        ProviderCountrySupport {
            provider: "paypal".to_string(),
            supported_countries: vec!["US".to_string(), "CN".to_string()],
            unsupported_countries: vec![],
            last_synced: chrono::Utc::now().to_rfc3339(),
        },
    ];

    success_response(providers)
}

/// GET /api/v1/country-support/sync
#[utoipa::path(
    get,
    path = "/api/v1/country-support/sync",
    params(
        ("provider" = Option<String>, Query, description = "Optional provider to sync")
    ),
    responses(
        (status = 200, description = "Sync result", body = crate::api::response::ApiResponse<SyncResult>)
    ),
    tag = "CountrySupport"
)]
pub async fn sync_country_support(
    State(_state): State<Arc<AppState>>,
    Query(query): Query<SyncQuery>,
) -> Result<Json<crate::api::response::ApiResponse<SyncResult>>, AppError> {
    // TODO: 实际实现应触发同步任务
    let result = SyncResult {
        provider: query.provider.unwrap_or_else(|| "all".to_string()),
        synced_countries: 100,
        updated_countries: 5,
        sync_time: chrono::Utc::now().to_rfc3339(),
        errors: vec![],
    };

    success_response(result)
}
