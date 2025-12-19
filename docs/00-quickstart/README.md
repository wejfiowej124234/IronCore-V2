# 🚀 新手快速上手指南

> 零基础？没关系！跟着这份指南，10分钟学会使用后端系统

## 📝 本文档适合谁？

- ✅ 刚加入团队的新同事
- ✅ 对区块链钱包不了解的开发者
- ✅ 需要快速了解系统功能的产品经理
- ✅ 想要学习项目架构的学生

## 🎯 学习路径

```
第1步: 了解项目是什么 (5分钟)
   ↓
第2步: 启动并运行项目 (10分钟)
   ↓
第3步: 尝试第一个API (5分钟)
   ↓
第4步: 理解核心概念 (15分钟)
   ↓
第5步: 深入学习 (按需)
```

---

## 第1步：这个项目是什么？ (5分钟)

### 一句话介绍

**ironforge_backend 是一个支持多条区块链的数字钱包后端系统**。

### 通俗解释

想象一下：
- 你有很多银行卡（不同的区块链）
- 每张卡都需要一个钱包管理（我们的系统）
- 你想在一个APP里管理所有卡（多链钱包）
- 你想安全地转账、查余额（后端API）

**这就是我们的系统！**

### 支持的区块链

| 链名称 | 用途 | 例子 |
|--------|------|------|
| **Ethereum** | 智能合约平台 | 以太坊钱包 |
| **BSC** | 币安智能链 | 低手续费交易 |
| **Polygon** | 侧链 | 快速便宜的交易 |
| **Bitcoin** | 数字黄金 | 比特币钱包 |
| **Solana** | 高性能链 | NFT、DeFi |
| **TON** | Telegram生态 | Telegram钱包 |

### 核心功能

```
┌─────────────────────────────────────┐
│         用户可以做什么？             │
├─────────────────────────────────────┤
│ ✅ 创建钱包                         │
│ ✅ 查看余额                         │
│ ✅ 发送加密货币                     │
│ ✅ 接收加密货币                     │
│ ✅ 查看交易历史                     │
│ ✅ 跨链兑换（如 ETH → BNB）        │
│ ✅ 设置通知（交易确认提醒）        │
└─────────────────────────────────────┘
```

---

## 第2步：启动并运行项目 (10分钟)

### 方案A：最简启动（推荐新手）

**无需安装数据库，直接运行！**

```bash
# 1. 进入后端目录
cd IronCore-V2

# 2. 创建配置文件
cat > config.toml << 'EOF'
[server]
bind_addr = "127.0.0.1:8088"
allow_degraded_start = true

[jwt]
secret = "dev-jwt-secret-min-32-chars-long-xxxxx"
token_expiry_secs = 3600

[logging]
level = "info"
format = "text"
EOF

# 3. 启动服务
cargo run

# 看到这个就成功了！
# ✅ Server running on http://127.0.0.1:8088
```

**测试是否成功**:
```bash
# 在浏览器打开或用 curl
curl http://localhost:8088/api/health

# 返回: {"status":"ok"} 就对了！
```

### 方案B：完整启动（需要数据库）

**适合想体验完整功能的同学**

```bash
# 1. 启动数据库（用 Docker）
cd ops
docker compose up -d

# 等待30秒，让数据库启动完成...

# 2. 设置环境变量
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
export REDIS_URL="redis://localhost:6379"

# 3. 启动服务
cd ..
cargo run
```

### 常见问题

**Q: cargo run 很慢怎么办？**
- A: 第一次编译需要10-20分钟，去喝杯咖啡吧 ☕

**Q: 端口 8088 被占用？**
- A: 修改 `config.toml` 中的 `bind_addr = "127.0.0.1:9999"`

**Q: Docker启动失败？**
- A: 用方案A，不需要 Docker

---

## 第3步：尝试第一个API (5分钟)

### 先调用一个“无需认证”的 API

```bash
# 1) 获取支持的链列表（公开API）
curl http://localhost:8088/api/v1/chains

# 2) Gas 费预估（公开API）
curl "http://localhost:8088/api/v1/gas/estimate-all?chain=ethereum"

# 3) 平台服务费计算（公开API）
curl -X POST http://localhost:8088/api/v1/fees/calculate \
  -H "Content-Type: application/json" \
  -d '{"fee_type":"send","amount_usd":100,"chain":"ethereum"}'
```

