# 费用系统完整文档 (Complete Fee System Documentation)

> **版本**: v2.0  
> **最后更新**: 2025-01-XX  
> **适用范围**: Gas费用、平台服务费、跨链桥费用  
> **状态**: ✅ 企业级完整文档

---

## 📋 目录

- [费用系统概览](#费用系统概览)
- [Gas费用系统](#gas费用系统)
- [平台服务费](#平台服务费)
- [跨链桥费用](#跨链桥费用)
- [费用计算公式](#费用计算公式)
- [费用审计](#费用审计)
- [费用配置](#费用配置)
- [API接口](#api接口)
- [监控与告警](#监控与告警)

---

## 费用系统概览

### 三种费用类型

我们的系统包含三种费用类型：

1. **Gas费用** - 区块链网络交易费用
2. **平台服务费** - 平台提供的服务费用
3. **跨链桥费用** - 跨链转账的额外费用

### 费用计算流程

```
用户操作
    ↓
计算Gas费用（从区块链获取）
    ↓
计算平台服务费（根据规则）
    ↓
如果是跨链：计算跨链桥费用
    ↓
总费用 = Gas费用 + 平台服务费 + 跨链桥费用
    ↓
记录到审计表
    ↓
返回给用户
```

---

## Gas费用系统

### 概述

Gas费用是区块链网络收取的交易费用，由网络拥堵程度和交易复杂度决定。

### Gas费用获取方式

#### 1. 实时获取（推荐）

从区块链RPC节点实时获取：

```rust
// 使用EIP-1559标准
let gas_price = rpc_client.get_fee_history().await?;
let base_fee = gas_price.base_fee;
let priority_fee = gas_price.max_priority_fee_per_gas;
```

#### 2. 聚合服务

使用第三方Gas聚合服务：

- **EthGasStation** - Ethereum Gas价格聚合
- **GasNow** - 实时Gas价格
- **1inch Gas API** - 多链Gas价格

#### 3. 缓存策略

- Redis缓存：15秒TTL
- 本地缓存：5秒TTL（moka）

### Gas费用估算

#### 基础转账

- **ETH转账**: 21,000 gas（EIP-1559标准）
- **ERC20转账**: ~65,000 gas
- **合约调用**: 根据合约复杂度估算

#### 动态估算

```rust
// 调用 eth_estimateGas RPC方法
let estimated_gas = rpc_client.estimate_gas(&tx_params).await?;
```

### Gas费用配置

环境变量配置：

```bash
# 标准ETH转账Gas Limit
STANDARD_ETH_TRANSFER_GAS_LIMIT=21000

# 链特定Gas Limit
STANDARD_ETH_TRANSFER_GAS_LIMIT_ETHEREUM=21000
STANDARD_ETH_TRANSFER_GAS_LIMIT_BSC=21000

# Gas估算失败时的默认值
SWAP_DEFAULT_GAS_LIMIT=150000
```

### 相关文档

- [Gas系统框架文档](../../docs/01-架构设计/GAS_SYSTEM_FRAMEWORK.md)
- [Gas估算API指南](../../docs/GAS_ESTIMATION_API_GUIDE.md)

---

## 平台服务费

### 概述

平台服务费是平台对用户操作收取的费用，用于维护平台运营。

### 费用类型

#### 1. 固定费用 (Flat Fee)

按固定金额收取：

```rust
fee = flat_amount
```

示例：
- 固定费用：0.001 USDT
- 无论交易金额大小，都收取0.001 USDT

#### 2. 百分比费用 (Percentage Fee)

按交易金额百分比收取：

```rust
fee = amount * percent_bp / 10000
fee = max(fee, min_fee)  // 有最低费用
fee = min(fee, max_fee)  // 有最高费用
```

示例：
- 百分比：50 bp (0.5%)
- 最低费用：0.0002 USDT
- 最高费用：0.05 USDT

#### 3. 混合费用 (Mixed Fee)

固定费用 + 百分比费用：

```rust
percent_fee = max(amount * percent_bp / 10000, min_fee)
fee = flat_amount + percent_fee
fee = min(fee, max_fee)  // 可选封顶
```

示例：
- 固定费用：0.002 USDT
- 百分比：25 bp (0.25%)
- 最低费用：0.001 USDT

### 费用规则配置

数据库表：`gas.platform_fee_rules`

```sql
CREATE TABLE gas.platform_fee_rules (
    id UUID PRIMARY KEY,
    chain VARCHAR(50) NOT NULL,
    operation VARCHAR(50) NOT NULL,  -- 'send', 'swap', 'bridge'
    fee_type VARCHAR(20) NOT NULL,   -- 'flat', 'percent', 'mixed'
    flat_amount DECIMAL(30, 18),
    percent_bp INTEGER,               -- basis points (100bp = 1%)
    min_fee DECIMAL(30, 18),
    max_fee DECIMAL(30, 18),
    active BOOLEAN DEFAULT TRUE,
    priority INTEGER DEFAULT 0,
    effective_at TIMESTAMPTZ DEFAULT NOW(),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 费用归集地址

数据库表：`gas.fee_collector_addresses`

```sql
CREATE TABLE gas.fee_collector_addresses (
    id UUID PRIMARY KEY,
    chain VARCHAR(50) NOT NULL,
    address VARCHAR(255) NOT NULL,
    active BOOLEAN DEFAULT TRUE,
    rotated_at TIMESTAMPTZ DEFAULT NOW()
);
```

### 相关文档

- [费用系统实施文档](../../../FEE_SYSTEM_IMPLEMENTATION.md)

---

## 跨链桥费用

### 概述

跨链桥费用是跨链转账时，跨链桥服务商收取的费用。

### 费用组成

跨链桥费用包含以下部分：

1. **源链Gas费** - 锁定资产的交易费用
2. **目标链Gas费** - 铸造/解锁资产的交易费用
3. **验证者费用** - 跨链验证网络的费用
4. **网络拥堵费用** - 动态调整的费用
5. **流动性深度费用** - 大额交易可能需要更高费用
6. **汇率波动补偿** - 跨链期间的价格风险补偿

### 费用计算

#### 基础费率

```rust
// 从环境变量读取基础费率
let base_rate = env::var("BRIDGE_BASE_FEE_RATE")
    .unwrap_or("0.003".to_string())  // 默认0.3%
    .parse::<f64>()?;

fee = amount * base_rate;
```

#### 链组合特定费率

```rust
// 从环境变量读取链组合特定费率
let chain_key = format!("BRIDGE_FEE_RATE_{}_{}", from_chain, to_chain);
let rate = env::var(&chain_key)
    .unwrap_or(base_rate.to_string())
    .parse::<f64>()?;
```

#### 费用配置

环境变量配置：

```bash
# 基础跨链桥费率（0.3%）
BRIDGE_BASE_FEE_RATE=0.003

# 链组合特定费率
BRIDGE_FEE_RATE_ETHEREUM_BSC=0.002      # Ethereum -> BSC: 0.2%
BRIDGE_FEE_RATE_BSC_POLYGON=0.0025      # BSC -> Polygon: 0.25%

# 配置文件中也有定义
[cross_chain]
bridge_fee_percentage = 0.003      # 桥接费 0.3%
transaction_fee_percentage = 0.001 # 交易费 0.1%
```

### 跨链桥服务商

当前支持的跨链桥：

- **1inch Bridge** - 多链跨链桥
- **LiFi Bridge** - 聚合跨链桥
- **其他桥服务商** - 根据配置选择

### 相关代码

- 位置：`IronCore-V2/src/api/`
- 配置：`backend/config.toml` 中的 `[cross_chain]` 部分

---

## 费用计算公式

### 通用公式

```
总费用 = Gas费用 + 平台服务费 + 跨链桥费用（如果是跨链）
```

### Gas费用计算

```
Gas费用 = Gas Limit × Gas Price

// EIP-1559
Gas费用 = Gas Limit × (Base Fee + Priority Fee)
```

### 平台服务费计算

#### Flat类型

```
平台服务费 = flat_amount
```

#### Percent类型

```
原始费用 = amount × percent_bp / 10000
平台服务费 = max(原始费用, min_fee)
如果定义了max_fee: 平台服务费 = min(平台服务费, max_fee)
```

#### Mixed类型

```
百分比部分 = max(amount × percent_bp / 10000, min_fee)
平台服务费 = flat_amount + 百分比部分
如果定义了max_fee: 平台服务费 = min(平台服务费, max_fee)
```

### 跨链桥费用计算

```
跨链桥费用 = amount × bridge_fee_rate

// 如果定义了链组合特定费率
跨链桥费用 = amount × chain_pair_rate
```

---

## 费用审计

### 审计表结构

#### 平台服务费审计

表：`gas.fee_audit`

```sql
CREATE TABLE gas.fee_audit (
    id UUID PRIMARY KEY,
    user_id UUID,
    wallet_address VARCHAR(255),
    chain VARCHAR(50),
    operation VARCHAR(50),
    amount DECIMAL(30, 18),          -- 原始金额
    fee DECIMAL(30, 18),             -- 计算出的费用
    fee_type VARCHAR(20),            -- 'platform_fee'
    rule_id UUID,                    -- 使用的规则ID
    collector_address VARCHAR(255),  -- 归集地址
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

#### 跨链桥费用审计

表：`bridge_transactions` (在swap_transactions中记录)

```sql
-- 在swap_transactions表中记录跨链费用
fee_amount DECIMAL(30, 18),  -- 跨链桥费用
fee_type VARCHAR(50),        -- 'bridge_fee'
```

### 审计流程

1. **计算费用** - 调用费用计算服务
2. **记录审计** - 写入审计表
3. **Immudb双写** - 写入不可变日志（可选）
4. **返回费用** - 返回给用户

### 审计查询

```sql
-- 查询某个用户的费用记录
SELECT * FROM gas.fee_audit
WHERE user_id = $1
ORDER BY created_at DESC;

-- 按链统计费用
SELECT chain, SUM(fee) as total_fee, COUNT(*) as transaction_count
FROM gas.fee_audit
WHERE created_at >= NOW() - INTERVAL '1 day'
GROUP BY chain;
```

---

## 费用配置

### 环境变量配置

```bash
# 启用费用系统
ENABLE_FEE_SYSTEM=1

# Gas费用配置
STANDARD_ETH_TRANSFER_GAS_LIMIT=21000
SWAP_DEFAULT_GAS_LIMIT=150000

# 跨链桥费用配置
BRIDGE_BASE_FEE_RATE=0.003
BRIDGE_FEE_RATE_ETHEREUM_BSC=0.002
```

### 配置文件

`backend/config.toml`:

```toml
[features]
enable_fee_system = true

[cross_chain]
bridge_fee_percentage = 0.003      # 桥接费 0.3%
transaction_fee_percentage = 0.001 # 交易费 0.1%
```

### 数据库配置

费用规则在数据库中配置：

```sql
-- 插入费用规则
INSERT INTO gas.platform_fee_rules (
    chain, operation, fee_type, 
    percent_bp, min_fee, max_fee
) VALUES (
    'ethereum', 'swap', 'percent',
    50, 0.0002, 0.05
);
```

---

## API接口

### 获取Gas费用

```http
GET /api/v1/gas/estimate?chain=ethereum&speed=normal

Response:
{
    "gas_price": "50000000000",  // Wei
    "gas_limit": "21000",
    "total_fee": "0.00105",      // ETH
    "max_fee_per_gas": "50000000000",
    "max_priority_fee_per_gas": "2000000000"
}
```

### 计算平台服务费

```http
POST /api/v1/fees/calculate
Content-Type: application/json

{
    "chain": "ethereum",
    "operation": "swap",
    "amount": "10.0"
}

Response:
{
    "amount": "10.0",
    "fee": "0.05",
    "fee_type": "percent",
    "rule_id": "uuid-here",
    "collector_address": "0x...",
    "total": "10.05"
}
```

### 计算跨链桥费用

```http
POST /api/v1/bridge/quote
Content-Type: application/json

{
    "from_chain": "ethereum",
    "to_chain": "bsc",
    "token": "USDT",
    "amount": 10.0,
    "slippage_bps": 50
}

Response:
{
    "bridge_fee": 0.00042,
    "bridge_protocol": "stargate",
    "estimated_time_seconds": 180,
    "route": {
        "provider": "stargate",
        "source_chain": "ethereum",
        "destination_chain": "bsc",
        "token_symbol": "USDT",
        "amount": "10.0",
        "message_fee_wei": "420000000000000",
        "steps": []
    }
}
```

---

## 监控与告警

### Prometheus指标

```rust
// 费用计算次数
fee_calculation_count{chain, operation}

// 费用总金额
fee_total_amount{chain}  // Counter

// 费用规则缺失
fee_rule_miss_total{chain, operation}

// 费用审计写入失败
fee_audit_write_fail_total
```

### 告警规则

```yaml
# Prometheus告警规则
groups:
  - name: fee_system
    rules:
      - alert: FeeRuleMissing
        expr: fee_rule_miss_total > 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "费用规则缺失"

      - alert: FeeAuditWriteFail
        expr: fee_audit_write_fail_total > 10
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "费用审计写入失败"
```

---

## 相关文档

- [Gas系统框架](../../../docs/01-架构设计/GAS_SYSTEM_FRAMEWORK.md)
- [费用系统实施文档](../../../FEE_SYSTEM_IMPLEMENTATION.md)
- [Gas估算API指南](../GAS_ESTIMATION_API_GUIDE.md)
- [数据库Schema文档](./DATABASE_SCHEMA.md)

---

**文档状态**: ✅ 企业级完整文档  
**维护者**: 后端开发团队  
**最后审查**: 2025-01-XX

