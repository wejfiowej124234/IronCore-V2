-- ============================================
-- 平台费率修正SQL脚本（对标行业最佳实践）
-- 日期: 2025-12-11
-- 目的: 降低费率至行业竞争水平，添加封顶保护
-- ============================================

BEGIN;

-- ========== 第1步：备份当前配置 ==========
CREATE TABLE IF NOT EXISTS gas.platform_fee_rules_backup_20251211 AS 
SELECT * FROM gas.platform_fee_rules;

COMMENT ON TABLE gas.platform_fee_rules_backup_20251211 IS '费率修正前备份 - 2025-12-11';

-- ========== 第2步：修正费率配置 ==========

-- 2.1 取消转账费（对标MetaMask/Trust Wallet - 免费）
UPDATE gas.platform_fee_rules 
SET 
    percent_bp = 0,          -- 0%
    min_fee = 0,
    max_fee = 0,
    active = true,
    updated_at = NOW()
WHERE operation = 'transfer'
  AND active = true;

COMMENT ON COLUMN gas.platform_fee_rules.percent_bp IS '费率（基点），100bp = 1%。转账已改为0（免费）';

-- 2.2 Swap费率保持0.5%，但添加封顶$100（保护大额用户）
UPDATE gas.platform_fee_rules 
SET 
    percent_bp = 50,         -- 0.5% (保持不变)
    min_fee = 1.0,           -- 最低$1
    max_fee = 100.0,         -- 封顶$100（新增）
    updated_at = NOW()
WHERE operation = 'swap'
  AND active = true;

-- 2.3 限价单费率保持0.5%，添加封顶$100
UPDATE gas.platform_fee_rules 
SET 
    percent_bp = 50,         -- 0.5%
    min_fee = 1.0,
    max_fee = 100.0,         -- 封顶$100
    updated_at = NOW()
WHERE operation = 'limit_order'
  AND active = true;

-- 2.4 跨链桥费率：1.0% → 0.3%，封顶$50
UPDATE gas.platform_fee_rules 
SET 
    percent_bp = 30,         -- 1.0% → 0.3%
    min_fee = 2.0,
    max_fee = 50.0,          -- 封顶$50
    updated_at = NOW()
WHERE operation = 'bridge'
  AND active = true;

-- 2.5 法币入金：2.0% → 0.3%（从供应商返佣获利），封顶$50
UPDATE gas.platform_fee_rules 
SET 
    percent_bp = 30,         -- 2.0% → 0.3%
    min_fee = 1.0,
    max_fee = 50.0,          -- 封顶$50
    updated_at = NOW()
WHERE operation = 'fiat_onramp'
  AND active = true;

-- 2.6 法币出金：2.5% → 0.5%，封顶$50（关键修正）
UPDATE gas.platform_fee_rules 
SET 
    percent_bp = 50,         -- 2.5% → 0.5% (降低80%)
    min_fee = 2.0,
    max_fee = 50.0,          -- 封顶$50（保护大额用户）
    updated_at = NOW()
WHERE operation = 'fiat_offramp'
  AND active = true;

-- ========== 第3步：验证修正结果 ==========

-- 验证所有链的费率配置
SELECT 
    chain,
    operation,
    ROUND(percent_bp / 100.0, 2) as fee_percent,
    min_fee,
    max_fee,
    CASE 
        WHEN max_fee IS NOT NULL THEN CONCAT(ROUND(percent_bp / 100.0, 2), '% (封顶$', max_fee, ')')
        ELSE CONCAT(ROUND(percent_bp / 100.0, 2), '%')
    END as display_rate
FROM gas.platform_fee_rules
WHERE active = true
ORDER BY chain, operation;

-- 对比修正前后（关键操作）
SELECT 
    '修正前' as version,
    operation,
    AVG(percent_bp) / 100.0 as avg_fee_percent,
    COUNT(*) as rule_count
FROM gas.platform_fee_rules_backup_20251211
WHERE operation IN ('transfer', 'fiat_offramp', 'bridge')
GROUP BY operation

UNION ALL

SELECT 
    '修正后' as version,
    operation,
    AVG(percent_bp) / 100.0 as avg_fee_percent,
    COUNT(*) as rule_count
FROM gas.platform_fee_rules
WHERE operation IN ('transfer', 'fiat_offramp', 'bridge')
  AND active = true
GROUP BY operation
ORDER BY operation, version;

-- ========== 第4步：用户场景测试 ==========

