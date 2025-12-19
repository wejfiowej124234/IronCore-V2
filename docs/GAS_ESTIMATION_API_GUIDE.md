# Gas 费预估 API 使用指南

## 概述

Gas 费预估 API 基于 **EIP-1559** 标准，提供三档速度的 Gas 费预估（slow/normal/fast），支持以太坊、BSC、Polygon 等主流链。

## API 端点

### 1. 单速度预估

**GET** `/api/v1/gas/estimate`

预估指定速度档位的 Gas 费用。

#### 请求参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| chain | string | 是 | 链名称（ethereum/bsc/polygon） |
| speed | string | 否 | 速度档位（slow/normal/fast），默认 normal |

#### 响应示例

```json
{
  "success": true,
  "data": {
    "base_fee": "0xba43b7400",
    "max_priority_fee": "0x77359400",
    "max_fee_per_gas": "0x1319c2b800",
    "estimated_time_seconds": 180,
    "base_fee_gwei": 50.0,
    "max_priority_fee_gwei": 2.0,
    "max_fee_per_gas_gwei": 52.0
  }
}
```

#### cURL 示例

```bash
# 预估以太坊正常速度 Gas 费
curl "http://localhost:8088/api/v1/gas/estimate?chain=ethereum&speed=normal"

# 预估 BSC 快速 Gas 费
curl "http://localhost:8088/api/v1/gas/estimate?chain=bsc&speed=fast"

# 预估 Polygon 慢速 Gas 费
curl "http://localhost:8088/api/v1/gas/estimate?chain=polygon&speed=slow"
```

### 2. 批量预估（所有速度）

**GET** `/api/v1/gas/estimate-all`

一次性返回三档速度的 Gas 费预估，便于前端展示。

#### 请求参数

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| chain | string | 是 | 链名称（ethereum/bsc/polygon） |

#### 响应示例

```json
{
  "success": true,
  "data": {
    "slow": {
      "base_fee": "0xba43b7400",
      "max_priority_fee": "0x77359400",
      "max_fee_per_gas": "0x1319c2b800",
      "estimated_time_seconds": 600,
      "base_fee_gwei": 50.0,
      "max_priority_fee_gwei": 2.0,
      "max_fee_per_gas_gwei": 52.0
    },
    "normal": {
      "base_fee": "0xe8890e200",
      "max_priority_fee": "0xb2d05e00",
      "max_fee_per_gas": "0x19a48e6c00",
      "estimated_time_seconds": 180,
      "base_fee_gwei": 62.5,
      "max_priority_fee_gwei": 3.0,
      "max_fee_per_gas_gwei": 65.5
    },
    "fast": {
      "base_fee": "0x11c37937e08",
      "max_priority_fee": "0xee6b2800",
      "max_fee_per_gas": "0x22b0456608",
      "estimated_time_seconds": 60,
      "base_fee_gwei": 75.0,
      "max_priority_fee_gwei": 4.0,
      "max_fee_per_gas_gwei": 79.0
    }
  }
}
```

#### cURL 示例

```bash
# 批量预估以太坊所有速度
curl "http://localhost:8088/api/v1/gas/estimate-all?chain=ethereum"

# 批量预估 Polygon 所有速度
curl "http://localhost:8088/api/v1/gas/estimate-all?chain=polygon"
```

## 支持的链

| 链名称 | 别名 | 预计确认时间（slow/normal/fast） |
|--------|------|----------------------------------|
| Ethereum | eth | 10分钟 / 3分钟 / 1分钟 |
| BSC | binance | 5分钟 / 1.5分钟 / 30秒 |
| Polygon | matic | 3分钟 / 1分钟 / 20秒 |

## 响应字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| base_fee | string | 基础费用（Wei，十六进制），用于交易签名 |
| max_priority_fee | string | 最大优先费用（Wei，十六进制） |
| max_fee_per_gas | string | 最大 Gas 单价（Wei，十六进制），EIP-1559 必需 |
| estimated_time_seconds | integer | 预计确认时间（秒） |
| base_fee_gwei | float | 基础费用（Gwei），便于展示 |
| max_priority_fee_gwei | float | 优先费用（Gwei） |
| max_fee_per_gas_gwei | float | 最大费用（Gwei） |

## 速度档位说明

| 速度 | 确认时间 | 优先费倍数 | 适用场景 |
|------|----------|------------|----------|
| slow | 10+ 分钟（链相关） | 1.0x | 非紧急交易，节省手续费 |
| normal | ~3 分钟（链相关） | 1.5x | 常规交易，平衡速度和费用 |
| fast | <1 分钟（链相关） | 2.0x | 紧急交易，优先确认 |

## 前端集成示例

### JavaScript/TypeScript

