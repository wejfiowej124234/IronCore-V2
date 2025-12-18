-- ============================================================================
-- Migration: 0041_fiat_orders_unified_view.sql
-- Description: 创建统一的法币订单视图（兼容旧代码）
-- ============================================================================

-- 创建统一视图：合并充值和提现订单
CREATE OR REPLACE VIEW fiat_orders AS
SELECT 
    id,
    user_id,
    tenant_id,
    fiat_amount,
    fiat_currency,
    crypto_amount,
    crypto_currency,
    target_chain AS chain,
    wallet_address AS address,
    status,
    'onramp' AS order_type,
    payment_method,
    payment_id,
    created_at,
    updated_at,
    completed_at
FROM fiat_onramp_orders
UNION ALL
SELECT 
    id,
    user_id,
    tenant_id,
    fiat_amount,
    fiat_currency,
    crypto_amount,
    crypto_currency,
    source_chain AS chain,
    NULL AS address,
    status,
    'offramp' AS order_type,
    NULL AS payment_method,
    NULL AS payment_id,
    created_at,
    updated_at,
    completed_at
FROM fiat_offramp_orders;

-- CockroachDB不支持COMMENT ON VIEW，注释移至SQL文件顶部
-- 统一法币订单视图：合并充值和提现订单（兼容层）

