-- ============================================================================
-- Migration: 0022_risk_control_tables.sql
-- Description: 风控相关表：提现风控日志、黑名单、安全告警
-- CockroachDB Compatible: 移除 DO $$ 块
-- ============================================================================

-- 1. 提现风控日志表
CREATE TABLE IF NOT EXISTS withdrawal_risk_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    amount_usd DECIMAL(18, 6) NOT NULL,
    chain TEXT NOT NULL,
    to_address TEXT NOT NULL,
    risk_level TEXT NOT NULL, -- Low, Medium, High, Reject
    allow BOOLEAN NOT NULL,
    triggered_rules JSONB NOT NULL, -- 触发的规则列表
    suggestion TEXT NOT NULL,
    requires_manual_review BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_withdrawal_risk_logs_user ON withdrawal_risk_logs(user_id, created_at DESC);
CREATE INDEX idx_withdrawal_risk_logs_risk ON withdrawal_risk_logs(risk_level, created_at DESC);

-- 2. 提现请求表
CREATE TABLE IF NOT EXISTS withdrawal_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    wallet_id UUID NOT NULL,
    chain TEXT NOT NULL,
    to_address TEXT NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    amount_usd DECIMAL(18, 6),
    status TEXT NOT NULL DEFAULT 'pending', -- pending, approved, processing, completed, rejected
    risk_level TEXT, -- Low, Medium, High
    tx_hash TEXT,
    reviewed_by UUID, -- 审核人（如果需要人工审核）
    reviewed_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_withdrawal_requests_user ON withdrawal_requests(user_id, created_at DESC);
CREATE INDEX idx_withdrawal_requests_status ON withdrawal_requests(status, created_at DESC) WHERE status IN ('pending', 'processing');

-- 3. 地址黑名单表
CREATE TABLE IF NOT EXISTS address_blacklist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    address TEXT NOT NULL UNIQUE,
    chain TEXT,
    reason TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    added_by UUID, -- 添加人
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_address_blacklist_active ON address_blacklist(address, is_active) WHERE is_active = true;

-- 4. 安全告警表
CREATE TABLE IF NOT EXISTS security_alerts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    alert_type TEXT NOT NULL, -- UNUSUAL_LOGIN, SUSPICIOUS_WITHDRAWAL, etc.
    severity TEXT NOT NULL, -- low, medium, high, critical
    description TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open', -- open, investigating, resolved, false_positive
    metadata JSONB,
    resolved_by UUID,
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_security_alerts_user ON security_alerts(user_id, created_at DESC);
CREATE INDEX idx_security_alerts_open ON security_alerts(status, severity, created_at DESC) WHERE status = 'open';

-- 5. 跨链交易表（如果不存在则创建）
CREATE TABLE IF NOT EXISTS cross_chain_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    source_chain TEXT NOT NULL,
    source_tx_hash TEXT NOT NULL,
    source_address TEXT NOT NULL,
    destination_chain TEXT NOT NULL,
    destination_tx_hash TEXT,
    destination_address TEXT NOT NULL,
    token_symbol TEXT NOT NULL,
    amount DECIMAL(36, 18) NOT NULL,
    status TEXT NOT NULL DEFAULT 'SourcePending', -- SourcePending, SourceConfirmed, BridgeProcessing, DestinationPending, DestinationConfirmed, Failed
    bridge_provider TEXT, -- LayerZero, Wormhole, Stargate, etc.
    fee_paid DECIMAL(18, 6),
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_cross_chain_transactions_user ON cross_chain_transactions(user_id, created_at DESC);
CREATE INDEX idx_cross_chain_transactions_status ON cross_chain_transactions(status, created_at ASC) WHERE status NOT IN ('DestinationConfirmed', 'Failed');
CREATE INDEX idx_cross_chain_transactions_source_tx ON cross_chain_transactions(source_tx_hash);

-- 6. 交易RBF日志表
CREATE TABLE IF NOT EXISTS transaction_rbf_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tx_id UUID NOT NULL,
    old_gas_price BIGINT NOT NULL,
    new_gas_price BIGINT NOT NULL,
    old_tx_hash TEXT,
    new_tx_hash TEXT,
    reason TEXT DEFAULT 'stuck_transaction',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_transaction_rbf_logs_tx ON transaction_rbf_logs(tx_id, created_at DESC);

-- 7. 添加transactions表的retry_count字段（如果不存在）
-- CockroachDB Compatible: 简化为直接 ALTER TABLE
ALTER TABLE transactions ADD COLUMN IF NOT EXISTS retry_count INT DEFAULT 0;

-- 8. 添加nonce_tracking表的唯一约束（如果不存在）
-- CockroachDB Compatible: 简化为直接 ALTER TABLE
ALTER TABLE nonce_tracking DROP CONSTRAINT IF EXISTS unique_chain_address;
ALTER TABLE nonce_tracking ADD CONSTRAINT unique_chain_address UNIQUE (chain, address);

-- 9. 注释
COMMENT ON TABLE withdrawal_risk_logs IS '提现风控日志：记录所有风控决策';
COMMENT ON TABLE withdrawal_requests IS '提现请求表：管理提现全流程';
COMMENT ON TABLE address_blacklist IS '地址黑名单：禁止向这些地址转账';
COMMENT ON TABLE security_alerts IS '安全告警：异常行为检测';
COMMENT ON TABLE cross_chain_transactions IS '跨链交易：源链到目标链的完整追踪';
COMMENT ON TABLE transaction_rbf_logs IS '交易RBF日志：Replace-By-Fee操作记录';
