-- ============================================================================
-- Migration: 0010_constraints.sql
-- Description: 添加所有外键约束和唯一约束
-- Standard: 遵循数据库最佳实践，在表创建后添加约束
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 0. 防御性修复：确保 notify 相关表存在
--
-- 背景：早期迁移只创建了 notify schema，但未创建通知系统表；
--      本迁移在创建唯一索引/外键时会引用 notify.* 表，从而在新库上失败。
-- 策略：在添加约束/索引前先 CREATE TABLE IF NOT EXISTS（保持幂等）。
-- ----------------------------------------------------------------------------

-- ----------------------------------------------------------------------------
-- 0.1 防御性修复：确保 tokens / events / fiat / asset 相关表存在
--
-- 背景：部分表在后续迁移（本文件/0011/0012/0017 等）中被引用，但在新库上没有被任何迁移创建。
--      这会导致 fresh DB 在 migrations early stage 直接失败（例如 tokens.registry / events.domain_events）。
-- 策略：在添加约束/索引前先 CREATE TABLE IF NOT EXISTS（保持幂等），确保新库能跑完整套迁移。
-- ----------------------------------------------------------------------------

CREATE SCHEMA IF NOT EXISTS tokens;
CREATE SCHEMA IF NOT EXISTS events;
CREATE SCHEMA IF NOT EXISTS fiat;

-- 代币注册表（用于 tokens/list 等接口 & 0010/0011/0012/0013）
CREATE TABLE IF NOT EXISTS tokens.registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    chain_id BIGINT NOT NULL,
    address TEXT NOT NULL,
    decimals BIGINT NOT NULL,
    is_native BOOLEAN NOT NULL DEFAULT false,
    is_stablecoin BOOLEAN NOT NULL DEFAULT false,
    logo_url TEXT,
    coingecko_id TEXT,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    priority BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 事件订阅（0010/0011 约束/索引依赖）
CREATE TABLE IF NOT EXISTS events.event_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    handler_name TEXT NOT NULL,
    event_type TEXT,
    active BOOLEAN NOT NULL DEFAULT true,
    last_processed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 领域事件持久化（event_bus.rs 使用，0011 索引依赖）
CREATE TABLE IF NOT EXISTS events.domain_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    published_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    retry_count INT NOT NULL DEFAULT 0,
    processed BOOLEAN NOT NULL DEFAULT false,
    processed_at TIMESTAMPTZ,
    last_error TEXT
);

-- 失败事件（0011 索引依赖；当前代码未强依赖但迁移引用）
CREATE TABLE IF NOT EXISTS events.failed_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID,
    handler_name TEXT,
    error TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    retry_scheduled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 资产快照（0011 索引依赖；此前资产表迁移被移到 migrations_backup）
CREATE TABLE IF NOT EXISTS asset_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    wallet_id UUID NOT NULL,
    chain_symbol TEXT NOT NULL,
    balance DECIMAL(30, 18) NOT NULL DEFAULT 0,
    balance_usdt DECIMAL(20, 8) NOT NULL DEFAULT 0,
    token_balances JSONB,
    snapshot_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 跨链交易记录（0011 索引依赖；cross_chain_bridge_service.rs 使用）
CREATE TABLE IF NOT EXISTS cross_chain_swaps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    source_chain TEXT NOT NULL,
    source_token TEXT NOT NULL,
    source_amount DECIMAL(30, 18) NOT NULL,
    source_wallet_id UUID NOT NULL,
    target_chain TEXT NOT NULL,
    target_token TEXT NOT NULL,
    target_amount DECIMAL(30, 18),
    estimated_amount DECIMAL(30, 18) NOT NULL,
    target_wallet_id UUID,
    exchange_rate DECIMAL(20, 8) NOT NULL,
    fee_usdt DECIMAL(20, 8) NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    bridge_protocol TEXT,
    bridge_tx_hash TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ
);

-- 法币服务商（0010/0011/0012/0017 依赖；ProviderService 使用）
CREATE TABLE IF NOT EXISTS fiat.providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT true,
    priority BIGINT NOT NULL DEFAULT 0,
    fee_min_percent DECIMAL(10, 4) NOT NULL DEFAULT 0,
    fee_max_percent DECIMAL(10, 4) NOT NULL DEFAULT 0,
    api_url TEXT NOT NULL DEFAULT '',
    webhook_url TEXT,
    timeout_seconds BIGINT NOT NULL DEFAULT 30,
    supported_countries TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    supported_payment_methods TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    health_status TEXT NOT NULL DEFAULT 'unknown',
    last_health_check TIMESTAMPTZ,
    consecutive_failures BIGINT NOT NULL DEFAULT 0,
    total_requests BIGINT NOT NULL DEFAULT 0,
    successful_requests BIGINT NOT NULL DEFAULT 0,
    average_response_time_ms BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 法币服务商国家支持（0010/0011 依赖；ProviderService 使用）
