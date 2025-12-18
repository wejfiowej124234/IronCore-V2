-- ============================================================================
-- Migration: 0043_fix_platform_addresses_schema.sql
-- Description: 修正 platform_addresses 表结构，使其与代码对齐
-- CockroachDB Compatible: 移除 DO $$ 块
-- ============================================================================

-- 1. 添加缺失的字段到 platform_addresses 表
ALTER TABLE platform_addresses ADD COLUMN IF NOT EXISTS current_balance DECIMAL(36, 18) DEFAULT 0;
ALTER TABLE platform_addresses ADD COLUMN IF NOT EXISTS warning_threshold DECIMAL(36, 18);
ALTER TABLE platform_addresses ADD COLUMN IF NOT EXISTS critical_threshold DECIMAL(36, 18);

-- 注意：在生产环境中，如果需要从旧列迁移数据，请使用以下SQL：
-- UPDATE platform_addresses SET warning_threshold = balance_threshold_warning WHERE warning_threshold IS NULL;
-- UPDATE platform_addresses SET critical_threshold = balance_threshold_critical WHERE critical_threshold IS NULL;
-- 
-- 当前迁移场景（重置数据库）无需迁移数据

-- 添加注释
COMMENT ON COLUMN platform_addresses.current_balance IS '当前余额：实时同步的链上余额';
COMMENT ON COLUMN platform_addresses.warning_threshold IS '预警阈值：低于此值触发预警';
COMMENT ON COLUMN platform_addresses.critical_threshold IS '临界阈值：低于此值触发紧急警报';
