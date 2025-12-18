-- ============================================================================
-- Migration: 0008_events_tables.sql
-- Description: 创建事件总线相关表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 领域事件表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events.domain_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    published_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    retry_count INT NOT NULL DEFAULT 0,
    processed BOOL NOT NULL DEFAULT false,
    processed_at TIMESTAMPTZ,
    error_message TEXT
);

-- ----------------------------------------------------------------------------
-- 2. 事件订阅表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events.event_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    handler_name TEXT NOT NULL,
    event_types TEXT[] NOT NULL,
    active BOOL NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- 3. 事件处理失败记录表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS events.failed_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_event_id UUID NOT NULL,
    event_type TEXT NOT NULL,
    event_data JSONB NOT NULL,
    handler_name TEXT NOT NULL,
    error_message TEXT NOT NULL,
    failed_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    retry_scheduled_at TIMESTAMPTZ
);

