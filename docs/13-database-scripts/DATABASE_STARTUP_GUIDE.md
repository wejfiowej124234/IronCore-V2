# 数据库启动指南

## 🚀 快速启动

### 方法 1: 使用启动脚本（推荐）

#### Windows
```bash
cd IronCore-V2
scripts\start-database.bat
```

#### Linux/Mac/Git Bash
```bash
cd IronCore-V2
./scripts/start-database.sh
```

### 方法 2: 手动启动

```bash
cd ops
docker compose up -d cockroach
```

### 方法 3: 启动所有服务

```bash
cd ops
docker compose up -d
```

---

## ✅ 验证数据库运行

### 检查容器状态

```bash
docker ps --filter "name=cockroachdb"
```

应该看到：
```
NAMES                      STATUS         PORTS
ironwallet-cockroachdb     Up X minutes   0.0.0.0:26257->26257/tcp, 0.0.0.0:8090->8080/tcp
```

### 测试连接

```bash
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "SELECT version();"
```

---

## 🔧 故障排查

### 问题 1: 容器未启动

**症状**: `docker ps` 看不到容器

**解决**:
```bash
# 检查所有容器（包括停止的）
docker ps -a --filter "name=cockroachdb"

# 启动容器
docker start ironwallet-cockroachdb

# 或重新创建
cd ops
docker compose up -d cockroach
```

### 问题 2: 端口被占用

**症状**: `Error: bind: address already in use`

**解决**:
```bash
# 检查端口占用
netstat -ano | findstr :26257  # Windows
lsof -i :26257                 # Linux/Mac

# 停止占用端口的进程或修改 docker-compose.yml 中的端口映射
```

### 问题 3: 连接被拒绝

**症状**: `error communicating with database: 由于目标计算机积极拒绝，无法连接`

**解决**:
1. 确保容器正在运行：`docker ps --filter "name=cockroachdb"`
2. 等待容器完全启动（通常需要 10-20 秒）
3. 检查容器日志：`docker logs ironwallet-cockroachdb`
4. 验证端口映射：`docker ps --filter "name=cockroachdb" --format "{{.Ports}}"`

### 问题 4: Docker 未运行

**症状**: `Cannot connect to the Docker daemon`

**解决**:
- Windows: 启动 Docker Desktop
- Linux: `sudo systemctl start docker`
- Mac: 启动 Docker Desktop

---

## 📊 数据库信息

### 连接信息

- **SQL 端口**: `26257`
- **Admin UI**: `http://localhost:8090`
- **数据库 URL**: `postgresql://root@localhost:26257/ironcore?sslmode=disable`

### 常用命令

```bash
# 进入数据库 CLI
docker exec -it ironwallet-cockroachdb cockroach sql --insecure

# 创建数据库
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "CREATE DATABASE IF NOT EXISTS ironcore;"

# 查看数据库列表
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "SHOW DATABASES;"

# 查看表
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "USE ironcore; SHOW TABLES;"

# 查看容器日志
docker logs ironwallet-cockroachdb

# 停止容器
docker stop ironwallet-cockroachdb

# 启动容器
docker start ironwallet-cockroachdb

# 重启容器
docker restart ironwallet-cockroachdb
```

---

## 🎯 启动后下一步

1. **验证数据库运行**
   ```bash
   docker ps --filter "name=cockroachdb"
   ```

2. **运行迁移**
   ```bash
   cd IronCore-V2
   ./scripts/run-migrations-cockroachdb.sh
   ```

3. **或启动应用（自动迁移）**
   ```bash
   cd IronCore-V2
   cargo run
   ```

---

## 📚 相关文档

- [数据库重置指南](./RESET_DATABASE_GUIDE.md)
- [迁移脚本修复说明](./MIGRATION_QUICK_FIX.md)

