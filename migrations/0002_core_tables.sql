-- ============================================================================
-- Migration: 0002_core_tables.sql
-- Description: 创建核心业务表（不包含外键约束）
-- Standard: 遵循数据库最佳实践，先创建表结构，后添加约束
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 租户表（最顶层，无依赖）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 2. 用户表（依赖：tenants）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    email_cipher TEXT NOT NULL,
    email TEXT,  -- 开发环境使用
    phone_cipher TEXT,
    phone TEXT,  -- 开发环境使用
    password_hash TEXT,
    role TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 3. 策略表（依赖：tenants）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS policies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    rules JSONB NOT NULL,
    version INT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 4. 钱包表（依赖：tenants, users, policies）
-- 支持多链钱包管理
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    chain_id INT NOT NULL,
    chain_symbol TEXT,
    address TEXT NOT NULL,
    pubkey TEXT,
    name TEXT,
    derivation_path TEXT,
    curve_type TEXT,
    account_index INT NOT NULL DEFAULT 0,
    address_index INT NOT NULL DEFAULT 0,
    policy_id UUID,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 5. 审批表（依赖：tenants, policies, users）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS approvals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    policy_id UUID NOT NULL,
    requester UUID NOT NULL,
    status TEXT NOT NULL,
    payload JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 6. API密钥表（依赖：tenants）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 7. 交易请求表（依赖：tenants, wallets）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tx_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    wallet_id UUID NOT NULL,
    chain_id INT NOT NULL,
    to_addr TEXT NOT NULL,
    amount DECIMAL(30, 18) NOT NULL,
    status TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 8. 交易广播表（依赖：tenants, tx_requests）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS tx_broadcasts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    tx_request_id UUID NOT NULL,
    tx_hash TEXT,
    receipt JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 9. 审计索引表（依赖：tenants）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS audit_index (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    business_id UUID,
    proof_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 10. Swap交易表（依赖：tenants, users, wallets）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS swap_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    user_id UUID NOT NULL,
    wallet_id UUID,
    chain TEXT, -- ✅添加chain字段
    network TEXT NOT NULL,
    from_token TEXT NOT NULL,
    to_token TEXT NOT NULL,
    from_amount DECIMAL(36, 18) NOT NULL, -- ✅统一精度
    to_amount DECIMAL(36, 18),
    to_amount_min DECIMAL(36, 18), -- ✅最小输出量（滑点保护）
    slippage DECIMAL(5, 2),
    swap_id TEXT NOT NULL,
    tx_hash TEXT,
    wallet_address TEXT, -- ✅添加地址字段
    status TEXT NOT NULL DEFAULT 'pending',
    fiat_order_id UUID, -- ✅关联法币订单
    gas_used TEXT,
    confirmations INT NOT NULL DEFAULT 0,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 11. 交易表（依赖：users, wallets）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID, -- ✅添加租户隔离
    user_id UUID NOT NULL,
    wallet_id UUID,
    chain TEXT, -- ✅简化字段名
    chain_type TEXT, -- 保留向后兼容
    tx_hash TEXT,
    tx_type TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    from_address TEXT NOT NULL,
    to_address TEXT NOT NULL,
    amount DECIMAL(36, 18), -- ✅使用DECIMAL替代TEXT
    token_symbol TEXT,
    gas_fee TEXT,
    nonce BIGINT,
    metadata JSONB, -- ✅添加元数据字段
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    confirmed_at TIMESTAMPTZ
);

-- ----------------------------------------------------------------------------
-- 12. Nonce追踪表（无依赖）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS nonce_tracking (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    address TEXT NOT NULL,
    last_nonce BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

