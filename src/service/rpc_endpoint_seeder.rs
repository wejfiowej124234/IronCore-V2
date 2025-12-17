use anyhow::Result;
use sqlx::PgPool;

#[derive(Clone, Copy)]
struct EndpointSeed {
    chain: &'static str,
    url: &'static str,
    priority: i32,
}

/// Seeds `admin.rpc_endpoints` with baseline MAINNET EVM RPC endpoints if the table is empty.
///
/// Why this exists:
/// - Several production endpoints (gas estimation, ERC20 balance, etc.) depend on `RpcSelector`,
///   which selects from `admin.rpc_endpoints`.
/// - If migrations were skipped or seed data wasn't applied, the table can be empty and the API
///   will return 500 with "No healthy RPC endpoint available".
///
/// Notes:
/// - We only seed EVM chains here because `RpcSelector` health probing uses `eth_blockNumber`.
/// - Non-EVM chains (Solana/Bitcoin/TON) are handled by other services via their own URLs.
pub async fn seed_rpc_endpoints_if_empty(pool: &PgPool) -> Result<()> {
    let table_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'admin' AND table_name = 'rpc_endpoints'
        )
        "#,
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        tracing::warn!("admin.rpc_endpoints table does not exist; skip RPC endpoint seeding");
        return Ok(());
    }

    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM admin.rpc_endpoints")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        tracing::info!(count, "RPC endpoints already present; skip seeding");
        return Ok(());
    }

    tracing::warn!("admin.rpc_endpoints is empty; seeding baseline MAINNET EVM RPC endpoints");

    let seeds: &[EndpointSeed] = &[
        // Ethereum mainnet
        EndpointSeed {
            chain: "ethereum",
            url: "https://eth.llamarpc.com",
            priority: 1,
        },
        EndpointSeed {
            chain: "ethereum",
            url: "https://rpc.ankr.com/eth",
            priority: 2,
        },
        EndpointSeed {
            chain: "ethereum",
            url: "https://cloudflare-eth.com",
            priority: 3,
        },
        // BSC mainnet
        EndpointSeed {
            chain: "bsc",
            url: "https://bsc-dataseed1.binance.org",
            priority: 1,
        },
        EndpointSeed {
            chain: "bsc",
            url: "https://rpc.ankr.com/bsc",
            priority: 2,
        },
        // Polygon mainnet
        EndpointSeed {
            chain: "polygon",
            url: "https://polygon-rpc.com",
            priority: 1,
        },
        EndpointSeed {
            chain: "polygon",
            url: "https://rpc.ankr.com/polygon",
            priority: 2,
        },
        // L2s (supported by API surface)
        EndpointSeed {
            chain: "arbitrum",
            url: "https://arb1.arbitrum.io/rpc",
            priority: 1,
        },
        EndpointSeed {
            chain: "optimism",
            url: "https://mainnet.optimism.io",
            priority: 1,
        },
        EndpointSeed {
            chain: "avalanche",
            url: "https://api.avax.network/ext/bc/C/rpc",
            priority: 1,
        },
    ];

    for s in seeds {
        sqlx::query(
            r#"
            INSERT INTO admin.rpc_endpoints (chain, url, priority, healthy, circuit_state)
            VALUES ($1, $2, $3, true, 'closed')
            ON CONFLICT (chain, url) DO NOTHING
            "#,
        )
        .bind(s.chain)
        .bind(s.url)
        .bind(s.priority)
        .execute(pool)
        .await?;
    }

    Ok(())
}
