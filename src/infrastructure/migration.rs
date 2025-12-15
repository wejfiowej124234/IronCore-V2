//! 数据库迁移管理模块
//! 提供迁移版本管理、执行日志和回滚功能

use anyhow::{Context, Result};
use chrono::Utc;
use sqlx::{PgPool, Row};

/// 迁移记录表名
#[allow(dead_code)]
const MIGRATION_TABLE: &str = "schema_migrations";

/// 迁移信息
#[derive(Debug, Clone)]
pub struct MigrationInfo {
    pub version: i64,
    pub name: String,
    pub applied_at: chrono::DateTime<Utc>,
}

/// 初始化迁移表
///
/// CockroachDB兼容：
/// - 使用BIGINT而非INTEGER，提高兼容性
/// - 使用TIMESTAMPTZ而非TIMESTAMP，支持时区
/// - 使用CURRENT_TIMESTAMP而非now()，更标准化
pub async fn init_migration_table(pool: &PgPool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            version BIGINT PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await
    .context("Failed to create migration table")?;

    Ok(())
}

/// 获取已应用的迁移版本列表
pub async fn get_applied_migrations(pool: &PgPool) -> Result<Vec<MigrationInfo>> {
    init_migration_table(pool).await?;

    let rows =
        sqlx::query("SELECT version, name, applied_at FROM schema_migrations ORDER BY version")
            .fetch_all(pool)
            .await
            .context("Failed to query applied migrations")?;

    let migrations = rows
        .into_iter()
        .map(|row| MigrationInfo {
            version: row.get(0),
            name: row.get(1),
            applied_at: row.get(2),
        })
        .collect();

    Ok(migrations)
}

/// 记录迁移执行
///
/// CockroachDB兼容：使用ON CONFLICT (version)确保幂等性
pub async fn record_migration(pool: &PgPool, version: i64, name: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO schema_migrations (version, name) VALUES ($1, $2) ON CONFLICT (version) DO NOTHING"
    )
    .bind(version)
    .bind(name)
    .execute(pool)
    .await
    .context("Failed to record migration")?;

    Ok(())
}

/// 检查迁移是否已应用
pub async fn is_migration_applied(pool: &PgPool, version: i64) -> Result<bool> {
    init_migration_table(pool).await?;

    let row = sqlx::query("SELECT 1 FROM schema_migrations WHERE version = $1")
        .bind(version)
        .fetch_optional(pool)
        .await
        .context("Failed to check migration status")?;

    Ok(row.is_some())
}

/// 执行迁移（带版本记录）
///
/// 注意：CockroachDB不支持advisory locks，所以sqlx migrate可能会失败
/// 如果失败，请手动运行: sqlx migrate run --database-url "$DATABASE_URL"
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    tracing::info!("Running database migrations...");

    // 初始化迁移表
    init_migration_table(pool).await?;

    // 使用sqlx的迁移功能
    // 注意：CockroachDB可能不支持advisory locks，导致迁移失败
    // 如果失败，迁移表仍然会被创建，但迁移不会执行
    let migrations = sqlx::migrate!("./migrations");

    // 运行迁移（CockroachDB兼容处理）
    // 注意：CockroachDB不支持PostgreSQL的advisory locks
    // 但sqlx migrate会尝试使用，如果失败会给出明确错误
    match migrations.run(pool).await {
        Ok(_) => {
            // 记录已应用的迁移
            let applied = get_applied_migrations(pool).await?;
            tracing::info!("✅ Applied {} migrations", applied.len());
            tracing::info!("✅ Database migrations completed successfully");
            Ok(())
        }
        Err(e) => {
            // 检查是否是advisory lock错误
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("advisory")
                || error_msg.contains("lock")
                || error_msg.contains("pg_advisory")
            {
                tracing::warn!("⚠️  CockroachDB doesn't support advisory locks.");
                tracing::info!("🔄 Attempting manual migration execution...");

                // 尝试手动执行迁移（绕过advisory locks）
                match crate::infrastructure::migration_cockroachdb::run_migrations_manual(pool)
                    .await
                {
                    Ok(_) => {
                        tracing::info!("✅ Manual migrations completed successfully");
                        // 记录已应用的迁移
                        let applied = get_applied_migrations(pool).await?;
                        tracing::info!("✅ Total {} migrations applied", applied.len());
                        return Ok(());
                    }
                    Err(manual_err) => {
                        tracing::warn!("⚠️  Manual migration also failed: {}", manual_err);
                        tracing::warn!("⚠️  Please run migrations manually using the script:");
                        tracing::warn!("⚠️  IronCore\\scripts\\run-migrations-cockroachdb.bat");
                        tracing::warn!(
                            "⚠️  Or manually: sqlx migrate run --database-url \"$DATABASE_URL\""
                        );
                        tracing::warn!("⚠️  Or set SKIP_MIGRATIONS=true to skip migrations");
                        // 对于CockroachDB，这不算致命错误，可以继续运行（但功能受限）
                        return Err(e).context(
                            "Migration failed due to CockroachDB advisory lock limitation",
                        );
                    }
                }
            }
            // 其他错误直接返回
            Err(e).context("Failed to run migrations")
        }
    }
}

/// 回滚到指定版本
///
/// # Arguments
/// * `pool` - 数据库连接池
/// * `target_version` - 目标版本号（回滚到此版本）
///
/// # Returns
/// 如果回滚成功返回Ok(())
///
/// # Note
/// 回滚会执行对应的.down.sql文件（如果存在）
/// 如果.down.sql文件不存在，只会删除迁移记录
pub async fn rollback_to_version(pool: &PgPool, target_version: i64) -> Result<()> {
    tracing::warn!("Rolling back to version {}", target_version);

    let applied = get_applied_migrations(pool).await?;

    // 找到需要回滚的迁移（版本号大于target_version的）
    let to_rollback: Vec<_> = applied
        .into_iter()
        .filter(|m| m.version > target_version)
        .collect();

    if to_rollback.is_empty() {
        tracing::info!("No migrations to rollback");
        return Ok(());
    }

    tracing::info!("Rolling back {} migrations", to_rollback.len());

    // 尝试执行回滚SQL（如果存在）
    for migration in to_rollback.iter().rev() {
        // 尝试执行回滚SQL
        if let Err(e) =
            crate::infrastructure::migration_rollback::execute_rollback_sql(pool, &migration.name)
                .await
        {
            tracing::warn!("Failed to execute rollback SQL for migration {}: {}. Continuing with record removal only.", migration.name, e);
        }

        // 删除迁移记录
        sqlx::query("DELETE FROM schema_migrations WHERE version = $1")
            .bind(migration.version)
            .execute(pool)
            .await
            .context(format!(
                "Failed to remove migration record for version {}",
                migration.version
            ))?;

        tracing::info!(
            "Removed migration record: {} (version {})",
            migration.name,
            migration.version
        );
    }

    tracing::info!("Rollback completed successfully");
    Ok(())
}

/// 获取当前迁移版本
pub async fn get_current_version(pool: &PgPool) -> Result<Option<i64>> {
    let migrations = get_applied_migrations(pool).await?;
    Ok(migrations.last().map(|m| m.version))
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_migration_table_creation() {
        // 这个测试需要实际的数据库连接
        // 在实际测试中，应该使用测试数据库
    }
}
