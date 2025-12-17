-- 添加以太坊主网 RPC 端点
-- 注意：这些是公共免费 RPC 端点，生产环境应使用付费的专业服务（如 Infura、Alchemy、QuickNode）

-- 先删除测试网端点（Sepolia）
DELETE FROM admin.rpc_endpoints WHERE url LIKE '%sepolia%';

-- 插入主网端点
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
-- Ethereum Mainnet (优先级最高)
('ethereum', 'https://eth.llamarpc.com', 1, true, 'closed'),
('ethereum', 'https://rpc.ankr.com/eth', 2, true, 'closed'),
('ethereum', 'https://ethereum.publicnode.com', 3, true, 'closed'),
('ethereum', 'https://eth.drpc.org', 4, true, 'closed'),

-- BSC Mainnet
('bsc', 'https://bsc-dataseed1.binance.org', 1, true, 'closed'),
('bsc', 'https://bsc-dataseed2.binance.org', 2, true, 'closed'),
('bsc', 'https://rpc.ankr.com/bsc', 3, true, 'closed'),

-- Polygon Mainnet
('polygon', 'https://polygon-rpc.com', 1, true, 'closed'),
('polygon', 'https://rpc.ankr.com/polygon', 2, true, 'closed'),
('polygon', 'https://polygon.llamarpc.com', 3, true, 'closed');
