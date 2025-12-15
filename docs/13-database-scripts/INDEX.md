# 🔧 IronCore 脚本索引

> ironforge_backend - IronCore 后端模块脚本导航

## 📂 脚本分类

- [setup/](./setup/) - **环境搭建** (1个)
- [test/](./test/) - **测试脚本** (1个)
- [utils/](./utils/) - **工具脚本** (1个)

**总计**: 3个脚本

## 🚀 常用命令

### 启动后端服务

#### Windows
```bash
scripts\setup\start-backend.bat
```

#### Linux/Mac
```bash
# 使用 Cargo
cd IronCore
cargo run

# 或使用配置文件
CONFIG_PATH=config.toml cargo run
```

**功能**:
- 启动 Axum Web 服务器
- 默认监听端口: 8088
- 连接数据库、Redis、Immudb
- 支持配置文件和环境变量

### 运行测试

#### 多链 API 测试
```bash
./scripts/test/test-multi-chain-api.sh
```

**测试内容**:
- Ethereum API 测试
- BSC API 测试
- Polygon API 测试
- 钱包创建与查询
- 交易发送与查询

### 工具脚本

#### 临时禁用多链功能
```bash
./scripts/utils/disable-multi-chain-temp.sh
```

**用途**:
- 开发时临时禁用多链功能
- 快速切换到单链模式
- 调试特定链的问题

## 📋 脚本详情

### setup/ - 环境搭建 (1个)

#### start-backend.bat (Windows)
```batch
@echo off
REM 启动 IronCore 后端服务
cd IronCore
cargo run
```

**用途**:
- Windows 系统启动脚本
- 启动 Axum 服务器
- 自动加载配置文件

**配置**:
- 读取 `config.toml`
- 支持 `CONFIG_PATH` 环境变量
- 支持 `.env` 文件

### test/ - 测试脚本 (1个)

#### test-multi-chain-api.sh
```bash
#!/bin/bash
# 测试多链钱包 API
set -e

echo "Testing Ethereum API..."
curl -X POST http://localhost:8088/api/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"chain":"ethereum"}'

echo "Testing BSC API..."
curl -X POST http://localhost:8088/api/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"chain":"bsc"}'

echo "Testing Polygon API..."
curl -X POST http://localhost:8088/api/wallet/create \
  -H "Content-Type: application/json" \
  -d '{"chain":"polygon"}'
```

**测试内容**:
- 钱包创建 API
- 钱包查询 API
- 交易发送 API
- 余额查询 API
- 多链支持验证

### utils/ - 工具脚本 (1个)

#### disable-multi-chain-temp.sh
```bash
#!/bin/bash
# 临时禁用多链功能
sed -i 's/enable_multi_chain = true/enable_multi_chain = false/' config.toml
echo "多链功能已禁用"
```

**用途**:
- 快速切换配置
- 开发调试使用
- 不影响代码

## 🔍 按场景查找

### 日常开发
```bash
# 1. 启动基础设施（Docker）
cd ops
docker-compose up -d

# 2. 启动后端服务
scripts/setup/start-backend.bat    # Windows
cargo run                           # Linux/Mac

# 3. 查看日志
tail -f IronCore/backend.log
```

### API 测试
```bash
# 运行多链 API 测试
./scripts/test/test-multi-chain-api.sh

# 或手动测试
curl http://localhost:8088/api/health
curl http://localhost:8088/api-docs/openapi.yaml
```

### 问题排查
```bash
# 检查服务状态
curl http://localhost:8088/api/health

# 查看日志
tail -f IronCore/backend.log

# 临时禁用多链
./scripts/utils/disable-multi-chain-temp.sh
```

### 生产部署
```bash
# 1. 构建 Release 版本
cargo build --release

# 2. 运行生产服务
./target/release/ironforge_backend

# 3. 使用 systemd（Linux）
sudo systemctl start ironforge-backend
sudo systemctl status ironforge-backend
```

## 📝 脚本开发规范

### 命名规范
- Windows: `kebab-case.bat`
- Linux/Mac: `kebab-case.sh`
- 使用描述性名称

### 文件头注释
```bash
#!/bin/bash
# ============================================
# 脚本名称: test-multi-chain-api.sh
# 功能描述: 测试多链钱包 API
# 使用方法: ./test-multi-chain-api.sh
# 前置条件: IronCore 服务已启动
# 作者: IronCore Team
# 更新日期: 2025-11-24
# ============================================
set -e  # 遇到错误立即退出
```

### 错误处理
- 使用 `set -e` 在错误时退出
- 提供清晰的错误信息
- 记录关键操作日志

## 🔗 相关资源

- [文档索引](../docs/INDEX.md) - backend 文档导航
- [部署指南](../docs/05-deployment/DEPLOYMENT.md) - 部署流程
- [多链架构](../docs/01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md) - 架构说明
- [根目录脚本索引](../../scripts/INDEX.md) - 项目总脚本索引

## 🛠️ 开发工具

### Cargo (Rust 构建工具)
```bash
cargo build                 # 编译
cargo run                   # 运行
cargo test                  # 测试
cargo clippy                # 代码检查
cargo fmt                   # 格式化
cargo build --release       # 生产构建
```

### 数据库工具
```bash
# SQLx 迁移
sqlx migrate run
sqlx migrate revert

# CockroachDB CLI
cockroach sql --insecure

# Redis CLI
redis-cli -h localhost -p 6379
```

### Docker 管理
```bash
# 启动所有服务
docker-compose -f ops/docker-compose.yml up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down
```

## 📊 监控与调试

### 健康检查
```bash
curl http://localhost:8088/api/health
```

### OpenAPI 文档
```bash
# 查看 API 文档
curl http://localhost:8088/api-docs/openapi.yaml

# 浏览器访问
http://localhost:8088/api-docs
```

### Prometheus 指标
```bash
curl http://localhost:8088/metrics
```

### 日志文件
- `IronCore/backend.log` - 主日志
- `IronCore/backend-debug-run.log` - 调试日志
- `IronCore/backend_output.log` - 输出日志

## 📅 脚本维护

### 新增脚本
1. 确定脚本类型（setup/test/utils）
2. 放入对应目录
3. 添加执行权限: `chmod +x script.sh`
4. 更新本索引文档

### 废弃脚本
1. 移至 `archive/` 目录
2. 更新本索引文档
3. 添加废弃说明

---

**脚本总数**: 3个  
**最后更新**: 2025-11-24  
**维护者**: IronCore Team
