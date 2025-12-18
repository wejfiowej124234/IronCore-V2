-- ============================================================================
-- Migration: 0042_add_missing_columns.sql
-- Description: 添加缺失的列到现有表
-- CockroachDB Compatible: 移除 DO $$ 块，使用 ADD COLUMN IF NOT EXISTS
-- ============================================================================

-- 1. transactions 表添加缺失字段
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS tenant_id UUID;
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS metadata JSONB DEFAULT '{}'::jsonb;
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS chain TEXT;
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;

-- 添加索引（CockroachDB：移除新列上的WHERE子句）
CREATE INDEX IF NOT EXISTS idx_transactions_tenant ON transactions(tenant_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_transactions_metadata ON transactions USING gin(metadata);
CREATE INDEX IF NOT EXISTS idx_transactions_chain ON transactions(chain, created_at DESC);

-- 2. wallets 表添加缺失字段
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;
ALTER TABLE wallets ADD COLUMN IF NOT EXISTS tenant_id UUID;

-- 添加索引（CockroachDB：移除新列上的WHERE子句）
CREATE INDEX IF NOT EXISTS idx_wallets_tenant ON wallets(tenant_id, created_at DESC);

-- 3. users 表添加缺失字段
ALTER TABLE users ADD COLUMN IF NOT EXISTS kyc_status TEXT DEFAULT 'not_submitted';
ALTER TABLE users ADD COLUMN IF NOT EXISTS tenant_id UUID;

-- 添加索引（CockroachDB：移除新列上的WHERE子句）
CREATE INDEX IF NOT EXISTS idx_users_kyc_status ON users(kyc_status);
CREATE INDEX IF NOT EXISTS idx_users_tenant ON users(tenant_id);

-- 4. cross_chain_transactions 表添加缺失字段
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;

-- 5. user_bank_accounts 表（如果不存在则创建）
CREATE TABLE IF NOT EXISTS user_bank_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID,
    bank_name TEXT NOT NULL,
    account_holder TEXT NOT NULL,
    account_number TEXT NOT NULL,
    routing_number TEXT,
    swift_code TEXT,
    country TEXT NOT NULL,
    currency TEXT NOT NULL,
    is_verified BOOLEAN DEFAULT false,
    is_default BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_user_bank_accounts_user ON user_bank_accounts(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_bank_accounts_default ON user_bank_accounts(user_id, is_default);

-- 6. platform_addresses 表添加缺失字段（如果需要）
ALTER TABLE platform_addresses ADD COLUMN IF NOT EXISTS balance_usd DECIMAL(18, 6);

-- 注释
COMMENT ON COLUMN transactions.metadata IS '扩展元数据（JSON格式）：用于存储额外的交易信息';
COMMENT ON COLUMN transactions.tenant_id IS '租户ID：多租户隔离';
COMMENT ON COLUMN transactions.chain IS '链标识：ETH, BSC, POLYGON等';
COMMENT ON COLUMN users.kyc_status IS 'KYC状态：not_submitted, pending, approved, rejected';
COMMENT ON TABLE user_bank_accounts IS '用户银行账户：用于法币提现';
