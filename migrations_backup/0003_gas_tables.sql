-- ============================================================================
-- Migration: 0003_gas_tables.sql
-- Description: 创建费用系统相关表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 平台费用规则表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gas.platform_fee_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    operation TEXT NOT NULL,
    fee_type TEXT NOT NULL,
    flat_amount DECIMAL(30, 8) NOT NULL DEFAULT 0,
    percent_bp INT NOT NULL DEFAULT 0,
    min_fee DECIMAL(30, 8) NOT NULL DEFAULT 0,
    max_fee DECIMAL(30, 8),
    priority INT NOT NULL DEFAULT 100,
    rule_version INT NOT NULL DEFAULT 1,
    active BOOL NOT NULL DEFAULT true,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 2. 费用归集地址表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gas.fee_collector_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    address TEXT NOT NULL,
    active BOOL NOT NULL DEFAULT true,
    rotated_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 3. 费用审计记录表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS gas.fee_audit (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tx_id UUID,
    user_id UUID,
    chain TEXT NOT NULL,
    operation TEXT NOT NULL,
    original_amount DECIMAL(30, 8) NOT NULL,
    platform_fee DECIMAL(30, 8) NOT NULL,
    fee_type TEXT NOT NULL,
    applied_rule UUID,
    collector_address TEXT NOT NULL,
    tx_hash TEXT,
    wallet_address TEXT,
    gas_used BIGINT DEFAULT 0,
    gas_fee_native DECIMAL(30, 8) DEFAULT 0,
    quote_source TEXT,
    rule_version INT DEFAULT 1,
    retry_count INT DEFAULT 0,
    last_retry_at TIMESTAMPTZ,
    block_number BIGINT,
    confirmations INT DEFAULT 0,
    tx_status SMALLINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

