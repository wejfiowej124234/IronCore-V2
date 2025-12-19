//! Bridge Route API (v1)
//!
//! Goal (Phase A): EVM↔EVM (Ethereum/BSC/Polygon) bridging for supported assets
//! using Stargate Router.swap() with explicit route steps (approve + swap).
//!
//! This endpoint is intentionally designed to be backwards-compatible with the
//! existing frontend `BridgeFeeService` which POSTs `{from_chain,to_chain,amount,token}`
//! and expects fee fields at the top level.

use std::sync::Arc;

use axum::{extract::State, Json};
use ethers::abi::{encode, Token};
use ethers::types::{Address, U256};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    api::response::{success_response, ApiResponse},
    app_state::AppState,
    error::AppError,
    utils::chain_normalizer,
};

#[derive(Debug, Deserialize, ToSchema)]
pub struct BridgeRouteQuoteRequest {
    #[serde(alias = "from_chain", alias = "source_chain")]
    pub from_chain: String,
    #[serde(alias = "to_chain", alias = "destination_chain", alias = "target_chain")]
    pub to_chain: String,

    #[serde(alias = "token_symbol", alias = "token")]
    pub token: String,

    /// Human-readable amount (e.g. "12.34")
    pub amount: f64,

    /// Optional destination address for fee estimation + route generation.
    /// If omitted, a zero address is used for fee quoting.
    #[serde(default)]
    pub destination_address: Option<String>,

    /// Optional refund/source address used by Stargate.
    /// If omitted, a zero address is used.
    #[serde(default)]
    pub source_address: Option<String>,