```typescript
async function estimateGas(chain: string, speed: 'slow' | 'normal' | 'fast') {
  const response = await fetch(
    `http://localhost:8088/api/v1/gas/estimate?chain=${chain}&speed=${speed}`
  );
  const result = await response.json();
  
  if (result.code === 0) {
    console.log(`Max Fee: ${result.data.max_fee_per_gas_gwei} Gwei`);
    console.log(`Estimated Time: ${result.data.estimated_time_seconds}s`);
    return result.data;
  }
}

async function estimateAllSpeeds(chain: string) {
  const response = await fetch(
    `http://localhost:8088/api/v1/gas/estimate-all?chain=${chain}`
  );
  const result = await response.json();
  
  if (result.code === 0) {
    return {
      slow: result.data.slow,
      normal: result.data.normal,
      fast: result.data.fast,
    };
  }
}

// 使用示例
const normalGas = await estimateGas('ethereum', 'normal');
const allSpeeds = await estimateAllSpeeds('polygon');
```

### React Hook 示例

```tsx
import { useState, useEffect } from 'react';

function useGasEstimate(chain: string) {
  const [gasData, setGasData] = useState(null);
  const [loading, setLoading] = useState(true);
  
  useEffect(() => {
    async function fetchGas() {
      try {
        const res = await fetch(
          `http://localhost:8088/api/v1/gas/estimate-all?chain=${chain}`
        );
        const data = await res.json();
        if (data.code === 0) {
          setGasData(data.data);
        }
      } catch (err) {
        console.error('Failed to fetch gas', err);
      } finally {
        setLoading(false);
      }
    }
    
    fetchGas();
    const interval = setInterval(fetchGas, 15000); // 15秒刷新
    return () => clearInterval(interval);
  }, [chain]);
  
  return { gasData, loading };
}

// 组件中使用
function TransactionForm() {
  const { gasData, loading } = useGasEstimate('ethereum');
  
  if (loading) return <div>Loading gas prices...</div>;
  
  return (
    <div>
      <div>Slow: {gasData.slow.max_fee_per_gas_gwei} Gwei</div>
      <div>Normal: {gasData.normal.max_fee_per_gas_gwei} Gwei</div>
      <div>Fast: {gasData.fast.max_fee_per_gas_gwei} Gwei</div>
    </div>
  );
}
```

## 错误处理

### 常见错误

| HTTP 状态码 | 错误原因 | 解决方案 |
|------------|----------|----------|
| 400 | 不支持的链名称 | 检查 chain 参数，仅支持 ethereum/bsc/polygon |
| 500 | RPC 节点不可用 | 检查 RPC 配置，等待节点恢复 |
| 500 | Gas 预估失败 | 链上数据异常，稍后重试 |

### 错误响应示例

```json
{
  "success": false,
  "error": {
    "code": "BAD_REQUEST",
    "message": "Unsupported chain: solana. Supported chains: ethereum, bsc, polygon"
  }
}
```

## 技术实现

### 数据来源

- **baseFeePerGas**: 从最新区块（`eth_getBlockByNumber("latest")`）获取
- **maxPriorityFeePerGas**: 优先使用 `eth_maxPriorityFeePerGas` RPC 方法，降级使用默认值

### 费用计算公式

```
maxFeePerGas = (baseFee * baseFeeMultiplier) + (priorityFee * priorityFeeMultiplier)
```

#### 以太坊配置示例

| 速度 | baseFeeMultiplier | priorityFeeMultiplier |
|------|-------------------|----------------------|
| slow | 1.0x | 1.0x |
| normal | 1.2x | 1.5x |
| fast | 1.5x | 2.0x |

### RPC 节点容错

- 使用 `RpcSelector` 自动选择健康节点
- 3 次重试，指数退避（1s * attempt）
- 降级策略：RPC 不支持时使用默认值

## 性能优化

- **响应时间**: <500ms（单速度），<1s（批量预估）
- **缓存策略**: 建议前端缓存 10-15 秒
- **并发支持**: 支持高并发请求（Axum 异步处理）

## 监控与日志

所有 Gas 预估请求会记录日志：

```
INFO estimating_gas chain=ethereum speed=normal
INFO gas_estimated chain=ethereum speed=normal max_fee_gwei=52.0
```

Prometheus 指标：

- `gas_estimate_requests_total`: 总请求数
- `gas_estimate_errors_total`: 失败请求数
- `gas_estimate_duration_seconds`: 响应时间分布

## 注意事项

1. **链上数据变化快**: Gas 费用每个区块都可能变化，建议实时查询
2. **速度仅供参考**: 预计确认时间受网络拥堵影响，可能延迟
3. **不保证成交**: 使用 slow 档位时，如果 Gas 费暴涨，交易可能长时间未确认
4. **降级机制**: RPC 节点异常时，使用保守的默认值，避免交易失败

## 相关文档

- [EIP-1559: Fee market change for ETH 1.0 chain](https://eips.ethereum.org/EIPS/eip-1559)
- [RPC Selector 文档](../infrastructure/rpc_selector.rs)
- [OpenAPI 文档](http://localhost:8088/docs)
