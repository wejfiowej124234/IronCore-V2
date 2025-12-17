use anyhow::Result;
use ironcore::infrastructure::db::PgPool;
use sqlx::Row;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn ensure_required_schemas(pool: &PgPool) -> Result<()> {
    for schema in ["gas", "admin", "notify", "tokens", "events", "fiat"] {
        sqlx::query(&format!("CREATE SCHEMA IF NOT EXISTS {schema};"))
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn run_migrations_with_checksum_repair(pool: &PgPool) -> Result<()> {
    let migrator = sqlx::migrate!("./migrations");

    async fn repair_checksum(pool: &PgPool, version: i64, checksum: &[u8]) -> Result<u64> {
        let rows = sqlx::query(
            "SELECT table_schema FROM information_schema.tables WHERE table_name = '_sqlx_migrations'",
        )
        .fetch_all(pool)
        .await?;

        let mut total_rows_affected: u64 = 0;

        for row in rows {
            let schema: String = row.try_get("table_schema")?;
            let schema_escaped = schema.replace('"', "\"\"");
            let sql = format!(
                "UPDATE \"{}\"._sqlx_migrations SET checksum = $1 WHERE version = $2",
                schema_escaped
            );
            let result = sqlx::query(&sql)
                .bind(checksum)
                .bind(version)
                .execute(pool)
                .await?;
            total_rows_affected = total_rows_affected.saturating_add(result.rows_affected());
        }

        if total_rows_affected == 0 {
            let result =
                sqlx::query("UPDATE _sqlx_migrations SET checksum = $1 WHERE version = $2")
                    .bind(checksum)
                    .bind(version)
                    .execute(pool)
                    .await?;
            total_rows_affected = total_rows_affected.saturating_add(result.rows_affected());
        }

        Ok(total_rows_affected)
    }

    for attempt in 1..=20 {
        match migrator.run(pool).await {
            Ok(_) => {
                tracing::info!("✅ Database migrations completed");
                return Ok(());
            }
            Err(sqlx::migrate::MigrateError::VersionMismatch(version)) => {
                let Some(migration) = migrator.migrations.iter().find(|m| m.version == version)
                else {
                    anyhow::bail!(
                        "migration checksum mismatch at version {version}, but migration is missing from binary"
                    );
                };

                tracing::warn!(
                    "⚠️ Migration {version} checksum mismatch; repairing _sqlx_migrations (attempt {attempt}/20)"
                );

                let rows_affected =
                    repair_checksum(pool, version, migration.checksum.as_ref()).await?;
                tracing::warn!(
                    "⚠️ Migration {version} checksum repair rows_affected={rows_affected}"
                );
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    anyhow::bail!("migration checksum repair exceeded retry limit")
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ironcore=info,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    ensure_required_schemas(&pool).await?;
    run_migrations_with_checksum_repair(&pool).await?;

    tracing::info!("✅ Migration runner finished successfully");
    Ok(())
}
