//! 数据库迁移回滚支持
//! 提供迁移回滚SQL执行功能

use anyhow::{Context, Result};
use sqlx::PgPool;
use std::fs;
use std::path::Path;

/// 执行迁移回滚SQL
///
/// # Arguments
/// * `pool` - 数据库连接池
/// * `migration_name` - 迁移文件名（不含扩展名）
///
/// # Returns
/// 如果回滚成功返回Ok(())
pub async fn execute_rollback_sql(pool: &PgPool, migration_name: &str) -> Result<()> {
    // 查找对应的down迁移文件
    // SQLx迁移文件命名格式：{timestamp}_{name}.sql
    // down迁移文件命名格式：{timestamp}_{name}.down.sql
    let migrations_dir = Path::new("./migrations");

    // 查找所有迁移文件
    let entries = fs::read_dir(migrations_dir).context("Failed to read migrations directory")?;

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            // 检查是否是down迁移文件
            if file_name.ends_with(".down.sql") {
                // 提取迁移名称（去掉时间戳和.down.sql）
                if let Some(base_name) = extract_migration_name(file_name) {
                    if base_name == migration_name {
                        // 读取并执行回滚SQL
                        let sql = fs::read_to_string(&path)
                            .context(format!("Failed to read rollback SQL file: {:?}", path))?;

                        tracing::info!("Executing rollback SQL for migration: {}", migration_name);
                        sqlx::query(&sql).execute(pool).await.context(format!(
                            "Failed to execute rollback SQL for migration: {}",
                            migration_name
                        ))?;

                        tracing::info!(
                            "Rollback SQL executed successfully for migration: {}",
                            migration_name
                        );
                        return Ok(());
                    }
                }
            }
        }
    }

    Err(anyhow::anyhow!(
        "Rollback SQL file not found for migration: {}",
        migration_name
    ))
}

/// 从文件名提取迁移名称
fn extract_migration_name(file_name: &str) -> Option<String> {
    // 文件名格式：{timestamp}_{name}.down.sql
    // 例如：0001_init.down.sql -> init
    if let Some(name_without_ext) = file_name.strip_suffix(".down.sql") {
        // 找到最后一个下划线的位置
        name_without_ext
            .rfind('_')
            .map(|underscore_pos| name_without_ext[underscore_pos + 1..].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_migration_name() {
        assert_eq!(
            extract_migration_name("0001_init.down.sql"),
            Some("init".to_string())
        );
        assert_eq!(
            extract_migration_name("0002_add_password_hash.down.sql"),
            Some("add_password_hash".to_string())
        );
    }
}
