-- 重置迁移状态（强制重新应用）
-- 使用场景：迁移文件被修改后需要重新运行

-- 删除所有迁移记录（保留数据）
DELETE FROM _sqlx_migrations WHERE version > 1;

-- 或者完全重置（谨慎使用）
-- TRUNCATE _sqlx_migrations;

SELECT 'Migration state reset. Run cargo sqlx migrate run again.' AS status;