> 非托管原则：后端不接收助记词/私钥。
> 如需登记钱包用于跨设备同步，请先完成注册/登录后，再调用 `POST /api/v1/wallets/batch`（提交地址、公钥等公开信息）。

### 查询地址余额（公开API）

```bash
curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x9858EfFD232B4033E47d90003D41EC34EcaEda94"
```

---

## 第4步：理解核心概念 (15分钟)

### 概念1: 什么是助记词 (Mnemonic)？

**通俗解释**: 
- 助记词 = 你钱包的"总密码"
- 12个英文单词（如: witch collapse practice feed shame open despair creek road again ice least）
- **丢了就找不回来！**（比银行卡密码更重要）

**技术原理**:
```
助记词 (12个单词)
    ↓ (BIP39)
种子 (Seed)
    ↓ (BIP32)
主私钥 (Master Private Key)
    ↓ (BIP44)
不同链的钱包地址
```

### 概念2: 什么是派生路径 (Derivation Path)？

**通俗解释**:
- 一个助记词可以生成无数个钱包地址
- 派生路径 = "生成第几个钱包"的规则
- 例如: `m/44'/60'/0'/0/0` = "以太坊的第一个钱包"

**标准路径**:
```
m/44'/60'/0'/0/0   → 以太坊第1个钱包
m/44'/60'/0'/0/1   → 以太坊第2个钱包
m/44'/0'/0'/0/0    → 比特币第1个钱包
m/44'/501'/0'/0'   → Solana第1个钱包
```

### 概念3: 什么是私钥和公钥？

```
┌──────────────────────────────────────┐
│         钥匙比喻                     │
├──────────────────────────────────────┤
│ 私钥 = 你家的钥匙 (绝对不能给别人)  │
│ 公钥 = 你家的门牌号 (可以告诉别人)  │
│ 地址 = 公钥的缩写 (转账用)          │
└──────────────────────────────────────┘
```

**重要规则**:
- ✅ 私钥 → 可以推导出 → 公钥 → 可以推导出 → 地址
- ❌ 地址 → 无法反推 → 公钥 → 无法反推 → 私钥

### 概念4: 什么是交易签名？

**通俗解释**:
1. 你想转账 1 ETH 给朋友
2. 用你的私钥"签名"这笔交易（证明是你本人）
3. 广播到区块链网络
4. 矿工验证签名，确认交易

**为什么安全？**
- 私钥只在你的手机/电脑本地
- 签名后的交易可以公开（无法伪造）
- 后端只负责广播，不接触私钥

---

## 第5步：深入学习 (按需)

### 新手推荐阅读顺序

#### 第1周：基础入门
1. [业务逻辑详解](../01-architecture/BUSINESS_LOGIC.md) - 理解核心功能
2. [API使用教程](./API_TUTORIAL.md) - 学会调用API
3. [常见问题FAQ](./FAQ.md) - 解决常见疑惑

#### 第2周：进阶理解
4. [多链钱包架构](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md) - 技术架构
5. [API路由映射](../01-architecture/API_ROUTES_MAP.md) - 完整API列表
6. [数据库设计](../02-configuration/DATABASE_SCHEMA.md) - 数据如何存储

#### 第3周：运维部署
7. [配置管理指南](../02-configuration/CONFIG_MANAGEMENT.md) - 配置文件详解
8. [部署指南](../05-deployment/DEPLOYMENT.md) - 生产环境部署
9. [监控告警](../07-monitoring/MONITORING.md) - 系统监控

#### 高级主题（开发者）
10. [错误处理指南](../08-error-handling/ERROR_HANDLING.md)
11. [性能优化指南](../07-monitoring/PERFORMANCE.md)
12. [安全策略](../02-configuration/SECURITY.md)
13. [管理员操作手册](../09-admin/ADMIN_GUIDE.md)

---

## 🎓 实战练习

### 练习1: 创建多链钱包

**目标**: 批量登记 3 条链的钱包公开信息（非托管：客户端派生地址/公钥）

```bash
# 1) 先登录获取 JWT（示例）
TOKEN=$(curl -s -X POST http://localhost:8088/api/v1/auth/login \
   -H "Content-Type: application/json" \
   -d '{"username":"alice","password":"SecurePass123!"}' | jq -r '.token')

# 2) 批量登记（地址/公钥由客户端派生）
curl -X POST http://localhost:8088/api/v1/wallets/batch \
   -H "Authorization: Bearer $TOKEN" \
   -H "Content-Type: application/json" \
   -d '{
      "wallets": [
         {"chain":"ETH","address":"0x...","public_key":"0x04..."},
         {"chain":"BTC","address":"bc1...","public_key":"02..."},
         {"chain":"SOL","address":"...","public_key":"..."}
      ]
   }'
```

