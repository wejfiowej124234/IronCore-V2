-- ============================================================================
-- Migration: 0001_schemas.sql
-- Description: 创建所有需要的数据库 Schema
-- Standard: 遵循数据库最佳实践，先创建 Schema
-- ============================================================================

-- 创建所有需要的 Schema
CREATE SCHEMA IF NOT EXISTS gas;
CREATE SCHEMA IF NOT EXISTS admin;
CREATE SCHEMA IF NOT EXISTS notify;
CREATE SCHEMA IF NOT EXISTS tokens;
CREATE SCHEMA IF NOT EXISTS events;
CREATE SCHEMA IF NOT EXISTS fiat;

-- 添加注释
COMMENT ON SCHEMA gas IS '平台费用与归集相关 schema';
COMMENT ON SCHEMA admin IS '运维与控制平面 schema (RPC, 配置)';
COMMENT ON SCHEMA notify IS '通知与活动消息系统 schema';
COMMENT ON SCHEMA tokens IS '代币注册与管理 schema';
COMMENT ON SCHEMA events IS '领域事件存储 schema (Event Sourcing)';
COMMENT ON SCHEMA fiat IS '法币充值与提现系统 schema';

