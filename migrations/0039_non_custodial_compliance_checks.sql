-- ============================================================================
-- Migration: 0039_non_custodial_compliance_checks.sql
-- Description: 非托管合规性检查和防御机制
-- Purpose: P3级修复 - 数据库层面强制非托管模式
-- Priority: P3 HIGH
-- CockroachDB Compatible: 简化验证逻辑，移除复杂的 DO $$ 块
-- ============================================================================

-- Step 1: 添加非托管合规检查约束
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS check_non_custodial_mode;
ALTER TABLE wallets
ADD CONSTRAINT check_non_custodial_mode 
CHECK (address IS NOT NULL AND LENGTH(address) > 0);

-- Step 2: 创建简化的合规性检查视图（替代复杂函数）
CREATE OR REPLACE VIEW non_custodial_compliance_check AS
SELECT 
    'Database Schema' AS category,
    'Wallets table has no custodial columns' AS check_item,
    CASE 
        WHEN NOT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name = 'wallets'
            AND (column_name ILIKE '%private_key%' OR column_name ILIKE '%mnemonic%')
        ) THEN 'PASS'
        ELSE 'FAIL'
    END AS status,
    'No sensitive key material stored in database' AS details
UNION ALL
SELECT 
    'Database Constraints',
    'Non-custodial constraints enabled',
    CASE 
        WHEN EXISTS (
            SELECT 1 FROM pg_constraint
            WHERE conname LIKE '%non_custodial%'
        ) THEN 'PASS'
        ELSE 'WARN'
    END,
    'Database enforces non-custodial rules'
UNION ALL
SELECT 
    'Data Integrity',
    'All wallets have valid addresses',
    CASE 
        WHEN NOT EXISTS (
            SELECT 1 FROM wallets
            WHERE address IS NULL OR LENGTH(address) = 0
        ) THEN 'PASS'
        ELSE 'FAIL'
    END,
    'Every wallet has a client-derived address'
UNION ALL
SELECT 
    'Dual Lock System',
    'Wallet unlock tokens table exists',
    CASE 
        WHEN EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_name = 'wallet_unlock_tokens'
        ) THEN 'PASS'
        ELSE 'WARN'
    END,
    'Supports wallet lock mechanism';

-- Step 3: 记录迁移（条件插入）
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
SELECT
    'NON_CUSTODIAL_COMPLIANCE_CHECKS_APPLIED',
    'system',
    jsonb_build_object(
        'migration', '0039_non_custodial_compliance_checks_cockroachdb',
        'description', 'Added non-custodial compliance checks (CockroachDB compatible)',
        'security_level', 'P3_HIGH',
        'cockroachdb_compatible', true
    ),
    CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs')
ON CONFLICT DO NOTHING;

-- Migration completed
