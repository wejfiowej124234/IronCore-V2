//! 批量创建钱包Handler实现
//! 完整的业务逻辑和错误处理

use crate::api::middleware::auth::AuthInfoExtractor;
use crate::api::response::{success_response, ApiResponse};
use crate::api::wallet_batch_create_api::{
    BatchCreateWalletsRequest, BatchCreateWalletsResponse, WalletCreateError, WalletCreateResult,
};
use crate::app_state::AppState;
use crate::error::AppError;
use axum::{extract::State, Json};
use std::sync::Arc;
use uuid::Uuid;

/// 批量创建钱包Handler
pub async fn handle_batch_create_wallets(
    state: Arc<AppState>,
    auth: AuthInfoExtractor,
    req: BatchCreateWalletsRequest,
) -> Result<Json<ApiResponse<BatchCreateWalletsResponse>>, AppError> {
    // 1. 验证请求
    validate_request(&req)?;

    let user_id = auth.0.user_id;
    let tenant_id = auth.0.tenant_id;

    let mut results = Vec::new();
    let mut errors = Vec::new();

    // 2. 创建钱包组（如果多个钱包有相同名称，说明是多链钱包）
    let group_id = if req.wallets.len() > 1 {
        // 提取第一个钱包的名称作为钱包组名称
        let group_name = req.wallets.first()
            .and_then(|w| w.name.as_ref())
            .unwrap_or(&"Multi-Chain Wallet".to_string())
            .clone();
        
        // 创建钱包组
        match create_wallet_group(&state, user_id, tenant_id, &group_name).await {
            Ok(gid) => Some(gid),
            Err(e) => {
                tracing::warn!("Failed to create wallet group: {}, continuing without group", e);
                None
            }
        }
    } else {
        None
    };

    // 3. 事务处理每个钱包（传入group_id）
    for wallet_item in req.wallets {
        match create_wallet_with_validation(
            &state,
            user_id,
            tenant_id,
            wallet_item,
            group_id,
        )
        .await
        {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    // 4. 记录审计日志
    if !results.is_empty() {
        log_batch_creation_audit(&state, user_id, tenant_id, &results, group_id).await;
    }

    // 5. 返回响应
    success_response(BatchCreateWalletsResponse {
        success: !results.is_empty(),
        wallets: results,
        failed: errors,
    })
}

/// 验证请求
fn validate_request(req: &BatchCreateWalletsRequest) -> Result<(), AppError> {
    if req.wallets.is_empty() {
        return Err(AppError::bad_request(
            "Wallets list cannot be empty".to_string(),
        ));
    }

    if req.wallets.len() > 20 {
        return Err(AppError::bad_request(
            "Maximum 20 wallets per request".to_string(),
        ));
    }

    Ok(())
}

/// 创建钱包组
async fn create_wallet_group(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    name: &str,
) -> Result<Uuid, AppError> {
    let group_id = Uuid::new_v4();
    
    sqlx::query(
        "INSERT INTO wallet_groups (id, tenant_id, user_id, name, created_at, updated_at)
         VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
    )
    .bind(group_id)
    .bind(tenant_id)
    .bind(user_id)
    .bind(name)
    .execute(&state.pool)
    .await
    .map_err(|e| AppError::internal_error(format!("Failed to create wallet group: {}", e)))?;
    
    Ok(group_id)
}

/// 创建单个钱包（带完整验证）
async fn create_wallet_with_validation(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    item: crate::api::wallet_batch_create_api::WalletCreateItem,
    group_id: Option<Uuid>,
) -> Result<WalletCreateResult, WalletCreateError> {
    // 1. 验证链
    let chain_id = validate_and_get_chain_id(&item.chain)?;

    // 2. 验证地址格式
    validate_address_format(&item.chain, &item.address)?;

    // 3. 检查地址是否已存在
    check_address_not_exists(state, &item.address, chain_id, user_id).await?;

    // 4. 插入数据库
    insert_wallet_to_db(state, user_id, tenant_id, chain_id, &item, group_id).await
}

/// 验证链并获取chain_id
fn validate_and_get_chain_id(
    chain: &str,
) -> Result<i64, WalletCreateError> {
    let chain_id = match chain.to_uppercase().as_str() {
        "ETH" | "ETHEREUM" => 1,
        "BSC" | "BINANCE" => 56,
        "POLYGON" | "MATIC" => 137,
        "BTC" | "BITCOIN" => 0,
        "SOL" | "SOLANA" => 501,
        "TON" => 607,
        _ => {
            return Err(WalletCreateError {
                chain: chain.to_string(),
                address: "".to_string(),
                error: format!("Unsupported chain: {}", chain),
            });
        }
    };
    Ok(chain_id)
}

/// 验证地址格式
fn validate_address_format(
    chain: &str,
    address: &str,
) -> Result<(), WalletCreateError> {
    let result = match chain.to_uppercase().as_str() {
        "ETH" | "ETHEREUM" | "BSC" | "BINANCE" | "POLYGON" | "MATIC" => {
            validate_evm_address(address)
        }
        "BTC" | "BITCOIN" => validate_btc_address(address),
        "SOL" | "SOLANA" => validate_sol_address(address),
        "TON" => validate_ton_address(address),
        _ => Err("Unsupported chain".to_string()),
    };

    result.map_err(|error| WalletCreateError {
        chain: chain.to_string(),
        address: address.to_string(),
        error,
    })
}

/// 验证EVM地址
fn validate_evm_address(address: &str) -> Result<(), String> {
    if !address.starts_with("0x") || address.len() != 42 {
        return Err("Invalid EVM address format (must be 0x + 40 hex chars)".to_string());
    }
    if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid hexadecimal characters".to_string());
    }
    Ok(())
}

/// 验证BTC地址
fn validate_btc_address(address: &str) -> Result<(), String> {
    if !address.starts_with("bc1")
        && !address.starts_with('1')
        && !address.starts_with('3')
    {
        return Err("Invalid Bitcoin address format".to_string());
    }
    if address.len() < 26 || address.len() > 62 {
        return Err("Invalid Bitcoin address length".to_string());
    }
    Ok(())
}

/// 验证Solana地址
fn validate_sol_address(address: &str) -> Result<(), String> {
    if address.len() < 32 || address.len() > 44 {
        return Err("Invalid Solana address length".to_string());
    }
    // Base58字符集
    if !address.chars().all(|c| {
        "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c)
    }) {
        return Err("Invalid Base58 characters".to_string());
    }
    Ok(())
}

/// 验证TON地址
fn validate_ton_address(address: &str) -> Result<(), String> {
    if !address.starts_with("0:") {
        return Err("Invalid TON address format (must start with 0:)".to_string());
    }
    if address.len() != 66 {
        return Err("Invalid TON address length".to_string());
    }
    if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid hexadecimal characters in TON address".to_string());
    }
    Ok(())
}

