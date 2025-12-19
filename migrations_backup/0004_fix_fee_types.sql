-- ============================================================================
-- Migration: 0004_fix_fee_types.sql
-- Description: 修复费用表类型不匹配问题 - 重建表结构
-- ============================================================================

-- 1. 创建临时表存储现有数据
CREATE TABLE IF NOT EXISTS gas.platform_fee_rules_backup AS 
SELECT * FROM gas.platform_fee_rules;

-- 2. 删除旧表
DROP TABLE gas.platform_fee_rules;

-- 3. 重建表使用正确的类型 (FLOAT8 而非 DECIMAL, INT 而非 BIGINT)
CREATE TABLE gas.platform_fee_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    operation TEXT NOT NULL,
    fee_type TEXT NOT NULL,
    flat_amount FLOAT8 NOT NULL DEFAULT 0,
    percent_bp INT NOT NULL DEFAULT 0,
    min_fee FLOAT8 NOT NULL DEFAULT 0,
    max_fee FLOAT8,
    priority INT NOT NULL DEFAULT 100,
    rule_version INT NOT NULL DEFAULT 1,
    active BOOL NOT NULL DEFAULT true,
    effective_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 4. 恢复数据 (DECIMAL/BIGINT 自动转换为 FLOAT8/INT)
INSERT INTO gas.platform_fee_rules
SELECT 
    id, chain, operation, fee_type,
    flat_amount::FLOAT8, percent_bp::INT, min_fee::FLOAT8, max_fee::FLOAT8,
    priority, rule_version, active, effective_at, created_at, updated_at
FROM gas.platform_fee_rules_backup;

-- 5. 删除备份表
DROP TABLE gas.platform_fee_rules_backup;

-- 6. 添加索引 (优化查询性能)
CREATE INDEX IF NOT EXISTS idx_fee_rules_lookup 
ON gas.platform_fee_rules(chain, operation, active, effective_at);

-- 7. 添加注释
COMMENT ON TABLE gas.platform_fee_rules IS '平台费用规则表 (使用 FLOAT8/INT 类型兼容 Rust f64/i32)';
COMMENT ON COLUMN gas.platform_fee_rules.flat_amount IS '固定费用金额 (FLOAT8)';
COMMENT ON COLUMN gas.platform_fee_rules.percent_bp IS '费率基点 (INT, 100 = 1%)';
COMMENT ON COLUMN gas.platform_fee_rules.min_fee IS '最小费用 (FLOAT8)';
COMMENT ON COLUMN gas.platform_fee_rules.max_fee IS '最大费用 (FLOAT8)';
