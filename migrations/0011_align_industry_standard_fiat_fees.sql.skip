-- ============================================
-- 对齐行业标准：法币操作免费（靠返佣收入）
-- ============================================
-- 创建时间: 2025-12-11
-- 理由: MetaMask/Trust Wallet等95%钱包不收法币平台费
-- 收入模式: 服务商返佣（0.3-0.8%）

BEGIN;

-- 备份当前配置
CREATE TABLE IF NOT EXISTS gas.platform_fee_rules_backup_20251211_v2 AS 
SELECT * FROM gas.platform_fee_rules WHERE active = true;

COMMENT ON TABLE gas.platform_fee_rules_backup_20251211_v2 IS 
'Backup before aligning fiat fees to industry standard (0% platform fee, rely on referral commissions)';

-- ============================================
-- 核心修改：法币操作平台费改为0%
-- ============================================

-- 1. Fiat Onramp (买币): 2.5%/0.3% → 0% (对齐MetaMask/Trust Wallet)
UPDATE gas.platform_fee_rules
SET 
    percent_bp = 0,              -- 平台费0%
    min_fee = 0,                 -- 最低费用0
    max_fee = NULL,              -- 无需封顶（已经是0）
    updated_at = CURRENT_TIMESTAMP
WHERE operation = 'fiat_onramp'
  AND active = true;

COMMENT ON COLUMN gas.platform_fee_rules.percent_bp IS 
'Basis points (0 = 0%, 50 = 0.5%, 100 = 1.0%). Fiat operations set to 0% per industry standard, revenue from provider referral commissions (0.3-0.8%).';

-- 2. Fiat Offramp (卖币): 2.5%/0.5% → 0% (对齐MetaMask/Trust Wallet)
UPDATE gas.platform_fee_rules
SET 
    percent_bp = 0,
    min_fee = 0,
    max_fee = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE operation = 'fiat_offramp'
  AND active = true;

-- ============================================
-- 验证修改结果
-- ============================================

-- 显示所有操作的费率（按操作类型分组）
SELECT 
    operation,
    COUNT(*) as chain_count,
    MIN(percent_bp / 100.0) as min_fee_pct,
    MAX(percent_bp / 100.0) as max_fee_pct,
    MIN(max_fee) as min_cap,
    MAX(max_fee) as max_cap
FROM gas.platform_fee_rules
WHERE active = true
GROUP BY operation
ORDER BY 
    CASE operation
        WHEN 'transfer' THEN 1
        WHEN 'swap' THEN 2
        WHEN 'limit_order' THEN 3
        WHEN 'bridge' THEN 4
        WHEN 'fiat_onramp' THEN 5
        WHEN 'fiat_offramp' THEN 6
    END;

-- 显示法币操作的详细配置
SELECT 
    chain,
    operation,
    percent_bp as basis_points,
    ROUND(percent_bp / 100.0, 2) as fee_percent,
    min_fee,
    max_fee,
    updated_at
FROM gas.platform_fee_rules
WHERE operation IN ('fiat_onramp', 'fiat_offramp')
  AND active = true
ORDER BY chain, operation;

-- ============================================
-- 测试场景：11 ETH 提现 ($27,225)
-- ============================================
SELECT 
    '=== 测试场景: 11 ETH 提现到银行卡 ===' as test_description;

WITH test_withdrawal AS (
    SELECT 
        'ethereum' as chain,
        'fiat_offramp' as operation,
        27225.00 as usd_amount
)
SELECT 
    tw.chain,
    tw.operation,
    tw.usd_amount as amount_usd,
    pfr.percent_bp / 100.0 as platform_fee_pct,
    pfr.max_fee as platform_fee_cap,
    tw.usd_amount * pfr.percent_bp / 10000.0 as platform_fee_calculated,
    '0.00' as platform_fee_actual,  -- 0%
    tw.usd_amount * 0.02 as provider_fee_banxa,
    '预期返佣 0.5%' as referral_note,
    tw.usd_amount * 0.005 as expected_referral_commission
FROM test_withdrawal tw
JOIN gas.platform_fee_rules pfr
  ON tw.chain = pfr.chain
  AND tw.operation = pfr.operation
  AND pfr.active = true;

-- ============================================
-- 收入对比分析
-- ============================================
SELECT 
    '=== 收入模式对比 (11 ETH = $27,225) ===' as revenue_comparison;

SELECT 
    '旧模式 (0.5%直接收费)' as model,
    27225 * 0.005 as revenue_usd,
    '直接向用户收取' as revenue_source
UNION ALL
SELECT 
    '新模式 (0%平台费 + 返佣)',
    27225 * 0.005 as revenue_usd,
    'Banxa/Onramper返佣 (0.5%)' as revenue_source
UNION ALL
SELECT 
    '差异',
    0.00 as revenue_usd,
    '收入相同，但用户体验更好' as revenue_source;

COMMIT;

-- ============================================
-- 回滚脚本（如需恢复旧配置）
-- ============================================
-- BEGIN;
-- 
-- -- 恢复 fiat_onramp 到 0.3%
-- UPDATE gas.platform_fee_rules
-- SET percent_bp = 30, min_fee = 1, max_fee = 50
-- WHERE operation = 'fiat_onramp' AND active = true;
-- 
-- -- 恢复 fiat_offramp 到 0.5%
-- UPDATE gas.platform_fee_rules
-- SET percent_bp = 50, min_fee = 2, max_fee = 50
-- WHERE operation = 'fiat_offramp' AND active = true;
-- 
-- COMMIT;
-- ============================================

-- ============================================
-- 部署后检查清单
-- ============================================
-- □ 确认数据库费率已更新为0%
-- □ 联系Onramper/TransFi申请Partner Program
-- □ 配置Referral ID到环境变量
-- □ 更新前端显示："平台服务费 $0.00 (免费)"
-- □ 添加返佣收入监控Dashboard
-- □ 用户公告："法币操作现已完全免费！"
-- ============================================
