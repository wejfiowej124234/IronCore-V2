-- ============================================================================
-- Migration: 0023_wallet_encrypted_private_key.sql
-- Description: 钱包表添加加密私钥字段（双锁机制）
-- Purpose: 支持托管模式，使用双锁机制安全存储私钥
-- ============================================================================

-- 添加加密私钥相关字段
ALTER TABLE wallets 
ADD COLUMN IF NOT EXISTS encrypted_private_key TEXT,
ADD COLUMN IF NOT EXISTS encryption_nonce TEXT,
ADD COLUMN IF NOT EXISTS encryption_algorithm TEXT DEFAULT 'AES-256-GCM-DUAL-LOCK',
ADD COLUMN IF NOT EXISTS encryption_version INT DEFAULT 1;

-- 添加索引（CockroachDB：不能在新列上立即创建partial index）
CREATE INDEX IF NOT EXISTS idx_wallets_encrypted 
ON wallets(user_id, chain_id);

-- CockroachDB注意：在新列上添加约束可能会导致backfill错误
-- 约束检查移至应用层验证
-- 或在生产环境单独执行：
-- ALTER TABLE wallets ADD CONSTRAINT check_encryption_consistency 
-- CHECK ((encrypted_private_key IS NULL AND encryption_nonce IS NULL) OR 
--        (encrypted_private_key IS NOT NULL AND encryption_nonce IS NOT NULL));

-- 注释
COMMENT ON COLUMN wallets.encrypted_private_key IS '加密的私钥（双锁机制：服务端密钥 + 用户密码）';
COMMENT ON COLUMN wallets.encryption_nonce IS '加密使用的Nonce（Base64编码）';
COMMENT ON COLUMN wallets.encryption_algorithm IS '加密算法标识';
COMMENT ON COLUMN wallets.encryption_version IS '加密版本号（用于未来升级）';

-- CockroachDB兼容：移除审计触发器
-- ⚠️  注意：此迁移添加的字段会在0030迁移中被删除
-- 建议：在新部署中完全跳过此迁移

-- 审计逻辑移至应用层（如需要）

