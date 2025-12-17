-- ============================================================================
-- Migration: 0011_indexes.sql
-- Description: 创建所有索引
-- Standard: 遵循数据库最佳实践，在约束之后创建索引
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 核心表索引
-- ----------------------------------------------------------------------------

-- 用户表索引
CREATE INDEX IF NOT EXISTS idx_users_tenant ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_tenant_email ON users(tenant_id, email_cipher);
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email) WHERE email IS NOT NULL;

-- 策略表索引
CREATE INDEX IF NOT EXISTS idx_policies_tenant ON policies(tenant_id);

-- 钱包表索引✅优化查询性能
CREATE INDEX IF NOT EXISTS idx_wallets_tenant_chain_addr ON wallets(tenant_id, chain_id, address);
CREATE INDEX IF NOT EXISTS idx_wallets_user_chain ON wallets(user_id, chain_id);
CREATE INDEX IF NOT EXISTS idx_wallets_curve_type ON wallets(curve_type, chain_id) WHERE curve_type IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_wallets_derivation ON wallets(derivation_path) WHERE derivation_path IS NOT NULL;
-- CREATE INDEX IF NOT EXISTS idx_wallets_chain_addr ON wallets(chain, address) WHERE chain IS NOT NULL; -- ❌ 移除：chain列在0042才添加
-- CREATE INDEX IF NOT EXISTS idx_wallets_balance_sync ON wallets(balance_updated_at DESC) WHERE balance_updated_at IS NOT NULL; -- ❌ 移除：balance_updated_at列在0015才添加

-- 审批表索引
CREATE INDEX IF NOT EXISTS idx_approvals_tenant_status ON approvals(tenant_id, status);

-- API密钥表索引
CREATE INDEX IF NOT EXISTS idx_api_keys_tenant_status ON api_keys(tenant_id, status);

-- 交易请求表索引
CREATE INDEX IF NOT EXISTS idx_tx_requests_tenant_status ON tx_requests(tenant_id, status);
CREATE INDEX IF NOT EXISTS idx_tx_requests_chain_time ON tx_requests(chain_id, created_at DESC);

-- 交易广播表索引
CREATE INDEX IF NOT EXISTS idx_tx_broadcasts_tenant_hash ON tx_broadcasts(tenant_id, tx_hash) WHERE tx_hash IS NOT NULL;

-- 审计索引表索引
CREATE INDEX IF NOT EXISTS idx_audit_index_tenant_event ON audit_index(tenant_id, event_type, created_at DESC);

-- 交易表索引
CREATE INDEX IF NOT EXISTS idx_transactions_user_id ON transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_transactions_wallet_id ON transactions(wallet_id);
CREATE INDEX IF NOT EXISTS idx_transactions_tx_hash ON transactions(tx_hash);
CREATE INDEX IF NOT EXISTS idx_transactions_from_address ON transactions(from_address);
CREATE INDEX IF NOT EXISTS idx_transactions_to_address ON transactions(to_address);
CREATE INDEX IF NOT EXISTS idx_transactions_status ON transactions(status);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_transactions_wallet_status ON transactions(wallet_id, status);
CREATE INDEX IF NOT EXISTS idx_transactions_user_status ON transactions(user_id, status) WHERE status = 'pending';

