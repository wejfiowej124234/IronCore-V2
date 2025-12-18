-- ============================================================================
-- Migration: 0034_broadcast_queue_table.sql
-- Description: 创建交易广播队列表（持久化）
-- Purpose: 支持服务重启后恢复pending的广播任务
-- Priority: P2优化
-- CockroachDB Compatible: 移除 DO $$ 块
-- ============================================================================

CREATE TABLE IF NOT EXISTS broadcast_queue (
    id UUID PRIMARY KEY,
    chain TEXT NOT NULL,
    signed_tx TEXT NOT NULL,  -- 已签名交易（客户端签名）
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    retry_count INT NOT NULL DEFAULT 0,
    max_retries INT NOT NULL DEFAULT 3,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending/broadcasting/success/failed/cancelled
    error_message TEXT,
    next_retry_at TIMESTAMPTZ,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 索引：查询待广播项目
CREATE INDEX IF NOT EXISTS idx_broadcast_queue_status_retry 
ON broadcast_queue(status, next_retry_at) 
WHERE status = 'pending';

-- 索引：清理旧记录
CREATE INDEX IF NOT EXISTS idx_broadcast_queue_created 
ON broadcast_queue(created_at) 
WHERE status = 'success';

-- 索引：用户查询
CREATE INDEX IF NOT EXISTS idx_broadcast_queue_user 
ON broadcast_queue(user_id, created_at DESC);

-- 注释
COMMENT ON TABLE broadcast_queue IS '交易广播队列：持久化待广播交易，支持服务重启恢复';
COMMENT ON COLUMN broadcast_queue.signed_tx IS '已签名交易（客户端签名，非托管）';
COMMENT ON COLUMN broadcast_queue.retry_count IS '当前重试次数';
COMMENT ON COLUMN broadcast_queue.next_retry_at IS '下次重试时间（指数退避：2min, 4min, 8min）';

-- 审计日志（条件插入）
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
SELECT
    'MIGRATION_BROADCAST_QUEUE_CREATED',
    'system',
    jsonb_build_object(
        'migration', '0034_broadcast_queue_table',
        'description', 'Created broadcast queue table for persistent transaction broadcasting',
        'features', jsonb_build_array('retry_mechanism', 'exponential_backoff', 'graceful_recovery')
    ),
    CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs')
ON CONFLICT DO NOTHING;
