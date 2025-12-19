-- ============================================================================
-- Migration: 0009_fiat_tables.sql
-- Description: 创建法币充值和提现系统表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 服务商配置表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    is_enabled BOOL NOT NULL DEFAULT true,
    priority INT NOT NULL DEFAULT 100,
    fee_min_percent DECIMAL(5, 2) NOT NULL,
    fee_max_percent DECIMAL(5, 2) NOT NULL,
    api_key_encrypted TEXT,
    api_url TEXT NOT NULL,
    webhook_url TEXT,
    timeout_seconds INT NOT NULL DEFAULT 30,
    supported_countries TEXT[],
    supported_payment_methods TEXT[],
    health_status TEXT NOT NULL DEFAULT 'unknown',
    last_health_check TIMESTAMPTZ,
    consecutive_failures INT NOT NULL DEFAULT 0,
    total_requests INT NOT NULL DEFAULT 0,
    successful_requests INT NOT NULL DEFAULT 0,
    average_response_time_ms INT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 2. 法币订单表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    order_type TEXT NOT NULL,
    payment_method TEXT NOT NULL,
    fiat_amount DECIMAL(20, 2) NOT NULL,
    fiat_currency TEXT NOT NULL DEFAULT 'USD',
    crypto_amount DECIMAL(36, 18) NOT NULL,
    crypto_token TEXT NOT NULL,
    exchange_rate DECIMAL(20, 8) NOT NULL,
    fee_amount DECIMAL(20, 2) NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    provider TEXT NOT NULL,
    provider_order_id TEXT,
    payment_url TEXT,
    wallet_address TEXT,
    recipient_info JSONB,
    quote_expires_at TIMESTAMPTZ,
    order_expires_at TIMESTAMPTZ,
    review_status TEXT DEFAULT 'auto_approved',
    reviewed_by UUID,
    reviewed_at TIMESTAMPTZ,
    swap_tx_hash TEXT,
    withdrawal_tx_hash TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ,
    metadata JSONB
);

-- ----------------------------------------------------------------------------
-- 3. 交易历史表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    wallet_id UUID,
    tx_type TEXT NOT NULL,
    from_token TEXT NOT NULL,
    to_token TEXT NOT NULL,
    from_amount DECIMAL(36, 18) NOT NULL,
    to_amount DECIMAL(36, 18) NOT NULL,
    tx_hash TEXT,
    fiat_order_id UUID,
    status TEXT NOT NULL DEFAULT 'pending',
    fee_amount DECIMAL(36, 18),
    gas_fee DECIMAL(36, 18),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ,
    metadata JSONB
);

-- ----------------------------------------------------------------------------
-- 4. 审计日志表
-- ----------------------------------------------------------------------------
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

-- ----------------------------------------------------------------------------
-- 5. 对账记录表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.reconciliation_records (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    reconciliation_date DATE NOT NULL,
    provider TEXT NOT NULL,
    total_orders INT NOT NULL DEFAULT 0,
    matched_orders INT NOT NULL DEFAULT 0,
    unmatched_orders INT NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 6. 异常告警表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    alert_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    order_id UUID,
    provider TEXT,
    status TEXT NOT NULL DEFAULT 'open',
    acknowledged_by UUID,
    acknowledged_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 7. 服务商国家支持映射表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.provider_country_support (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID NOT NULL,
    country_code TEXT NOT NULL,
    is_supported BOOL NOT NULL DEFAULT true,
    last_verified_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

