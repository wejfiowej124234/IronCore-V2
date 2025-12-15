# 🔐 生产环境配置指南

## 概述

Backend 项目现已完全移除所有硬编码的测试/Demo数据，改为使用**配置驱动**的生产级实现。

---

## ⚠️ 关键配置项

### 1. 区块链 RPC 端点配置

**配置文件**: `backend/config.toml`

```toml
[blockchain]
eth_rpc_url = "https://eth-mainnet.g.alchemy.com/v2/YOUR_ALCHEMY_API_KEY"
bsc_rpc_url = "https://bsc-dataseed1.binance.org"
polygon_rpc_url = "https://polygon-rpc.com"
solana_rpc_url = "https://api.mainnet-beta.solana.com"
bitcoin_rpc_url = "https://blockstream.info/api"
```

**环境变量**（优先级高于配置文件）:
```bash
export ETH_RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
export BSC_RPC_URL="https://bsc-dataseed1.binance.org"
export POLYGON_RPC_URL="https://polygon-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
export BITCOIN_RPC_URL="https://blockstream.info/api"
```

### 2. 跨链桥手续费配置

**配置文件**:
```toml
[cross_chain]
bridge_fee_percentage = 0.003      # 桥接费 0.3%
transaction_fee_percentage = 0.001 # 交易费 0.1%
```

**环境变量**:
```bash
export BRIDGE_FEE_PERCENTAGE="0.003"      # 0.3%
export TRANSACTION_FEE_PERCENTAGE="0.001" # 0.1%
```

---

## 🚀 快速启动

### 本地开发环境

```bash
# 1. 复制示例配置
cp backend/config.example.toml backend/config.toml

# 2. 编辑配置文件，填入真实 API 密钥
nano backend/config.toml

# 3. 启动基础设施（可选，如果不需要数据库可跳过）
cd ops && docker compose up -d

# 4. 启动后端
cd backend && CONFIG_PATH=config.toml cargo run
```

### 生产环境部署

```bash
# 使用环境变量（推荐）
export ETH_RPC_URL="https://eth-mainnet.g.alchemy.com/v2/PRODUCTION_API_KEY"
export BSC_RPC_URL="https://bsc-dataseed1.binance.org"
export POLYGON_RPC_URL="https://polygon-rpc.com"
export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
export BITCOIN_RPC_URL="https://blockstream.info/api"

export BRIDGE_FEE_PERCENTAGE="0.004"  # 生产环境可能需要更高手续费
export TRANSACTION_FEE_PERCENTAGE="0.001"

cargo run --release
```

---

## 🔑 API 密钥获取

