# 数据库重置指南

## ⚠️ 警告

**这些重置功能仅用于开发环境！生产环境请勿使用！**

重置会删除所有数据库表和数据，包括：
- 所有用户数据
- 所有钱包数据
- 所有交易记录
- 所有配置数据

---

## 🎯 为什么需要重置？

在开发环境中，重置数据库可以：
- ✅ **干净开始**：从空数据库开始，避免旧数据干扰
- ✅ **测试迁移**：验证迁移脚本是否正确执行
- ✅ **解决冲突**：清除部分迁移状态导致的冲突
- ✅ **快速恢复**：比手动清理表更快

---

## 🚀 使用方法

### 方法1：使用 Docker 重置（推荐）✅ 已完成

**支持所有平台**：
- ✅ Windows PowerShell
- ✅ Windows CMD
- ✅ Linux/Mac/Git Bash

完全删除容器和数据卷，重新创建：

```bash
# Windows (PowerShell) - 推荐
.\scripts\reset-database.ps1

# Windows (CMD)
scripts\reset-database.bat

# Linux/Mac/Git Bash (Bash)
./scripts/reset-database.sh

# 跳过确认提示
.\scripts\reset-database.ps1 -Force        # PowerShell
./scripts/reset-database.sh --force        # Bash
```

**功能特点**：
- ✅ **自动检测**：自动查找所有相关容器和数据卷
- ✅ **完全清理**：删除容器和数据卷，彻底清空
- ✅ **健康检查**：等待数据库完全就绪
- ✅ **详细日志**：显示每个步骤的执行状态
- ✅ **错误处理**：完善的错误处理和提示

**执行步骤**：
1. 查找并停止所有 CockroachDB 容器
2. 删除所有容器
3. 查找并删除所有数据卷
4. 重新启动容器
5. 等待数据库就绪（最多60秒）

**优点**：
- 最彻底，完全清空
- 不依赖应用代码
- 适合 Docker 环境
- 自动化程度高

---

### 方法2：通过环境变量重置（代码方式）

在启动应用时设置 `RESET_DB=true`：

```bash
# Windows (CMD)
set RESET_DB=true
cargo run

# Windows (PowerShell)
$env:RESET_DB="true"
cargo run

# Linux/Mac/Git Bash
export RESET_DB=true
cargo run
# 或者一行命令
RESET_DB=true cargo run
```

**优点**：
- 不需要手动操作 Docker
- 重置后自动运行迁移
- 适合快速开发迭代

**注意**：重置完成后，应用会继续运行。如果只想重置不启动服务，可以使用方法3。

---

### 方法3：使用简单脚本

```bash
# Windows (CMD)
scripts\reset-db-simple.bat

# Windows (PowerShell)
.\scripts\reset-db-simple.bat

# Linux/Mac/Git Bash
./scripts/reset-db-simple.sh
```

这个脚本会：
1. 确认操作
2. 设置 `RESET_DB=true`
3. 启动应用（重置后会自动退出）

---

## 📋 重置后的步骤

重置完成后：

1. **数据库已清空**：所有表和数据已删除
2. **迁移已执行**：所有迁移文件已重新运行
3. **数据库结构完整**：表结构已恢复到最新版本

现在可以：
- 启动应用正常使用
- 运行测试
- 重新创建测试数据

---

## 🔧 技术细节

### 重置过程

1. **删除所有表**：包括所有 schema 中的表
2. **删除迁移记录**：清空 `schema_migrations` 表
3. **重新运行迁移**：按顺序执行所有迁移文件

### 相关函数

- `drop_all_tables()` - 删除所有表
- `reset_database_clean()` - 完全重置并重新迁移
- `reset_migration_records()` - 只重置迁移记录

---

## 🛡️ 安全建议

1. **开发环境专用**：生产环境绝对不要使用
2. **备份重要数据**：重置前确保不需要的数据
3. **版本控制**：确保迁移文件已提交到 Git
4. **测试迁移**：重置后验证迁移是否正确执行

---

## ❓ 常见问题

### Q: 重置后数据还能恢复吗？
A: 不能。重置会永久删除所有数据。请确保在开发环境中使用。

### Q: 重置会影响 Docker 容器吗？
A: 方法1会删除容器和数据卷。方法2和方法3只操作数据库内容，不删除容器。

### Q: 重置后迁移失败怎么办？
A: 检查迁移文件是否有语法错误，或查看日志了解具体错误。

### Q: 可以只重置迁移记录吗？
A: 可以，使用 `reset_migration_records()` 函数，但表已存在时迁移可能会失败。

---

## 📝 示例

### 完整重置流程

```bash
# 1. 停止应用（如果正在运行）
# Ctrl+C

# 2. 重置数据库
.\scripts\reset-database.ps1

# 3. 启动应用（迁移会自动执行）
cargo run
```

### 快速重置（代码方式）

```bash
# 一行命令完成重置和启动
$env:RESET_DB="true"; cargo run
```

---

## 🔗 相关文档

- [数据库迁移指南](../docs/11-development/DATABASE_MIGRATION_GUIDE.md)
- [Docker Compose 配置](../../ops/docker-compose.yml)
- [迁移文件目录](../migrations/)

