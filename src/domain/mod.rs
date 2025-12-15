//! Domain 模块
//!
//! 包含核心业务逻辑和领域模型

pub mod chain_config;
pub mod derivation;
pub mod derivation_path_validator; // ✅ P1: 派生路径验证器
pub mod multi_chain_wallet;
pub mod transaction_status;
pub mod wallet_non_custodial; // ✅ 非托管钱包领域模型

// Re-exports
// 重新导出常用类型
pub use chain_config::{AddressFormat, ChainConfig, ChainRegistry, CurveType};
pub use derivation::{DerivationStrategy, DerivationStrategyFactory, DerivedWallet};
pub use derivation_path_validator::DerivationPathValidator;
pub use multi_chain_wallet::{CreateWalletRequest, CreateWalletResponse, MultiChainWalletService};
pub use transaction_status::TransactionStatus;
