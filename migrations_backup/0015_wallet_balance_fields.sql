-- ============================================================================
-- Migration: 0015_wallet_balance_fields.sql
-- Description: 为wallets表添加余额缓存字段
-- ============================================================================

-- 添加余额缓存字段到wallets表
ALTER TABLE IF EXISTS wallets 
ADD COLUMN IF NOT EXISTS balance DECIMAL(36, 18),
ADD COLUMN IF NOT EXISTS balance_updated_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS chain TEXT;

-- 创建索引以优化余额查询（CockroachDB不支持在新列上立即创建partial index）
CREATE INDEX IF NOT EXISTS idx_wallets_balance_update ON wallets(balance_updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_wallets_chain_address ON wallets(chain, address);

-- CockroachDB注意：不能在添加列后立即UPDATE（backfill问题）
-- chain字段将在应用层首次使用时填充
-- 或在生产环境中使用单独的数据迁移脚本

COMMENT ON COLUMN wallets.balance IS '余额缓存（原生代币余额，如ETH/BNB/SOL等）';
COMMENT ON COLUMN wallets.balance_updated_at IS '余额最后更新时间';
COMMENT ON COLUMN wallets.chain IS '链标识（小写，如ethereum/bsc/solana等）';

