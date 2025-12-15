//! Public Configuration API
//! 前端获取后端公共配置（token过期时间、服务器时间等）

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{
    api::response::{success_response, ApiResponse},
    app_state::AppState,
};

/// 公共配置响应（前端可见）
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct PublicConfigResponse {
    /// JWT Token过期时间（秒）
    pub token_expiry_secs: u64,

    /// Refresh Token过期时间（秒）
    pub refresh_token_expiry_secs: u64,

    /// 服务器当前时间戳（秒）- 用于前后端时钟同步
    pub server_time: i64,

    /// API版本
    pub api_version: String,

    /// 支持的区块链网络
    pub supported_chains: Vec<ChainInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ChainInfo {
    pub chain_id: i32,
    pub symbol: String,
    pub name: String,
}

/// GET /api/v1/config/public - 获取公共配置
///
/// **无需认证**：此接口公开可访问，前端可在登录前调用
///
/// 用途：
/// - 前端同步token过期时间配置
/// - 前后端时钟校准（防止客户端时间不准导致token验证失败）
/// - 获取支持的区块链列表
#[utoipa::path(
    get,
    path = "/api/v1/config/public",
    tag = "Config",
    responses(
        (status = 200, description = "返回公共配置", body = PublicConfigResponse),
    )
)]
pub async fn get_public_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ApiResponse<PublicConfigResponse>>, crate::error::AppError> {
    let now = chrono::Utc::now().timestamp();

    // 从配置中读取JWT配置
    let token_expiry = state.config.jwt.token_expiry_secs;
    let refresh_expiry = state.config.jwt.refresh_token_expiry_secs;

    // 构造支持的区块链列表
    let supported_chains = vec![
        ChainInfo {
            chain_id: 1,
            symbol: "ETH".to_string(),
            name: "Ethereum Mainnet".to_string(),
        },
        ChainInfo {
            chain_id: 11155111,
            symbol: "ETH".to_string(),
            name: "Ethereum Sepolia Testnet".to_string(),
        },
        ChainInfo {
            chain_id: 56,
            symbol: "BNB".to_string(),
            name: "BSC Mainnet".to_string(),
        },
        ChainInfo {
            chain_id: 97,
            symbol: "BNB".to_string(),
            name: "BSC Testnet".to_string(),
        },
        ChainInfo {
            chain_id: 137,
            symbol: "MATIC".to_string(),
            name: "Polygon Mainnet".to_string(),
        },
        ChainInfo {
            chain_id: 80002,
            symbol: "MATIC".to_string(),
            name: "Polygon Amoy Testnet".to_string(),
        },
    ];

    let response = PublicConfigResponse {
        token_expiry_secs: token_expiry,
        refresh_token_expiry_secs: refresh_expiry,
        server_time: now,
        api_version: env!("CARGO_PKG_VERSION").to_string(),
        supported_chains,
    };

    success_response(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_config_serialization() {
        let config = PublicConfigResponse {
            token_expiry_secs: 3600,
            refresh_token_expiry_secs: 2592000,
            server_time: 1733478000,
            api_version: "0.1.0".to_string(),
            supported_chains: vec![ChainInfo {
                chain_id: 1,
                symbol: "ETH".to_string(),
                name: "Ethereum Mainnet".to_string(),
            }],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("token_expiry_secs"));
        assert!(json.contains("3600"));
    }
}
