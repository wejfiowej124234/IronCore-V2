-- ============================================================================
-- Migration: 0035_wallet_unlock_tokens.sql
-- Description: 钱包解锁令牌表（双锁机制增强）
-- Purpose: 后端验证客户端已解锁钱包（B项修复）
-- CockroachDB Compatible: 移除 DO $$ 块
-- ============================================================================

-- 钱包解锁令牌表
CREATE TABLE IF NOT EXISTS wallet_unlock_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    wallet_id TEXT NOT NULL,
    unlock_token TEXT NOT NULL,     -- 服务端生成的随机令牌（64字符hex）
    unlock_proof TEXT NOT NULL,     -- 客户端生成的证明（32+字符）
    expires_at TIMESTAMPTZ NOT NULL, -- 过期时间（15分钟）
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, wallet_id)  -- 每个用户每个钱包只有一个有效token
);

-- 索引：快速查询（CockroachDB不支持CURRENT_TIMESTAMP在WHERE子句）
CREATE INDEX IF NOT EXISTS idx_wallet_unlock_tokens_lookup
ON wallet_unlock_tokens(user_id, wallet_id, expires_at);

-- 索引：清理过期token（应用层过滤过期时间）
CREATE INDEX IF NOT EXISTS idx_wallet_unlock_tokens_expired
ON wallet_unlock_tokens(expires_at);

-- 注释
COMMENT ON TABLE wallet_unlock_tokens IS '钱包解锁令牌：验证客户端已使用钱包密码解锁（双锁机制）';
COMMENT ON COLUMN wallet_unlock_tokens.unlock_token IS '服务端生成的随机令牌，有效期15分钟';
COMMENT ON COLUMN wallet_unlock_tokens.unlock_proof IS '客户端生成的证明，验证客户端确实解锁了钱包';
COMMENT ON COLUMN wallet_unlock_tokens.expires_at IS '过期时间，15分钟自动失效';

-- CockroachDB兼容：移除函数，改为应用层定时任务
-- 应用层可以执行简单的DELETE查询清理过期token：
-- DELETE FROM wallet_unlock_tokens WHERE expires_at < CURRENT_TIMESTAMP - INTERVAL '1 hour';

-- 审计日志（条件插入）
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
SELECT
    'WALLET_UNLOCK_TOKENS_TABLE_CREATED',
    'system',
    jsonb_build_object(
        'migration', '0035_wallet_unlock_tokens',
        'description', 'Created wallet unlock tokens table for dual-lock mechanism (CockroachDB compatible)',
        'purpose', 'Backend verification of client-side wallet unlock',
        'security_feature', 'dual_lock_system',
        'cockroachdb_compatible', true
    ),
    CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs')
ON CONFLICT DO NOTHING;
