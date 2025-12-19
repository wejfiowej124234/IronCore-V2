# 迁移脚本修复说明

## 🔧 修复内容

已修复所有迁移脚本中从 `config.toml` 读取数据库 URL 的问题。

### 修复的脚本
- ✅ `run-migrations-cockroachdb.sh` - Bash 脚本
- ✅ `run-migrations-cockroachdb.bat` - Windows 批处理脚本
- ✅ `migrate-database.sh` - 通用迁移脚本
- ✅ `migrate-database.bat` - 通用迁移脚本

### 修复的问题
- **问题**: 脚本错误地读取了 `config.toml` 中的 URL，导致读取到 "disable" 而不是完整的数据库 URL
- **原因**: 正则表达式匹配不准确，没有正确匹配 `[database]` 部分的 `url` 字段
- **修复**: 使用更精确的匹配逻辑，确保读取完整的数据库 URL

---

## 🚀 现在可以正常使用

### 方法 1: 使用环境变量（推荐）

```bash
export DATABASE_URL="postgresql://root@localhost:26257/ironcore?sslmode=disable"
./scripts/run-migrations-cockroachdb.sh
```

### 方法 2: 从 config.toml 自动读取（已修复）

```bash
# 脚本会自动从 config.toml 读取数据库 URL
./scripts/run-migrations-cockroachdb.sh
```

### 方法 3: 启动应用自动迁移（推荐）

```bash
cd IronCore-V2
cargo run
```

应用启动时会自动执行迁移。

---

## ✅ 验证修复

运行迁移脚本，应该看到正确的数据库 URL：

```bash
./scripts/run-migrations-cockroachdb.sh
```

输出应该显示：
```
[INFO] Database URL: postgresql://root@localhost:26257/ironcore?sslmode=disable
```

而不是之前的：
```
[INFO] Database URL: disable  ❌
```

---

## 📝 注意事项

1. **确保 CockroachDB 正在运行**
   ```bash
   docker ps | grep cockroachdb
   ```

2. **确保数据库已创建**
   - 应用启动时会自动创建数据库
   - 或手动创建：`docker exec ironwallet-cockroachdb cockroach sql --insecure -e "CREATE DATABASE IF NOT EXISTS ironcore;"`

3. **如果迁移失败**
   - 检查数据库连接
   - 检查 CockroachDB 是否运行
   - 检查 `config.toml` 中的数据库 URL 是否正确

---

## 🔗 相关文档

- [数据库重置指南](./RESET_DATABASE_GUIDE.md)
- [迁移文件说明](../migrations/README.md)

