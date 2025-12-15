-- ============================================================================
-- Migration: 0014_asset_mapping_tables.sql
-- Description: 创建USDT到各链资产映射表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 资产映射表：记录USDT到各链资产的自动映射
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS fiat.asset_mappings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL UNIQUE REFERENCES fiat.orders(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    source_token TEXT NOT NULL DEFAULT 'USDT',
    target_chain TEXT NOT NULL,
    target_token TEXT,
    source_amount DECIMAL(36, 18) NOT NULL,
    target_amount DECIMAL(36, 18),
    status TEXT NOT NULL DEFAULT 'pending',
    swap_tx_id UUID, -- ✅关联swap_transactions表
    swap_tx_hash TEXT,
    bridge_tx_id UUID, -- ✅关联bridge_transactions表
    bridge_tx_hash TEXT,
    error_message TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,
    next_retry_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB
);

-- 创建索引
CREATE INDEX IF NOT EXISTS idx_asset_mappings_order_id ON fiat.asset_mappings(order_id);
CREATE INDEX IF NOT EXISTS idx_asset_mappings_user_id ON fiat.asset_mappings(user_id);
CREATE INDEX IF NOT EXISTS idx_asset_mappings_status ON fiat.asset_mappings(status);
CREATE INDEX IF NOT EXISTS idx_asset_mappings_next_retry ON fiat.asset_mappings(next_retry_at) WHERE status = 'failed';

-- 添加注释
COMMENT ON TABLE fiat.asset_mappings IS 'USDT到各链资产的自动映射记录表';

-- ----------------------------------------------------------------------------
-- 跨链桥交易表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.bridge_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    from_chain TEXT NOT NULL,
    to_chain TEXT NOT NULL,
    token TEXT NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    from_wallet_id UUID,
    to_wallet_id UUID,
    status TEXT NOT NULL DEFAULT 'pending',
    tx_hash_source TEXT,
    tx_hash_target TEXT,
    bridge_protocol TEXT,
    fee_amount DECIMAL(36, 18),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ,
    metadata JSONB
);

CREATE INDEX IF NOT EXISTS idx_bridge_tx_user ON public.bridge_transactions(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_bridge_tx_status ON public.bridge_transactions(status);

-- ----------------------------------------------------------------------------
-- 余额同步任务表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS public.balance_sync_tasks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    triggered_by TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT,
    retry_count INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMPTZ,
    UNIQUE(chain, wallet_address, status)
);

CREATE INDEX IF NOT EXISTS idx_balance_sync_status ON public.balance_sync_tasks(status, created_at);
CREATE INDEX IF NOT EXISTS idx_balance_sync_address ON public.balance_sync_tasks(chain, wallet_address);
