-- ============================================================================
-- Migration: 0013_initial_data.sql
-- Description: 插入初始数据（代币注册、价格等）
-- Standard: 遵循数据库最佳实践，最后插入初始数据
-- ============================================================================

-- ----------------------------------------------------------------------------
-- 1. 插入初始价格数据
-- ----------------------------------------------------------------------------
INSERT INTO prices (symbol, price_usdt, source) VALUES
    ('ETH', 3500.00, 'coingecko'),
    ('SOL', 150.00, 'coingecko'),
    ('BTC', 75000.00, 'coingecko'),
    ('BNB', 650.00, 'coingecko'),
    ('MATIC', 1.20, 'coingecko')
ON CONFLICT (symbol, source) DO NOTHING;

-- ----------------------------------------------------------------------------
-- 2. 插入代币注册数据
-- ----------------------------------------------------------------------------

-- Ethereum (chain_id: 1)
INSERT INTO tokens.registry (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority) VALUES
    ('ETH', 'Ethereum', 1, '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE', 18, true, false, 1),
    ('USDT', 'Tether USD', 1, '0xdAC17F958D2ee523a2206206994597C13D831ec7', 6, false, true, 2),
    ('USDC', 'USD Coin', 1, '0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48', 6, false, true, 3),
    ('DAI', 'Dai Stablecoin', 1, '0x6B175474E89094C44Da98b954EedeAC495271d0F', 18, false, true, 4),
    ('WBTC', 'Wrapped Bitcoin', 1, '0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599', 8, false, false, 5)
ON CONFLICT (chain_id, symbol) DO NOTHING;

-- BSC (chain_id: 56)
INSERT INTO tokens.registry (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority) VALUES
    ('BNB', 'Binance Coin', 56, '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE', 18, true, false, 1),
    ('USDT', 'Tether USD', 56, '0x55d398326f99059fF775485246999027B3197955', 18, false, true, 2),
    ('USDC', 'USD Coin', 56, '0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d', 18, false, true, 3),
    ('BUSD', 'Binance USD', 56, '0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56', 18, false, true, 4)
ON CONFLICT (chain_id, symbol) DO NOTHING;

-- Polygon (chain_id: 137)
INSERT INTO tokens.registry (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority) VALUES
    ('MATIC', 'Polygon', 137, '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE', 18, true, false, 1),
    ('USDT', 'Tether USD', 137, '0xc2132D05D31c914a87C6611C10748AEb04B58e8F', 6, false, true, 2),
    ('USDC', 'USD Coin', 137, '0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174', 6, false, true, 3)
ON CONFLICT (chain_id, symbol) DO NOTHING;

-- Arbitrum (chain_id: 42161)
INSERT INTO tokens.registry (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority) VALUES
    ('ETH', 'Ethereum', 42161, '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE', 18, true, false, 1),
    ('USDT', 'Tether USD', 42161, '0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9', 6, false, true, 2),
    ('USDC', 'USD Coin', 42161, '0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8', 6, false, true, 3)
ON CONFLICT (chain_id, symbol) DO NOTHING;

-- Optimism (chain_id: 10)
INSERT INTO tokens.registry (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority) VALUES
    ('ETH', 'Ethereum', 10, '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE', 18, true, false, 1),
    ('USDT', 'Tether USD', 10, '0x94b008aA00579c1307B0EF2c499aD98a8ce58e58', 6, false, true, 2),
    ('USDC', 'USD Coin', 10, '0x7F5c764cBc14f9669B88837ca1490cCa17c31607', 6, false, true, 3)
ON CONFLICT (chain_id, symbol) DO NOTHING;

-- Avalanche (chain_id: 43114)
INSERT INTO tokens.registry (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority) VALUES
    ('AVAX', 'Avalanche', 43114, '0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE', 18, true, false, 1),
    ('USDT', 'Tether USD', 43114, '0x9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7', 6, false, true, 2),
    ('USDC', 'USD Coin', 43114, '0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E', 6, false, true, 3)
ON CONFLICT (chain_id, symbol) DO NOTHING;

