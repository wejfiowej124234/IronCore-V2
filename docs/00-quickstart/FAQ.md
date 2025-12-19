# ❓ 常见问题 FAQ

> 新手最常遇到的问题和解决方案

## 📚 目录

- [环境配置](#环境配置)
- [API使用](#api使用)
- [钱包相关](#钱包相关)
- [交易相关](#交易相关)
- [错误处理](#错误处理)
- [性能优化](#性能优化)
- [安全相关](#安全相关)

---

## 环境配置

### Q1: `cargo run` 第一次启动很慢？

**A**: 正常现象！

- **原因**: Rust需要编译所有依赖（50+个crate）
- **耗时**: 10-20分钟（取决于电脑性能）
- **解决方案**: 
  ```bash
  # 使用国内镜像加速（修改 ~/.cargo/config.toml）
  [source.crates-io]
  replace-with = 'ustc'
  
  [source.ustc]
  registry = "https://mirrors.ustc.edu.cn/crates.io-index"
  ```

### Q2: 端口8088被占用怎么办？

**A**: 修改配置文件

```toml
# config.toml
[server]
bind_addr = "127.0.0.1:9999"  # 改成其他端口
```

或者杀掉占用进程：
```bash
# Windows
netstat -ano | findstr 8088
taskkill /PID <进程ID> /F

# Linux/Mac
lsof -ti:8088 | xargs kill -9
```

### Q3: Docker启动失败？

**A**: 检查以下几点

1. **Docker是否运行？**
   ```bash
   docker ps  # 应该能看到容器列表
   ```

2. **端口是否冲突？**
   ```bash
   # CockroachDB: 26257
   # Redis: 6379
   # Immudb: 3322
   netstat -ano | findstr "26257 6379 3322"
   ```

3. **降级方案（无Docker）**
   ```toml
   # config.toml
   [server]
   allow_degraded_start = true  # 允许无数据库启动
   ```

### Q4: 找不到 `config.toml` 文件？

**A**: 手动创建

```bash
cd backend
cp config.example.toml config.toml

# 或者直接创建
cat > config.toml << 'EOF'
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = true

[jwt]
secret = "dev-jwt-secret-min-32-chars-long-xxxxx"
token_expiry_secs = 3600

[logging]
level = "info"
EOF
```

### Q5: 环境变量怎么设置？

**A**: 根据操作系统

**Windows (PowerShell)**:
```powershell
$env:DATABASE_URL = "postgres://root@localhost:26257/ironcore"
$env:REDIS_URL = "redis://localhost:6379"
```

**Linux/Mac (Bash)**:
```bash
export DATABASE_URL="postgres://root@localhost:26257/ironcore"
export REDIS_URL="redis://localhost:6379"
```

**永久设置（.env文件）**:
```bash
# backend/.env
DATABASE_URL=postgres://root@localhost:26257/ironcore?sslmode=disable
REDIS_URL=redis://localhost:6379
JWT_SECRET=your-secret-key
```

---

## API使用

### Q6: 如何测试API？

**A**: 三种方法

**方法1: 浏览器（GET请求）**
```
http://localhost:8088/api/health
http://localhost:8088/api/v1/chains
```

**方法2: curl命令**
```bash
# GET请求
curl http://localhost:8088/api/health

# POST请求（公开API示例：平台服务费计算）
curl -X POST http://localhost:8088/api/v1/fees/calculate \
  -H "Content-Type: application/json" \
  -d '{"fee_type":"send","amount_usd":100,"chain":"ethereum"}'
```

**方法3: Postman/Apifox**
- 导入OpenAPI文档: `http://localhost:8088/openapi.yaml`
- 直接可视化测试所有API

### Q7: API返回401 Unauthorized？

**A**: 需要JWT认证

1. **先注册/登录获取token**
   ```bash
  curl -X POST http://localhost:8088/api/v1/auth/login \
     -H "Content-Type: application/json" \
     -d '{"username":"admin","password":"password"}'
   
   # 返回: {"token":"eyJhbGc..."}
   ```

2. **带token访问API**
   ```bash
  curl http://localhost:8088/api/v1/wallets \
     -H "Authorization: Bearer eyJhbGc..."
   ```

### Q8: API返回400 Bad Request？

**A**: 检查请求格式

**常见错误**:
```bash
# ❌ 错误：缺少 Content-Type
curl -X POST http://localhost:8088/api/v1/fees/calculate \
  -d '{"fee_type":"send","amount_usd":100,"chain":"ethereum"}'

# ✅ 正确：加上 Content-Type
curl -X POST http://localhost:8088/api/v1/fees/calculate \
  -H "Content-Type: application/json" \
  -d '{"fee_type":"send","amount_usd":100,"chain":"ethereum"}'
```

**JSON格式错误**:
```json
// ❌ 错误：多余的逗号
{"mnemonic": "...",}

// ✅ 正确
{"mnemonic": "..."}
```

### Q9: 如何查看完整的API列表？

**A**: 三种方式

1. **OpenAPI文档**: `http://localhost:8088/openapi.yaml`
2. **文档**: [API路由映射](../01-architecture/API_ROUTES_MAP.md)
3. **Swagger UI**: `http://localhost:8088/docs`


> 非托管提醒：IronCore-V2 不接收助记词/私钥。
> 如需登记钱包用于跨设备同步，请使用 `POST /api/v1/wallets/batch`（提交地址、公钥等公开信息，需JWT）。

---

## 钱包相关

### Q10: 什么是助记词？我应该用哪个？

**A**: 助记词详解

**标准助记词（12个单词）**:
```
witch collapse practice feed shame open despair 
creek road again ice least
```

**测试助记词**:
```bash
# BIP39标准测试助记词（不要在生产环境用！）
abandon abandon abandon abandon abandon abandon 
abandon abandon abandon abandon abandon about
```

**生成新助记词**:
```rust
// 使用在线工具: https://iancoleman.io/bip39/
// IronCore-V2 非托管：不提供“生成助记词”后端API（助记词只应在客户端/本地生成）
```

### Q11: 同一个助记词能生成多少个地址？

**A**: 理论上无限个！

```
助记词 → 种子
  ↓
第1个钱包: m/44'/60'/0'/0/0
第2个钱包: m/44'/60'/0'/0/1
第3个钱包: m/44'/60'/0'/0/2
...
第N个钱包: m/44'/60'/0'/0/(N-1)
```

**实际使用**:
- 大部分用户只用第1个（index=0）
- 需要多个地址时递增 index

### Q12: 为什么同一个助记词，以太坊和比特币地址不同？

**A**: 因为派生路径不同！

| 链 | 派生路径 | 地址格式 |
|---|---------|---------|
| Ethereum | m/44'/60'/0'/0/0 | 0x... |
| Bitcoin | m/84'/0'/0'/0/0 | bc1... |
| Solana | m/44'/501'/0'/0' | Base58 |

**同一个助记词 + 不同路径 = 不同地址**

### Q13: 钱包地址能改吗？

**A**: 不能！

- 地址由私钥/公钥生成（数学推导）
- 私钥固定 → 地址固定
- 想要新地址 → 用新助记词或新index

### Q14: 如何验证地址是否有效？

**A**: 使用余额查询作为最小验证（地址格式错误会返回 400）

```bash
curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1"

# 返回
{
  "balance": "0",
  "chain_id": 1,
  "confirmed": true
}
```

---

## 交易相关

### Q15: 为什么交易需要Gas费？

**A**: Gas = 区块链的"过路费"

**通俗解释**:
- 矿工/验证者帮你打包交易
- 需要消耗电力和计算资源
- Gas费就是给他们的报酬

**Gas构成**:
```
Total Fee = Gas Used × Gas Price

例如:
21000 gas × 20 gwei = 0.00042 ETH
```

### Q16: Gas价格怎么选？

**A**: 根据优先级

```bash
# 查询当前Gas建议（estimate-all）
curl "http://localhost:8088/api/v1/gas/estimate-all?chain=ethereum"

# 返回
{
  "slow": { "max_fee_per_gas_gwei": "...", "estimated_time_seconds": 600 },
  "normal": { "max_fee_per_gas_gwei": "...", "estimated_time_seconds": 180 },
  "fast": { "max_fee_per_gas_gwei": "...", "estimated_time_seconds": 30 },
  "timestamp": "..."
}
```

**选择建议**:
- ⏰ 不急 → slow
- ⚡ 一般 → normal
- 🚀 很急 → fast

### Q17: 交易失败会退Gas费吗？

**A**: 不会！

- ✅ Gas费是给矿工的劳务费
- ❌ 交易失败矿工也工作了
- 💡 所以Gas费照扣不误

**避免失败**:
- 确认余额充足（包括Gas费）
- 设置合理的Gas Limit
- 检查合约地址正确

### Q18: 交易状态有哪些？

**A**: 6种状态

| 状态 | 说明 | 持续时间 |
|------|------|---------|
| **pending** | 等待广播 | 1-5秒 |
| **broadcasted** | 已广播到网络 | 即时 |
| **confirming** | 确认中 | 15秒-5分钟 |
| **confirmed** | 已确认 | 永久 |
| **failed** | 失败 | 永久 |
| **dropped** | 被网络丢弃 | - |

### Q19: 如何查询交易状态？

**A**: 使用交易ID查询

```bash
curl "http://localhost:8088/api/v1/transactions/{tx_hash}/status" \
  -H "Authorization: Bearer <token>"

# 返回
{
  "code": 0,
  "message": "success",
  "data": {
    "tx_hash": "0xabcd...",
  "status": "confirmed",
    "confirmations": 12,
    "last_seen": 1732443300
  }
}
```

---

## 错误处理

### Q20: 常见错误码含义？

**A**: 错误码对照表

| 错误码 | 含义 | 常见原因 |
|-------|------|---------|
| **400** | 请求格式错误 | JSON格式错误、缺少必填字段 |
| **401** | 未授权 | token过期或无效 |
| **403** | 禁止访问 | 权限不足 |
| **404** | 资源不存在 | 钱包ID、交易ID不存在 |
| **429** | 请求过快 | 触发限流（100次/分钟） |
| **500** | 服务器错误 | 后端bug、数据库连接失败 |
| **503** | 服务不可用 | 服务重启、维护中 |

### Q21: "Insufficient funds" 错误？

**A**: 余额不足

**检查清单**:
```bash
# 1. 查询余额
curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x..."

# 2. 确认有足够的币
# 需要: 转账金额 + Gas费
# 例如: 转1 ETH，Gas费0.001 ETH → 需要至少1.001 ETH

# 3. 如果是代币（如USDT）
# 需要: USDT余额 + ETH（用于Gas费）
```

### Q22: "Nonce too low" 错误？

**A**: Nonce（交易序号）冲突

**原因**:
- 同一个地址的交易必须按顺序（nonce递增）
- 你发送了重复的nonce

**解决方案**:
```bash
# 1. 查询当前nonce
curl "http://localhost:8088/api/v1/transactions/nonce?chain_id=1&address=0x..."

# 2. 等待前一笔交易确认
# 3. 使用正确的nonce重新发送
```

### Q23: "Execution reverted" 错误？

**A**: 智能合约执行失败

**常见原因**:
- 合约限制（如代币余额不足、权限不够）
- 合约bug
- Gas Limit太低

**调试方法**:
```bash
# 1. 增加 Gas Limit（例如从21000改成100000）
# 2. 检查合约是否有限制条件
# 3. 在测试网先试验
```

---

## 性能优化

### Q24: API响应慢怎么办？

**A**: 优化策略

**检查响应时间**:
```bash
curl -w "@curl-format.txt" http://localhost:8088/api/health

# curl-format.txt 内容:
# time_total: %{time_total}s
```

**优化方案**:
1. **启用Redis缓存**
   ```toml
   [redis]
   url = "redis://localhost:6379"
   ```

2. **数据库连接池**
   ```toml
   [database]
   max_connections = 20
   min_connections = 5
   ```

3. **使用CDN**（生产环境）

### Q25: 如何减少Gas费？

**A**: 省钱技巧

1. **选择低峰时段**
   - 凌晨0-6点（UTC）Gas费最低
   - 周末通常比工作日便宜

2. **使用Layer 2**
   - Polygon（便宜100倍）
   - Arbitrum、Optimism

3. **批量操作**
   - 一次转账多个地址（如果合约支持）

---

## 安全相关

### Q26: 私钥会被后端看到吗？

**A**: 不会！系统设计是非托管的

**安全架构**:
```
客户端（你的电脑/手机）
  ↓ 生成私钥
  ↓ 签名交易
  ↓ 只发送签名后的交易
后端（我们的服务器）
  ↓ 接收签名交易
  ↓ 广播到区块链
  ↓ 不接触私钥！
```

### Q27: 助记词丢了能找回吗？

**A**: 不能！

- ❌ 没有"忘记密码"功能
- ❌ 我们也帮不了你
- ✅ 请务必备份助记词！

**备份建议**:
1. 手写在纸上（2-3份）
2. 存在密码管理器（1Password、Bitwarden）
3. 不要截图、不要发邮件、不要存云盘

### Q28: 如何防止被钓鱼？

**A**: 安全检查清单

✅ 确认域名正确（localhost:8088 或 正式域名）
✅ 使用HTTPS（生产环境）
✅ 不要在不明网站输入助记词
✅ 检查合约地址（与官方对比）
✅ 大额交易先测试小额

### Q29: JWT token过期怎么办？

**A**: 重新登录或刷新

```bash
# 方法1: 重新登录
curl -X POST http://localhost:8088/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"password"}'

# 方法2: 刷新token（如果实现了）
curl -X POST http://localhost:8088/api/v1/auth/refresh \
  -H "Authorization: Bearer <old_token>"
```

**token有效期**: 默认1小时

---

## 🆘 还有问题？

### 查看日志

```bash
# 默认情况下日志输出到运行终端（或容器日志）
# 如启用了文件日志，请按 config.toml 中的 logging.file 配置查找对应日志文件
```

### 联系支持

- 📖 查看详细文档: [文档索引](../INDEX.md)
- 💬 技术群: [Slack/Discord链接]
- 📧 邮件: backend-team@example.com
- 🐛 提Bug: [GitHub Issues]

### 深入学习

- [新手快速上手](./README.md)
- [API使用教程](./API_TUTORIAL.md)
- [业务逻辑详解](../01-architecture/BUSINESS_LOGIC.md)
- [错误处理指南](../08-error-handling/ERROR_HANDLING.md)

---

**最后更新**: 2025-11-24  
**维护者**: Backend Team
