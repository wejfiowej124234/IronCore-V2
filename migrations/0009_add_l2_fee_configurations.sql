-- ================================================================
-- 为新增链添加费用收集器地址和平台费用规则
-- 日期: 2025-12-11
-- 链: Arbitrum, Optimism, Avalanche, Solana, Bitcoin
-- ================================================================

-- ================================================================
-- 1. 费用收集器地址配置 (gas.fee_collector_addresses)
-- ================================================================

-- NOTE: 生产环境可能已配置真实的多签地址；该迁移仅在缺失时插入默认值，不删除已有配置。
INSERT INTO gas.fee_collector_addresses (chain, address, active, created_at)
SELECT v.chain, v.address, v.active, v.created_at
FROM (
    VALUES
    -- L2 Chains (EVM 兼容，使用以太坊地址格式)
    -- ⚠️ 注意：这些是示例地址，生产环境请替换为实际的多签钱包地址
        ('arbitrum', '0x1234567890123456789012345678901234567890', true, CURRENT_TIMESTAMP),
        ('optimism', '0x2345678901234567890123456789012345678901', true, CURRENT_TIMESTAMP),
        ('avalanche', '0x3456789012345678901234567890123456789012', true, CURRENT_TIMESTAMP),
    
    -- Non-EVM Chains
    -- ⚠️ 注意：这些是示例地址，生产环境请替换为实际的多签钱包地址
        ('solana', '9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM', true, CURRENT_TIMESTAMP),
        ('bitcoin', 'bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh', true, CURRENT_TIMESTAMP)
) AS v(chain, address, active, created_at)
WHERE NOT EXISTS (
    SELECT 1
    FROM gas.fee_collector_addresses a
    WHERE a.chain = v.chain
      AND a.address = v.address
);

-- ================================================================
-- 2. 平台费用规则配置 (gas.platform_fee_rules)
-- ================================================================

-- NOTE: 仅在缺失时插入默认规则，不删除已有规则。

INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    percent_bp, min_fee, max_fee,
    priority, active, effective_at, created_at
)
SELECT
    v.chain, v.operation, v.fee_type,
    v.percent_bp, v.min_fee, v.max_fee,
    v.priority, v.active, v.effective_at, v.created_at
FROM (
    VALUES
        -- Arbitrum
        ('arbitrum', 'transfer',   'percent', 5::INT,   0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('arbitrum', 'swap',       'percent', 30::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('arbitrum', 'limit_order','percent', 50::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('arbitrum', 'bridge',     'percent', 80::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('arbitrum', 'fiat_onramp','percent', 150::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('arbitrum', 'fiat_offramp','percent',200::INT, 2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),

        -- Optimism
        ('optimism', 'transfer',   'percent', 5::INT,   0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('optimism', 'swap',       'percent', 30::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('optimism', 'limit_order','percent', 50::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('optimism', 'bridge',     'percent', 80::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('optimism', 'fiat_onramp','percent', 150::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('optimism', 'fiat_offramp','percent',200::INT, 2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),

        -- Avalanche
        ('avalanche', 'transfer',  'percent', 10::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('avalanche', 'swap',      'percent', 50::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('avalanche', 'limit_order','percent',50::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('avalanche', 'bridge',    'percent', 100::INT, 0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('avalanche', 'fiat_onramp','percent',200::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('avalanche', 'fiat_offramp','percent',250::INT,2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),

        -- Solana
        ('solana', 'transfer',     'percent', 5::INT,   0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('solana', 'swap',         'percent', 30::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('solana', 'limit_order',  'percent', 50::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('solana', 'bridge',       'percent', 80::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('solana', 'fiat_onramp',  'percent', 150::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('solana', 'fiat_offramp', 'percent', 200::INT, 2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),

        -- Bitcoin
        ('bitcoin', 'transfer',    'percent', 20::INT,  0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('bitcoin', 'bridge',      'percent', 120::INT, 0.0::DECIMAL(36,18), NULL::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('bitcoin', 'fiat_onramp', 'percent', 200::INT, 5.0::DECIMAL(36,18), 500.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP),
        ('bitcoin', 'fiat_offramp','percent', 250::INT, 5.0::DECIMAL(36,18), 500.0::DECIMAL(36,18), 100::INT, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
) AS v(chain, operation, fee_type, percent_bp, min_fee, max_fee, priority, active, effective_at, created_at)
WHERE NOT EXISTS (
    SELECT 1
    FROM gas.platform_fee_rules r
    WHERE r.chain = v.chain
      AND r.operation = v.operation
      AND r.fee_type = v.fee_type
      AND r.active = true
);

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
