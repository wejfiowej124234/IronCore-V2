# 事件主题与载荷规范（Kafka）

## 主题列表
- `tx.broadcast.requested`：请求广播交易
- `tx.broadcast.result`：交易广播结果（含 TxHash / 失败原因）
- `notify.user`：用户通知（短信/邮件/站内）
- `report.increment`：报表增量（聚合/指标更新）

## 通用字段
```json
{
  "event": "tx.broadcast.requested",
  "tenant_id": "t_123",
  "actor": "u_456",
  "ts": "2025-11-16T12:00:00Z",
  "trace_id": "ulid_..."
}
```

## 载荷示例
### tx.broadcast.requested
```json
{
  "event": "tx.broadcast.requested",
  "tenant_id": "t_123",
  "wallet_id": "w_abc",
  "chain_id": 1,
  "raw_tx": "0x...",
  "ts": "2025-11-16T12:00:00Z"
}
```

### tx.broadcast.result
```json
{
  "event": "tx.broadcast.result",
  "tenant_id": "t_123",
  "tx_request_id": "r_abc",
  "ok": true,
  "tx_hash": "0x...",
  "error": null,
  "ts": "2025-11-16T12:00:02Z"
}
```

### notify.user
```json
{
  "event": "notify.user",
  "tenant_id": "t_123",
  "user_id": "u_456",
  "channel": "email",
  "template": "tx_broadcasted",
  "data": {"tx_hash": "0x..."},
  "ts": "2025-11-16T12:00:05Z"
}
```

### report.increment
```json
{
  "event": "report.increment",
  "tenant_id": "t_123",
  "chain_id": 1,
  "metric": "tx_count",
  "value": 1,
  "at": "2025-11-16T12:00:10Z"
}
```

## 分区键建议
- 默认：`tenant_id` 或 `{tenant_id}:{chain_id}`
- 需要顺序保证的场景，使用业务 ID（如 `tx_request_id`）。


