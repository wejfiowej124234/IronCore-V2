-- ============================================================================
-- Migration: 0037_database_constraints_enhancement.sql
-- Description: 数据库约束增强（K项深度优化）
-- Purpose: 添加缺失的唯一约束、检查约束、外键约束
-- CockroachDB Compatible: 移除所有 DO $$ 块
-- ============================================================================

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 1: Swap交易表约束
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- 添加唯一约束：swap_id必须唯一（幂等性保护）
ALTER TABLE swap_transactions DROP CONSTRAINT IF EXISTS unique_swap_id;
ALTER TABLE swap_transactions ADD CONSTRAINT unique_swap_id UNIQUE (swap_id);

-- 添加检查约束：金额必须大于0
ALTER TABLE swap_transactions DROP CONSTRAINT IF EXISTS check_swap_amount_positive;
ALTER TABLE swap_transactions
ADD CONSTRAINT check_swap_amount_positive 
CHECK (from_amount > 0);

-- 添加检查约束：滑点范围0-100%
ALTER TABLE swap_transactions DROP CONSTRAINT IF EXISTS check_swap_slippage_range;
ALTER TABLE swap_transactions
ADD CONSTRAINT check_swap_slippage_range
CHECK (slippage IS NULL OR (slippage >= 0 AND slippage <= 100));

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 2: 交易表约束
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- 添加检查约束：nonce不能为负
ALTER TABLE transactions DROP CONSTRAINT IF EXISTS check_tx_nonce_non_negative;
ALTER TABLE transactions
ADD CONSTRAINT check_tx_nonce_non_negative
CHECK (nonce IS NULL OR nonce >= 0);

-- 添加索引：tx_hash唯一性（同一链上）
CREATE UNIQUE INDEX IF NOT EXISTS unique_tx_hash_per_chain
ON transactions(tx_hash, chain)
WHERE tx_hash IS NOT NULL;

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 3: 钱包表约束
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- 添加唯一约束：同一用户同一链的地址唯一
CREATE UNIQUE INDEX IF NOT EXISTS unique_wallet_per_user_chain
ON wallets(user_id, chain_id, address);

-- 添加检查约束：account_index和address_index非负
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS check_wallet_indexes_non_negative;
ALTER TABLE wallets
ADD CONSTRAINT check_wallet_indexes_non_negative
CHECK (account_index >= 0 AND address_index >= 0);

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 4: Nonce追踪表约束
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- 注意：nonce_tracking表在0032迁移中已重构
-- 列名已变更：chain → chain_symbol, last_nonce → nonce
-- 唯一约束unique_nonce_per_chain_address已在0032中创建，无需再次添加

-- 添加nonce非负检查约束
ALTER TABLE nonce_tracking DROP CONSTRAINT IF EXISTS check_nonce_non_negative;
ALTER TABLE nonce_tracking ADD CONSTRAINT check_nonce_non_negative CHECK (nonce >= 0);

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 5: 跨链交易表约束
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- 添加检查约束：源链和目标链不能相同
ALTER TABLE cross_chain_transactions DROP CONSTRAINT IF EXISTS check_different_chains;
ALTER TABLE cross_chain_transactions
ADD CONSTRAINT check_different_chains
CHECK (source_chain != destination_chain);

-- 添加检查约束：确认数非负
ALTER TABLE cross_chain_transactions DROP CONSTRAINT IF EXISTS check_confirmations_non_negative;
ALTER TABLE cross_chain_transactions
ADD CONSTRAINT check_confirmations_non_negative
CHECK (
    (source_confirmations IS NULL OR source_confirmations >= 0)
    AND (destination_confirmations IS NULL OR destination_confirmations >= 0)
);

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 6: 用户表约束（生产环境清理）
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

-- 添加注释提醒
COMMENT ON COLUMN users.email IS '开发环境使用，生产环境应删除此列，只使用email_cipher';
COMMENT ON COLUMN users.phone IS '开发环境使用，生产环境应删除此列，只使用phone_cipher';

-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
-- Step 7: 审计日志
-- ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
SELECT
    'DATABASE_CONSTRAINTS_ENHANCED',
    'system',
    jsonb_build_object(
        'migration', '0037_database_constraints_enhancement_cockroachdb',
        'description', 'Enhanced database constraints for data integrity (CockroachDB compatible)',
        'constraints_added', jsonb_build_array(
            'unique_swap_id',
            'unique_wallet_per_user_chain',
            'check_swap_amount_positive',
            'check_nonce_non_negative',
            'check_different_chains'
        ),
        'cockroachdb_compatible', true
    ),
    CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs')
ON CONFLICT DO NOTHING;
