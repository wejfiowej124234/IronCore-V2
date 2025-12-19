-- 插入平台服务费规则配置
-- 基于行业标准：参考 MetaMask, Trust Wallet, Coinbase Wallet 等非托管钱包

-- NOTE: 该迁移需要在已有生产数据的数据库上可安全执行（不会重复插入）。
-- 规则按 (chain, operation, fee_type, active) 做“存在则跳过”的逐行检查。

-- 规则集合（全量），逐行“若不存在则插入”
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
)
SELECT
    v.chain, v.operation, v.fee_type,
    v.flat_amount, v.percent_bp, v.min_fee, v.max_fee,
    v.priority, v.active
FROM (
    VALUES
        -- 1. 代币交换 (Swap) - 0.5%
        ('ethereum', 'swap', 'percent', 0::DECIMAL(36,18), 50::INT, 0.001::DECIMAL(36,18), 0.05::DECIMAL(36,18), 100::INT, true),
        ('bsc',      'swap', 'percent', 0::DECIMAL(36,18), 50::INT, 0.01::DECIMAL(36,18),  0.5::DECIMAL(36,18),  100::INT, true),
        ('polygon',  'swap', 'percent', 0::DECIMAL(36,18), 50::INT, 0.1::DECIMAL(36,18),   5.0::DECIMAL(36,18),  100::INT, true),

        -- 2. 基础转账 (Transfer) - 0.1%
        ('ethereum', 'transfer', 'percent', 0::DECIMAL(36,18), 10::INT, 0.0001::DECIMAL(36,18), 0.01::DECIMAL(36,18), 100::INT, true),
        ('bsc',      'transfer', 'percent', 0::DECIMAL(36,18), 10::INT, 0.001::DECIMAL(36,18),  0.1::DECIMAL(36,18),  100::INT, true),
        ('polygon',  'transfer', 'percent', 0::DECIMAL(36,18), 10::INT, 0.01::DECIMAL(36,18),   1.0::DECIMAL(36,18),  100::INT, true),

        -- 3. 法币入金 (Fiat Onramp) - 2.0%
        ('ethereum', 'fiat_onramp', 'percent', 0::DECIMAL(36,18), 200::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true),
        ('bsc',      'fiat_onramp', 'percent', 0::DECIMAL(36,18), 200::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true),
        ('polygon',  'fiat_onramp', 'percent', 0::DECIMAL(36,18), 200::INT, 1.0::DECIMAL(36,18), 100.0::DECIMAL(36,18), 100::INT, true),

        -- 4. 法币出金 (Fiat Offramp) - 2.5%
        ('ethereum', 'fiat_offramp', 'percent', 0::DECIMAL(36,18), 250::INT, 2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true),
        ('bsc',      'fiat_offramp', 'percent', 0::DECIMAL(36,18), 250::INT, 2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true),
        ('polygon',  'fiat_offramp', 'percent', 0::DECIMAL(36,18), 250::INT, 2.0::DECIMAL(36,18), 150.0::DECIMAL(36,18), 100::INT, true),

        -- 5. 限价单 (Limit Order) - 0.5%
        ('ethereum', 'limit_order', 'percent', 0::DECIMAL(36,18), 50::INT, 0.001::DECIMAL(36,18), 0.05::DECIMAL(36,18), 100::INT, true),
        ('bsc',      'limit_order', 'percent', 0::DECIMAL(36,18), 50::INT, 0.01::DECIMAL(36,18),  0.5::DECIMAL(36,18),  100::INT, true),
        ('polygon',  'limit_order', 'percent', 0::DECIMAL(36,18), 50::INT, 0.1::DECIMAL(36,18),   5.0::DECIMAL(36,18),  100::INT, true),

        -- 6. 跨链桥 (Bridge) - 1.0%
        ('ethereum', 'bridge', 'percent', 0::DECIMAL(36,18), 100::INT, 0.005::DECIMAL(36,18), 0.1::DECIMAL(36,18), 100::INT, true),
        ('bsc',      'bridge', 'percent', 0::DECIMAL(36,18), 100::INT, 0.05::DECIMAL(36,18),  1.0::DECIMAL(36,18),  100::INT, true),
        ('polygon',  'bridge', 'percent', 0::DECIMAL(36,18), 100::INT, 0.5::DECIMAL(36,18),   10.0::DECIMAL(36,18), 100::INT, true)
) AS v(chain, operation, fee_type, flat_amount, percent_bp, min_fee, max_fee, priority, active)
WHERE NOT EXISTS (
    SELECT 1
    FROM gas.platform_fee_rules r
    WHERE r.chain = v.chain
      AND r.operation = v.operation
      AND r.fee_type = v.fee_type
      AND r.active = true
);
