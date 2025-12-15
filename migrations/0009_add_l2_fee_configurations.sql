-- ================================================================
-- 为新增链添加费用收集器地址和平台费用规则
-- 日期: 2025-12-11
-- 链: Arbitrum, Optimism, Avalanche, Solana, Bitcoin
-- ================================================================

-- ================================================================
-- 1. 费用收集器地址配置 (gas.fee_collector_addresses)
-- ================================================================

-- 删除可能存在的旧数据
DELETE FROM gas.fee_collector_addresses 
WHERE chain IN ('arbitrum', 'optimism', 'avalanche', 'solana', 'bitcoin');

INSERT INTO gas.fee_collector_addresses (chain, address, active, created_at)
VALUES 
    -- L2 Chains (EVM 兼容，使用以太坊地址格式)
    -- ⚠️ 注意：这些是示例地址，生产环境请替换为实际的多签钱包地址
    ('arbitrum', '0x1234567890123456789012345678901234567890', true, CURRENT_TIMESTAMP),
    ('optimism', '0x2345678901234567890123456789012345678901', true, CURRENT_TIMESTAMP),
    ('avalanche', '0x3456789012345678901234567890123456789012', true, CURRENT_TIMESTAMP),
    
    -- Non-EVM Chains
    -- ⚠️ 注意：这些是示例地址，生产环境请替换为实际的多签钱包地址
    ('solana', '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM', true, CURRENT_TIMESTAMP),
    ('bitcoin', 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh', true, CURRENT_TIMESTAMP);

-- ================================================================
-- 2. 平台费用规则配置 (gas.platform_fee_rules)
-- ================================================================

-- 删除可能存在的旧规则
DELETE FROM gas.platform_fee_rules 
WHERE chain IN ('arbitrum', 'optimism', 'avalanche', 'solana', 'bitcoin');

-- 2.1 Arbitrum 费用规则（L2 链，费用更低）
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type, percent_bp, min_fee, max_fee, 
    priority, active, effective_at, created_at
) VALUES 
    ('arbitrum', 'transfer', 'percent', 5, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('arbitrum', 'swap', 'percent', 30, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('arbitrum', 'limit_order', 'percent', 50, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('arbitrum', 'bridge', 'percent', 80, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('arbitrum', 'fiat_onramp', 'percent', 150, 1.0, 100.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('arbitrum', 'fiat_offramp', 'percent', 200, 2.0, 150.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

-- 2.2 Optimism 费用规则（L2 链，费用最低）
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type, percent_bp, min_fee, max_fee, 
    priority, active, effective_at, created_at
) VALUES 
    ('optimism', 'transfer', 'percent', 5, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('optimism', 'swap', 'percent', 30, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('optimism', 'limit_order', 'percent', 50, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('optimism', 'bridge', 'percent', 80, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('optimism', 'fiat_onramp', 'percent', 150, 1.0, 100.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('optimism', 'fiat_offramp', 'percent', 200, 2.0, 150.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

-- 2.3 Avalanche 费用规则（C-Chain，类似 Ethereum）
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type, percent_bp, min_fee, max_fee, 
    priority, active, effective_at, created_at
) VALUES 
    ('avalanche', 'transfer', 'percent', 10, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('avalanche', 'swap', 'percent', 50, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('avalanche', 'limit_order', 'percent', 50, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('avalanche', 'bridge', 'percent', 100, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('avalanche', 'fiat_onramp', 'percent', 200, 1.0, 100.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('avalanche', 'fiat_offramp', 'percent', 250, 2.0, 150.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

-- 2.4 Solana 费用规则（非 EVM，费用极低）
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type, percent_bp, min_fee, max_fee, 
    priority, active, effective_at, created_at
) VALUES 
    ('solana', 'transfer', 'percent', 5, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('solana', 'swap', 'percent', 30, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('solana', 'limit_order', 'percent', 50, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('solana', 'bridge', 'percent', 80, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('solana', 'fiat_onramp', 'percent', 150, 1.0, 100.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('solana', 'fiat_offramp', 'percent', 200, 2.0, 150.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

-- 2.5 Bitcoin 费用规则（UTXO 模型，仅支持转账和桥接）
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type, percent_bp, min_fee, max_fee, 
    priority, active, effective_at, created_at
) VALUES 
    ('bitcoin', 'transfer', 'percent', 20, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('bitcoin', 'bridge', 'percent', 120, 0.0, NULL, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('bitcoin', 'fiat_onramp', 'percent', 200, 5.0, 500.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
    ('bitcoin', 'fiat_offramp', 'percent', 250, 5.0, 500.0, 100, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);

-- ================================================================
-- 3. 验证插入结果
-- ================================================================

-- 查看新增的费用收集器地址
SELECT 
    chain, 
    address, 
    active, 
    created_at 
FROM gas.fee_collector_addresses 
WHERE chain IN ('arbitrum', 'optimism', 'avalanche', 'solana', 'bitcoin')
ORDER BY chain;

-- 查看新增的平台费用规则
SELECT 
    chain, 
    operation, 
    percent_bp, 
    CAST(percent_bp AS FLOAT) / 100 AS percent,
    min_fee, 
    max_fee, 
    active 
FROM gas.platform_fee_rules 
WHERE chain IN ('arbitrum', 'optimism', 'avalanche', 'solana', 'bitcoin')
    AND active = true
ORDER BY chain, operation;

-- 统计每条链的费用规则数量
SELECT 
    chain, 
    COUNT(*) as rule_count
FROM gas.platform_fee_rules 
WHERE active = true
GROUP BY chain
ORDER BY chain;
