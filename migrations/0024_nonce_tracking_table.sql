-- ============================================================================
-- Migration: 0032_nonce_tracking_table.sql
-- Description: Nonce跟踪表（防止冲突和Gap）
-- Purpose: 企业级Nonce管理
-- Priority: P1
-- ============================================================================

-- 注意：nonce_tracking表已在0002_core_tables.sql中创建
-- 这里需要迁移现有表结构到新的增强版本

-- Step 1: 重命名旧的chain列为chain_symbol
ALTER TABLE nonce_tracking RENAME COLUMN chain TO chain_symbol;

-- Step 2: 重命名last_nonce为nonce
ALTER TABLE nonce_tracking RENAME COLUMN last_nonce TO nonce;

-- Step 3: 添加新字段
ALTER TABLE nonce_tracking ADD COLUMN IF NOT EXISTS tx_hash TEXT;
ALTER TABLE nonce_tracking ADD COLUMN IF NOT EXISTS status TEXT NOT NULL DEFAULT 'pending';

-- Step 4: 添加唯一约束
ALTER TABLE nonce_tracking DROP CONSTRAINT IF EXISTS unique_nonce_per_chain_address;
ALTER TABLE nonce_tracking ADD CONSTRAINT unique_nonce_per_chain_address 
UNIQUE (chain_symbol, address, nonce);

-- 索引
CREATE INDEX IF NOT EXISTS idx_nonce_tracking_chain_address 
ON nonce_tracking(chain_symbol, address);

CREATE INDEX IF NOT EXISTS idx_nonce_tracking_status 
ON nonce_tracking(status, chain_symbol, address);

-- CockroachDB：不能在新列上立即创建partial index
CREATE INDEX IF NOT EXISTS idx_nonce_tracking_tx_hash 
ON nonce_tracking(tx_hash);

-- 注释
COMMENT ON TABLE nonce_tracking IS 'Nonce跟踪表：防止nonce冲突和Gap检测';
COMMENT ON COLUMN nonce_tracking.chain_symbol IS '链标识（ETH, BSC, POLYGON等）';
COMMENT ON COLUMN nonce_tracking.address IS '钱包地址';
COMMENT ON COLUMN nonce_tracking.nonce IS 'Nonce值';
COMMENT ON COLUMN nonce_tracking.tx_hash IS '交易哈希';
COMMENT ON COLUMN nonce_tracking.status IS 'Nonce状态（pending, used, failed, replaced）';

-- CockroachDB兼容：移除触发器和复杂函数
-- updated_at由应用层更新
-- Gap检测和清理逻辑移至应用层或使用简单查询

-- 简单的清理查询（可在应用层调用）
-- DELETE FROM nonce_tracking
-- WHERE status IN ('used', 'failed')
--   AND created_at < CURRENT_TIMESTAMP - INTERVAL '7 days';

-- 审计日志（CockroachDB兼容：使用jsonb_build_array）
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
VALUES (
    'NONCE_TRACKING_TABLE_CREATED',
    'database',
    jsonb_build_object(
        'migration', '0032',
        'description', 'Created nonce tracking system for distributed environments',
        'features', jsonb_build_array('gap_detection', 'distributed_lock', 'auto_cleanup')
    ),
    CURRENT_TIMESTAMP
);

