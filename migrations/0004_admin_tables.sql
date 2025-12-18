-- ============================================================================
-- Migration: 0004_admin_tables.sql
-- Description: 创建管理员和RPC端点相关表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. RPC端点表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS admin.rpc_endpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    chain TEXT NOT NULL,
    url TEXT NOT NULL,
    provider TEXT,
    priority INT NOT NULL DEFAULT 100,
    healthy BOOL NOT NULL DEFAULT true,
    fail_count INT NOT NULL DEFAULT 0,
    last_fail_at TIMESTAMPTZ,
    avg_latency_ms INT NOT NULL DEFAULT 0,
    last_latency_ms INT NOT NULL DEFAULT 0,
    circuit_state TEXT NOT NULL DEFAULT 'closed',
    last_checked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 2. 管理员操作日志表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS admin.admin_operation_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    operator_user_id UUID NOT NULL,
    operator_role TEXT NOT NULL,
    action TEXT NOT NULL,
    target_ref TEXT,
    payload_hash TEXT NOT NULL,
    payload_summary TEXT,
    success BOOL NOT NULL DEFAULT true,
    error_message TEXT,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

