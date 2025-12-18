-- ============================================================================
-- Migration: 0016_limit_orders_table.sql
-- Description: 创建限价单表
-- ============================================================================

CREATE TABLE IF NOT EXISTS public.limit_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    order_type TEXT NOT NULL, -- 'buy' or 'sell'
    from_token TEXT NOT NULL,
    to_token TEXT NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    limit_price DECIMAL(36, 18) NOT NULL,
    network TEXT NOT NULL,
    wallet_id UUID,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, filled, cancelled, expired
    filled_amount DECIMAL(36, 18),
    filled_price DECIMAL(36, 18),
    tx_hash TEXT,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    filled_at TIMESTAMPTZ,
    metadata JSONB
);

CREATE INDEX IF NOT EXISTS idx_limit_orders_user ON public.limit_orders(user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_limit_orders_status ON public.limit_orders(status, expires_at) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_limit_orders_network ON public.limit_orders(network, from_token, to_token) WHERE status = 'pending';

COMMENT ON TABLE public.limit_orders IS '限价单表';