-- Swap交易表索引✅完善
CREATE INDEX IF NOT EXISTS idx_swap_transactions_tenant_user ON swap_transactions(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_swap_transactions_wallet ON swap_transactions(wallet_id) WHERE wallet_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_swap_transactions_swap_id ON swap_transactions(swap_id);
CREATE INDEX IF NOT EXISTS idx_swap_transactions_tx_hash ON swap_transactions(tx_hash) WHERE tx_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_swap_transactions_status ON swap_transactions(status);
CREATE INDEX IF NOT EXISTS idx_swap_transactions_network ON swap_transactions(network);
CREATE INDEX IF NOT EXISTS idx_swap_transactions_chain ON swap_transactions(chain) WHERE chain IS NOT NULL; -- ✅ chain列在0002已定义
CREATE INDEX IF NOT EXISTS idx_swap_transactions_fiat_order ON swap_transactions(fiat_order_id) WHERE fiat_order_id IS NOT NULL; -- ✅保留
CREATE INDEX IF NOT EXISTS idx_swap_transactions_created_at ON swap_transactions(created_at DESC);

-- Nonce追踪表索引
CREATE INDEX IF NOT EXISTS idx_nonce_tracking_chain_address ON nonce_tracking(chain, address);
CREATE INDEX IF NOT EXISTS idx_nonce_tracking_updated ON nonce_tracking(updated_at DESC);

-- ----------------------------------------------------------------------------
-- 费用系统索引
-- ----------------------------------------------------------------------------

-- 平台费用规则索引
CREATE INDEX IF NOT EXISTS idx_platform_fee_rules_chain_op ON gas.platform_fee_rules(chain, operation, active, priority);

-- 费用归集地址索引
CREATE INDEX IF NOT EXISTS idx_fee_collector_chain_active ON gas.fee_collector_addresses(chain, active);

-- 费用审计记录索引
CREATE INDEX IF NOT EXISTS idx_fee_audit_user_time ON gas.fee_audit(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fee_audit_chain_time ON gas.fee_audit(chain, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fee_audit_chain_wallet_time ON gas.fee_audit(chain, wallet_address, created_at DESC) WHERE wallet_address IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_fee_audit_pending_tx ON gas.fee_audit(tx_hash, gas_used, retry_count) WHERE tx_hash IS NOT NULL AND gas_used IS NULL;
CREATE INDEX IF NOT EXISTS idx_fee_audit_stale_tx ON gas.fee_audit(created_at, tx_hash, gas_used) WHERE tx_hash IS NOT NULL AND gas_used IS NULL;

-- ----------------------------------------------------------------------------
-- 管理员系统索引
-- ----------------------------------------------------------------------------

-- NOTE: admin.* tables are created in 0017_admin_tables.sql due to historical
-- duplicate migration version numbers. Admin indexes are also created there to
-- avoid ordering issues.

-- ----------------------------------------------------------------------------
-- 通知系统索引
-- ----------------------------------------------------------------------------

-- 通知模板索引
CREATE INDEX IF NOT EXISTS idx_notify_templates_code ON notify.templates(code) WHERE active = true;

-- 用户通知偏好索引
CREATE INDEX IF NOT EXISTS idx_user_pref_user ON notify.user_preferences(user_id);
CREATE INDEX IF NOT EXISTS idx_user_preferences_user_id ON notify.user_preferences(user_id);
CREATE INDEX IF NOT EXISTS idx_user_preferences_type ON notify.user_preferences(user_id, notification_type);
CREATE INDEX IF NOT EXISTS idx_user_preferences_enabled ON notify.user_preferences(user_id) WHERE enabled = true;

-- 通知实例索引
CREATE INDEX IF NOT EXISTS idx_notifications_category_time ON notify.notifications(category, created_at DESC);

-- 投递记录索引
CREATE INDEX IF NOT EXISTS idx_deliveries_user_status ON notify.deliveries(user_id, status);

-- 用户端点索引
CREATE INDEX IF NOT EXISTS idx_endpoints_user ON notify.endpoints(user_id);

-- 活动批次索引
CREATE INDEX IF NOT EXISTS idx_campaigns_category ON notify.campaigns(category);

-- 通知历史索引
CREATE INDEX IF NOT EXISTS idx_notification_history_user_id ON notify.notification_history(user_id);
CREATE INDEX IF NOT EXISTS idx_notification_history_sent_at ON notify.notification_history(sent_at DESC);

-- ----------------------------------------------------------------------------
-- 资产系统索引
-- ----------------------------------------------------------------------------

-- 价格缓存索引
CREATE INDEX IF NOT EXISTS idx_prices_updated ON prices(last_updated DESC);

-- 资产快照索引
CREATE INDEX IF NOT EXISTS idx_snapshots_user ON asset_snapshots(user_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_wallet ON asset_snapshots(wallet_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_time ON asset_snapshots(snapshot_at DESC);

-- 跨链交易索引
CREATE INDEX IF NOT EXISTS idx_swaps_user ON cross_chain_swaps(user_id);
CREATE INDEX IF NOT EXISTS idx_swaps_status ON cross_chain_swaps(status);
CREATE INDEX IF NOT EXISTS idx_swaps_created ON cross_chain_swaps(created_at DESC);

-- ----------------------------------------------------------------------------
-- 代币系统索引
-- ----------------------------------------------------------------------------

-- 代币注册表索引
CREATE INDEX IF NOT EXISTS idx_tokens_registry_chain_symbol ON tokens.registry(chain_id, symbol);
CREATE INDEX IF NOT EXISTS idx_tokens_registry_chain_address ON tokens.registry(chain_id, address);
CREATE INDEX IF NOT EXISTS idx_tokens_registry_enabled ON tokens.registry(is_enabled) WHERE is_enabled = true;
CREATE INDEX IF NOT EXISTS idx_tokens_registry_stablecoin ON tokens.registry(is_stablecoin) WHERE is_stablecoin = true;
CREATE INDEX IF NOT EXISTS idx_tokens_registry_priority ON tokens.registry(priority);

-- ----------------------------------------------------------------------------
-- 事件系统索引
-- ----------------------------------------------------------------------------

-- 领域事件索引
CREATE INDEX IF NOT EXISTS idx_event_type ON events.domain_events(event_type);
CREATE INDEX IF NOT EXISTS idx_published_at ON events.domain_events(published_at DESC);
CREATE INDEX IF NOT EXISTS idx_processed ON events.domain_events(processed) WHERE processed = false;

-- 事件订阅索引
CREATE INDEX IF NOT EXISTS idx_handler_active ON events.event_subscriptions(handler_name, active);

-- 失败事件索引
CREATE INDEX IF NOT EXISTS idx_failed_events_retry ON events.failed_events(retry_scheduled_at) WHERE retry_scheduled_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_failed_events_handler ON events.failed_events(handler_name);

-- ----------------------------------------------------------------------------
-- 法币系统索引
-- ----------------------------------------------------------------------------

-- 服务商配置索引
CREATE INDEX IF NOT EXISTS idx_providers_enabled ON fiat.providers(is_enabled) WHERE is_enabled = true;
CREATE INDEX IF NOT EXISTS idx_providers_priority ON fiat.providers(priority);

-- 法币订单索引
CREATE INDEX IF NOT EXISTS idx_fiat_orders_tenant_user ON fiat.orders(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_orders_status ON fiat.orders(status);
CREATE INDEX IF NOT EXISTS idx_fiat_orders_provider_order_id ON fiat.orders(provider_order_id) WHERE provider_order_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_fiat_orders_created_at ON fiat.orders(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fiat_orders_expires_at ON fiat.orders(order_expires_at) WHERE status = 'pending';

-- 法币交易索引
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_tenant_user ON fiat.transactions(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_tx_hash ON fiat.transactions(tx_hash) WHERE tx_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_status ON fiat.transactions(status);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_created_at ON fiat.transactions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fiat_transactions_fiat_order_id ON fiat.transactions(fiat_order_id) WHERE fiat_order_id IS NOT NULL;

-- 法币审计日志索引
CREATE INDEX IF NOT EXISTS idx_fiat_audit_logs_tenant_user ON fiat.audit_logs(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_fiat_audit_logs_order_id ON fiat.audit_logs(order_id) WHERE order_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_fiat_audit_logs_action ON fiat.audit_logs(action);
CREATE INDEX IF NOT EXISTS idx_fiat_audit_logs_created_at ON fiat.audit_logs(created_at DESC);

-- 对账记录索引
CREATE INDEX IF NOT EXISTS idx_reconciliation_records_date ON fiat.reconciliation_records(reconciliation_date DESC);
CREATE INDEX IF NOT EXISTS idx_reconciliation_records_status ON fiat.reconciliation_records(status);

-- 异常告警索引
CREATE INDEX IF NOT EXISTS idx_fiat_alerts_tenant_status ON fiat.alerts(tenant_id, status) WHERE status = 'open';
CREATE INDEX IF NOT EXISTS idx_fiat_alerts_severity ON fiat.alerts(severity, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_fiat_alerts_type ON fiat.alerts(alert_type, created_at DESC);

-- 服务商国家支持索引
CREATE INDEX IF NOT EXISTS idx_provider_country_support_provider ON fiat.provider_country_support(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_country_support_country ON fiat.provider_country_support(country_code, is_supported) WHERE is_supported = true;

