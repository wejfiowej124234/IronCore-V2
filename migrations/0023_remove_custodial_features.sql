-- ============================================================================
-- Migration: 0030_remove_custodial_features.sql
-- Description: 移除所有托管化功能，确保纯非托管架构
-- Purpose: P0级安全修复 - 删除后端私钥存储能力
-- Priority: CRITICAL
-- CockroachDB Compatible: 移除 DO $$ 块
-- ============================================================================

-- Step 1: 先删除相关索引（必须在删除列之前）
DROP INDEX IF EXISTS idx_wallets_encrypted;

-- Step 2: 删除相关约束（CockroachDB要求先删除约束）
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS check_encryption_consistency;

-- Step 3: 然后删除托管相关字段
ALTER TABLE wallets 
DROP COLUMN IF EXISTS encrypted_private_key,
DROP COLUMN IF EXISTS encryption_nonce,
DROP COLUMN IF EXISTS encryption_algorithm,
DROP COLUMN IF EXISTS encryption_version;

-- Step 4: 删除相关触发器和函数（CockroachDB可能不支持）
-- CockroachDB不支持触发器，这些语句可以安全跳过
-- DROP TRIGGER IF EXISTS trigger_wallet_key_audit ON wallets;
-- DROP FUNCTION IF EXISTS log_wallet_key_access();

-- Step 5: 清理相关审计日志（删除敏感操作记录）
-- 注意：audit_logs 表现在在 0025 创建，此时已存在
-- 但是由于没有实际记录，无需清理

-- Step 6: 更新表注释
COMMENT ON TABLE wallets IS '非托管钱包表：仅存储公开地址和元数据。严格禁止存储私钥、助记词或任何敏感密钥材料。';
COMMENT ON COLUMN wallets.address IS '钱包公开地址（可公开）';
COMMENT ON COLUMN wallets.pubkey IS '公钥（可公开）';
COMMENT ON COLUMN wallets.derivation_path IS 'BIP44派生路径（可公开）';

-- Step 7: 添加简单的地址非空约束（CockroachDB Compatible）
ALTER TABLE wallets DROP CONSTRAINT IF EXISTS check_wallet_address_not_empty;
ALTER TABLE wallets
ADD CONSTRAINT check_wallet_address_not_empty 
CHECK (address IS NOT NULL AND LENGTH(address) > 0);

-- Step 8: 添加非托管声明审计日志
-- 注意：审计日志将在应用层记录

-- Migration completed successfully