/// 检查地址是否已存在
async fn check_address_not_exists(
    state: &AppState,
    address: &str,
    chain_id: i64,
    user_id: Uuid,
) -> Result<(), WalletCreateError> {
    let existing = sqlx::query("SELECT id FROM wallets WHERE address = $1 AND chain_id = $2 AND user_id = $3")
        .bind(address)
        .bind(chain_id)
        .bind(user_id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| WalletCreateError {
            chain: "".to_string(),
            address: address.to_string(),
            error: format!("Database error: {}", e),
        })?;

    if existing.is_some() {
        return Err(WalletCreateError {
            chain: "".to_string(),
            address: address.to_string(),
            error: "Wallet already exists for this user".to_string(),
        });
    }

    Ok(())
}

/// 插入钱包到数据库
async fn insert_wallet_to_db(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    chain_id: i64,
    item: &crate::api::wallet_batch_create_api::WalletCreateItem,
    group_id: Option<Uuid>,
) -> Result<WalletCreateResult, WalletCreateError> {
    let wallet_id = Uuid::new_v4();
    let wallet_name = item
        .name
        .clone()
        .unwrap_or_else(|| format!("{} Wallet", item.chain));

    let _ = sqlx::query(
        "INSERT INTO wallets 
         (id, tenant_id, user_id, chain_id, chain_symbol, address, pubkey, name, derivation_path, curve_type, group_id, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP)"
    )
    .bind(wallet_id)
    .bind(tenant_id)
    .bind(user_id)
    .bind(chain_id)
    .bind(item.chain.to_uppercase())
    .bind(&item.address)
    .bind(&item.public_key)
    .bind(&wallet_name)
    .bind(&item.derivation_path)
    .bind(&item.curve_type)
    .bind(group_id)
    .execute(&state.pool)
    .await
    .map_err(|e| WalletCreateError {
        chain: item.chain.clone(),
        address: item.address.clone(),
        error: format!("Failed to create wallet: {}", e),
    })?;

    Ok(WalletCreateResult {
        id: wallet_id.to_string(),
        chain: item.chain.clone(),
        address: item.address.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        status: "created".to_string(),
    })
}

/// 记录批量创建审计日志
async fn log_batch_creation_audit(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    results: &[WalletCreateResult],
    group_id: Option<Uuid>,
) {
    let chains: Vec<_> = results.iter().map(|r| &r.chain).collect();
    
    let _ = sqlx::query(
        "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
         VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)"
    )
    .bind("WALLETS_BATCH_CREATED")
    .bind("wallet")
    .bind(user_id)
    .bind(serde_json::json!({
        "user_id": user_id,
        "tenant_id": tenant_id,
        "success_count": results.len(),
        "chains": chains,
        "group_id": group_id
    }))
    .execute(&state.pool)
    .await
    .ok(); // 审计日志失败不影响主流程
}

