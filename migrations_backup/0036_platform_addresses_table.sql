-- ============================================================================
-- Migration: 0036_platform_addresses_table.sql
-- Description: 平台托管地址表（法币双向兑换）
-- Purpose: 管理平台的充值/提现托管地址（H项修复）
-- ============================================================================

-- 平台地址表
CREATE TABLE IF NOT EXISTS platform_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,           -- ETH, BSC, POLYGON, SOL, BTC, TON
    address TEXT NOT NULL,         -- 平台控制的地址
    address_type TEXT NOT NULL,    -- onramp, offramp, fee_collection, hot_wallet
    is_active BOOLEAN NOT NULL DEFAULT true,
    balance_threshold_warning DECIMAL(36, 18),  -- 余额预警阈值
    balance_threshold_critical DECIMAL(36, 18), -- 余额临界阈值
    hot_wallet_limit DECIMAL(36, 18),  -- 热钱包最大持有量
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (chain, address_type)  -- 每条链每种类型只有一个活跃地址
);

-- 平台地址余额表（实时同步）
CREATE TABLE IF NOT EXISTS platform_address_balances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    platform_address_id UUID NOT NULL REFERENCES platform_addresses(id),
    balance DECIMAL(36, 18) NOT NULL DEFAULT 0,
    balance_usd DECIMAL(18, 2),
    last_sync_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (platform_address_id)
);

-- 平台地址交易记录表
CREATE TABLE IF NOT EXISTS platform_address_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    platform_address_id UUID NOT NULL REFERENCES platform_addresses(id),
    tx_hash TEXT NOT NULL,
    tx_type TEXT NOT NULL,  -- inbound, outbound
    amount DECIMAL(36, 18) NOT NULL,
    from_address TEXT,
    to_address TEXT,
    fiat_order_id UUID,  -- 关联法币订单
    status TEXT NOT NULL DEFAULT 'pending',
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (tx_hash, platform_address_id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_platform_addresses_chain_type
ON platform_addresses(chain, address_type)
WHERE is_active = true;

CREATE INDEX IF NOT EXISTS idx_platform_address_txs_order
ON platform_address_transactions(fiat_order_id)
WHERE fiat_order_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_platform_address_txs_status
ON platform_address_transactions(platform_address_id, status, created_at DESC);

-- 注释
COMMENT ON TABLE platform_addresses IS '平台托管地址：用于法币双向兑换的平台控制地址';
COMMENT ON COLUMN platform_addresses.address_type IS 'onramp=充值托管, offramp=提现托管, fee_collection=手续费收集, hot_wallet=热钱包';
COMMENT ON TABLE platform_address_balances IS '平台地址余额：实时同步链上余额，监控资金安全';
COMMENT ON TABLE platform_address_transactions IS '平台地址交易记录：追踪所有进出平台地址的交易';

-- 初始化示例数据（生产环境需要配置真实地址）
INSERT INTO platform_addresses (chain, address, address_type, hot_wallet_limit) VALUES
('ETH', '0x1111111111111111111111111111111111111111', 'onramp', 1000000),  -- 100万USDT
('ETH', '0x2222222222222222222222222222222222222222', 'offramp', 500000),  -- 50万USDT
('ETH', '0x3333333333333333333333333333333333333333', 'fee_collection', 100000),  -- 10万USDT
('BSC', '0x4444444444444444444444444444444444444444', 'onramp', 1000000),
('BSC', '0x5555555555555555555555555555555555555555', 'offramp', 500000),
('POLYGON', '0x6666666666666666666666666666666666666666', 'onramp', 500000),
('POLYGON', '0x7777777777777777777777777777777777777777', 'offramp', 250000)
ON CONFLICT (chain, address_type) DO NOTHING;

-- 审计日志
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
VALUES (
    'PLATFORM_ADDRESSES_TABLE_CREATED',
    'system',
    jsonb_build_object(
        'migration', '0036_platform_addresses_table',
        'description', 'Created platform addresses management tables',
        'purpose', 'Manage platform custodial addresses for fiat on/off ramp',
        'security_note', 'Platform addresses are separate from user non-custodial wallets'
    ),
    CURRENT_TIMESTAMP
);