**预期结果**: 返回 `success/wallets/failed`，并为每条登记生成 `id`

### 练习2: 查询Gas价格

**目标**: 了解以太坊当前的手续费

```bash
curl "http://localhost:8088/api/v1/gas/estimate-all?chain=ethereum"
```

**预期结果**:
```json
{
  "slow": { "gwei": 10, "eth": 0.00021 },
  "normal": { "gwei": 20, "eth": 0.00042 },
  "fast": { "gwei": 50, "eth": 0.00105 }
}
```

### 练习3: 验证地址格式

**目标**: 检查地址是否可查询余额（以余额查询作为“格式可用”的最小验证）

```bash
curl "http://localhost:8088/api/v1/balance?chain=ethereum&address=0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb1"
```

---

## 📚 术语表（小白必看）

| 术语 | 解释 | 比喻 |
|------|------|------|
| **助记词 (Mnemonic)** | 12个英文单词，钱包的总密码 | 保险箱的钥匙 |
| **私钥 (Private Key)** | 64位十六进制字符串，控制资产 | 银行卡密码 |
| **公钥 (Public Key)** | 由私钥生成，可以公开 | 银行卡号 |
| **地址 (Address)** | 公钥的缩写，转账用 | 收款码 |
| **Gas** | 以太坊交易手续费 | 过路费 |
| **Gwei** | Gas的单位 (1 ETH = 10亿 Gwei) | "分"（人民币单位） |
| **区块确认** | 交易被打包进区块 | 银行到账通知 |
| **派生路径** | 生成钱包地址的路径 | "第几个子钱包" |
| **非托管** | 私钥用户自己保管 | 自己保管钱 |
| **托管** | 私钥交给平台保管 | 钱存银行 |

---

## 🆘 遇到问题？

### 问题诊断步骤

1. **服务启动失败？**
   - 检查端口是否被占用: `netstat -ano | findstr 8088`
   - 查看日志: `backend/debug.log`
   - 尝试降级模式: `allow_degraded_start = true`

2. **API调用失败？**
   - 检查URL是否正确: `http://localhost:8088`
   - 检查请求格式: Content-Type 必须是 `application/json`
   - 查看错误信息: 返回的 JSON 中有详细说明

3. **钱包地址不对？**
   - 确认助记词正确（12个单词，空格分隔）
   - 确认链名称正确（小写: ethereum, bitcoin, solana）
   - 确认派生路径符合标准

### 获取帮助

- 📖 查看 [常见问题FAQ](./FAQ.md)
- 📧 联系团队: backend-team@example.com
- 💬 技术群: [加入Slack/Discord]
- 📝 提Issue: [GitHub Issues]

---

## ✅ 下一步建议

完成本教程后，根据你的角色选择：

### 前端开发者 → 学习
- [API使用教程](./API_TUTORIAL.md)
- [API路由映射](../01-architecture/API_ROUTES_MAP.md)

### 后端开发者 → 学习
- [业务逻辑详解](../01-architecture/BUSINESS_LOGIC.md)
- [数据库设计](../02-configuration/DATABASE_SCHEMA.md)
- [错误处理指南](../08-error-handling/ERROR_HANDLING.md)

### 测试工程师 → 学习
- [API测试指南](./API_TESTING_GUIDE.md)
- [多链钱包测试报告](../04-testing/MULTI_CHAIN_WALLET_TEST_REPORT.md)

### 产品经理 → 学习
- [业务逻辑详解](../01-architecture/BUSINESS_LOGIC.md)
- [多链钱包架构](../01-architecture/MULTI_CHAIN_WALLET_ARCHITECTURE.md)

### 运维工程师 → 学习
- [部署指南](../05-deployment/DEPLOYMENT.md)
- [监控告警](../07-monitoring/MONITORING.md)
- [配置管理](../02-configuration/CONFIG_MANAGEMENT.md)

---

**🎉 恭喜你完成新手教程！继续探索更多功能吧！**

最后更新: 2025-11-24  
维护者: Backend Team
