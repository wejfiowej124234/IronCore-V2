//! CockroachDB 迁移执行模块
//! 手动执行迁移文件，绕过 advisory locks 限制

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::fs;
use std::path::{Path, PathBuf};

/// 分割SQL语句（按分号分割，但保留字符串和注释的完整性）
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut string_char = None;
    let mut in_comment = false;
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                current.push(ch);
            } else {
                current.push(ch);
            }
            continue;
        }

        if !in_string && ch == '-' && chars.peek() == Some(&'-') {
            // 单行注释开始
            in_comment = true;
            current.push(ch);
            if let Some(next) = chars.next() {
                current.push(next);
            }
            continue;
        }

        if !in_string && (ch == '\'' || ch == '"') {
            // 字符串开始
            in_string = true;
            string_char = Some(ch);
            current.push(ch);
        } else if in_string && ch == string_char.unwrap() {
            // 检查是否是转义的引号
            if chars.peek() == Some(&ch) {
                // 转义的引号（'' 或 ""）
                current.push(ch);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else {
                // 字符串结束
                in_string = false;
                string_char = None;
                current.push(ch);
            }
        } else if !in_string && ch == ';' {
            // 语句结束
            current.push(ch);
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                statements.push(trimmed);
            }
            current.clear();
        } else {
            current.push(ch);
        }
    }

    // 添加最后一个语句（如果没有以分号结尾）
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        statements.push(trimmed);
    }

    statements
}

/// 查找迁移目录
/// 按优先级查找：./migrations -> ./IronCore/migrations -> ../migrations
fn find_migrations_dir() -> Result<PathBuf> {
    let candidates = vec![
        Path::new("./migrations"),
        Path::new("./IronCore/migrations"),
        Path::new("../migrations"),
        Path::new("migrations"),
    ];

    let searched_paths: Vec<String> = candidates
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    for candidate in &candidates {
        if candidate.exists() && candidate.is_dir() {
            tracing::debug!("Found migrations directory: {:?}", candidate);
            return Ok(candidate.to_path_buf());
        }
    }

    anyhow::bail!(
        "Migrations directory not found. Searched: {:?}",
        searched_paths
    );
}

