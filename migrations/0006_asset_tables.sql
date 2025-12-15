-- ============================================================================
-- Migration: 0006_asset_tables.sql
-- Description: 创建资产聚合相关表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 价格缓存表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS prices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol TEXT NOT NULL,
    price_usdt DECIMAL(20, 8) NOT NULL,
    source TEXT NOT NULL DEFAULT 'coingecko',
    last_updated TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 2. 资产快照表
-- ----------------------------------------------------------------------------
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

-- ----------------------------------------------------------------------------
-- 3. 跨链交易记录表
-- ----------------------------------------------------------------------------
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

