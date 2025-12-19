# 数据库迁移指南

> 基于标准化迁移文件的完整迁移指南

## 📋 目录

- [迁移文件结构](#迁移文件结构)
- [执行迁移](#执行迁移)
- [重置数据库](#重置数据库)
- [迁移最佳实践](#迁移最佳实践)
- [故障排查](#故障排查)

---

## 迁移文件结构

所有迁移文件按照数据库标准最佳实践组织，位于 `IronCore/migrations/` 目录：

### 执行顺序

1. **0001_schemas.sql** - 创建所有 Schema
2. **0002_core_tables.sql** - 核心业务表（不含外键）
3. **0003_gas_tables.sql** - 费用系统表
4. **0004_admin_tables.sql** - 管理员和RPC表
5. **0005_notify_tables.sql** - 通知系统表
6. **0006_asset_tables.sql** - 资产聚合表
7. **0007_tokens_tables.sql** - 代币注册表
8. **0008_events_tables.sql** - 事件总线表
9. **0009_fiat_tables.sql** - 法币系统表
10. **0010_constraints.sql** - 外键和唯一约束
11. **0011_indexes.sql** - 所有索引
12. **0012_check_constraints.sql** - 检查约束
13. **0013_initial_data.sql** - 初始数据

### 设计原则

1. **分离关注点**: Schema → 表 → 约束 → 索引 → 数据
2. **依赖顺序**: 先创建被依赖的表，再创建依赖表
3. **幂等性**: 所有操作可重复执行
4. **CockroachDB 兼容**: 使用标准 SQL 语法

---

## 执行迁移

### 自动迁移（推荐）

启动应用时自动执行：

```bash
cd IronCore-V2
cargo run
```

应用启动时会：
1. 检查数据库连接
2. 自动执行未应用的迁移
3. 记录迁移状态

### 手动迁移

#### Windows

```bash
scripts\run-migrations-cockroachdb.bat
```

#### Linux/Mac/Git Bash

```bash
./scripts/run-migrations-cockroachdb.sh
```

#### 使用 sqlx-cli

```bash
# 安装 sqlx-cli
cargo install sqlx-cli

# 执行迁移
cd IronCore-V2
sqlx migrate run --database-url "postgresql://root@localhost:26257/ironcore?sslmode=disable"
```

### 环境变量

- `DATABASE_URL`: 数据库连接URL（可选，会从 `config.toml` 读取）
- `RESET_DB=true`: 重置数据库（开发环境）

---

## 重置数据库

### 完全重置（开发环境）

```bash
# 使用重置脚本
./scripts/reset-database.sh --force

# 或使用环境变量
RESET_DB=true cargo run
```

### 手动重置

```bash
# 1. 停止应用
# 2. 删除数据库
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "DROP DATABASE IF EXISTS ironcore;"

# 3. 重新创建数据库
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "CREATE DATABASE ironcore;"

# 4. 运行迁移
./scripts/run-migrations-cockroachdb.sh
```

---

## 迁移最佳实践

### 1. 创建新迁移

```bash
# 使用 sqlx-cli 创建新迁移文件
sqlx migrate add <migration_name>

# 例如
sqlx migrate add add_user_avatar_column
```

### 2. 迁移文件命名

- 使用版本号前缀：`0014_<description>.sql`
- 描述要清晰：`0014_add_user_avatar_column.sql`
- 保持版本号连续

### 3. 迁移内容规范

```sql
-- 使用 IF NOT EXISTS
CREATE TABLE IF NOT EXISTS new_table (...);

-- 使用 DROP IF EXISTS 然后 ADD
ALTER TABLE existing_table
    DROP CONSTRAINT IF EXISTS old_constraint;
ALTER TABLE existing_table
    ADD CONSTRAINT new_constraint ...;

-- 使用 ON CONFLICT DO NOTHING
INSERT INTO table VALUES (...)
ON CONFLICT (key) DO NOTHING;
```

### 4. 测试迁移

```bash
# 1. 在测试环境测试
# 2. 验证迁移可重复执行
# 3. 检查数据完整性
# 4. 验证性能影响
```

---

## 故障排查

### 迁移失败

1. **检查数据库连接**
   ```bash
   docker ps | grep cockroachdb
   ```

2. **检查迁移状态**
   ```sql
   SELECT * FROM _sqlx_migrations ORDER BY version;
   ```

3. **查看错误日志**
   - 检查应用日志
   - 检查数据库日志

### 常见问题

#### 1. 迁移已应用但表不存在

```bash
# 清理迁移记录
DELETE FROM _sqlx_migrations WHERE version = <version>;

# 重新运行迁移
sqlx migrate run
```

#### 2. 外键约束错误

```bash
# 检查依赖顺序
# 确保被依赖的表先创建
```

#### 3. 唯一约束冲突

```bash
# 检查现有数据
SELECT * FROM <table> WHERE <column> = <value>;

# 清理重复数据
DELETE FROM <table> WHERE id NOT IN (
    SELECT MIN(id) FROM <table> GROUP BY <column>
);
```

---

## 迁移状态查询

### 查看已应用的迁移

```sql
SELECT version, name, applied_at 
FROM _sqlx_migrations 
ORDER BY version;
```

### 查看表结构

```sql
-- 查看所有表
SELECT table_name 
FROM information_schema.tables 
WHERE table_schema = 'public'
ORDER BY table_name;

-- 查看表结构
\d <table_name>
```

---

## 相关文档

- [数据库 Schema 文档](../02-configuration/DATABASE_SCHEMA.md)
- [迁移文件说明](../../migrations/README.md)
- [数据库重置指南](../../scripts/RESET_DATABASE_GUIDE.md)

