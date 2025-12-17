-- ============================================================================
-- Migration: 0010_constraints.sql
-- Description: 添加所有外键约束和唯一约束
-- Standard: 遵循数据库最佳实践，在表创建后添加约束
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 唯一约束
-- ----------------------------------------------------------------------------

-- 钱包：同一租户+链+地址唯一
-- 注意：CockroachDB不支持DROP UNIQUE CONSTRAINT
-- 使用CREATE UNIQUE INDEX IF NOT EXISTS代替
CREATE UNIQUE INDEX IF NOT EXISTS uq_wallet_tenant_chain_addr 
ON wallets(tenant_id, chain_id, address);

-- API Key：哈希唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_api_key_hash 
ON api_keys(key_hash);

-- Swap交易：swap_id唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_swap_transactions_swap_id 
ON swap_transactions(swap_id);

-- Nonce追踪：链+地址唯一（CockroachDB兼容）
-- 注意：nonce_tracking在0032中已重构，使用chain_symbol列
CREATE UNIQUE INDEX IF NOT EXISTS uq_nonce_tracking_chain_address 
ON nonce_tracking(chain, address);

-- 价格：符号+数据源唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_prices_symbol_source 
ON prices(symbol, source);

-- 费用归集地址：链+地址唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_fee_collector 
ON gas.fee_collector_addresses(chain, address);

-- RPC端点：链+URL唯一（CockroachDB兼容）
-- NOTE: admin.rpc_endpoints is created in 0017_admin_tables.sql.
-- Keep admin-specific constraints/indexes there to avoid ordering issues caused by
-- historical duplicate migration versions.

-- 通知模板：code唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_notify_templates_code 
ON notify.templates(code);

-- 用户通知偏好：用户+类型唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_user_pref 
ON notify.user_preferences(user_id, notification_type);

-- 投递记录：通知+用户+渠道唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_delivery 
ON notify.deliveries(notification_id, user_id, channel);

-- 代币注册：链+符号唯一，链+地址唯一（CockroachDB兼容）
DROP INDEX IF EXISTS uq_tokens_registry_chain_symbol;
CREATE UNIQUE INDEX IF NOT EXISTS uq_tokens_registry_chain_symbol 
ON tokens.registry(chain_id, symbol);

DROP INDEX IF EXISTS uq_tokens_registry_chain_address;
CREATE UNIQUE INDEX IF NOT EXISTS uq_tokens_registry_chain_address 
ON tokens.registry(chain_id, address);

-- 事件订阅：处理器名称唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_event_subscriptions_handler 
ON events.event_subscriptions(handler_name);

-- 法币服务商：名称唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_fiat_providers_name 
ON fiat.providers(name);

-- 法币对账记录：日期+服务商唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_reconciliation_date_provider 
ON fiat.reconciliation_records(reconciliation_date, provider);

-- 法币服务商国家支持：服务商+国家唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_provider_country 
ON fiat.provider_country_support(provider_id, country_code);

-- ----------------------------------------------------------------------------
-- 2. 外键约束（按依赖顺序添加）
-- ----------------------------------------------------------------------------

-- 用户表外键
ALTER TABLE users
    DROP CONSTRAINT IF EXISTS fk_users_tenant,
    DROP CONSTRAINT IF EXISTS users_tenant_id_fkey;
ALTER TABLE users
    ADD CONSTRAINT fk_users_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE;

-- 策略表外键
ALTER TABLE policies
    DROP CONSTRAINT IF EXISTS fk_policies_tenant,
    DROP CONSTRAINT IF EXISTS policies_tenant_id_fkey;
ALTER TABLE policies
    ADD CONSTRAINT fk_policies_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE;

-- 钱包表外键
ALTER TABLE wallets
    DROP CONSTRAINT IF EXISTS fk_wallets_tenant,
    DROP CONSTRAINT IF EXISTS fk_wallets_user,
    DROP CONSTRAINT IF EXISTS wallets_tenant_id_fkey,
    DROP CONSTRAINT IF EXISTS wallets_user_id_fkey;
ALTER TABLE wallets
    ADD CONSTRAINT fk_wallets_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_wallets_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE CASCADE;

-- 审批表外键
ALTER TABLE approvals
    DROP CONSTRAINT IF EXISTS fk_approvals_tenant,
    DROP CONSTRAINT IF EXISTS fk_approvals_policy,
    DROP CONSTRAINT IF EXISTS fk_approvals_user,
    DROP CONSTRAINT IF EXISTS approvals_tenant_id_fkey,
    DROP CONSTRAINT IF EXISTS approvals_policy_id_fkey,
    DROP CONSTRAINT IF EXISTS approvals_requester_fkey;
