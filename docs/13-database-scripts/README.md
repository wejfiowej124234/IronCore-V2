# 数据库脚本使用指南

## 📋 可用脚本

### 🔄 数据库重置脚本

#### 1. `reset-database.ps1` (PowerShell)
完全重置数据库 - Windows PowerShell 版本

```powershell
cd IronCore-V2
.\scripts\reset-database.ps1
.\scripts\reset-database.ps1 -Force  # 跳过确认
```

#### 2. `reset-database.bat` (CMD)
完全重置数据库 - Windows CMD 版本

```cmd
cd IronCore-V2
scripts\reset-database.bat
```

#### 3. `reset-database.sh` (Bash) ✅ 新增
完全重置数据库 - Linux/Mac/Git Bash 版本

```bash
cd IronCore-V2
./scripts/reset-database.sh
./scripts/reset-database.sh --force  # 跳过确认
```

**功能**：
- ✅ 自动检测并停止所有 CockroachDB 容器
- ✅ 自动检测并删除所有数据卷
- ✅ 重新启动容器
- ✅ 等待数据库就绪（健康检查）

---

### 🚀 数据库迁移脚本

#### 1. `run-migrations-cockroachdb.bat` (CMD)
运行数据库迁移 - Windows CMD 版本

```cmd
cd IronCore-V2
scripts\run-migrations-cockroachdb.bat
```

#### 2. `run-migrations-cockroachdb.sh` (Bash) ✅ 新增
运行数据库迁移 - Linux/Mac/Git Bash 版本

```bash
cd IronCore-V2
./scripts/run-migrations-cockroachdb.sh
```

**功能**：
- ✅ 自动检测 DATABASE_URL
- ✅ 从 config.toml 读取配置
- ✅ 使用 sqlx migrate run

---

### 🔧 简单重置脚本（代码方式）

#### 1. `reset-db-simple.bat` (CMD)
通过环境变量触发重置

```cmd
cd IronCore-V2
scripts\reset-db-simple.bat
```

#### 2. `reset-db-simple.sh` (Bash) ✅ 新增
通过环境变量触发重置

```bash
cd IronCore-V2
./scripts/reset-db-simple.sh
```

**功能**：
- ✅ 设置 RESET_DB=true
- ✅ 启动应用自动重置
- ✅ 重置后退出

---

## 🎯 快速开始

### 在 Git Bash 中使用

```bash
# 1. 进入项目目录
cd IronCore-V2

# 2. 重置数据库（完全清空）
./scripts/reset-database.sh

# 3. 启动应用（迁移会自动执行）
cargo run
```

### 在 PowerShell 中使用

```powershell
# 1. 进入项目目录
cd IronCore-V2

# 2. 重置数据库（完全清空）
.\scripts\reset-database.ps1

# 3. 启动应用（迁移会自动执行）
cargo run
```

### 在 CMD 中使用

```cmd
# 1. 进入项目目录
cd IronCore-V2

# 2. 重置数据库（完全清空）
scripts\reset-database.bat

# 3. 启动应用（迁移会自动执行）
cargo run
```

---

## 📚 详细文档

查看完整文档：[RESET_DATABASE_GUIDE.md](./RESET_DATABASE_GUIDE.md)

---

## ⚠️ 重要提示

1. **仅用于开发环境**：所有重置脚本都会删除所有数据
2. **生产环境禁止使用**：绝对不要在生产环境运行重置脚本
3. **备份重要数据**：重置前确保不需要的数据
4. **检查环境**：确保 Docker 和 docker-compose 已安装并运行

---

## 🔍 故障排查

### 脚本无法执行（Bash）

```bash
# 确保脚本有执行权限（Linux/Mac）
chmod +x scripts/*.sh

# 在 Git Bash 中，脚本通常可以直接运行
```

### Docker 未运行

```bash
# 检查 Docker 状态
docker ps

# 启动 Docker Desktop（Windows/Mac）
# 或启动 Docker 服务（Linux）
sudo systemctl start docker
```

### 找不到 docker-compose.yml

```bash
# 确保在项目根目录运行脚本
# 脚本会自动查找 ops/docker-compose.yml
```

---

## 📞 支持

如有问题，请查看：
- [数据库迁移指南](../../docs/11-development/DATABASE_MIGRATION_GUIDE.md)
- [故障排查指南](../../docs/01-开发指南-Development-Guide/01-基础开发-Basic-Development/07-故障排查-Troubleshooting/TROUBLESHOOTING.md)
