//! IronCore - 企业级多链钱包系统后端
//!
//! 非托管模式：后端零私钥、零助记词、零密码存储

pub mod api;
pub mod app_state;
pub mod config;
pub mod domain;
pub mod error;
pub mod error_body;
pub mod error_map;
pub mod infrastructure;
pub mod metrics;
pub mod repository;
pub mod security;
pub mod service;
pub mod utils;

// 重新导出常用类型
pub use app_state::AppState;
pub use error::{AppError, AppErrorCode};

// 企业级标准：统一模块导出
pub mod prelude {
    pub use crate::{
        app_state::AppState,
        domain::{ChainConfig, ChainRegistry, MultiChainWalletService},
        error::{AppError, AppErrorCode},
    };
}