CREATE TABLE IF NOT EXISTS fiat.provider_country_support (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID NOT NULL,
    country_code TEXT NOT NULL,
    is_supported BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 法币订单（0011/0012 依赖；FiatService/ReconciliationService/AuditService 使用）
CREATE TABLE IF NOT EXISTS fiat.orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    order_type TEXT NOT NULL,
    payment_method TEXT NOT NULL,
    fiat_amount DECIMAL(18, 6) NOT NULL,
    fiat_currency TEXT NOT NULL,
    crypto_amount DECIMAL(36, 18) NOT NULL,
    crypto_token TEXT NOT NULL,
    exchange_rate DECIMAL(18, 6) NOT NULL,
    fee_amount DECIMAL(18, 6) NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    provider TEXT NOT NULL DEFAULT 'unknown',
    provider_name TEXT,
    provider_order_id TEXT,
    payment_url TEXT,
    wallet_address TEXT,
    recipient_info JSONB,
    quote_expires_at TIMESTAMPTZ,
    order_expires_at TIMESTAMPTZ,
    review_status TEXT NOT NULL DEFAULT 'auto_approved',
    reviewed_by UUID,
    reviewed_at TIMESTAMPTZ,
    swap_tx_hash TEXT,
    withdrawal_tx_hash TEXT,
    completed_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 法币交易（0011/0012 依赖；当前代码弱依赖但迁移引用）
CREATE TABLE IF NOT EXISTS fiat.transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    user_id UUID,
    fiat_order_id UUID,
    wallet_id UUID,
    tx_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    tx_hash TEXT,
    provider TEXT,
    amount DECIMAL(36, 18),
    currency TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 已存在表的增量修复（确保后续外键可创建）
ALTER TABLE fiat.transactions ADD COLUMN IF NOT EXISTS wallet_id UUID;

-- 法币审计日志（0011 索引依赖；AuditService/FiatService 使用）
CREATE TABLE IF NOT EXISTS fiat.audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID,
    order_id UUID,
    action TEXT NOT NULL,
    amount DECIMAL(36, 18),
    status TEXT,
    provider TEXT,
    ip_address TEXT,
    user_agent TEXT,
    metadata JSONB,
    immudb_proof_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 法币对账记录（0010/0011/0012 依赖；ReconciliationService 使用）
CREATE TABLE IF NOT EXISTS fiat.reconciliation_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reconciliation_date DATE NOT NULL,
    provider TEXT NOT NULL,
    total_orders BIGINT NOT NULL DEFAULT 0,
    matched_orders BIGINT NOT NULL DEFAULT 0,
    unmatched_orders BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 法币告警（0011/0012 依赖；ReconciliationService 使用）
CREATE TABLE IF NOT EXISTS fiat.alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    order_id UUID,
    provider TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    acknowledged_by UUID,
    acknowledged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE fiat.alerts ADD COLUMN IF NOT EXISTS acknowledged_by UUID;
ALTER TABLE fiat.alerts ADD COLUMN IF NOT EXISTS acknowledged_at TIMESTAMPTZ;

CREATE SCHEMA IF NOT EXISTS notify;

-- 通知模板
CREATE TABLE IF NOT EXISTS notify.templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL,
    name TEXT,
    title_template TEXT,
    body_template TEXT,
    channel TEXT NOT NULL DEFAULT 'in_app',
    active BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 通知实例
CREATE TABLE IF NOT EXISTS notify.notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    category TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    scope TEXT NOT NULL DEFAULT 'global',
    creator_role TEXT NOT NULL DEFAULT 'system',
    revoked BOOLEAN NOT NULL DEFAULT false,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 通知投递记录
CREATE TABLE IF NOT EXISTS notify.deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notification_id UUID NOT NULL,
    user_id UUID NOT NULL,
    channel TEXT NOT NULL DEFAULT 'in_app',
    status TEXT NOT NULL DEFAULT 'pending',
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 用户通知偏好
CREATE TABLE IF NOT EXISTS notify.user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    notification_type TEXT NOT NULL,
    channels JSONB NOT NULL DEFAULT '[]'::jsonb,
    frequency TEXT NOT NULL DEFAULT '"immediate"',
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 用户通知端点（如推送/邮件等）
CREATE TABLE IF NOT EXISTS notify.endpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    endpoint_type TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 活动/通知活动批次
CREATE TABLE IF NOT EXISTS notify.campaigns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    category TEXT NOT NULL,
    name TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 通知历史（发送记录/审计）
CREATE TABLE IF NOT EXISTS notify.notification_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    notification_id UUID,
    channel TEXT,
    status TEXT,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB
);

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
CREATE UNIQUE INDEX IF NOT EXISTS uq_rpc_endpoint 
ON admin.rpc_endpoints(chain, url);

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

