//! 服务商管理API
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::{
    api::response::{convert_error, success_response},
    app_state::AppState,
    error::AppError,
    service::provider_service::ProviderService,
};

/// GET /api/providers - 获取服务商列表
#[derive(Debug, Serialize)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderResponse>,
}

#[derive(Debug, Serialize)]
pub struct ProviderResponse {
    pub name: String,
    pub display_name: String,
    pub is_enabled: bool,
    pub priority: i64,
    pub fee_min_percent: f64,
    pub fee_max_percent: f64,
    pub supported_countries: Vec<String>,
    pub health_status: String,
    pub response_time_ms: Option<i64>,
    pub success_rate: f64,
}

pub async fn get_providers(
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<crate::api::response::ApiResponse<ProvidersResponse>>, AppError> {
    let provider_service = ProviderService::new(state.pool.clone());

    let providers = provider_service
        .get_enabled_providers()
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let provider_responses: Vec<ProviderResponse> = providers
        .into_iter()
        .map(|p| {
            let success_rate = if p.total_requests > 0 {
                (p.successful_requests as f64 / p.total_requests as f64) * 100.0
            } else {
                100.0
            };

            ProviderResponse {
                name: p.name,
                display_name: p.display_name,
                is_enabled: p.is_enabled,
                priority: p.priority,
                fee_min_percent: p.fee_min_percent.to_string().parse().unwrap_or(0.0),
                fee_max_percent: p.fee_max_percent.to_string().parse().unwrap_or(0.0),
                supported_countries: p.supported_countries,
                health_status: p.health_status,
                response_time_ms: p.average_response_time_ms,
                success_rate,
            }
        })
        .collect();

    success_response(ProvidersResponse {
        providers: provider_responses,
    })
}

/// GET /api/providers/country-support - 检查国家支持
#[derive(Debug, Deserialize)]
pub struct CountrySupportQuery {
    pub country: String,
}

#[derive(Debug, Serialize)]
pub struct CountrySupportResponse {
    pub country: String,
    pub supported_providers: Vec<String>,
    pub unsupported_providers: Vec<String>,
}

pub async fn check_country_support(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CountrySupportQuery>,
) -> Result<axum::Json<crate::api::response::ApiResponse<CountrySupportResponse>>, AppError> {
    let provider_service = ProviderService::new(state.pool.clone());

    let providers = provider_service
        .get_enabled_providers()
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut supported = Vec::new();
    let mut unsupported = Vec::new();

    for provider in &providers {
        let is_supported = provider_service
            .check_country_support(&provider.name, &query.country)
            .await
            .unwrap_or(false);

        if is_supported {
            supported.push(provider.name.clone());
        } else {
            unsupported.push(provider.name.clone());
        }
    }

    success_response(CountrySupportResponse {
        country: query.country,
        supported_providers: supported,
        unsupported_providers: unsupported,
    })
}

/// GET /api/providers/:provider/countries - 获取服务商支持的国家列表
#[derive(Debug, Serialize)]
pub struct ProviderCountriesResponse {
    pub provider: String,
    pub supported_countries: Vec<String>,
    pub last_updated: String,
}

pub async fn get_provider_countries(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(provider_name): axum::extract::Path<String>,
) -> Result<axum::Json<crate::api::response::ApiResponse<ProviderCountriesResponse>>, AppError> {
    let provider_service = ProviderService::new(state.pool.clone());

    let countries = provider_service
        .get_supported_countries(&provider_name)
        .await
        .map_err(|e| convert_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    success_response(ProviderCountriesResponse {
        provider: provider_name,
        supported_countries: countries,
        last_updated: chrono::Utc::now().to_rfc3339(),
    })
}
