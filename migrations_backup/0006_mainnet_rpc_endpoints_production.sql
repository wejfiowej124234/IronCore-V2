-- ============================================================================
-- 生产环境主网 RPC 端点配置（删除所有测试网）
-- ============================================================================

-- 1. 清理所有测试网端点
DELETE FROM admin.rpc_endpoints WHERE 
    url LIKE '%testnet%' 
    OR url LIKE '%sepolia%' 
    OR url LIKE '%goerli%' 
    OR url LIKE '%amoy%' 
    OR url LIKE '%mumbai%'
    OR url LIKE '%rinkeby%';

-- 2. 插入生产级主网 RPC 端点
-- NOTE: 生产环境可能已有自定义/付费节点；这里采用幂等插入，不做全量清空。

-- 3. 插入生产级主网 RPC 端点
-- 注意：这些是免费公共节点，生产环境应使用付费服务（Infura/Alchemy/QuickNode）

-- ============================================================================
-- Ethereum Mainnet (Chain ID: 1)
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('ethereum', 'https://eth.llamarpc.com', 1, true, 'closed'),
('ethereum', 'https://rpc.ankr.com/eth', 2, true, 'closed'),
('ethereum', 'https://ethereum.publicnode.com', 3, true, 'closed'),
('ethereum', 'https://eth.drpc.org', 4, true, 'closed'),
('ethereum', 'https://cloudflare-eth.com', 5, true, 'closed'),
('ethereum', 'https://rpc.flashbots.net', 6, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- BSC (Binance Smart Chain) Mainnet (Chain ID: 56)
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('bsc', 'https://bsc-dataseed1.binance.org', 1, true, 'closed'),
('bsc', 'https://bsc-dataseed2.binance.org', 2, true, 'closed'),
('bsc', 'https://bsc-dataseed3.binance.org', 3, true, 'closed'),
('bsc', 'https://rpc.ankr.com/bsc', 4, true, 'closed'),
('bsc', 'https://bsc.publicnode.com', 5, true, 'closed'),
('bsc', 'https://bsc.drpc.org', 6, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- Polygon (Matic) Mainnet (Chain ID: 137)
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('polygon', 'https://polygon-rpc.com', 1, true, 'closed'),
('polygon', 'https://rpc.ankr.com/polygon', 2, true, 'closed'),
('polygon', 'https://polygon.llamarpc.com', 3, true, 'closed'),
('polygon', 'https://polygon.drpc.org', 4, true, 'closed'),
('polygon', 'https://polygon-bor-rpc.publicnode.com', 5, true, 'closed'),
('polygon', 'https://polygon-mainnet.g.alchemy.com/v2/demo', 6, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- Solana Mainnet Beta
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('solana', 'https://api.mainnet-beta.solana.com', 1, true, 'closed'),
('solana', 'https://solana-api.projectserum.com', 2, true, 'closed'),
('solana', 'https://rpc.ankr.com/solana', 3, true, 'closed'),
('solana', 'https://solana.publicnode.com', 4, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- Bitcoin Mainnet
-- 注意：Bitcoin 使用不同的 RPC 协议，需要 Bitcoin Core 或第三方服务
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('bitcoin', 'https://blockstream.info/api', 1, true, 'closed'),
('bitcoin', 'https://blockchain.info', 2, true, 'closed'),
('bitcoin', 'https://btc.getblock.io/mainnet', 3, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- Arbitrum One (Layer 2) Mainnet (Chain ID: 42161)
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('arbitrum', 'https://arb1.arbitrum.io/rpc', 1, true, 'closed'),
('arbitrum', 'https://rpc.ankr.com/arbitrum', 2, true, 'closed'),
('arbitrum', 'https://arbitrum.llamarpc.com', 3, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- Optimism (Layer 2) Mainnet (Chain ID: 10)
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('optimism', 'https://mainnet.optimism.io', 1, true, 'closed'),
('optimism', 'https://rpc.ankr.com/optimism', 2, true, 'closed'),
('optimism', 'https://optimism.publicnode.com', 3, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- ============================================================================
-- Avalanche C-Chain Mainnet (Chain ID: 43114)
-- ============================================================================
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state) VALUES
('avalanche', 'https://api.avax.network/ext/bc/C/rpc', 1, true, 'closed'),
('avalanche', 'https://rpc.ankr.com/avalanche', 2, true, 'closed'),
('avalanche', 'https://avalanche.publicnode.com', 3, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- 验证插入结果
SELECT chain, COUNT(*) as endpoint_count 
FROM admin.rpc_endpoints 
GROUP BY chain 
ORDER BY chain;
