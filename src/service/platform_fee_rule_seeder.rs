use anyhow::Result;
use rust_decimal::Decimal;
use sqlx::PgPool;

async fn query_active_rule_count(pool: &PgPool) -> Result<i64> {
    // If the table doesn't exist or query fails (e.g. migrations incomplete), don't crash startup.
    // The fee endpoint will still surface the problem, but the service stays up.
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM gas.platform_fee_rules WHERE active = true",
    )
    .fetch_one(pool)
    .await?;

    Ok(count)
}

async fn ensure_collector_address(pool: &PgPool, chain: &str, address: &str) -> Result<()> {
    sqlx::query(
        r#"
        INSERT INTO gas.fee_collector_addresses (chain, address, active)
        SELECT $1, $2, true
        WHERE NOT EXISTS (
            SELECT 1 FROM gas.fee_collector_addresses
            WHERE chain = $1 AND address = $2
        )
        "#,
    )
    .bind(chain)
    .bind(address)
    .execute(pool)
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ensure_platform_fee_rule(
    pool: &PgPool,
    chain: &str,
    operation: &str,
    fee_type: &str,
    flat_amount: Decimal,
    percent_bp: i32,
    min_fee: Decimal,
    max_fee: Option<Decimal>,
    priority: i32,
) -> Result<()> {
    // Insert only if no active rule exists for the same (chain, operation) at the same priority.
    // This keeps the operation idempotent even if startup runs multiple times.
    sqlx::query(
        r#"
        INSERT INTO gas.platform_fee_rules (
            chain, operation, fee_type,
            flat_amount, percent_bp, min_fee, max_fee,
            priority, active
        )
        SELECT $1, $2, $3, $4, $5, $6, $7, $8, true
        WHERE NOT EXISTS (
            SELECT 1 FROM gas.platform_fee_rules
            WHERE chain = $1 AND operation = $2 AND active = true AND priority = $8
        )
        "#,
    )
    .bind(chain)
    .bind(operation)
    .bind(fee_type)
    .bind(flat_amount)
    .bind(percent_bp)
    .bind(min_fee)
    .bind(max_fee)
    .bind(priority)
    .execute(pool)
    .await?;

    Ok(())
}

/// Seed baseline platform fee rules + collector addresses when the DB is empty.
///
/// Why this exists:
/// - In production, migrations may fail/stop early but the server continues (best-effort migrations).
/// - Without seeded `gas.platform_fee_rules`, `/api/v1/fees/calculate` returns 400 and breaks frontend flows.
pub async fn seed_platform_fee_rules_if_empty(pool: &PgPool) -> Result<()> {
    let active_count = match query_active_rule_count(pool).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(error = %e, "platform_fee_rules_check_failed");
            return Ok(());
        }
    };

    if active_count > 0 {
        return Ok(());
    }

    tracing::warn!("no_active_platform_fee_rules_found; seeding_baseline_defaults");

    // Collector addresses (baseline defaults; must be valid EVM addresses)
    ensure_collector_address(
        pool,
        "ethereum",
        "0x742d35cc6634c0532925a3b844bc9e7595f0beb6",
    )
    .await?;
    ensure_collector_address(pool, "bsc", "0x8894E0a0c962CB723c1976a4421c95949bE2D4E3").await?;
    ensure_collector_address(
        pool,
        "polygon",
        "0x9965507D1a55bcC2695C58ba16FB37d819B0A4dc",
    )
    .await?;

    // Platform fee rules (subset aligned with migrations/0005_insert_platform_fee_rules.sql)
    let d0 = Decimal::new(0, 0);

    // Swap - 0.5% (50 bp)
    ensure_platform_fee_rule(
        pool,
        "ethereum",
        "swap",
        "percent",
        d0,
        50,
        Decimal::new(1, 3),       // 0.001
        Some(Decimal::new(5, 2)), // 0.05
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "bsc",
        "swap",
        "percent",
        d0,
        50,
        Decimal::new(1, 2),       // 0.01
        Some(Decimal::new(5, 1)), // 0.5
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "polygon",
        "swap",
        "percent",
        d0,
        50,
        Decimal::new(1, 1),       // 0.1
        Some(Decimal::new(5, 0)), // 5.0
        100,
    )
    .await?;

    // Transfer - 0.1% (10 bp)
    ensure_platform_fee_rule(
        pool,
        "ethereum",
        "transfer",
        "percent",
        d0,
        10,
        Decimal::new(1, 4),       // 0.0001
        Some(Decimal::new(1, 2)), // 0.01
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "bsc",
        "transfer",
        "percent",
        d0,
        10,
        Decimal::new(1, 3),       // 0.001
        Some(Decimal::new(1, 1)), // 0.1
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "polygon",
        "transfer",
        "percent",
        d0,
        10,
        Decimal::new(1, 2),       // 0.01
        Some(Decimal::new(1, 0)), // 1.0
        100,
    )
    .await?;

    // Fiat onramp - 2.0% (200 bp)
    for chain in ["ethereum", "bsc", "polygon"] {
        ensure_platform_fee_rule(
            pool,
            chain,
            "fiat_onramp",
            "percent",
            d0,
            200,
            Decimal::new(1, 0),         // 1.0
            Some(Decimal::new(100, 0)), // 100.0
            100,
        )
        .await?;
    }

    // Fiat offramp - 2.5% (250 bp)
    for chain in ["ethereum", "bsc", "polygon"] {
        ensure_platform_fee_rule(
            pool,
            chain,
            "fiat_offramp",
            "percent",
            d0,
            250,
            Decimal::new(2, 0),         // 2.0
            Some(Decimal::new(150, 0)), // 150.0
            100,
        )
        .await?;
    }

    // Limit order - 0.5% (50 bp)
    ensure_platform_fee_rule(
        pool,
        "ethereum",
        "limit_order",
        "percent",
        d0,
        50,
        Decimal::new(1, 3),
        Some(Decimal::new(5, 2)),
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "bsc",
        "limit_order",
        "percent",
        d0,
        50,
        Decimal::new(1, 2),
        Some(Decimal::new(5, 1)),
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "polygon",
        "limit_order",
        "percent",
        d0,
        50,
        Decimal::new(1, 1),
        Some(Decimal::new(5, 0)),
        100,
    )
    .await?;

    // Bridge - 1.0% (100 bp)
    ensure_platform_fee_rule(
        pool,
        "ethereum",
        "bridge",
        "percent",
        d0,
        100,
        Decimal::new(5, 3),
        Some(Decimal::new(1, 1)),
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "bsc",
        "bridge",
        "percent",
        d0,
        100,
        Decimal::new(5, 2),
        Some(Decimal::new(1, 0)),
        100,
    )
    .await?;
    ensure_platform_fee_rule(
        pool,
        "polygon",
        "bridge",
        "percent",
        d0,
        100,
        Decimal::new(5, 1),
        Some(Decimal::new(10, 0)),
        100,
    )
    .await?;

    tracing::info!("platform_fee_rules_seeded");
    Ok(())
}
