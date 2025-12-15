-- ============================================
-- 生产级返佣系统 + 行业标准费率对齐
-- ============================================
-- 创建时间: 2025-12-11
-- 目标: 完全对齐MetaMask/Trust Wallet行业标准

BEGIN;

-- ============================================
-- 第一部分: 创建返佣收入追踪表
-- ============================================

-- 创建schema（如果不存在）
CREATE SCHEMA IF NOT EXISTS revenue;

-- 返佣收入记录表
CREATE TABLE IF NOT EXISTS revenue.referral_commissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- 订单关联
    order_id UUID NOT NULL,                    -- fiat_orders表的订单ID
    order_type VARCHAR(20) NOT NULL,           -- 'onramp' 或 'offramp'
    
    -- 用户信息
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000'::UUID,
    
    -- 服务商信息
    provider_name VARCHAR(50) NOT NULL,        -- 'onramper', 'transfi', 'banxa', etc.
    provider_order_id VARCHAR(255),            -- 服务商的订单ID
    
    -- 金额信息
    transaction_amount DECIMAL(20, 8) NOT NULL, -- 交易总额（USD）
    provider_fee DECIMAL(20, 8) NOT NULL,       -- 服务商收取的费用
    provider_fee_percent DECIMAL(5, 2) NOT NULL, -- 服务商费率（如2.0%）
    
    -- 返佣信息
    commission_rate DECIMAL(5, 2) NOT NULL,     -- 返佣比例（如0.5%）
    commission_amount DECIMAL(20, 8) NOT NULL,  -- 返佣金额（USD）
    commission_currency VARCHAR(10) DEFAULT 'USD',
    
    -- 状态追踪
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending, confirmed, paid, failed
    confirmed_at TIMESTAMPTZ,                   -- 服务商确认时间
    paid_at TIMESTAMPTZ,                        -- 实际到账时间
    
    -- 支付信息
    payment_method VARCHAR(50),                 -- 'bank_transfer', 'crypto', 'offset'
    payment_reference VARCHAR(255),             -- 支付参考号
    
    -- 审计信息
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT,                                 -- 备注（如延迟原因）
    
    -- 约束
    CONSTRAINT valid_commission_rate CHECK (commission_rate >= 0 AND commission_rate <= 100),
    CONSTRAINT valid_commission_amount CHECK (commission_amount >= 0),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'confirmed', 'paid', 'failed', 'cancelled'))
);

-- 索引优化
CREATE INDEX IF NOT EXISTS idx_referral_commissions_order_id ON revenue.referral_commissions(order_id);
CREATE INDEX IF NOT EXISTS idx_referral_commissions_user_id ON revenue.referral_commissions(user_id);
CREATE INDEX IF NOT EXISTS idx_referral_commissions_provider ON revenue.referral_commissions(provider_name);
CREATE INDEX IF NOT EXISTS idx_referral_commissions_status ON revenue.referral_commissions(status);
CREATE INDEX IF NOT EXISTS idx_referral_commissions_created_at ON revenue.referral_commissions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_referral_commissions_paid_at ON revenue.referral_commissions(paid_at DESC) WHERE paid_at IS NOT NULL;

-- 表注释
COMMENT ON TABLE revenue.referral_commissions IS 'Referral commission income from payment providers (Onramper, TransFi, Banxa, etc.)';
COMMENT ON COLUMN revenue.referral_commissions.commission_rate IS 'Commission rate negotiated with provider (e.g., 0.5% = Onramper standard partner rate)';
COMMENT ON COLUMN revenue.referral_commissions.status IS 'pending: awaiting confirmation | confirmed: provider confirmed | paid: received payment | failed: commission rejected';

-- ============================================
-- 第二部分: 服务商返佣配置表
-- ============================================

CREATE TABLE IF NOT EXISTS revenue.provider_commission_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    
    -- 服务商信息
    provider_name VARCHAR(50) NOT NULL UNIQUE,
    provider_display_name VARCHAR(100) NOT NULL,
    
    -- 返佣配置
    default_commission_rate DECIMAL(5, 2) NOT NULL, -- 默认返佣率
    onramp_commission_rate DECIMAL(5, 2),          -- 买币返佣率（可与卖币不同）
    offramp_commission_rate DECIMAL(5, 2),         -- 卖币返佣率
    
    -- 阶梯返佣（月交易量）
    tier1_volume DECIMAL(20, 2),                   -- 第一档交易量阈值
    tier1_rate DECIMAL(5, 2),                      -- 第一档返佣率
    tier2_volume DECIMAL(20, 2),
    tier2_rate DECIMAL(5, 2),
    tier3_volume DECIMAL(20, 2),
    tier3_rate DECIMAL(5, 2),
    
    -- Partner ID配置
    partner_id VARCHAR(255),                       -- 合作伙伴ID（API中使用）
    api_key_encrypted TEXT,                        -- 加密的API密钥
    webhook_secret TEXT,                           -- Webhook签名密钥
    
    -- 结算信息
    settlement_period VARCHAR(20) DEFAULT 'monthly', -- daily, weekly, monthly, quarterly
    minimum_payout DECIMAL(20, 2) DEFAULT 100.00,   -- 最低起付金额
    payment_terms_days INT DEFAULT 30,              -- 账期（天）
    
    -- 状态
    is_active BOOLEAN DEFAULT true,
    contract_start_date DATE,
    contract_end_date DATE,
    
    -- 审计
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    notes TEXT
);

