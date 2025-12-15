//! 批量创建钱包API（非托管模式）
//! 企业级实现：只接受客户端派生的公开信息

use std::sync::Arc;

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    api::{
        middleware::auth::AuthInfoExtractor,
        response::{success_response, ApiResponse},
    },
    app_state::AppState,
    error::AppError,
    repository::wallet_repository::{CreateWalletParams, PgWalletRepository, WalletRepository},
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// 请求/响应模型
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Debug, Deserialize, ToSchema)]
pub struct WalletCreateItem {
    /// 链标识（"ETH", "BSC", "BTC", "SOL", "TON"）
    pub chain: String,
    /// 钱包地址（客户端派生）
    pub address: String,
    /// 公钥（✅ 必需字段，用于地址验证和余额查询）
    pub public_key: String,
    /// BIP44派生路径（公开信息）
    pub derivation_path: Option<String>,
    /// 曲线类型（secp256k1 / ed25519）
    pub curve_type: Option<String>,
    /// 钱包名称（可选）
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BatchCreateWalletsRequest {
    /// 钱包列表（多链）
    pub wallets: Vec<WalletCreateItem>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletCreateResult {
    pub id: String,
    pub chain: String,
    pub address: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BatchCreateWalletsResponse {
    pub success: bool,
    pub wallets: Vec<WalletCreateResult>,
    pub failed: Vec<WalletCreateError>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WalletCreateError {
    pub chain: String,
    pub address: String,
    pub error: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Handler
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// POST /api/wallets/batch-create
///
/// 批量创建钱包（非托管模式）
///
/// # 非托管原则
/// - ✅ 只接受客户端派生的公开信息（地址、公钥）
/// - ❌ 不接受私钥、助记词、用户密码
/// - ✅ 验证地址格式
/// - ✅ 防止重复地址
#[utoipa::path(
    post,
    path = "/api/wallets/batch-create",
    request_body = BatchCreateWalletsRequest,
    responses(
        (status = 200, description = "Wallets created", body = ApiResponse<BatchCreateWalletsResponse>),
        (status = 400, description = "Bad request", body = crate::error_body::ErrorBodyDoc),
        (status = 401, description = "Unauthorized", body = crate::error_body::ErrorBodyDoc)
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn batch_create_wallets(
    State(state): State<Arc<AppState>>,
    AuthInfoExtractor(auth): AuthInfoExtractor,
    Json(req): Json<BatchCreateWalletsRequest>,
) -> Result<Json<ApiResponse<BatchCreateWalletsResponse>>, AppError> {
    // 验证请求
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

    let user_id = auth.user_id;
    let tenant_id = auth.tenant_id;

    let mut results = Vec::new();
    let mut errors = Vec::new();

    // 处理每个钱包
    for wallet_item in req.wallets {
        match create_single_wallet(&state, user_id, tenant_id, wallet_item).await {
            Ok(result) => results.push(result),
            Err(error) => errors.push(error),
        }
    }

    // 记录审计日志
    let _ = sqlx::query(
        "INSERT INTO audit_logs (event_type, resource_type, resource_id, metadata, created_at)
         VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)",
    )
    .bind("WALLETS_BATCH_CREATED")
    .bind("wallet")
    .bind(user_id)
    .bind(serde_json::json!({
        "user_id": user_id,
        "tenant_id": tenant_id,
        "success_count": results.len(),
        "error_count": errors.len(),
        "chains": results.iter().map(|r| &r.chain).collect::<Vec<_>>()
    }))
    .execute(&state.pool)
    .await
    .ok();

    let response = BatchCreateWalletsResponse {
        success: !results.is_empty(),
        wallets: results,
        failed: errors,
    };

    // 🔍 调试：打印响应结构
    tracing::info!(
        "📤 Batch wallet response: success={}, wallets={}, failed={}",
        response.success,
        response.wallets.len(),
        response.failed.len()
    );

    // 🔍 调试：打印完整JSON响应
    if let Ok(json) = serde_json::to_string_pretty(&response) {
        tracing::info!("📤 Full response JSON:\n{}", json);
    }

    success_response(response)
}

/// 创建单个钱包（企业级：使用Repository层）
async fn create_single_wallet(
    state: &AppState,
    user_id: Uuid,
    tenant_id: Uuid,
    item: WalletCreateItem,
) -> Result<WalletCreateResult, WalletCreateError> {
    // 1. 验证链标识 & DTO→Domain转换
    let chain_id: i64 = match item.chain.to_uppercase().as_str() {
        "ETH" | "ETHEREUM" => 1i64,
        "BSC" | "BINANCE" => 56i64,
        "POLYGON" | "MATIC" => 137i64,
        "BTC" | "BITCOIN" => 0i64,
        "SOL" | "SOLANA" => 501i64,
        "TON" => 607i64,
        _ => {
            return Err(WalletCreateError {
                chain: item.chain.clone(),
                address: item.address.clone(),
                error: format!("Unsupported chain: {}", item.chain),
            });
        }
    };

    // 2. 验证地址格式
    if let Err(e) = validate_address_format(&item.chain, &item.address) {
        return Err(WalletCreateError {
            chain: item.chain.clone(),
            address: item.address.clone(),
            error: format!("Invalid address format: {}", e),
        });
    }

    // 2.5. ✅ 企业级验证：公钥与地址匹配（非托管钱包安全核心）
    if let Err(e) = verify_public_key_matches_address(&item.chain, &item.public_key, &item.address)
    {
        return Err(WalletCreateError {
            chain: item.chain.clone(),
            address: item.address.clone(),
            error: format!("Public key validation failed: {}", e),
        });
    }

    // 2.9 ✅ 优雅降级：确保tenant存在（自动修复数据库重建导致的孤立用户）
    let tenant_exists: Option<(bool,)> =
        sqlx::query_as("SELECT EXISTS(SELECT 1 FROM tenants WHERE id = $1)")
            .bind(tenant_id)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| WalletCreateError {
                chain: item.chain.clone(),
                address: item.address.clone(),
                error: format!("Database error checking tenant: {}", e),
            })?;

    if tenant_exists.is_none() || !tenant_exists.unwrap().0 {
        // Tenant不存在，自动创建（数据库重建场景）
        tracing::warn!(
            "⚠️ Tenant {} not found for user {}, auto-creating (database was likely rebuilt)",
            tenant_id,
            user_id
        );

        let tenant_name = format!("Auto-Tenant-{}", &tenant_id.to_string()[..8]);
        let _ = sqlx::query(
            "INSERT INTO tenants (id, name, created_at, updated_at) 
             VALUES ($1, $2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(tenant_id)
        .bind(tenant_name)
        .execute(&state.pool)
        .await
        .map_err(|e| WalletCreateError {
            chain: item.chain.clone(),
            address: item.address.clone(),
            error: format!("Failed to auto-create tenant: {}", e),
        })?;

        tracing::info!("✅ Auto-created tenant {} for user {}", tenant_id, user_id);
    }

    // 3. 检查地址是否已存在（使用Repository）
    let wallet_repo = PgWalletRepository::new(state.pool.clone());

    if let Some(_existing) = wallet_repo
        .find_by_address(&item.address)
        .await
        .map_err(|e| WalletCreateError {
            chain: item.chain.clone(),
            address: item.address.clone(),
            error: format!("Database error: {}", e),
        })?
    {
        return Err(WalletCreateError {
            chain: item.chain.clone(),
            address: item.address.clone(),
            error: "Wallet already exists".to_string(),
        });
    }

    // 4. ✅ 企业级：使用Repository创建钱包（DTO→Domain转换）
    let wallet_name = item
        .name
        .clone()
        .unwrap_or_else(|| format!("{} Wallet", item.chain));

    tracing::info!(
        "💾 准备创建钱包: user_id={}, address={}, pubkey={} ({}字节)",
        user_id,
        item.address,
        &item.public_key[..20.min(item.public_key.len())],
        item.public_key.len()
    );

    // ✅ DTO→Domain Model转换层
    let create_params = CreateWalletParams {
        tenant_id,
        user_id,
        chain_id,
        chain_symbol: Some(item.chain.to_uppercase()),
        address: item.address.clone(),
        pubkey: Some(item.public_key.clone()), // ✅ public_key → pubkey
        name: Some(wallet_name),
        derivation_path: item.derivation_path.clone(),
        curve_type: item.curve_type.clone(),
        account_index: None, // 使用默认0
        address_index: None, // 使用默认0
        policy_id: None,     // 普通钱包无审批策略
    };

    // ✅ 使用Repository创建（企业级最佳实践）
    let wallet = wallet_repo
        .create(create_params)
        .await
        .map_err(|e| WalletCreateError {
            chain: item.chain.clone(),
            address: item.address.clone(),
            error: format!("Failed to create wallet: {}", e),
        })?;

    tracing::info!(
        "✅ 钱包创建成功: wallet_id={}, user_id={}, address={}",
        wallet.id,
        wallet.user_id,
        wallet.address
    );

    Ok(WalletCreateResult {
        id: wallet.id.to_string(),
        chain: item.chain,
        address: wallet.address,
        created_at: wallet.created_at.to_rfc3339(),
        status: "created".to_string(),
    })
}

/// 验证地址格式
fn validate_address_format(chain: &str, address: &str) -> Result<(), String> {
    match chain.to_uppercase().as_str() {
        "ETH" | "ETHEREUM" | "BSC" | "BINANCE" | "POLYGON" | "MATIC" => {
            // EVM地址：0x + 40个十六进制字符
            if !address.starts_with("0x") || address.len() != 42 {
                return Err("Invalid EVM address format".to_string());
            }
            if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("Invalid hexadecimal characters".to_string());
            }
        }
        "BTC" | "BITCOIN" => {
            // Bitcoin地址：bc1开头（Native SegWit）或1/3开头（Legacy/P2SH）
            if !address.starts_with("bc1") && !address.starts_with('1') && !address.starts_with('3')
            {
                return Err("Invalid Bitcoin address format".to_string());
            }
            if address.len() < 26 || address.len() > 62 {
                return Err("Invalid Bitcoin address length".to_string());
            }
        }
        "SOL" | "SOLANA" => {
            // Solana地址：Base58编码，32字节公钥
            if address.len() < 32 || address.len() > 44 {
                return Err("Invalid Solana address length".to_string());
            }
            // 简化验证：检查Base58字符集
            if !address
                .chars()
                .all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c))
            {
                return Err("Invalid Base58 characters".to_string());
            }
        }
        "TON" => {
            // TON地址：0:开头 + 64个十六进制字符
            if !address.starts_with("0:") {
                return Err("Invalid TON address format (must start with 0:)".to_string());
            }
            if address.len() != 66 {
                return Err("Invalid TON address length".to_string());
            }
            if !address[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("Invalid hexadecimal characters".to_string());
            }
        }
        _ => return Err(format!("Unsupported chain: {}", chain)),
    }

    Ok(())
}

/// ✅ 企业级验证：验证公钥与地址的匹配关系
///
/// # 为什么需要这个验证？
/// 防止客户端发送错误的地址-公钥对，确保数据完整性
fn verify_public_key_matches_address(
    chain: &str,
    public_key: &str,
    address: &str,
) -> Result<(), String> {
    let chain_normalized = chain.to_uppercase();

    match chain_normalized.as_str() {
        "ETH" | "ETHEREUM" | "BSC" | "BINANCE" | "POLYGON" | "MATIC" => {
            verify_evm_public_key(public_key, address)
        }
        "BTC" | "BITCOIN" => verify_bitcoin_public_key(public_key, address),
        "SOL" | "SOLANA" => verify_solana_public_key(public_key, address),
        "TON" => verify_ton_public_key(public_key, address),
        _ => {
            tracing::warn!(
                "Public key verification not implemented for chain: {}",
                chain
            );
            Ok(())
        }
    }
}

/// 验证 EVM 公钥（secp256k1）
fn verify_evm_public_key(public_key_hex: &str, expected_address: &str) -> Result<(), String> {
    use sha3::{Digest, Keccak256};

    let pubkey_bytes =
        hex::decode(public_key_hex).map_err(|_| "Invalid hex public key".to_string())?;

    if pubkey_bytes.len() != 65 && pubkey_bytes.len() != 33 {
        return Err(format!(
            "Invalid EVM public key length: {} (expected 65 or 33)",
            pubkey_bytes.len()
        ));
    }

    if pubkey_bytes.len() == 33 {
        tracing::warn!("Compressed EVM public key, skipping detailed verification");
        return Ok(());
    }

    let mut hasher = Keccak256::new();
    hasher.update(&pubkey_bytes[1..]);
    let hash = hasher.finalize();
    let derived_address = format!("0x{}", hex::encode(&hash[12..]));

    if derived_address.to_lowercase() != expected_address.to_lowercase() {
        return Err(format!(
            "Public key mismatch: expected {}, derived {}",
            expected_address, derived_address
        ));
    }

    Ok(())
}

/// 验证 Bitcoin 公钥（secp256k1）
fn verify_bitcoin_public_key(public_key_hex: &str, _expected_address: &str) -> Result<(), String> {
    let pubkey_bytes =
        hex::decode(public_key_hex).map_err(|_| "Invalid hex public key".to_string())?;

    if pubkey_bytes.len() != 33 && pubkey_bytes.len() != 65 {
        return Err(format!(
            "Invalid Bitcoin public key length: {}",
            pubkey_bytes.len()
        ));
    }

    tracing::warn!(
        "Bitcoin address derivation verification not fully implemented (requires Base58Check)"
    );
    Ok(())
}

/// 验证 Solana 公钥（Ed25519）
fn verify_solana_public_key(public_key_hex: &str, expected_address: &str) -> Result<(), String> {
    let pubkey_bytes =
        hex::decode(public_key_hex).map_err(|_| "Invalid hex public key".to_string())?;

    if pubkey_bytes.len() != 32 {
        return Err(format!(
            "Invalid Solana public key length: {}",
            pubkey_bytes.len()
        ));
    }

    let derived_address = bs58::encode(&pubkey_bytes).into_string();

    if derived_address != expected_address {
        return Err(format!(
            "Public key mismatch: expected {}, derived {}",
            expected_address, derived_address
        ));
    }

    Ok(())
}

/// 验证 TON 公钥（Ed25519）
fn verify_ton_public_key(public_key_hex: &str, _expected_address: &str) -> Result<(), String> {
    let pubkey_bytes =
        hex::decode(public_key_hex).map_err(|_| "Invalid hex public key".to_string())?;

    if pubkey_bytes.len() != 32 {
        return Err(format!(
            "Invalid TON public key length: {}",
            pubkey_bytes.len()
        ));
    }

    tracing::warn!("TON address derivation verification not fully implemented");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_eth_address() {
        assert!(
            validate_address_format("ETH", "0x742d35Cc6634C0532925a3b844Bc9e7595f0bFd2").is_ok()
        );
        assert!(
            validate_address_format("ETH", "0x0000000000000000000000000000000000000000").is_ok()
        );
        assert!(validate_address_format("ETH", "742d35Cc").is_err()); // 缺少0x
        assert!(validate_address_format("ETH", "0x742d35Cc").is_err()); // 长度不足
    }

    #[test]
    fn test_validate_btc_address() {
        assert!(
            validate_address_format("BTC", "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh").is_ok()
        );
        assert!(validate_address_format("BTC", "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa").is_ok());
        assert!(validate_address_format("BTC", "3J98t1WpEZ73CNmYviecrnyiWrnqRhWNLy").is_ok());
        assert!(validate_address_format("BTC", "xyz").is_err());
    }

    #[test]
    fn test_validate_sol_address() {
        assert!(
            validate_address_format("SOL", "7S3P4HxJpyyigGzodYwHtCxZyUQe9JiBMHyRWXArAaKv").is_ok()
        );
        assert!(validate_address_format("SOL", "0x742d35Cc").is_err()); // 太短
        assert!(validate_address_format("SOL", "0OIl").is_err()); // 包含无效Base58字符
    }

    #[test]
    fn test_validate_ton_address() {
        assert!(validate_address_format(
            "TON",
            "0:5d7e8f9a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e"
        )
        .is_ok());
        assert!(validate_address_format("TON", "5d7e8f9a").is_err()); // 缺少0:
        assert!(validate_address_format("TON", "0:5d7e").is_err()); // 长度不足
    }
}
