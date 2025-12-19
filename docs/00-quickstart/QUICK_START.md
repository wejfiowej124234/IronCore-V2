# 🚀 快速启动指南

## ✅ 当前状态

- ✅ CockroachDB 已启动并运行
- ✅ 数据库 `ironcore` 已创建
- ⏳ 等待执行迁移

---

## 🎯 下一步：执行迁移

### 方法 1: 启动应用自动迁移（最简单，推荐）

```bash
cd IronCore-V2
cargo run
```

应用启动时会：
1. 自动连接数据库
2. 自动执行所有迁移文件
3. 创建所有表和索引
4. 插入初始数据

### 方法 2: 使用 sqlx-cli 手动迁移

```bash
# 安装 sqlx-cli（如果还没有）
cargo install sqlx-cli

# 设置数据库 URL
export DATABASE_URL="postgresql://root@localhost:26257/ironcore?sslmode=disable"

# 执行迁移
cd IronCore-V2
sqlx migrate run
```

### 方法 3: 使用迁移脚本

```bash
cd IronCore-V2
./scripts/run-migrations-cockroachdb.sh
```

---

## 📊 验证迁移

迁移完成后，可以验证：

```bash
# 查看所有表
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "USE ironcore; SELECT table_schema, COUNT(*) FROM information_schema.tables WHERE table_schema IN ('public', 'gas', 'admin', 'notify', 'tokens', 'events', 'fiat') GROUP BY table_schema;"

# 查看迁移记录
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "USE ironcore; SELECT version, name FROM schema_migrations ORDER BY version;"
```

应该看到：
- 7 个 Schema
- 38 个表
- 13 个迁移记录

---

## 🔧 如果遇到问题

### 问题 1: 迁移失败

**解决**: 检查数据库连接
```bash
docker ps --filter "name=cockroachdb"
docker logs ironwallet-cockroachdb
```

### 问题 2: 表已存在错误

**解决**: 重置数据库
```bash
cd IronCore-V2
RESET_DB=true cargo run
```

或使用重置脚本：
```bash
./scripts/reset-database.sh --force
```

### 问题 3: 连接被拒绝

**解决**: 确保 CockroachDB 正在运行
```bash
cd ops
docker compose up -d cockroach
```

---

## ✅ 迁移完成后

迁移完成后，应用就可以正常使用了！

- ✅ 所有表已创建
- ✅ 所有约束已添加
- ✅ 所有索引已优化
- ✅ 初始数据已插入

现在可以：
1. 启动应用：`cargo run`
2. 访问 API：`http://localhost:8088`
3. 查看 Admin UI：`http://localhost:8090`

---

## 📚 相关文档

- [数据库启动指南](./scripts/DATABASE_STARTUP_GUIDE.md)
- [迁移脚本修复说明](./scripts/MIGRATION_QUICK_FIX.md)
- [数据库重置指南](./scripts/RESET_DATABASE_GUIDE.md)

