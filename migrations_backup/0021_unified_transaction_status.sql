-- ============================================================================
-- Migration: 0021_unified_transaction_status.sql
-- Description: 统一交易状态枚举和状态机约束
-- Purpose: 所有交易表使用统一的状态枚举，防止非法状态转换
-- CockroachDB Compatible: 移除 DO $$ 块，使用简单的 ALTER TABLE 语句
-- ============================================================================

-- 1. 更新 transactions 表（添加CHECK约束）
-- 先删除可能存在的约束
ALTER TABLE transactions DROP CONSTRAINT IF EXISTS check_transaction_status_enum;

-- 添加新约束
ALTER TABLE transactions
ADD CONSTRAINT check_transaction_status_enum CHECK (
    status IN ('created', 'signed', 'pending', 'executing', 'confirmed', 'failed', 'timeout', 'replaced', 'cancelled')
);

-- 设置默认值
ALTER TABLE transactions ALTER COLUMN status SET DEFAULT 'pending';

-- 2. 更新 swap_transactions 表
-- 清理可能存在的旧列
ALTER TABLE swap_transactions DROP COLUMN IF EXISTS status_old;

-- 删除旧约束
ALTER TABLE swap_transactions DROP CONSTRAINT IF EXISTS check_swap_transaction_status;

-- 添加新约束
ALTER TABLE swap_transactions
ADD CONSTRAINT check_swap_transaction_status CHECK (
    status IN ('created', 'signed', 'pending', 'executing', 'confirmed', 'failed', 'timeout', 'replaced', 'cancelled')
);

-- 设置默认值
ALTER TABLE swap_transactions ALTER COLUMN status SET DEFAULT 'pending';

-- CockroachDB注意：不能在刚添加约束后立即UPDATE数据
-- 标准化数据将在应用层处理或使用单独的数据迁移脚本

-- 3. 更新 gas.fee_audit 表
-- 检查并删除旧的 tx_status 列（如果存在）
ALTER TABLE gas.fee_audit DROP COLUMN IF EXISTS tx_status;

-- 添加 status 列（如果不存在）
-- CockroachDB 不支持在 ALTER TABLE ADD COLUMN 中使用复杂的 IF NOT EXISTS 逻辑
-- 使用简单的方式：先删除再添加，或者直接 ADD COLUMN IF NOT EXISTS
ALTER TABLE gas.fee_audit ADD COLUMN IF NOT EXISTS status TEXT DEFAULT 'pending' NOT NULL;

-- 删除旧约束
ALTER TABLE gas.fee_audit DROP CONSTRAINT IF EXISTS check_fee_audit_status;

-- 添加新约束
ALTER TABLE gas.fee_audit
ADD CONSTRAINT check_fee_audit_status CHECK (
    status IN ('created', 'signed', 'pending', 'executing', 'confirmed', 'failed', 'timeout', 'replaced', 'cancelled')
);

-- 4. 添加索引（优化查询性能）
-- CockroachDB注意：不能在新添加的列上立即创建partial index，移除WHERE子句
CREATE INDEX IF NOT EXISTS idx_transactions_status_created 
ON transactions(status, created_at);

CREATE INDEX IF NOT EXISTS idx_swap_transactions_status_created 
ON swap_transactions(status, created_at);

CREATE INDEX IF NOT EXISTS idx_fee_audit_status
ON gas.fee_audit(status, created_at DESC);

-- 5. 添加注释
COMMENT ON COLUMN transactions.status IS 'Transaction status: created -> signed -> pending -> executing -> confirmed/failed/timeout/replaced/cancelled (using CHECK constraint)';
COMMENT ON COLUMN swap_transactions.status IS 'Transaction status: using CHECK constraint for valid values';
COMMENT ON COLUMN gas.fee_audit.status IS 'Fee audit status: using CHECK constraint for valid values';

-- Migration completed
