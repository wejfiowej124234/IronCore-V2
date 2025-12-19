-- ============================================================================
-- Migration: 0046_fix_transactions_schema_simple.sql
-- Description: 修复 transactions 表（简化版，跳过类型转换）
-- Purpose: P1 级字段一致性修复
-- Issues Fixed:
--   1. 删除冗余的 chain_type 字段（统一使用 chain）
--   2. 添加性能索引
--   3. gas_fee 类型转换已跳过（CockroachDB 限制）
-- ============================================================================

-- Step 1: 删除冗余的 chain_type 字段
ALTER TABLE transactions DROP COLUMN IF EXISTS chain_type;

-- Step 2: 添加注释
COMMENT ON COLUMN transactions.gas_fee IS '交易 Gas 费用（单位：原生代币）';
COMMENT ON COLUMN transactions.chain IS '链标识（统一使用此字段，例如：ETH, BSC, SOL, BTC, TON）';
COMMENT ON COLUMN transactions.amount IS '交易金额（精度 18 位小数）';

-- Step 3: 添加性能索引
CREATE INDEX IF NOT EXISTS idx_transactions_user_chain 
ON transactions(user_id, chain, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_transactions_address 
ON transactions(from_address, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_transactions_to_address 
ON transactions(to_address, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_transactions_tx_hash 
ON transactions(tx_hash) 
WHERE tx_hash IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_transactions_status_time 
ON transactions(status, created_at DESC);

-- Step 4: 审计日志
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
SELECT
    'TRANSACTIONS_SCHEMA_FIXED',
    'system',
    jsonb_build_object(
        'migration', '0046_fix_transactions_schema_simple',
        'changes', jsonb_build_array(
            'Removed redundant chain_type column',
            'Added performance indexes for user queries',
            'Added tx_hash index for quick lookup',
            'Note: gas_fee type conversion skipped due to CockroachDB limitations'
        ),
        'performance_improvement', 'Added 5 optimized indexes',
        'data_integrity', 'Unified chain field naming'
    ),
    CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs')
ON CONFLICT DO NOTHING;

-- ============================================================================
-- Migration completed successfully
-- ============================================================================
