-- ============================================================================
-- Migration: 0012_check_constraints.sql
-- Description: 添加检查约束和数据验证
-- Standard: 遵循数据库最佳实践，添加数据完整性约束
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 交易表检查约束
-- ----------------------------------------------------------------------------
ALTER TABLE transactions
    DROP CONSTRAINT IF EXISTS chk_transactions_status;
ALTER TABLE transactions
    ADD CONSTRAINT chk_transactions_status 
    CHECK (status IN ('pending', 'confirmed', 'failed', 'dropped'));

-- ----------------------------------------------------------------------------
-- Swap交易表检查约束
-- ----------------------------------------------------------------------------
ALTER TABLE swap_transactions
    DROP CONSTRAINT IF EXISTS chk_swap_status;
ALTER TABLE swap_transactions
    ADD CONSTRAINT chk_swap_status 
    CHECK (status IN ('pending', 'executing', 'confirmed', 'failed', 'cancelled'));

ALTER TABLE swap_transactions
    DROP CONSTRAINT IF EXISTS chk_swap_confirmations;
ALTER TABLE swap_transactions
    ADD CONSTRAINT chk_swap_confirmations 
    CHECK (confirmations >= 0);

ALTER TABLE swap_transactions
    DROP CONSTRAINT IF EXISTS chk_swap_slippage;
ALTER TABLE swap_transactions
    ADD CONSTRAINT chk_swap_slippage 
    CHECK (slippage IS NULL OR (slippage >= 0 AND slippage <= 100));

-- ----------------------------------------------------------------------------
-- 代币注册表检查约束
-- ----------------------------------------------------------------------------
ALTER TABLE tokens.registry
    DROP CONSTRAINT IF EXISTS chk_decimals;
ALTER TABLE tokens.registry
    ADD CONSTRAINT chk_decimals 
    CHECK (decimals >= 0 AND decimals <= 18);

ALTER TABLE tokens.registry
    DROP CONSTRAINT IF EXISTS chk_priority;
ALTER TABLE tokens.registry
    ADD CONSTRAINT chk_priority 
    CHECK (priority >= 0);

-- ----------------------------------------------------------------------------
-- 法币系统检查约束
-- ----------------------------------------------------------------------------

-- 法币订单表
ALTER TABLE fiat.orders
    DROP CONSTRAINT IF EXISTS chk_order_type;
ALTER TABLE fiat.orders
    ADD CONSTRAINT chk_order_type 
    CHECK (order_type IN ('onramp', 'offramp'));

ALTER TABLE fiat.orders
    DROP CONSTRAINT IF EXISTS chk_fiat_status;
ALTER TABLE fiat.orders
    ADD CONSTRAINT chk_fiat_status 
    CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled', 'refunded', 'expired'));

ALTER TABLE fiat.orders
    DROP CONSTRAINT IF EXISTS chk_review_status;
ALTER TABLE fiat.orders
    ADD CONSTRAINT chk_review_status 
    CHECK (review_status IN ('auto_approved', 'pending_review', 'approved', 'rejected'));

-- 法币交易表
ALTER TABLE fiat.transactions
    DROP CONSTRAINT IF EXISTS chk_tx_type;
ALTER TABLE fiat.transactions
    ADD CONSTRAINT chk_tx_type 
    CHECK (tx_type IN ('swap', 'onramp', 'offramp'));

ALTER TABLE fiat.transactions
    DROP CONSTRAINT IF EXISTS chk_tx_status;
ALTER TABLE fiat.transactions
    ADD CONSTRAINT chk_tx_status 
    CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled'));

-- 法币对账记录表
ALTER TABLE fiat.reconciliation_records
    DROP CONSTRAINT IF EXISTS chk_reconciliation_status;
ALTER TABLE fiat.reconciliation_records
    ADD CONSTRAINT chk_reconciliation_status 
    CHECK (status IN ('pending', 'running', 'completed', 'failed'));

-- 法币告警表
ALTER TABLE fiat.alerts
    DROP CONSTRAINT IF EXISTS chk_alert_severity;
ALTER TABLE fiat.alerts
    ADD CONSTRAINT chk_alert_severity 
    CHECK (severity IN ('low', 'medium', 'high', 'critical'));

ALTER TABLE fiat.alerts
    DROP CONSTRAINT IF EXISTS chk_alert_status;
ALTER TABLE fiat.alerts
    ADD CONSTRAINT chk_alert_status 
    CHECK (status IN ('open', 'acknowledged', 'resolved', 'closed'));

-- 法币服务商表
ALTER TABLE fiat.providers
    DROP CONSTRAINT IF EXISTS chk_health_status;
ALTER TABLE fiat.providers
    ADD CONSTRAINT chk_health_status 
    CHECK (health_status IN ('healthy', 'unhealthy', 'unknown'));

