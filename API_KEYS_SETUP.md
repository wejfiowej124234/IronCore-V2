# IronCore 生产环境 API 密钥配置指南

## ⚠️ 重要提示

IronCore 是**生产级别**的钱包后端系统，所有外部 API 集成必须使用真实的 API 密钥。**不使用任何 Mock 数据**。

---

## 🔑 必需的 API 密钥

### 1. **1inch Swap Aggregator** (必需 - Swap功能)

**用途**: 提供代币交换聚合服务  
**申请地址**: https://portal.1inch.dev/  
**费用**: 免费层级可用 (有请求限额)

**配置步骤**:
1. 访问 https://portal.1inch.dev/ 注册账号
2. 创建新的 API Key
3. 复制 API Key
4. 配置到 `config.toml`:

```toml
[external_apis.oneinch]
api_key = "YOUR_1INCH_API_KEY_HERE"
enabled = true
```

**或使用环境变量**:
```bash
export ONEINCH_API_KEY="YOUR_1INCH_API_KEY_HERE"
```

**支持的链**:
- Ethereum (chain_id: 1)
- BSC (chain_id: 56)
- Polygon (chain_id: 137)
- Optimism (chain_id: 10)
- Arbitrum (chain_id: 42161)

---

### 2. **CoinGecko Price API** (可选 - 价格数据)

**用途**: 实时代币价格数据  
**申请地址**: https://www.coingecko.com/en/api  
**费用**: 免费层级 10-50 calls/min，企业版无限制

**配置步骤**:
1. 访问 https://www.coingecko.com/en/api 注册
2. 获取 API Key
3. 配置到 `config.toml`:

```toml
[external_apis.coingecko]
api_key = "YOUR_COINGECKO_API_KEY_HERE"
enabled = true
rate_limit_per_minute = 50
```

---

## 🚀 启动检查清单

在启动生产环境之前，确保：

- [ ] ✅ 1inch API Key 已配置 (`config.toml` 或环境变量)
- [ ] ✅ 1inch API `enabled = true`
- [ ] ✅ 数据库连接正常 (CockroachDB/PostgreSQL)
- [ ] ✅ Redis 连接正常
- [ ] ✅ ImmuDB 连接正常
- [ ] ✅ JWT Secret 已配置 (强随机密钥)
- [ ] ✅ `allow_degraded_start = false` (生产模式)

---

## 🛡️ 安全最佳实践

1. **不要将 API 密钥提交到 Git**
   - 使用 `.env` 文件或环境变量
   - `.env` 文件已在 `.gitignore` 中

2. **使用密钥管理服务**
   - AWS Secrets Manager
   - Azure Key Vault
   - HashiCorp Vault

3. **定期轮换密钥**
   - 每 90 天轮换一次 API 密钥
   - 使用多个密钥实现零停机轮换

4. **监控 API 使用量**
   - 设置 1inch API 请求限额告警
   - 监控 502/503 错误率

---

## 🔧 故障排查

### 问题: Swap API 返回 401 Unauthorized

**原因**: 1inch API Key 未配置或无效

**解决方案**:
1. 检查 `config.toml` 中的 `api_key` 配置
2. 检查环境变量 `ONEINCH_API_KEY`
3. 验证 API Key 是否有效（访问 1inch Portal 检查）
4. 确认 `enabled = true`

### 问题: Swap API 返回 502 Bad Gateway

**可能原因**:
1. 1inch API 服务暂时不可用
2. API Key 配额已用完
3. 网络连接问题

**解决方案**:
1. 检查 1inch API 状态: https://status.1inch.io/
2. 检查 API 使用量配额
3. 查看后端日志: `journalctl -u ironcore -f`

### 问题: Swap API 返回 429 Too Many Requests

**原因**: API 请求速率超限

**解决方案**:
1. 升级到更高的 1inch API 层级
2. 实现客户端请求缓存
3. 添加请求去重机制

---

## 📊 生产环境配置示例

### config.toml (生产环境)

```toml
[database]
url = "postgresql://root@prod-db:26257/ironcore?sslmode=require"
max_connections = 32
min_connections = 8

[redis]
url = "rediss://:STRONG_PASSWORD@prod-redis:6379"

[jwt]
secret = "CRYPTOGRAPHICALLY_STRONG_RANDOM_SECRET_64_BYTES"
token_expiry_secs = 3600

[server]
bind_addr = "0.0.0.0:8088"
allow_degraded_start = false  # 生产模式：不允许降级启动
skip_db_check = false

[logging]
level = "info"
format = "json"
enable_file_logging = true

[monitoring]
enable_prometheus = true
enable_health_check = true

[external_apis.oneinch]
api_key = "YOUR_PRODUCTION_1INCH_API_KEY"
enabled = true
timeout_secs = 30
supported_chains = [1, 56, 137, 10, 42161]

[external_apis.coingecko]
api_key = "YOUR_PRODUCTION_COINGECKO_API_KEY"
enabled = true
rate_limit_per_minute = 100
```

### 环境变量方式 (.env)

```bash
# Database
DATABASE_URL=postgresql://root@prod-db:26257/ironcore?sslmode=require

# Redis
REDIS_URL=rediss://:STRONG_PASSWORD@prod-redis:6379

# JWT
JWT_SECRET=CRYPTOGRAPHICALLY_STRONG_RANDOM_SECRET_64_BYTES

# External APIs
ONEINCH_API_KEY=YOUR_PRODUCTION_1INCH_API_KEY
COINGECKO_API_KEY=YOUR_PRODUCTION_COINGECKO_API_KEY

# Monitoring
PROMETHEUS_ENABLED=true
```

---

## 📝 注意事项

1. **免费层级限制**:
   - 1inch: ~300-500 请求/分钟
   - CoinGecko: 10-50 请求/分钟

2. **企业级建议**:
   - 使用 1inch Enterprise Plan (无限制 + 专属支持)
   - 使用 CoinGecko Pro Plan (更高限额)

3. **成本估算**:
   - 1inch Free: $0/月 (适合测试)
   - 1inch Growth: $49/月 (适合小型生产)
   - 1inch Business: 自定义报价 (适合大规模生产)

---

## 🔗 相关资源

- 1inch API 文档: https://docs.1inch.io/
- 1inch Portal: https://portal.1inch.dev/
- CoinGecko API 文档: https://www.coingecko.com/en/api/documentation
- IronCore 架构文档: `docs/ARCHITECTURE_OVERVIEW.md`
- 部署指南: `docs/DEPLOYMENT_GUIDE.md`

---

**生成时间**: 2025-12-11  
**维护者**: IronCore Development Team  
**状态**: 生产级别 - 不使用任何Mock数据