### Ethereum/Polygon (Alchemy)
1. 访问 [https://www.alchemy.com/](https://www.alchemy.com/)
2. 注册账号
3. 创建 App，选择 Ethereum Mainnet 或 Polygon Mainnet
4. 复制 API Key 替换 `YOUR_ALCHEMY_API_KEY`

### Ethereum/Polygon (Infura)
1. 访问 [https://www.infura.io/](https://www.infura.io/)
2. 注册账号
3. 创建项目
4. 使用 `https://mainnet.infura.io/v3/YOUR_PROJECT_ID`

### BSC (Binance Smart Chain)
公共 RPC 端点：
- `https://bsc-dataseed1.binance.org`
- `https://bsc-dataseed2.binance.org`
- `https://bsc-dataseed3.binance.org`

无需 API 密钥，但有速率限制。

### Solana
公共 RPC 端点：
- `https://api.mainnet-beta.solana.com` (免费，有限速)

推荐使用 [QuickNode](https://www.quicknode.com/) 或 [Helius](https://www.helius.dev/) 获取高性能端点。

### Bitcoin
公共 API：
- `https://blockstream.info/api` (Blockstream)
- `https://blockchain.info/rawaddr/ADDRESS` (Blockchain.com)

---

## 📊 配置验证

### 检查配置是否正确加载

启动后端后，查看日志：

```bash
# 应该看到以下日志
[INFO] Blockchain RPC configuration loaded:
  - ETH: https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY
  - BSC: https://bsc-dataseed1.binance.org
  - Polygon: https://polygon-rpc.com
  - Solana: https://api.mainnet-beta.solana.com
  - Bitcoin: https://blockstream.info/api

[INFO] Cross-chain fee configuration:
  - Bridge fee: 0.3%
  - Transaction fee: 0.1%
```

### 测试 RPC 连接

```bash
# 测试 Ethereum RPC
curl https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'

# 测试 Solana RPC
curl https://api.mainnet-beta.solana.com \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

---

## ⚙️ 高级配置

### 配置优先级

1. **环境变量** (最高优先级)
2. **配置文件** (`config.toml`)
3. **默认值** (代码中的 `Default` 实现)

### 动态调整手续费

生产环境可以通过环境变量动态调整手续费，无需重启服务：

```bash
# 方法1：启动前设置
export BRIDGE_FEE_PERCENTAGE="0.005"  # 0.5%

# 方法2：Docker 容器
docker run -e BRIDGE_FEE_PERCENTAGE=0.005 ironforge-backend
```

### 多环境配置

```bash
# 开发环境
CONFIG_PATH=config.dev.toml cargo run

# 测试环境
CONFIG_PATH=config.test.toml cargo run

# 生产环境
CONFIG_PATH=config.prod.toml cargo run
```

---

## 🛡️ 安全最佳实践

### 1. API 密钥保护

❌ **不要**：
- 将 API 密钥提交到 Git
- 在日志中打印完整 API 密钥
- 在客户端代码中硬编码 API 密钥

✅ **应该**：
- 使用环境变量或密钥管理系统（如 AWS Secrets Manager）
- 在 `.gitignore` 中添加 `config.toml`
- 日志中只显示脱敏后的密钥（如 `***KEY_SUFFIX`）

### 2. RPC 端点监控

生产环境建议：
- 使用 RPC 故障转移功能（`enable_rpc_failover = true`）
- 配置多个备用 RPC 端点
- 监控 RPC 调用成功率和延迟

### 3. 手续费合理性检查

建议手续费范围：
- **桥接费**: 0.1% - 1.0% (典型值 0.3%)
- **交易费**: 0.05% - 0.5% (典型值 0.1%)

如果手续费过高，用户可能流失；如果过低，可能无法覆盖成本。

---

## 🔍 故障排查

### 问题1: "Invalid API key" 错误

**原因**: API 密钥未正确配置或已过期

**解决**:
```bash
# 检查环境变量
echo $ETH_RPC_URL

# 测试 API 密钥
curl https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

### 问题2: "Rate limit exceeded" 错误

**原因**: 免费 RPC 端点达到速率限制

**解决**:
1. 升级到付费 RPC 服务（Alchemy Pro, Infura Growth）
2. 启用 RPC 故障转移，配置多个端点
3. 实现本地缓存减少 RPC 调用

### 问题3: 余额查询失败

**原因**: RPC 端点不可用或地址格式错误

**解决**:
```bash
# 检查 RPC 端点健康状态
curl https://api.mainnet-beta.solana.com \
  -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# 检查日志
grep "Failed to fetch balance" backend/logs/app.log
```

---

## 📝 配置示例

### 完整生产配置 (`config.prod.toml`)

```toml
# IronForge Backend - Production Configuration

[database]
url = "postgresql://user:password@db-prod.example.com:5432/ironforge?sslmode=require"
max_connections = 32
min_connections = 8
acquire_timeout_secs = 10
idle_timeout_secs = 600

[redis]
url = "redis://:REDIS_PASSWORD@redis-prod.example.com:6379"

[immudb]
addr = "immudb-prod.example.com:3322"
user = "immudb"
password = "SECURE_PASSWORD"
database = "ironforge_audit"

[jwt]
secret = "PRODUCTION_JWT_SECRET_AT_LEAST_32_CHARACTERS_LONG"
token_expiry_secs = 3600      # 1小时
refresh_token_expiry_secs = 2592000  # 30天

[server]
bind_addr = "0.0.0.0:8088"
allow_degraded_start = false  # 生产环境必须完整启动

[logging]
level = "info"
format = "json"  # 生产环境使用 JSON 格式便于解析
enable_file_logging = true
log_file_path = "/var/log/ironforge/app.log"
max_file_size_mb = 200
max_files = 30

[monitoring]
enable_prometheus = true
prometheus_bind_addr = "0.0.0.0:9090"
enable_health_check = true

[features]
enable_fee_system = true
enable_rpc_failover = true
enable_notify_system = true

[blockchain]
eth_rpc_url = "https://eth-mainnet.g.alchemy.com/v2/PRODUCTION_API_KEY"
bsc_rpc_url = "https://bsc-dataseed1.binance.org"
polygon_rpc_url = "https://polygon-mainnet.g.alchemy.com/v2/PRODUCTION_API_KEY"
solana_rpc_url = "https://solana-mainnet.g.alchemy.com/v2/PRODUCTION_API_KEY"
bitcoin_rpc_url = "https://blockstream.info/api"

[cross_chain]
bridge_fee_percentage = 0.004      # 0.4%
transaction_fee_percentage = 0.001 # 0.1%
```

---

## 🎯 总结

### 已移除的硬编码内容
✅ Demo Alchemy API 端点 (`/v2/demo`)  
✅ 固定的跨链桥手续费 (0.4%)  
✅ 硬编码的 Solana RPC URL  
✅ 测试用的 example.com 域名（仅限测试代码）  

### 现在使用的生产级方案
✅ 配置文件驱动的 RPC 端点  
✅ 环境变量支持  
✅ 可调整的手续费配置  
✅ 多链 RPC 统一管理  
✅ 完整的配置验证和日志记录  

### 后续优化建议
- [ ] 实现 RPC 端点健康检查和自动故障转移
- [ ] 添加 RPC 调用缓存减少请求频率
- [ ] 支持自定义 RPC 超时配置
- [ ] 实现 API 密钥轮换机制
- [ ] 添加 Prometheus 指标监控 RPC 调用性能

---

**文档版本**: v1.0  
**更新日期**: 2025-11-24  
**联系方式**: 见项目 README.md
