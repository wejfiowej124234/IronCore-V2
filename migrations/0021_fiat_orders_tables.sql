-- ============================================================================
-- Migration: 0024_fiat_orders_tables.sql
-- Description: 法币充值提现订单表
-- ============================================================================

-- 1. 法币充值订单表（Onramp）
CREATE TABLE IF NOT EXISTS fiat_onramp_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    fiat_amount DECIMAL(18, 6) NOT NULL,
    fiat_currency TEXT NOT NULL, -- USD, EUR, CNY
    crypto_amount DECIMAL(36, 18) NOT NULL,
    crypto_currency TEXT NOT NULL, -- USDT, USDC, ETH
    target_chain TEXT NOT NULL,
    wallet_address TEXT NOT NULL,
    payment_method TEXT NOT NULL, -- credit_card, bank_transfer, alipay
    status TEXT NOT NULL DEFAULT 'pending_payment', -- pending_payment, paid, processing, completed, failed, cancelled
    exchange_rate DECIMAL(18, 6) NOT NULL,
    fee_amount DECIMAL(18, 6) NOT NULL,
    payment_provider TEXT, -- Stripe, PayPal, Alipay
    payment_id TEXT, -- 第三方支付ID
    payment_url TEXT,
    expires_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_fiat_onramp_orders_user ON fiat_onramp_orders(user_id, created_at DESC);
CREATE INDEX idx_fiat_onramp_orders_status ON fiat_onramp_orders(status, created_at DESC) WHERE status NOT IN ('completed', 'failed', 'cancelled');
CREATE INDEX idx_fiat_onramp_orders_payment ON fiat_onramp_orders(payment_id) WHERE payment_id IS NOT NULL;

-- 2. 法币提现订单表（Offramp）
CREATE TABLE IF NOT EXISTS fiat_offramp_orders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    crypto_amount DECIMAL(36, 18) NOT NULL,
    crypto_currency TEXT NOT NULL,
    source_chain TEXT NOT NULL,
    fiat_amount DECIMAL(18, 6) NOT NULL,
    fiat_currency TEXT NOT NULL,
    bank_account_info JSONB NOT NULL, -- 银行账户信息（加密存储）
    status TEXT NOT NULL DEFAULT 'pending_review', -- pending_review, approved, processing, completed, failed, rejected
    exchange_rate DECIMAL(18, 6) NOT NULL,
    fee_amount DECIMAL(18, 6) NOT NULL,
    risk_level TEXT, -- Low, Medium, High
    tx_hash TEXT, -- 源链交易哈希
    reviewed_by UUID,
    reviewed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_fiat_offramp_orders_user ON fiat_offramp_orders(user_id, created_at DESC);
CREATE INDEX idx_fiat_offramp_orders_status ON fiat_offramp_orders(status, created_at DESC) WHERE status NOT IN ('completed', 'failed', 'rejected');
CREATE INDEX idx_fiat_offramp_orders_review ON fiat_offramp_orders(status, risk_level, created_at ASC) WHERE status = 'pending_review';

-- 3. 支付回调日志表（幂等性检查）
CREATE TABLE IF NOT EXISTS payment_callback_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    order_id UUID NOT NULL,
    provider TEXT NOT NULL,
    callback_data JSONB NOT NULL,
    idempotency_key TEXT UNIQUE, -- 幂等性key
    processed BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_payment_callback_logs_order ON payment_callback_logs(order_id, created_at DESC);
CREATE INDEX idx_payment_callback_logs_key ON payment_callback_logs(idempotency_key) WHERE idempotency_key IS NOT NULL;

-- 注释
COMMENT ON TABLE fiat_onramp_orders IS '法币充值订单：法币 → 加密货币';
COMMENT ON TABLE fiat_offramp_orders IS '法币提现订单：加密货币 → 法币';
COMMENT ON TABLE payment_callback_logs IS '支付回调日志：幂等性保护';

