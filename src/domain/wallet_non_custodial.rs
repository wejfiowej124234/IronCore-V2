//! 非托管钱包领域模型
//! 纯非托管架构：只包含公开信息

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 非托管钱包（只存储公开信息）
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NonCustodialWallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub chain_id: i64,
    pub chain_symbol: String,
    /// 公开地址（可公开）
    pub address: String,
    /// 公钥（可公开）
    #[sqlx(rename = "pubkey")]
    pub public_key: Option<String>,
    /// BIP44派生路径（可公开）
    pub derivation_path: Option<String>,
    /// 曲线类型（可公开）
    pub curve_type: Option<String>,
    pub name: Option<String>,    // ✅ 改为Option（与SQL一致）
    pub account_index: i64,      // ✅ 新增 - INT8/BIGINT
    pub address_index: i64,      // ✅ 新增 - INT8/BIGINT
    pub policy_id: Option<Uuid>, // ✅ 新增
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 钱包创建请求（非托管模式）
#[derive(Debug, Clone, Deserialize)]
pub struct CreateNonCustodialWalletRequest {
    pub chain: String,
    pub address: String,
    pub public_key: Option<String>,
    pub derivation_path: Option<String>,
    pub curve_type: Option<String>,
    pub name: Option<String>,
}

/// 多链钱包创建请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateMultiChainWalletRequest {
    pub wallets: Vec<CreateNonCustodialWalletRequest>,
}

/// 钱包响应
#[derive(Debug, Clone, Serialize)]
pub struct WalletResponse {
    pub id: String,
    pub chain: String,
    pub address: String,
    pub public_key: Option<String>,
    pub derivation_path: Option<String>,
    pub name: String,
    pub created_at: String,
}

impl From<NonCustodialWallet> for WalletResponse {
    fn from(wallet: NonCustodialWallet) -> Self {
        Self {
            id: wallet.id.to_string(),
            chain: wallet.chain_symbol,
            address: wallet.address,
            public_key: wallet.public_key,
            derivation_path: wallet.derivation_path,
            name: wallet.name.unwrap_or_else(|| "Unnamed Wallet".to_string()), // ✅ 修复
            created_at: wallet.created_at.to_rfc3339(),
        }
    }
}

/// 非托管钱包规则验证
pub struct NonCustodialWalletRules;

impl NonCustodialWalletRules {
    /// 验证：确保不包含敏感信息
    pub fn validate_no_sensitive_data(
        request: &CreateNonCustodialWalletRequest,
    ) -> Result<(), String> {
        // 1. 地址不应该是私钥格式
        if request.address.len() == 66 && request.address.starts_with("0x") {
            // 可能是私钥（32字节 = 64位十六进制 + 0x）
            return Err("Address looks like a private key - rejected for security".to_string());
        }

        // 2. 公钥长度验证
        if let Some(ref pubkey) = request.public_key {
            if pubkey.len() < 64 || pubkey.len() > 134 {
                return Err("Invalid public key length".to_string());
            }
        }

        Ok(())
    }

    /// 验证派生路径格式
    pub fn validate_derivation_path(path: &str) -> Result<(), String> {
        if !path.starts_with("m/") {
            return Err("Derivation path must start with m/".to_string());
        }

        // BIP44标准路径：m/purpose'/coin_type'/account'/change/address_index
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() < 4 || parts.len() > 7 {
            return Err("Invalid derivation path depth".to_string());
        }

        Ok(())
    }

    /// 验证曲线类型
    pub fn validate_curve_type(curve_type: &str) -> Result<(), String> {
        match curve_type {
            "secp256k1" | "ed25519" => Ok(()),
            _ => Err(format!("Unsupported curve type: {}", curve_type)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_no_sensitive_data() {
        // 正常地址
        let req = CreateNonCustodialWalletRequest {
            chain: "ETH".to_string(),
            address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bFd2".to_string(),
            public_key: None,
            derivation_path: None,
            curve_type: None,
            name: None,
        };
        assert!(NonCustodialWalletRules::validate_no_sensitive_data(&req).is_ok());

        // 疑似私钥
        let req_bad = CreateNonCustodialWalletRequest {
            chain: "ETH".to_string(),
            address: "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .to_string(),
            public_key: None,
            derivation_path: None,
            curve_type: None,
            name: None,
        };
        assert!(NonCustodialWalletRules::validate_no_sensitive_data(&req_bad).is_err());
    }

    #[test]
    fn test_validate_derivation_path() {
        assert!(NonCustodialWalletRules::validate_derivation_path("m/44'/60'/0'/0/0").is_ok());
        assert!(NonCustodialWalletRules::validate_derivation_path("m/84'/0'/0'/0/0").is_ok());
        assert!(NonCustodialWalletRules::validate_derivation_path("44'/60'/0'/0/0").is_err());
        assert!(NonCustodialWalletRules::validate_derivation_path("m/").is_err());
    }

    #[test]
    fn test_validate_curve_type() {
        assert!(NonCustodialWalletRules::validate_curve_type("secp256k1").is_ok());
        assert!(NonCustodialWalletRules::validate_curve_type("ed25519").is_ok());
        assert!(NonCustodialWalletRules::validate_curve_type("rsa").is_err());
    }
}