    /// Slippage in basis points for minAmountLD.
    #[serde(default)]
    pub slippage_bps: Option<u16>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BridgeRouteQuoteResponse {
    /// Backwards-compatible: Stargate message fee (native gas token), in native units.
    pub bridge_fee: f64,
    /// Backwards-compatible placeholder (frontend can estimate)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_gas_fee: Option<f64>,
    /// Backwards-compatible placeholder
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_gas_fee: Option<f64>,
    pub bridge_protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_time_seconds: Option<u64>,

    /// New: executable route steps for the client to sign.
    pub route: BridgeRoute,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BridgeRoute {
    pub provider: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub token_symbol: String,
    pub amount: String,
    pub message_fee_wei: String,
    pub steps: Vec<BridgeRouteStep>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BridgeRouteStep {
    /// "approve" or "swap"
    pub kind: String,
    pub chain: String,
    pub to: String,
    pub value_wei: String,
    pub data: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Stargate config (Phase A scope)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Clone, Copy)]
struct StargateChain {
    lz_chain_id: u16,
    router: Address,
}

fn stargate_chain(chain: &str) -> Option<StargateChain> {
    // Source: https://stargateprotocol.gitbook.io/stargate/developers/contract-addresses/mainnet
    match chain {
        "ethereum" => Some(StargateChain {
            lz_chain_id: 101,
            router: addr("0x8731d54E9D02c286767d56ac03e8037C07e01e98"),
        }),
        "bsc" => Some(StargateChain {
            lz_chain_id: 102,
            router: addr("0x4a364f8c717cAAD9A442737Eb7b8A55cc6cf18D8"),
        }),
        "polygon" => Some(StargateChain {
            lz_chain_id: 109,
            router: addr("0x45A01E4e04F14f7A4a6702c74187c5F6222033cd"),
        }),
        _ => None,
    }
}

fn stargate_pool_id(chain: &str, token_symbol: &str) -> Option<u64> {
    // Source: https://stargateprotocol.gitbook.io/stargate/developers/pool-ids
    // NOTE: Pool availability is chain-dependent (e.g. BSC has USDT but not USDC in the published list).
    let sym = token_symbol.trim().to_uppercase();
    match (chain, sym.as_str()) {
        ("ethereum", "USDC") => Some(1),
        ("ethereum", "USDT") => Some(2),
        ("ethereum", "DAI") => Some(3),
        ("bsc", "USDT") => Some(2),
        ("bsc", "BUSD") => Some(5),
        ("polygon", "USDC") => Some(1),
        ("polygon", "USDT") => Some(2),
        ("polygon", "DAI") => Some(3),
        _ => None,
    }
}

fn addr(s: &str) -> Address {
    s.parse::<Address>().expect("valid address literal")
}

fn selector(sig: &str) -> [u8; 4] {
    let hash = ethers::utils::keccak256(sig.as_bytes());
    [hash[0], hash[1], hash[2], hash[3]]
}

fn encode_call(sig: &str, args: Vec<Token>) -> String {
    let mut out = Vec::with_capacity(4 + 32 * args.len());
    out.extend_from_slice(&selector(sig));
    out.extend_from_slice(&encode(&args));
    format!("0x{}", hex::encode(out))
}

fn address_to_bytes20(address: Address) -> Vec<u8> {
    address.as_bytes().to_vec()
}

fn parse_u256_from_abi_word(result_hex: &str, word_index: usize) -> Result<U256, AppError> {
    let hex_part = result_hex
        .trim()
        .strip_prefix("0x")
        .unwrap_or(result_hex.trim());
    let bytes = hex::decode(hex_part)
        .map_err(|_| AppError::internal_error("Invalid eth_call hex result".to_string()))?;
    if bytes.len() < 32 * (word_index + 1) {
        return Err(AppError::internal_error(
            "Invalid eth_call ABI result length".to_string(),
        ));
    }
    let start = 32 * word_index;
    let end = start + 32;
    Ok(U256::from_big_endian(&bytes[start..end]))
}

fn amount_f64_to_u256_units(amount: f64, decimals: u32) -> Result<U256, AppError> {
    if amount <= 0.0 || !amount.is_finite() {
        return Err(AppError::bad_request("Invalid amount".to_string()));
    }
    // Convert f64 to string with enough precision; avoid scientific notation.
    // Phase A: frontend uses user-entered decimal strings; for now we accept f64.
    let s = format!("{amount:.18}");
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    let mut parts = s.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("");
    if parts.next().is_some() {
        return Err(AppError::bad_request("Invalid amount format".to_string()));
    }

    let int_u256 = U256::from_dec_str(int_part)
        .map_err(|_| AppError::bad_request("Invalid amount integer".to_string()))?;

    let mut frac = frac_part.to_string();
    if frac.len() > decimals as usize {
        frac.truncate(decimals as usize);
    }
    while frac.len() < decimals as usize {
        frac.push('0');
    }
    let frac_u256 = if frac.is_empty() {
        U256::zero()
    } else {
        U256::from_dec_str(&frac)
            .map_err(|_| AppError::bad_request("Invalid amount fractional".to_string()))?
    };

    let scale = U256::from(10u64).pow(U256::from(decimals));
    Ok(int_u256
        .checked_mul(scale)
        .and_then(|v| v.checked_add(frac_u256))
        .ok_or_else(|| AppError::bad_request("Amount overflow".to_string()))?)
}

async fn resolve_token_on_chain(
    state: &AppState,
    chain_id: i64,
    symbol: &str,
) -> Result<(String, u32, bool), AppError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        address: String,
        decimals: i64,
        is_native: bool,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT address, decimals, is_native FROM tokens.registry WHERE symbol = $1 AND chain_id = $2 AND is_enabled = true ORDER BY priority DESC LIMIT 1",
    )
    .bind(symbol)
    .bind(chain_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| AppError::database_error(e.to_string()))?
    .ok_or_else(|| {
        AppError::bad_request(format!(
            "Token not found in registry: symbol={} chain_id={}",
            symbol, chain_id
        ))
    })?;

    let decimals_u32: u32 = row
        .decimals
        .try_into()
        .map_err(|_| AppError::internal_error("Invalid decimals in registry".to_string()))?;
    Ok((row.address, decimals_u32, row.is_native))
}

