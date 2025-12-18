-- ============================================================================
-- Migration: 0040_audit_logs_global_table.sql
-- Description: 创建全局审计日志表（位于 public schema）
-- ============================================================================

-- 创建全局审计日志表
CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    user_id UUID,
    tenant_id UUID,
    metadata JSONB,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_audit_logs_event_type ON audit_logs(event_type, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_resource ON audit_logs(resource_type, resource_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user ON audit_logs(user_id, created_at DESC) WHERE user_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_tenant ON audit_logs(tenant_id, created_at DESC) WHERE tenant_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);

-- 注释
COMMENT ON TABLE audit_logs IS '全局审计日志：记录所有关键操作';
COMMENT ON COLUMN audit_logs.event_type IS '事件类型：wallet_created, transaction_signed, etc.';
COMMENT ON COLUMN audit_logs.resource_type IS '资源类型：wallet, transaction, user, etc.';
COMMENT ON COLUMN audit_logs.metadata IS '事件元数据（JSON格式）';

