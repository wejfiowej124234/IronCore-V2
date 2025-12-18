-- ============================================================================
-- Migration: 0031_fiat_orders_non_custodial_fields.sql
-- Description: 为法币订单表添加非托管模式必需字段
-- Purpose: 支持客户端签名的链上转账交易
-- Priority: P1
-- CockroachDB Compatible: 移除 DO $$ 块
-- ============================================================================

-- 法币提现订单表添加字段
ALTER TABLE fiat_offramp_orders ADD COLUMN IF NOT EXISTS source_address TEXT;
ALTER TABLE fiat_offramp_orders ADD COLUMN IF NOT EXISTS transfer_tx_hash TEXT;

-- 添加索引（CockroachDB：移除新列上的WHERE子句）
CREATE INDEX IF NOT EXISTS idx_offramp_tx_hash 
ON fiat_offramp_orders(transfer_tx_hash);

CREATE INDEX IF NOT EXISTS idx_offramp_source_address 
ON fiat_offramp_orders(source_address, user_id);

-- 注释
COMMENT ON COLUMN fiat_offramp_orders.source_address IS '用户源地址（发送加密货币的地址）';
COMMENT ON COLUMN fiat_offramp_orders.transfer_tx_hash IS '链上转账交易哈希（客户端签名，后端广播）';

-- 跨链桥订单表添加字段
ALTER TABLE cross_chain_transactions ADD COLUMN IF NOT EXISTS signed_source_tx TEXT;

COMMENT ON COLUMN cross_chain_transactions.signed_source_tx IS '已签名的源链交易（客户端签名）';

-- CockroachDB注意：不能在新列上立即添加约束
-- 约束检查移至应用层验证

-- 审计日志（条件插入）
INSERT INTO audit_logs (event_type, resource_type, metadata, created_at)
SELECT
    'NON_CUSTODIAL_FIELDS_ADDED',
    'database',
    jsonb_build_object(
        'migration', '0031',
        'description', 'Added non-custodial fields to fiat orders tables',
        'fields_added', jsonb_build_array('source_address', 'transfer_tx_hash', 'signed_source_tx')
    ),
    CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'audit_logs')
ON CONFLICT DO NOTHING;
