-- ============================================================================
-- Migration: 0033_cross_chain_transactions_enhancements.sql
-- Description: 增强跨链交易表（支持完整生命周期）
-- Purpose: 完善跨链桥功能
-- Priority: P1
-- ============================================================================

-- 注意：cross_chain_transactions表已在0022_risk_control_tables.sql中创建
-- 这里需要添加增强字段到现有表

-- Step 1: 添加新字段
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS source_confirmations INT DEFAULT 0;
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS destination_confirmations INT DEFAULT 0;
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS progress_percentage INT DEFAULT 0;
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS bridge_protocol TEXT;
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS completed_at TIMESTAMPTZ;

-- Step 2: signed_source_tx was added in 0031, so it should already exist

-- 索引
CREATE INDEX IF NOT EXISTS idx_cross_chain_user_status 
ON cross_chain_transactions(user_id, status);

CREATE INDEX IF NOT EXISTS idx_cross_chain_source_tx 
ON cross_chain_transactions(source_tx_hash) 
WHERE source_tx_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_cross_chain_dest_tx 
ON cross_chain_transactions(destination_tx_hash) 
WHERE destination_tx_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_cross_chain_status 
ON cross_chain_transactions(status, created_at DESC);

-- Step 3: 添加进度百分比检查约束
ALTER TABLE cross_chain_transactions DROP CONSTRAINT IF EXISTS check_progress_percentage;
ALTER TABLE cross_chain_transactions 
ADD CONSTRAINT check_progress_percentage 
CHECK (progress_percentage >= 0 AND progress_percentage <= 100);

-- Step 4: 更新状态枚举约束
ALTER TABLE cross_chain_transactions DROP CONSTRAINT IF EXISTS check_bridge_status;
ALTER TABLE cross_chain_transactions
ADD CONSTRAINT check_bridge_status CHECK (
    status IN (
        'SourcePending',      -- 源链交易待确认
        'SourceConfirming',   -- 源链交易确认中
        'SourceConfirmed',    -- 源链交易已确认
        'BridgeProcessing',   -- 跨链桥处理中
        'DestinationPending', -- 目标链交易待发送
        'DestinationConfirming', -- 目标链交易确认中
        'Completed',          -- 完成
        'Failed',             -- 失败
        'Cancelled'           -- 已取消
    )
);

-- CockroachDB兼容：移除触发器（改为应用层实现）
-- 以下逻辑移至 BridgeService：
-- 1. updated_at 自动更新 → Service层UPDATE时设置
-- 2. completed_at 自动设置 → status=Completed时设置  
-- 3. progress_percentage 计算 → BridgeService::calculate_progress()
-- 4. 状态变更审计 → BridgeService::audit_status_change()

-- 注释
COMMENT ON TABLE cross_chain_transactions IS '跨链交易表（非托管模式：只存储交易哈希，不存储私钥）';
COMMENT ON COLUMN cross_chain_transactions.signed_source_tx IS '客户端签名的源链交易（非托管模式必需）';
COMMENT ON COLUMN cross_chain_transactions.status IS '交易状态（自动驱动进度百分比）';
COMMENT ON COLUMN cross_chain_transactions.progress_percentage IS '进度百分比（0-100，自动计算）';

-- 审计日志（CockroachDB兼容：使用jsonb_build_array）
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
VALUES (
    'CROSS_CHAIN_TABLE_ENHANCED',
    'database',
    jsonb_build_object(
        'migration', '0033',
        'description', 'Enhanced cross-chain transactions table with lifecycle management',
        'features', jsonb_build_array('status_tracking', 'progress_percentage', 'auto_audit')
    ),
    CURRENT_TIMESTAMP
);