-- 场景1: 提现 11 ETH ($27,500)
WITH test_scenario AS (
    SELECT 
        'fiat_offramp' as operation,
        'ethereum' as chain,
        27500.0 as usd_value
)
SELECT 
    r.chain,
    r.operation,
    t.usd_value as transaction_usd,
    r.percent_bp / 100.0 as fee_percent,
    -- 计算平台服务费
    LEAST(
        GREATEST(
            t.usd_value * (r.percent_bp / 10000.0),
            r.min_fee
        ),
        COALESCE(r.max_fee, 999999)
    ) as platform_fee,
    -- 模拟第三方费用 (2.0%)
    t.usd_value * 0.02 as provider_fee,
    -- 总费用
    LEAST(
        GREATEST(
            t.usd_value * (r.percent_bp / 10000.0),
            r.min_fee
        ),
        COALESCE(r.max_fee, 999999)
    ) + (t.usd_value * 0.02) + 1.0 as total_fees
FROM gas.platform_fee_rules r
CROSS JOIN test_scenario t
WHERE r.operation = t.operation
  AND r.chain = t.chain
  AND r.active = true;

-- 场景2: Swap 5 ETH ($12,500)
WITH test_scenario AS (
    SELECT 
        'swap' as operation,
        'ethereum' as chain,
        12500.0 as usd_value
)
SELECT 
    r.chain,
    r.operation,
    t.usd_value as transaction_usd,
    r.percent_bp / 100.0 as fee_percent,
    LEAST(
        GREATEST(
            t.usd_value * (r.percent_bp / 10000.0),
            r.min_fee
        ),
        COALESCE(r.max_fee, 999999)
    ) as platform_fee,
    'N/A' as provider_fee,
    LEAST(
        GREATEST(
            t.usd_value * (r.percent_bp / 10000.0),
            r.min_fee
        ),
        COALESCE(r.max_fee, 999999)
    ) + 5.0 as total_fees
FROM gas.platform_fee_rules r
CROSS JOIN test_scenario t
WHERE r.operation = t.operation
  AND r.chain = t.chain
  AND r.active = true;

-- ========== 第5步：添加审计日志 ==========

INSERT INTO gas.fee_audit (
    user_id,
    chain,
    operation,
    original_amount,
    platform_fee,
    fee_type,
    applied_rule,
    collector_address,
    rule_version,
    notes
)
VALUES (
    '00000000-0000-0000-0000-000000000000',  -- 系统用户
    'system',
    'config_update',
    0,
    0,
    'system',
    'fee_rate_adjustment_2025_12_11',
    'system',
    'v2.0',
    '费率修正: transfer 0%, fiat_offramp 0.5%(封顶$50), bridge 0.3%(封顶$50)'
);

-- ========== 第6步：更新费用说明文档 ==========

-- 如果有metadata表，更新费用说明
-- UPDATE gas.fee_metadata 
-- SET description = '...', updated_at = NOW()
-- WHERE key = 'fee_explanation';

COMMIT;

-- ========== 验证SQL输出示例 ==========

/*
预期输出（验证查询）:

chain      | operation      | fee_percent | min_fee | max_fee | display_rate
-----------|----------------|-------------|---------|---------|-------------------
ethereum   | transfer       | 0.00        | 0       | 0       | 0.00%
ethereum   | swap           | 0.50        | 1       | 100     | 0.50% (封顶$100)
ethereum   | limit_order    | 0.50        | 1       | 100     | 0.50% (封顶$100)
ethereum   | bridge         | 0.30        | 2       | 50      | 0.30% (封顶$50)
ethereum   | fiat_onramp    | 0.30        | 1       | 50      | 0.30% (封顶$50)
ethereum   | fiat_offramp   | 0.50        | 2       | 50      | 0.50% (封顶$50)

对比修正前后:

version  | operation      | avg_fee_percent | rule_count
---------|----------------|----------------|------------
修正前   | transfer       | 0.10           | 8
修正后   | transfer       | 0.00           | 8
修正前   | fiat_offramp   | 2.50           | 8
修正后   | fiat_offramp   | 0.50           | 8
修正前   | bridge         | 1.00           | 8
修正后   | bridge         | 0.30           | 8

场景1测试（提现11 ETH）:

chain      | operation    | transaction_usd | fee_percent | platform_fee | provider_fee | total_fees
-----------|--------------|-----------------|-------------|--------------|--------------|------------
ethereum   | fiat_offramp | 27500.00        | 0.50        | 50.00        | 550.00       | 601.00

场景2测试（Swap 5 ETH）:

chain      | operation | transaction_usd | fee_percent | platform_fee | provider_fee | total_fees
-----------|-----------|-----------------|-------------|--------------|--------------|------------
ethereum   | swap      | 12500.00        | 0.50        | 62.50        | N/A          | 67.50
*/

-- ========== 回滚SQL（如需要）==========

/*
-- 回滚到修正前配置
BEGIN;

DELETE FROM gas.platform_fee_rules WHERE active = true;

INSERT INTO gas.platform_fee_rules 
SELECT * FROM gas.platform_fee_rules_backup_20251211;

COMMIT;
*/