/// 获取迁移状态摘要
pub async fn get_migration_status(pool: &PgPool) -> Result<MigrationStatus> {
    // 初始化迁移表（如果不存在）
    crate::infrastructure::migration::init_migration_table(pool).await?;

    // 获取已应用的迁移
    let applied = crate::infrastructure::migration::get_applied_migrations(pool).await?;

    // 获取所有迁移文件
    let migrations_dir = find_migrations_dir()?;
    let entries = fs::read_dir(&migrations_dir).context("Failed to read migrations directory")?;

    let mut total_files = 0;
    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sql" {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy();
                    if let Some(version_str) = file_name_str.split('_').next() {
                        if version_str.parse::<i64>().is_ok() {
                            total_files += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(MigrationStatus {
        total_migrations: total_files,
        applied_count: applied.len(),
        latest_version: applied.last().map(|m| m.version),
    })
}

/// 迁移状态信息
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    pub total_migrations: usize,
    pub applied_count: usize,
    pub latest_version: Option<i64>,
}

/// 手动执行迁移文件（用于CockroachDB）
/// 绕过advisory locks限制，直接执行SQL文件
pub async fn run_migrations_manual(pool: &PgPool) -> Result<()> {
    tracing::info!("🚀 Starting database migration upgrade...");

    // 显示迁移状态
    match get_migration_status(pool).await {
        Ok(status) => {
            tracing::info!(
                "📊 Migration status: {}/{} applied, latest version: {:?}",
                status.applied_count,
                status.total_migrations,
                status.latest_version
            );
        }
        Err(e) => {
            tracing::warn!("⚠️  Could not get migration status: {}", e);
        }
    }

    // 初始化迁移表
    crate::infrastructure::migration::init_migration_table(pool).await?;

    // 获取迁移目录（自动查找）
    let migrations_dir = find_migrations_dir()?;
    tracing::info!("📁 Using migrations directory: {:?}", migrations_dir);

    // 读取所有迁移文件
    let mut migration_files: Vec<(i64, String, String)> = Vec::new();
    let entries = fs::read_dir(&migrations_dir).context("Failed to read migrations directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "sql" {
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy();
                    // 解析文件名格式: 0001_name.sql
                    if let Some(version_str) = file_name_str.split('_').next() {
                        if let Ok(version) = version_str.parse::<i64>() {
                            let content = fs::read_to_string(&path)
                                .context(format!("Failed to read migration file: {:?}", path))?;
                            let name = file_name_str
                                .strip_suffix(".sql")
                                .unwrap_or(&file_name_str)
                                .to_string();
                            migration_files.push((version, name, content));
                        }
                    }
                }
            }
        }
    }

    // 按版本排序
    migration_files.sort_by_key(|(v, _, _)| *v);

    if migration_files.is_empty() {
        tracing::warn!("⚠️  No migration files found in {:?}", migrations_dir);
        return Ok(());
    }

    tracing::info!("📋 Found {} migration file(s)", migration_files.len());

    // 执行每个迁移
    let mut applied_count = 0;
    let mut skipped_count = 0;
    for (version, name, sql) in migration_files {
        // 检查是否已应用
        let is_applied = crate::infrastructure::migration::is_migration_applied(pool, version)
            .await
            .context("Failed to check migration status")?;

        if is_applied {
            tracing::debug!(
                "⏭️  Migration {} ({}) already applied, skipping",
                version,
                name
            );
            skipped_count += 1;
            continue;
        }

        tracing::info!("🔄 Applying migration {}: {}", version, name);

        // 分割SQL语句（按分号分割，但忽略字符串和注释中的分号）
        let statements = split_sql_statements(&sql);

        if statements.is_empty() {
            tracing::warn!(
                "⚠️  No SQL statements found in migration {}: {}",
                version,
                name
            );
            continue;
        }

        tracing::debug!(
            "  Found {} SQL statement(s) in migration {}",
            statements.len(),
            version
        );

        // 为每个语句使用独立的事务，避免"约束已存在"错误导致整个事务失败
        for (idx, statement) in statements.iter().enumerate() {
            let statement = statement.trim();
            // 跳过空语句和注释
            if statement.is_empty() || statement.starts_with("--") {
                continue;
            }

            // 显示更详细的执行信息
            let statement_preview = if statement.len() > 80 {
                format!("{}...", &statement[..80])
            } else {
                statement.to_string()
            };

            tracing::debug!(
                "Executing statement {}/{} of migration {}: {}",
                idx + 1,
                statements.len(),
                version,
                statement_preview
            );

            // 为每个语句使用独立的事务
            let mut tx = pool.begin().await.context(format!(
                "Failed to start transaction for statement {} of migration {}: {}",
                idx + 1,
                version,
                name
            ))?;

            // 执行SQL语句，捕获详细错误
            match sqlx::query(statement).execute(&mut *tx).await {
                Ok(_) => {
                    tracing::debug!("  ✓ Statement {} executed successfully", idx + 1);
                    // 提交事务
                    tx.commit().await.context(format!(
                        "Failed to commit statement {} of migration {}: {}",
                        idx + 1,
                        version,
                        name
                    ))?;
                }
                Err(e) => {
                    let error_str = e.to_string();
                    // 检查是否是"约束已存在"或"IF NOT EXISTS"相关的错误
                    // 这些错误可以安全忽略，因为约束/索引已经存在
                    if error_str.contains("already exists")
                        || error_str.contains("duplicate key")
                        || error_str.contains("duplicate constraint name")
                        || (error_str.contains("constraint")
                            && error_str.contains("already exists"))
                        || (error_str.contains("index") && error_str.contains("already exists"))
                    {
                        // 回滚当前事务（虽然失败了，但需要清理）
                        let _ = tx.rollback().await;

                        tracing::warn!(
                            "  ⚠️  Statement {}: Constraint/index already exists, skipping: {}",
                            idx + 1,
                            if error_str.len() > 150 {
                                format!("{}...", &error_str[..150])
                            } else {
                                error_str.clone()
                            }
                        );
                        // 继续执行下一个语句，不返回错误
                        continue;
                    }

                    // 检查是否是 CockroachDB 的特殊错误：需要使用 DROP INDEX 代替 DROP CONSTRAINT
                    if error_str.contains("cannot drop UNIQUE constraint")
                        && error_str.contains("use DROP INDEX CASCADE")
                    {
                        // 回滚当前事务
                        let _ = tx.rollback().await;

                        tracing::warn!(
                            "  ⚠️  Statement {}: CockroachDB requires DROP INDEX for UNIQUE constraints, skipping: {}",
                            idx + 1,
                            if error_str.len() > 150 {
                                format!("{}...", &error_str[..150])
                            } else {
                                error_str.clone()
                            }
                        );
                        // 继续执行下一个语句，不返回错误
                        continue;
                    }

                    // 回滚事务
                    let _ = tx.rollback().await;

                    let error_msg =
                        format!(
                        "Failed to execute statement {} of migration {}: {}\nSQL: {}\nError: {}",
                        idx + 1, version, name,
                        if statement.len() > 500 {
                            format!("{}...", &statement[..500])
                        } else {
                            statement.to_string()
                        },
                        e
                    );
                    tracing::error!("{}", error_msg);
                    return Err(anyhow::anyhow!(error_msg));
                }
            }
        }

        // 记录迁移
        crate::infrastructure::migration::record_migration(pool, version, &name)
            .await
            .context(format!("Failed to record migration {}: {}", version, name))?;

        applied_count += 1;
        tracing::info!("✅ Migration {} ({}) applied successfully", version, name);
    }

    // 显示最终状态
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if applied_count > 0 {
        tracing::info!("✅ Migration upgrade completed!");
        tracing::info!("   • Applied: {} migration(s)", applied_count);
        if skipped_count > 0 {
            tracing::info!(
                "   • Skipped: {} migration(s) (already applied)",
                skipped_count
            );
        }
    } else {
        tracing::info!("✅ All migrations already applied");
        tracing::info!("   • Total: {} migration(s)", skipped_count);
    }
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// 重置数据库：删除所有迁移记录，强制重新运行所有迁移
/// ⚠️ 警告：这会删除所有迁移记录，下次启动时会重新运行所有迁移
/// 如果表已存在，迁移可能会失败（因为表已存在）
pub async fn reset_migration_records(pool: &PgPool) -> Result<()> {
    tracing::warn!("⚠️  Resetting migration records - all migrations will be re-run on next start");

    sqlx::query("DELETE FROM schema_migrations")
        .execute(pool)
        .await
        .context("Failed to delete migration records")?;

    tracing::info!("✅ Migration records cleared");
    Ok(())
}

