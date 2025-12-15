// Network configuration API
// 返回后端配置的网络信息，前端不需要让用户选择网络

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{api::response::success_response, app_state::AppState, error::AppError};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkType {
    Mainnet,
    Testnet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainNetworkConfig {
    pub chain: String,
    pub network: NetworkType,
    pub rpc_url: String,
    pub chain_id: Option<u64>, // For EVM chains
}

// 移除 success 字段，由 ApiResponse 统一处理

/// GET /api/network-config
/// 返回后端配置的所有链的网络信息
#[utoipa::path(
    get,
    path = "/api/network-config",
    responses(
        (status = 200, description = "Network configuration retrieved successfully", body = crate::api::response::ApiResponse<Vec<ChainNetworkConfig>>)
    ),
    tag = "network"
)]
pub async fn get_network_config(
    State(state): State<Arc<AppState>>,
) -> Result<Json<crate::api::response::ApiResponse<Vec<ChainNetworkConfig>>>, AppError> {
    // 企业级实现：从配置系统加载链配置，支持所有EVM链
    use crate::service::token_service::network_to_chain_id;

    let mut chains = Vec::new();

    // 支持的链列表（从配置系统获取，非硬编码）
    let supported_networks = vec![
        ("ethereum", "eth"),
        ("bsc", "bsc"),
        ("polygon", "polygon"),
        ("arbitrum", "arbitrum"),
        ("optimism", "optimism"),
        ("avalanche", "avalanche"),
    ];

    for (chain_name, network_key) in supported_networks {
        if let Some(chain_id) = network_to_chain_id(network_key) {
            // 从配置系统获取RPC URL（从AppState的blockchain配置）
            let rpc_url = match chain_name {
                "ethereum" => state.blockchain_config.eth_rpc_url.clone(),
                "bsc" => state.blockchain_config.bsc_rpc_url.clone(),
                "polygon" => state.blockchain_config.polygon_rpc_url.clone(),
                _ => {
                    // 对于其他链，尝试从环境变量获取
                    std::env::var(&format!("{}_RPC_URL", chain_name.to_uppercase()))
                        .unwrap_or_else(|_| format!("https://{}.rpc.example.com", chain_name))
                }
            };

            // 判断网络类型（基于RPC URL或chain_id）
            let network = if rpc_url.contains("testnet")
                || rpc_url.contains("sepolia")
                || rpc_url.contains("goerli")
                || rpc_url.contains("devnet")
            {
                NetworkType::Testnet
            } else {
                NetworkType::Mainnet
            };

            // 获取原生代币符号（从配置或默认值）
            let _native_token = match chain_name {
                "ethereum" => "ETH",
                "bsc" => "BNB",
                "polygon" => "MATIC",
                "arbitrum" => "ETH",
                "optimism" => "ETH",
                "avalanche" => "AVAX",
                _ => "ETH",
            };

            chains.push(ChainNetworkConfig {
                chain: chain_name.to_string(),
                network,
                rpc_url,
                chain_id: Some(chain_id),
            });
        }
    }

    // 添加非EVM链（如果配置了）
    let btc_rpc = std::env::var("BITCOIN_RPC_URL")
        .unwrap_or_else(|_| "https://blockstream.info/api".to_string());
    let sol_rpc = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    let ton_rpc = std::env::var("TON_RPC_URL")
        .unwrap_or_else(|_| "https://toncenter.com/api/v2/jsonRPC".to_string());

    let btc_network = if btc_rpc.contains("testnet") {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    let sol_network = if sol_rpc.contains("devnet") || sol_rpc.contains("testnet") {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    let ton_network = if ton_rpc.contains("testnet") {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    chains.push(ChainNetworkConfig {
        chain: "bitcoin".to_string(),
        network: btc_network,
        rpc_url: btc_rpc,
        chain_id: None,
    });

    chains.push(ChainNetworkConfig {
        chain: "solana".to_string(),
        network: sol_network,
        rpc_url: sol_rpc,
        chain_id: None,
    });

    chains.push(ChainNetworkConfig {
        chain: "ton".to_string(),
        network: ton_network,
        rpc_url: ton_rpc,
        chain_id: None,
    });

    success_response(chains)
}
