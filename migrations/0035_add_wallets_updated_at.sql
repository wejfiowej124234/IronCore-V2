-- ============================================================================
-- Migration: 0044_add_wallets_updated_at.sql
-- Description: 添加 updated_at 字段到 wallets 表
-- Purpose: 修复 Domain 模型与数据库 schema 不一致问题
-- Priority: P1 - 字段缺失修复
-- ============================================================================

-- Step 1: 添加 updated_at 列（如果不存在）
ALTER TABLE wallets 
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP;

-- Step 2: 为现有记录设置 updated_at = created_at
UPDATE wallets 
SET updated_at = created_at 
WHERE updated_at = CURRENT_TIMESTAMP;

-- Step 3: 添加注释
COMMENT ON COLUMN wallets.updated_at IS '钱包最后更新时间（自动更新触发器管理）';

-- ============================================================================
-- Migration completed successfully
-- ============================================================================
