-- ============================================================================
-- Migration: 0020_unified_fee_configurations.sql
-- Description: 统一费率配置表
-- Purpose: 所有费率集中管理，前后端统一调用
-- ============================================================================

-- 费率配置表
CREATE TABLE IF NOT EXISTS fee_configurations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    fee_type TEXT NOT NULL,
    chain TEXT DEFAULT 'global', -- 'global' 表示全局费率（CockroachDB不支持NULL在PRIMARY KEY）
    rate_percentage DECIMAL(10, 4) NOT NULL DEFAULT 0.0, -- 百分比费率 (0-100)
    min_fee_usd DECIMAL(18, 6), -- 最小费用（USD）
    max_fee_usd DECIMAL(18, 6), -- 最大费用（USD）
    fixed_fee_usd DECIMAL(18, 6), -- 固定费用（USD，优先于百分比）
    enabled BOOLEAN NOT NULL DEFAULT true,
    description TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- 唯一约束：费率类型 + 链
    UNIQUE (fee_type, chain)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_fee_configurations_type 
ON fee_configurations(fee_type) WHERE enabled = true;

-- CockroachDB兼容：移除审计触发器（改为应用层实现）
-- 费率变更审计逻辑移至 Service 层的 FeeConfigService

-- 注释
COMMENT ON TABLE fee_configurations IS '统一费率配置表：所有费率集中管理';
COMMENT ON COLUMN fee_configurations.fee_type IS '费率类型：SwapServiceFee, GasFee, BridgeFee, WithdrawalFee, FiatDepositFee, FiatWithdrawalFee';
COMMENT ON COLUMN fee_configurations.chain IS '链标识（NULL=全局费率）';
COMMENT ON COLUMN fee_configurations.rate_percentage IS '百分比费率 (0-100)';
COMMENT ON COLUMN fee_configurations.fixed_fee_usd IS '固定费用（优先于百分比）';

