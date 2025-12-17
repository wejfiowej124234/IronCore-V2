-- 插入平台服务费规则配置
-- 基于行业标准：参考 MetaMask, Trust Wallet, Coinbase Wallet 等非托管钱包

-- 1. 代币交换 (Swap) - 0.5%
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
) VALUES 
('ethereum', 'swap', 'percent', 0, 50, 0.001, 0.05, 100, true),
('bsc', 'swap', 'percent', 0, 50, 0.01, 0.5, 100, true),
('polygon', 'swap', 'percent', 0, 50, 0.1, 5.0, 100, true);

-- 2. 基础转账 (Transfer) - 0.1%
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
) VALUES 
('ethereum', 'transfer', 'percent', 0, 10, 0.0001, 0.01, 100, true),
('bsc', 'transfer', 'percent', 0, 10, 0.001, 0.1, 100, true),
('polygon', 'transfer', 'percent', 0, 10, 0.01, 1.0, 100, true);

-- 3. 法币入金 (Fiat Onramp) - 2.0%
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
) VALUES 
('ethereum', 'fiat_onramp', 'percent', 0, 200, 1.0, 100.0, 100, true),
('bsc', 'fiat_onramp', 'percent', 0, 200, 1.0, 100.0, 100, true),
('polygon', 'fiat_onramp', 'percent', 0, 200, 1.0, 100.0, 100, true);

-- 4. 法币出金 (Fiat Offramp) - 2.5%
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
) VALUES 
('ethereum', 'fiat_offramp', 'percent', 0, 250, 2.0, 150.0, 100, true),
('bsc', 'fiat_offramp', 'percent', 0, 250, 2.0, 150.0, 100, true),
('polygon', 'fiat_offramp', 'percent', 0, 250, 2.0, 150.0, 100, true);

-- 5. 限价单 (Limit Order) - 0.5%
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
) VALUES 
('ethereum', 'limit_order', 'percent', 0, 50, 0.001, 0.05, 100, true),
('bsc', 'limit_order', 'percent', 0, 50, 0.01, 0.5, 100, true),
('polygon', 'limit_order', 'percent', 0, 50, 0.1, 5.0, 100, true);

-- 6. 跨链桥 (Bridge) - 1.0%
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type,
    flat_amount, percent_bp, min_fee, max_fee,
    priority, active
) VALUES 
('ethereum', 'bridge', 'percent', 0, 100, 0.005, 0.1, 100, true),
('bsc', 'bridge', 'percent', 0, 100, 0.05, 1.0, 100, true),
('polygon', 'bridge', 'percent', 0, 100, 0.5, 10.0, 100, true);
