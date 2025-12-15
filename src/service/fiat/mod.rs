//! Fiat服务商客户端模块
//!
//! 生产级实现，集成真实的第三方支付服务商API
//!
//! 支持的服务商：
//! - Onramper（聚合器，推荐优先使用）
//! - TransFi（中国市场）
//! - AlchemyPay（亚洲市场）
//! - Ramp Network（欧洲市场）
//! - MoonPay（美国市场）

pub mod onramper_client;
pub mod transfi_client;

pub use onramper_client::OnramperClient;
pub use transfi_client::TransFiClient;