ALTER TABLE approvals
    ADD CONSTRAINT fk_approvals_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_approvals_policy
    FOREIGN KEY (policy_id) 
    REFERENCES policies(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_approvals_user
    FOREIGN KEY (requester) 
    REFERENCES users(id) 
    ON DELETE CASCADE;

-- API密钥表外键
ALTER TABLE api_keys
    DROP CONSTRAINT IF EXISTS fk_api_keys_tenant,
    DROP CONSTRAINT IF EXISTS api_keys_tenant_id_fkey;
ALTER TABLE api_keys
    ADD CONSTRAINT fk_api_keys_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE;

-- 交易请求表外键
ALTER TABLE tx_requests
    DROP CONSTRAINT IF EXISTS fk_tx_requests_tenant,
    DROP CONSTRAINT IF EXISTS fk_tx_requests_wallet,
    DROP CONSTRAINT IF EXISTS tx_requests_tenant_id_fkey,
    DROP CONSTRAINT IF EXISTS tx_requests_wallet_id_fkey;
ALTER TABLE tx_requests
    ADD CONSTRAINT fk_tx_requests_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_tx_requests_wallet
    FOREIGN KEY (wallet_id) 
    REFERENCES wallets(id) 
    ON DELETE CASCADE;

-- 交易广播表外键
ALTER TABLE tx_broadcasts
    DROP CONSTRAINT IF EXISTS fk_tx_broadcasts_tenant,
    DROP CONSTRAINT IF EXISTS fk_tx_broadcasts_tx_req,
    DROP CONSTRAINT IF EXISTS tx_broadcasts_tenant_id_fkey,
    DROP CONSTRAINT IF EXISTS tx_broadcasts_tx_request_id_fkey;
ALTER TABLE tx_broadcasts
    ADD CONSTRAINT fk_tx_broadcasts_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_tx_broadcasts_tx_req
    FOREIGN KEY (tx_request_id) 
    REFERENCES tx_requests(id) 
    ON DELETE CASCADE;

-- 审计索引表外键
ALTER TABLE audit_index
    DROP CONSTRAINT IF EXISTS fk_audit_index_tenant,
    DROP CONSTRAINT IF EXISTS audit_index_tenant_id_fkey;
ALTER TABLE audit_index
    ADD CONSTRAINT fk_audit_index_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE;

-- 交易表外键
ALTER TABLE transactions
    DROP CONSTRAINT IF EXISTS fk_transactions_user,
    DROP CONSTRAINT IF EXISTS fk_transactions_wallet;
ALTER TABLE transactions
    ADD CONSTRAINT fk_transactions_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_transactions_wallet
    FOREIGN KEY (wallet_id) 
    REFERENCES wallets(id) 
    ON DELETE CASCADE;

-- 交易表唯一约束：同一链上交易哈希唯一（CockroachDB兼容）
CREATE UNIQUE INDEX IF NOT EXISTS uq_transactions_tx_hash_chain 
ON transactions(tx_hash, chain_type) 
WHERE tx_hash IS NOT NULL;

-- Swap交易表外键
ALTER TABLE swap_transactions
    DROP CONSTRAINT IF EXISTS fk_swap_transactions_tenant,
    DROP CONSTRAINT IF EXISTS fk_swap_transactions_user,
    DROP CONSTRAINT IF EXISTS fk_swap_transactions_wallet;
ALTER TABLE swap_transactions
    ADD CONSTRAINT fk_swap_transactions_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_swap_transactions_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_swap_transactions_wallet
    FOREIGN KEY (wallet_id) 
    REFERENCES wallets(id) 
    ON DELETE CASCADE;

-- 资产快照表外键（可选）
-- ALTER TABLE asset_snapshots
--     ADD CONSTRAINT fk_snapshots_user
--     FOREIGN KEY (user_id) 
--     REFERENCES users(id) 
--     ON DELETE CASCADE,
--     ADD CONSTRAINT fk_snapshots_wallet
--     FOREIGN KEY (wallet_id) 
--     REFERENCES wallets(id) 
--     ON DELETE CASCADE;

-- 跨链交易表外键（可选）
-- ALTER TABLE cross_chain_swaps
--     ADD CONSTRAINT fk_swaps_user
--     FOREIGN KEY (user_id) 
--     REFERENCES users(id) 
--     ON DELETE CASCADE;

-- 通知投递表外键
ALTER TABLE notify.deliveries
    DROP CONSTRAINT IF EXISTS fk_deliveries_notification;
ALTER TABLE notify.deliveries
    ADD CONSTRAINT fk_deliveries_notification
    FOREIGN KEY (notification_id) 
    REFERENCES notify.notifications(id) 
    ON DELETE CASCADE;

-- 通知历史表外键
ALTER TABLE notify.notification_history
    DROP CONSTRAINT IF EXISTS fk_notification_history_user;
ALTER TABLE notify.notification_history
    ADD CONSTRAINT fk_notification_history_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE CASCADE;

-- 法币订单表外键
ALTER TABLE fiat.orders
    DROP CONSTRAINT IF EXISTS fk_fiat_orders_tenant,
    DROP CONSTRAINT IF EXISTS fk_fiat_orders_user,
    DROP CONSTRAINT IF EXISTS fk_fiat_orders_reviewed_by;
ALTER TABLE fiat.orders
    ADD CONSTRAINT fk_fiat_orders_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_fiat_orders_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_fiat_orders_reviewed_by
    FOREIGN KEY (reviewed_by) 
    REFERENCES users(id) 
    ON DELETE SET NULL;

-- 法币交易表外键
ALTER TABLE fiat.transactions
    DROP CONSTRAINT IF EXISTS fk_fiat_transactions_tenant,
    DROP CONSTRAINT IF EXISTS fk_fiat_transactions_user,
    DROP CONSTRAINT IF EXISTS fk_fiat_transactions_wallet,
    DROP CONSTRAINT IF EXISTS fk_fiat_transactions_order;
ALTER TABLE fiat.transactions
    ADD CONSTRAINT fk_fiat_transactions_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_fiat_transactions_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_fiat_transactions_wallet
    FOREIGN KEY (wallet_id) 
    REFERENCES wallets(id) 
    ON DELETE SET NULL,
    ADD CONSTRAINT fk_fiat_transactions_order
    FOREIGN KEY (fiat_order_id) 
    REFERENCES fiat.orders(id) 
    ON DELETE SET NULL;

-- 法币审计日志表外键
ALTER TABLE fiat.audit_logs
    DROP CONSTRAINT IF EXISTS fk_fiat_audit_logs_tenant,
    DROP CONSTRAINT IF EXISTS fk_fiat_audit_logs_user,
    DROP CONSTRAINT IF EXISTS fk_fiat_audit_logs_order;
ALTER TABLE fiat.audit_logs
    ADD CONSTRAINT fk_fiat_audit_logs_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_fiat_audit_logs_user
    FOREIGN KEY (user_id) 
    REFERENCES users(id) 
    ON DELETE SET NULL,
    ADD CONSTRAINT fk_fiat_audit_logs_order
    FOREIGN KEY (order_id) 
    REFERENCES fiat.orders(id) 
    ON DELETE SET NULL;

-- 法币告警表外键
ALTER TABLE fiat.alerts
    DROP CONSTRAINT IF EXISTS fk_fiat_alerts_tenant,
    DROP CONSTRAINT IF EXISTS fk_fiat_alerts_order,
    DROP CONSTRAINT IF EXISTS fk_fiat_alerts_acknowledged_by;
ALTER TABLE fiat.alerts
    ADD CONSTRAINT fk_fiat_alerts_tenant
    FOREIGN KEY (tenant_id) 
    REFERENCES tenants(id) 
    ON DELETE CASCADE,
    ADD CONSTRAINT fk_fiat_alerts_order
    FOREIGN KEY (order_id) 
    REFERENCES fiat.orders(id) 
    ON DELETE SET NULL,
    ADD CONSTRAINT fk_fiat_alerts_acknowledged_by
    FOREIGN KEY (acknowledged_by) 
    REFERENCES users(id) 
    ON DELETE SET NULL;

-- 法币服务商国家支持表外键
ALTER TABLE fiat.provider_country_support
    DROP CONSTRAINT IF EXISTS fk_provider_country_support_provider;
ALTER TABLE fiat.provider_country_support
    ADD CONSTRAINT fk_provider_country_support_provider
    FOREIGN KEY (provider_id) 
    REFERENCES fiat.providers(id) 
    ON DELETE CASCADE;