/// 完全重置数据库：删除所有表和数据
/// ⚠️ 警告：这会删除所有数据！仅用于开发环境
pub async fn drop_all_tables(pool: &PgPool) -> Result<()> {
    tracing::warn!("⚠️  DROPPING ALL TABLES - ALL DATA WILL BE LOST!");

    // 获取所有表名（包括所有schema）
    let tables: Vec<String> = sqlx::query_scalar(
        r#"
        SELECT table_schema || '.' || table_name as full_table_name
        FROM information_schema.tables
        WHERE table_type = 'BASE TABLE'
        AND table_schema NOT IN ('pg_catalog', 'information_schema', 'pg_extension', 'crdb_internal')
        ORDER BY table_schema, table_name
        "#
    )
    .fetch_all(pool)
    .await
    .context("Failed to query table names")?;

    if tables.is_empty() {
        tracing::info!("No tables to drop");
        return Ok(());
    }

    tracing::info!("Found {} tables to drop", tables.len());

    // 删除所有表（CASCADE会自动处理外键）
    let mut tx = pool.begin().await.context("Failed to start transaction")?;

    for table in &tables {
        tracing::info!("Dropping table: {}", table);
        sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table))
            .execute(&mut *tx)
            .await
            .with_context(|| format!("Failed to drop table: {}", table))?;
    }

    tx.commit().await.context("Failed to commit transaction")?;

    tracing::info!("✅ All tables dropped successfully");
    Ok(())
}

/// 完全重置数据库并重新运行迁移（开发环境专用）
///
/// 这会：
/// 1. 删除所有表和数据
/// 2. 删除迁移记录
/// 3. 重新运行所有迁移
///
/// ⚠️ 警告：这会删除所有数据！仅用于开发环境
pub async fn reset_database_clean(pool: &PgPool) -> Result<()> {
    tracing::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::warn!("⚠️  COMPLETE DATABASE RESET - ALL DATA WILL BE LOST!");
    tracing::warn!("⚠️  This is for DEVELOPMENT ONLY!");
    tracing::warn!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // 步骤1: 删除所有表
    tracing::info!("Step 1/3: Dropping all tables...");
    drop_all_tables(pool).await?;

    // 步骤2: 删除迁移记录（如果表还存在）
    tracing::info!("Step 2/3: Clearing migration records...");
    // 先尝试删除迁移表（如果还存在）
    let _ = sqlx::query("DROP TABLE IF EXISTS schema_migrations CASCADE")
        .execute(pool)
        .await;

    // 步骤3: 重新运行所有迁移
    tracing::info!("Step 3/3: Running migrations on clean database...");
    run_migrations_manual(pool).await?;

    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    tracing::info!("✅ Database reset complete! Fresh database ready.");
    tracing::info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
