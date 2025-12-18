-- ============================================================================
-- Migration: 0005_notify_tables.sql
-- Description: 创建通知系统相关表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 通知模板表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT NOT NULL,
    channel TEXT NOT NULL,
    category TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    locale TEXT NOT NULL DEFAULT 'zh-CN',
    version INT NOT NULL DEFAULT 1,
    active BOOL NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 2. 用户通知偏好表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.user_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    notification_type TEXT NOT NULL,
    channels JSONB NOT NULL DEFAULT '[]',
    frequency TEXT NOT NULL,
    enabled BOOL NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 3. 通知实例表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_id UUID,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    category TEXT NOT NULL,
    severity TEXT NOT NULL DEFAULT 'info',
    scope TEXT NOT NULL,
    segment_expr TEXT,
    creator_role TEXT NOT NULL,
    signature TEXT,
    scheduled_at TIMESTAMPTZ,
    revoked BOOL NOT NULL DEFAULT false,
    revoked_at TIMESTAMPTZ,
    revoked_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 4. 投递记录表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    notification_id UUID NOT NULL,
    user_id UUID NOT NULL,
    channel TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    retries INT NOT NULL DEFAULT 0,
    last_attempt_at TIMESTAMPTZ,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 5. 用户端点表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.endpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    endpoint_type TEXT NOT NULL,
    token TEXT NOT NULL,
    platform TEXT,
    active BOOL NOT NULL DEFAULT true,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 6. 活动批次表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.campaigns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    total_target INT,
    created_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 7. 通知历史表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS notify.notification_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    notification_type TEXT NOT NULL,
    channel TEXT NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    sent_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    delivery_status TEXT NOT NULL DEFAULT 'pending',
    error_message TEXT
);