async fn eth_call(
    state: &AppState,
    chain: &str,
    to: &str,
    data: &str,
) -> Result<String, AppError> {
    let endpoint = state
        .rpc_selector
        .select(chain)
        .await
        .ok_or_else(|| AppError::internal_error(format!("No RPC endpoint available for {chain}")))?;

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [
            {"to": to, "data": data},
            "latest"
        ]
    });

    let client = reqwest::Client::new();

    let response = client
        .post(&endpoint.url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::internal_error(format!("RPC eth_call failed: {e}")))?;

    if !response.status().is_success() {
        return Err(AppError::internal_error(format!(
            "RPC eth_call failed with status: {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| AppError::internal_error(format!("RPC parse failed: {e}")))?;

    crate::infrastructure::rpc_validator::validate_rpc_response(&json)
        .map_err(|e| AppError::internal_error(format!("Invalid RPC response: {e}")))?;

    json.get("result")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::internal_error("Missing eth_call result".to_string()))
}

/// POST /api/v1/bridge/quote
///
/// Returns an executable route (steps) for non-custodial signing.
pub async fn quote_bridge_route(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BridgeRouteQuoteRequest>,
) -> Result<Json<ApiResponse<BridgeRouteQuoteResponse>>, AppError> {
    let source_chain = chain_normalizer::normalize_chain_identifier(&req.from_chain)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let destination_chain = chain_normalizer::normalize_chain_identifier(&req.to_chain)
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    if source_chain == destination_chain {
        return Err(AppError::bad_request(
            "Source and destination chains must differ".to_string(),
        ));
    }

    if !chain_normalizer::is_evm_chain(&source_chain)
        || !chain_normalizer::is_evm_chain(&destination_chain)
    {
        return Err(AppError::bad_request(
            "Phase A supports EVM↔EVM only".to_string(),
        ));
    }

    let src_cfg = stargate_chain(&source_chain).ok_or_else(|| {
        AppError::bad_request(format!("Unsupported Stargate source chain: {source_chain}"))
    })?;
    let dst_cfg = stargate_chain(&destination_chain).ok_or_else(|| {
        AppError::bad_request(format!(
            "Unsupported Stargate destination chain: {destination_chain}"
        ))
    })?;

    let src_pool_id = stargate_pool_id(&source_chain, &req.token).ok_or_else(|| {
        AppError::bad_request(format!(
            "Token not supported by Stargate pools on source chain: {} {}",
            source_chain, req.token
        ))
    })?;
    let dst_pool_id = stargate_pool_id(&destination_chain, &req.token).ok_or_else(|| {
        AppError::bad_request(format!(
            "Token not supported by Stargate pools on destination chain: {} {}",
            destination_chain, req.token
        ))
    })?;

    let src_chain_id = chain_normalizer::get_chain_id(&source_chain)
        .map_err(|e| AppError::bad_request(e.to_string()))?;
    let (token_address, decimals, is_native) =
        resolve_token_on_chain(&state, src_chain_id, req.token.trim().to_uppercase().as_str())
            .await?;

    if is_native {
        return Err(AppError::bad_request(
            "Phase A bridging currently supports ERC20 pool assets only (e.g. USDT/USDC/DAI)".to_string(),
        ));
    }

    let amount_ld = amount_f64_to_u256_units(req.amount, decimals)?;
    let slippage_bps = req.slippage_bps.unwrap_or(50).min(5_000);
    let min_amount_ld = amount_ld
        .checked_mul(U256::from(10_000u64 - slippage_bps as u64))
        .and_then(|v| v.checked_div(U256::from(10_000u64)))
        .ok_or_else(|| AppError::bad_request("Invalid slippage".to_string()))?;

    let destination_address = req
        .destination_address
        .as_deref()
        .unwrap_or("0x0000000000000000000000000000000000000000")
        .parse::<Address>()
        .map_err(|_| AppError::bad_request("Invalid destination_address".to_string()))?;

    let refund_address = req
        .source_address
        .as_deref()
        .unwrap_or("0x0000000000000000000000000000000000000000")
        .parse::<Address>()
        .map_err(|_| AppError::bad_request("Invalid source_address".to_string()))?;

    let to_bytes = address_to_bytes20(destination_address);
    let empty_payload: Vec<u8> = Vec::new();
    let lz_tx_obj = Token::Tuple(vec![
        Token::Uint(U256::zero()),
        Token::Uint(U256::zero()),
        Token::Bytes(Vec::new()),
    ]);

    // 1) quoteLayerZeroFee to determine msg.value
    let quote_fee_data = encode_call(
        "quoteLayerZeroFee(uint16,uint8,bytes,bytes,(uint256,uint256,bytes))",
        vec![
            Token::Uint(U256::from(dst_cfg.lz_chain_id)),
            Token::Uint(U256::from(1u64)),
            Token::Bytes(to_bytes.clone()),
            Token::Bytes(empty_payload.clone()),
            lz_tx_obj.clone(),
        ],
    );

    let quote_result = eth_call(
        &state,
        &source_chain,
        &format!("0x{}", hex::encode(src_cfg.router.as_bytes())),
        &quote_fee_data,
    )
    .await?;
    let mut message_fee_wei = parse_u256_from_abi_word(&quote_result, 0)?;
    // Add a small safety margin (5%) to avoid underpaying.
    message_fee_wei = message_fee_wei
        .checked_mul(U256::from(105u64))
        .and_then(|v| v.checked_div(U256::from(100u64)))
        .unwrap_or(message_fee_wei);

    // 2) approve(router, amount)
    let approve_data = encode_call(
        "approve(address,uint256)",
        vec![Token::Address(src_cfg.router), Token::Uint(amount_ld)],
    );

    // 3) swap(...)
    let swap_data = encode_call(
        "swap(uint16,uint256,uint256,address,uint256,uint256,(uint256,uint256,bytes),bytes,bytes)",
        vec![
            Token::Uint(U256::from(dst_cfg.lz_chain_id)),
            Token::Uint(U256::from(src_pool_id)),
            Token::Uint(U256::from(dst_pool_id)),
            Token::Address(refund_address),
            Token::Uint(amount_ld),
            Token::Uint(min_amount_ld),
            lz_tx_obj,
            Token::Bytes(to_bytes),
            Token::Bytes(empty_payload),
        ],
    );

    let router_to = format!("0x{}", hex::encode(src_cfg.router.as_bytes()));

    let route = BridgeRoute {
        provider: "stargate".to_string(),
        source_chain: source_chain.clone(),
        destination_chain: destination_chain.clone(),
        token_symbol: req.token.trim().to_uppercase(),
        amount: req.amount.to_string(),
        message_fee_wei: message_fee_wei.to_string(),
        steps: vec![
            BridgeRouteStep {
                kind: "approve".to_string(),
                chain: source_chain.clone(),
                to: token_address.clone(),
                value_wei: "0".to_string(),
                data: approve_data,
            },
            BridgeRouteStep {
                kind: "swap".to_string(),
                chain: source_chain.clone(),
                to: router_to,
                value_wei: message_fee_wei.to_string(),
                data: swap_data,
            },
        ],
    };

    let bridge_fee_native = message_fee_wei
        .to_string()
        .parse::<f64>()
        .ok()
        .and_then(|wei| Some(wei / 1e18))
        .unwrap_or(0.0);

    success_response(BridgeRouteQuoteResponse {
        bridge_fee: bridge_fee_native,
        source_gas_fee: None,
        target_gas_fee: None,
        bridge_protocol: "stargate".to_string(),
        estimated_time_seconds: Some(15 * 60),
        route,
    })
}
