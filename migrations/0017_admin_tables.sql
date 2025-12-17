-- ============================================================================
-- Migration: 0017_admin_tables.sql
-- Description: 创建管理员和RPC端点相关表（补齐历史重复版本号问题）
-- Note: 该迁移用于替代历史的 0004_admin_tables.sql（已改为 .sql.skip）
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

-- RPC端点：链+URL唯一（CockroachDB兼容，使用唯一索引代替 UNIQUE CONSTRAINT）
CREATE UNIQUE INDEX IF NOT EXISTS uq_rpc_endpoint
ON admin.rpc_endpoints(chain, url);

-- RPC端点索引（提升健康探测/选择性能）
CREATE INDEX IF NOT EXISTS idx_rpc_endpoints_chain_health ON admin.rpc_endpoints(chain, healthy, priority);
CREATE INDEX IF NOT EXISTS idx_rpc_endpoints_chain_circuit ON admin.rpc_endpoints(chain, circuit_state);
CREATE INDEX IF NOT EXISTS idx_rpc_endpoints_chain ON admin.rpc_endpoints(chain);
CREATE INDEX IF NOT EXISTS idx_rpc_endpoints_healthy ON admin.rpc_endpoints(healthy) WHERE healthy = true;

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

-- 管理员操作日志索引
CREATE INDEX IF NOT EXISTS idx_admin_log_operator ON admin.admin_operation_log(operator_user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_log_action ON admin.admin_operation_log(action, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_log_target ON admin.admin_operation_log(target_ref) WHERE target_ref IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_admin_op_log_user ON admin.admin_operation_log(operator_user_id);
CREATE INDEX IF NOT EXISTS idx_admin_op_log_created ON admin.admin_operation_log(created_at DESC);