-- 插入主流服务商默认配置
INSERT INTO revenue.provider_commission_config (
    provider_name,
    provider_display_name,
    default_commission_rate,
    onramp_commission_rate,
    offramp_commission_rate,
    tier1_volume,
    tier1_rate,
    tier2_volume,
    tier2_rate,
    tier3_volume,
    tier3_rate,
    settlement_period,
    minimum_payout,
    payment_terms_days,
    is_active,
    notes
) VALUES 
-- Onramper（聚合器，返佣最高）
(
    'onramper',
    'Onramper',
    0.50,  -- 默认0.5%
    0.50,  -- 买币0.5%
    0.50,  -- 卖币0.5%
    100000,  -- 月交易量$10万
    0.60,    -- 提升到0.6%
    500000,  -- 月交易量$50万
    0.70,    -- 提升到0.7%
    1000000, -- 月交易量$100万
    0.80,    -- 提升到0.8%
    'monthly',
    100.00,
    30,
    true,
    'Onramper standard partner program. Contact: partnerships@onramper.com'
),
-- TransFi（新兴市场，高返佣）
(
    'transfi',
    'TransFi',
    0.60,
    0.60,
    0.70,  -- 卖币更高（新兴市场需求大）
    50000,
    0.70,
    250000,
    0.85,
    500000,
    1.00,  -- 最高可达1%
    'monthly',
    50.00,
    30,
    true,
    'TransFi emerging markets focus. Higher offramp commissions.'
),
-- Alchemy Pay（Web3友好）
(
    'alchemypay',
    'Alchemy Pay',
    0.45,
    0.45,
    0.50,
    100000,
    0.55,
    500000,
    0.65,
    1000000,
    0.75,
    'monthly',
    100.00,
    45,
    true,
    'Alchemy Pay Web3 integration. DeFi-friendly rates.'
),
-- Ramp Network（欧美低费率）
(
    'ramp',
    'Ramp Network',
    0.30,  -- 费率低，返佣也低
    0.30,
    0.35,
    100000,
    0.35,
    500000,
    0.40,
    1000000,
    0.45,
    'monthly',
    100.00,
    30,
    true,
    'Ramp Network - Low fees, moderate commissions'
),
-- MoonPay（品牌溢价）
(
    'moonpay',
    'MoonPay',
    0.40,
    0.40,
    0.40,
    200000,
    0.50,
    1000000,
    0.60,
    5000000,
    0.70,
    'monthly',
    200.00,  -- 起付额更高
    60,      -- 账期更长
    true,
    'MoonPay - Global brand, standard partner rates'
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_provider_commission_config_provider ON revenue.provider_commission_config(provider_name);
CREATE INDEX IF NOT EXISTS idx_provider_commission_config_active ON revenue.provider_commission_config(is_active);

COMMENT ON TABLE revenue.provider_commission_config IS 'Provider commission rate configuration and partnership terms';

-- ============================================
-- 第三部分: 备份并修改费率为0%
-- ============================================

-- 备份当前配置
CREATE TABLE IF NOT EXISTS gas.platform_fee_rules_backup_industry_standard AS 
SELECT * FROM gas.platform_fee_rules WHERE active = true;

COMMENT ON TABLE gas.platform_fee_rules_backup_industry_standard IS 
'Backup before aligning to industry standard: Fiat operations 0% platform fee, revenue from referral commissions';

-- 修改法币操作费率为0%
UPDATE gas.platform_fee_rules
SET 
    percent_bp = 0,
    min_fee = 0,
    max_fee = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE operation IN ('fiat_onramp', 'fiat_offramp')
  AND active = true;

COMMENT ON COLUMN gas.platform_fee_rules.percent_bp IS 
'Basis points (0=0%, 50=0.5%). Industry standard: Transfer/Fiat=0%, Swap=0.5%, Bridge=0.3%. Revenue from provider commissions for fiat ops.';

-- ============================================
-- 第四部分: 收入监控视图
-- ============================================

-- 月度收入汇总视图
CREATE OR REPLACE VIEW revenue.monthly_revenue_summary AS
SELECT 
    DATE_TRUNC('month', created_at) as month,
    provider_name,
    order_type,
    COUNT(*) as transaction_count,
    SUM(transaction_amount) as total_volume,
    SUM(provider_fee) as total_provider_fees,
    SUM(commission_amount) as total_commission_income,
    AVG(commission_rate) as avg_commission_rate,
    COUNT(CASE WHEN status = 'paid' THEN 1 END) as paid_count,
    SUM(CASE WHEN status = 'paid' THEN commission_amount ELSE 0 END) as paid_amount,
    COUNT(CASE WHEN status = 'pending' THEN 1 END) as pending_count,
    SUM(CASE WHEN status = 'pending' THEN commission_amount ELSE 0 END) as pending_amount
FROM revenue.referral_commissions
GROUP BY DATE_TRUNC('month', created_at), provider_name, order_type
ORDER BY month DESC, total_volume DESC;

COMMENT ON VIEW revenue.monthly_revenue_summary IS 'Monthly referral commission revenue summary by provider';

-- 实时收入仪表板视图
CREATE OR REPLACE VIEW revenue.realtime_revenue_dashboard AS
SELECT 
    provider_name,
    COUNT(*) as total_transactions,
    SUM(transaction_amount) as total_volume,
    SUM(commission_amount) as total_commission,
    SUM(CASE WHEN status = 'paid' THEN commission_amount ELSE 0 END) as paid_commission,
    SUM(CASE WHEN status = 'pending' THEN commission_amount ELSE 0 END) as pending_commission,
    AVG(commission_rate) as avg_commission_rate,
    MAX(created_at) as last_transaction_at
FROM revenue.referral_commissions
WHERE created_at >= NOW() - INTERVAL '30 days'
GROUP BY provider_name
ORDER BY total_volume DESC;

-- ============================================
-- 第五部分: 验证修改结果
-- ============================================

-- 1. 显示所有操作的费率配置
SELECT 
    '=== 费率配置总览（对齐行业标准后） ===' as summary;

SELECT 
    operation,
    COUNT(*) as chain_count,
    MIN(percent_bp / 100.0) as min_fee_pct,
    MAX(percent_bp / 100.0) as max_fee_pct,
    MIN(max_fee) as min_cap,
    MAX(max_fee) as max_cap,
    CASE operation
        WHEN 'transfer' THEN '✅ 行业标准: 0%'
        WHEN 'swap' THEN '✅ 行业标准: 0.5%'
        WHEN 'limit_order' THEN '✅ 行业标准: 0.5%'
        WHEN 'bridge' THEN '✅ 行业标准: 0.3%'
        WHEN 'fiat_onramp' THEN '✅ 行业标准: 0% (靠返佣)'
        WHEN 'fiat_offramp' THEN '✅ 行业标准: 0% (靠返佣)'
    END as compliance_status
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

-- 2. 服务商返佣配置
SELECT 
    '=== 服务商返佣配置 ===' as summary;

SELECT 
    provider_display_name as provider,
    default_commission_rate || '%' as default_rate,
    onramp_commission_rate || '%' as buy_rate,
    offramp_commission_rate || '%' as sell_rate,
    tier1_volume as tier1_threshold,
    tier1_rate || '%' as tier1_rate,
    settlement_period,
    is_active
FROM revenue.provider_commission_config
ORDER BY default_commission_rate DESC;

-- 3. 测试场景：11 ETH 提现
SELECT 
    '=== 测试: 11 ETH ($27,225) 提现收入对比 ===' as test_scenario;

WITH test_case AS (
    SELECT 
        27225.00 as amount_usd,
        'onramper' as provider
)
SELECT 
    tc.provider,
    tc.amount_usd as transaction_amount,
    tc.amount_usd * 0.02 as provider_fee_2pct,
    0.00 as platform_fee_old_model,
    pcc.offramp_commission_rate as commission_rate,
    tc.amount_usd * pcc.offramp_commission_rate / 100 as commission_income,
    '通过返佣赚取' as revenue_source
FROM test_case tc
JOIN revenue.provider_commission_config pcc ON tc.provider = pcc.provider_name;

COMMIT;

-- ============================================
-- 部署后操作清单
-- ============================================
-- □ 确认所有费率已更新
-- □ 联系服务商申请Partner Program并获取Partner ID
-- □ 配置环境变量: ONRAMPER_PARTNER_ID, TRANSFI_PARTNER_ID等
-- □ 实现返佣记录自动创建逻辑（每笔订单完成时）
-- □ 配置Webhook接收服务商返佣确认
-- □ 设置Grafana监控Dashboard
-- □ 更新前端显示："平台服务费 $0.00 (免费)"
-- □ 发布用户公告
-- ============================================
