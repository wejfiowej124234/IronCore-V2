-- ============================================================================
-- Migration: 0007_tokens_tables.sql
-- Description: 创建代币注册表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 代币注册表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tokens.registry (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol TEXT NOT NULL,
    name TEXT NOT NULL,
    chain_id INT NOT NULL,
    address TEXT NOT NULL,
    decimals INT NOT NULL,
    is_native BOOL NOT NULL DEFAULT false,
    is_stablecoin BOOL NOT NULL DEFAULT false,
    logo_url TEXT,
    coingecko_id TEXT,
    is_enabled BOOL NOT NULL DEFAULT true,
    priority INT NOT NULL DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

