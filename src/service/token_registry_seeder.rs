use anyhow::Result;
use sqlx::PgPool;

#[derive(Clone, Copy)]
struct TokenSeed {
    symbol: &'static str,
    name: &'static str,
    chain_id: i64,
    address: &'static str,
    decimals: i64,
    is_native: bool,
    is_stablecoin: bool,
    priority: i64,
}

/// Seeds `tokens.registry` with baseline tokens if it's empty.
///
/// This unblocks:
/// - `/api/v1/tokens/list`
/// - `/api/v1/tokens/:address/info`
/// - swap quote token resolution (symbol -> address)
///
/// The seeds match the project's migration `0013_initial_data.sql`.
pub async fn seed_token_registry_if_empty(pool: &PgPool) -> Result<()> {
    let table_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'tokens' AND table_name = 'registry'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        tracing::warn!("tokens.registry table does not exist; skip token registry seeding");
        return Ok(());
    }

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tokens.registry")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        tracing::info!(count, "tokens.registry already has rows; skip seeding");
        return Ok(());
    }

    tracing::warn!("tokens.registry is empty; seeding baseline token registry rows");

    let seeds: &[TokenSeed] = &[
        // Ethereum (1)
        TokenSeed {
            symbol: "ETH",
            name: "Ethereum",
            chain_id: 1,
            address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            decimals: 18,
            is_native: true,
            is_stablecoin: false,
            priority: 1,
        },
        TokenSeed {
            symbol: "USDT",
            name: "Tether USD",
            chain_id: 1,
            address: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 2,
        },
        TokenSeed {
            symbol: "USDC",
            name: "USD Coin",
            chain_id: 1,
            address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 3,
        },
        TokenSeed {
            symbol: "DAI",
            name: "Dai Stablecoin",
            chain_id: 1,
            address: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
            decimals: 18,
            is_native: false,
            is_stablecoin: true,
            priority: 4,
        },
        TokenSeed {
            symbol: "WBTC",
            name: "Wrapped Bitcoin",
            chain_id: 1,
            address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
            decimals: 8,
            is_native: false,
            is_stablecoin: false,
            priority: 5,
        },
        // BSC (56)
        TokenSeed {
            symbol: "BNB",
            name: "Binance Coin",
            chain_id: 56,
            address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            decimals: 18,
            is_native: true,
            is_stablecoin: false,
            priority: 1,
        },
        TokenSeed {
            symbol: "USDT",
            name: "Tether USD",
            chain_id: 56,
            address: "0x55d398326f99059fF775485246999027B3197955",
            decimals: 18,
            is_native: false,
            is_stablecoin: true,
            priority: 2,
        },
        TokenSeed {
            symbol: "USDC",
            name: "USD Coin",
            chain_id: 56,
            address: "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
            decimals: 18,
            is_native: false,
            is_stablecoin: true,
            priority: 3,
        },
        TokenSeed {
            symbol: "BUSD",
            name: "Binance USD",
            chain_id: 56,
            address: "0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56",
            decimals: 18,
            is_native: false,
            is_stablecoin: true,
            priority: 4,
        },
        // Polygon (137)
        TokenSeed {
            symbol: "MATIC",
            name: "Polygon",
            chain_id: 137,
            address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            decimals: 18,
            is_native: true,
            is_stablecoin: false,
            priority: 1,
        },
        TokenSeed {
            symbol: "USDT",
            name: "Tether USD",
            chain_id: 137,
            address: "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 2,
        },
        TokenSeed {
            symbol: "USDC",
            name: "USD Coin",
            chain_id: 137,
            address: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 3,
        },
        // Arbitrum (42161)
        TokenSeed {
            symbol: "ETH",
            name: "Ethereum",
            chain_id: 42161,
            address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            decimals: 18,
            is_native: true,
            is_stablecoin: false,
            priority: 1,
        },
        TokenSeed {
            symbol: "USDT",
            name: "Tether USD",
            chain_id: 42161,
            address: "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 2,
        },
        TokenSeed {
            symbol: "USDC",
            name: "USD Coin",
            chain_id: 42161,
            address: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 3,
        },
        // Optimism (10)
        TokenSeed {
            symbol: "ETH",
            name: "Ethereum",
            chain_id: 10,
            address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            decimals: 18,
            is_native: true,
            is_stablecoin: false,
            priority: 1,
        },
        TokenSeed {
            symbol: "USDT",
            name: "Tether USD",
            chain_id: 10,
            address: "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 2,
        },
        TokenSeed {
            symbol: "USDC",
            name: "USD Coin",
            chain_id: 10,
            address: "0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 3,
        },
        // Avalanche (43114)
        TokenSeed {
            symbol: "AVAX",
            name: "Avalanche",
            chain_id: 43114,
            address: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
            decimals: 18,
            is_native: true,
            is_stablecoin: false,
            priority: 1,
        },
        TokenSeed {
            symbol: "USDT",
            name: "Tether USD",
            chain_id: 43114,
            address: "0x9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 2,
        },
        TokenSeed {
            symbol: "USDC",
            name: "USD Coin",
            chain_id: 43114,
            address: "0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E",
            decimals: 6,
            is_native: false,
            is_stablecoin: true,
            priority: 3,
        },
    ];

    for s in seeds {
        sqlx::query(
            r#"
            INSERT INTO tokens.registry
                (symbol, name, chain_id, address, decimals, is_native, is_stablecoin, priority)
            VALUES
                ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (chain_id, symbol) DO NOTHING
            "#,
        )
        .bind(s.symbol)
        .bind(s.name)
        .bind(s.chain_id)
        .bind(s.address)
        .bind(s.decimals)
        .bind(s.is_native)
        .bind(s.is_stablecoin)
        .bind(s.priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}
