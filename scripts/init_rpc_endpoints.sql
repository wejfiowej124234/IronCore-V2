-- 初始化 RPC 端点配置
-- 用于 Gas 费用估算和区块链交互

-- Ethereum Sepolia 测试网
-- CockroachDB兼容：ON CONFLICT需要指定唯一约束列名
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state)
VALUES 
    ('ethereum', 'https://ethereum-sepolia-rpc.publicnode.com', 1, true, 'closed'),
    ('ethereum', 'https://rpc.sepolia.org', 2, true, 'closed'),
    ('ethereum', 'https://sepolia.infura.io/v3/9aa3d95b3bc440fa88ea12eaa4456161', 3, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- Ethereum Mainnet (如果需要)
-- CockroachDB兼容：ON CONFLICT需要指定唯一约束列名
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state)
VALUES 
    ('ethereum_mainnet', 'https://ethereum-rpc.publicnode.com', 1, true, 'closed'),
    ('ethereum_mainnet', 'https://eth.llamarpc.com', 2, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- Bitcoin Testnet
-- CockroachDB兼容：ON CONFLICT需要指定唯一约束列名
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state)
VALUES 
    ('bitcoin', 'https://mempool.space/testnet/api', 1, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- Solana Devnet
-- CockroachDB兼容：ON CONFLICT需要指定唯一约束列名
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state)
VALUES 
    ('solana', 'https://api.devnet.solana.com', 1, true, 'closed'),
    ('solana', 'https://solana-devnet-rpc.publicnode.com', 2, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;

-- TON Testnet
-- CockroachDB兼容：ON CONFLICT需要指定唯一约束列名
INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state)
VALUES 
    ('ton', 'https://testnet.toncenter.com/api/v2', 1, true, 'closed')
ON CONFLICT (chain, url) DO NOTHING;
