pub mod api_keys;
pub mod approvals;
pub mod asset_service;
pub mod audit_service;
pub mod auth;
pub mod balance_sync_event; // ✅ 余额同步事件驱动
pub mod balance_sync_service; // NEW: 余额同步服务
pub mod blockchain_client;
pub mod bridge_sdk;
pub mod bridge_state_machine; // ✅ G项核心: 跨链桥状态机
pub mod broadcast_reliability_enhancer; // ✅ P2: 交易广播可靠性增强
pub mod cross_chain_bridge_service;
pub mod cross_chain_event_listener; // ✅ P0-9: 跨链事件监听
pub mod cross_chain_event_monitor; // ✅ G项实现: 事件监控服务
pub mod cross_chain_non_custodial_bridge; // ✅ P1: 跨链桥非托管模式
pub mod dynamic_fee_service; // NEW: 动态费用计算服务
pub mod fee_non_custodial_validator; // ✅ P2: 费用非托管验证器
pub mod fee_service;
pub mod fiat; // ✅ 生产级: 支付服务商客户端（Onramper, TransFi等）
pub mod fiat_provider_seeder; // ✅ 法币服务商种子数据
pub mod fiat_service;
pub mod gas_estimation_service; // ✅ 统一Gas估算服务
pub mod gas_estimation_service_enhanced; // ✅ 增强版Gas估算（多速度、拥堵检测）
pub mod gas_estimator;
pub mod multi_node_verifier; // ✅ G项和P项修复: 多节点验证防欺骗
pub mod nonce_manager;
pub mod nonce_manager_enhanced; // ✅ 增强版Nonce管理（分布式锁+Gap检测）
pub mod notification_service;
pub mod onchain_data_sync_service; // NEW: 链上数据同步服务
pub mod oneinch_service;
pub mod order_state_machine; // ✅ 订单状态机
pub mod platform_address_manager; // ✅ H项核心: 平台地址管理+余额监控
pub mod platform_fee_rule_seeder; // ✅ 平台费规则种子数据（防止生产环境空表）
pub mod rpc_endpoint_seeder; // ✅ 生产环境RPC端点种子数据（防止空表导致500）
pub mod policies;
pub mod price_service;
pub mod provider_service;
pub mod reconciliation_service;
pub mod referral_commission_service; // ✅ 返佣收入追踪（对齐行业标准）
pub mod sensitive_operation_guard; // ✅ 敏感操作二次验证
pub mod tenants;
pub mod token_service;
pub mod token_registry_seeder; // ✅ 代币注册表种子数据（防止空表/缺数据）
pub mod transaction_auto_recovery; // ✅ P0-10: 交易自动恢复
pub mod transaction_builder; // NEW: 统一交易构建器
pub mod transaction_monitor;
pub mod transaction_monitor_backfill; // ✅ Gas费用回填服务
pub mod transaction_retry;
pub mod tx;
pub mod tx_broadcasts;
pub mod unified_balance_service; // ✅ P0-11: 统一余额服务
pub mod unified_fee_config_service; // ✅ P0-5: 统一费率配置
pub mod usdt_mapping_service; // NEW: USDT到各链资产映射服务
pub mod users;
pub mod wallet_batch_register_service; // ✅ 多链批量注册（事务性）
pub mod wallets;
pub mod webhook_validator;
pub mod withdrawal_risk_control; // ✅ P0-4: 提现风控
                                 // REMOVED: multi_chain_wallet_enhanced (托管模式，已删除以符合非托管标准)
