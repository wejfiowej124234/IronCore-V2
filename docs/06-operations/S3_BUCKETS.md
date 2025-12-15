# S3 归档与桶策略建议

## 桶与前缀
- `iron-archive/report/`：业务报表（CSV/Parquet）
- `iron-archive/audit/`：审计快照与证明
- `iron-archive/export/`：批量导出（用户可下载）

## 生命周期与版本化
- 开启版本化与访问日志
- 报表/导出：90 天转低频；365 天转归档
- 审计快照：保留 ≥ 7 年

## 加密与访问
- KMS 加密（SSE-KMS）
- 细粒度 IAM：服务账号最小权限（PutObject/GetObject/ListBucket）

## 命名规范
`{prefix}/{tenant_id}/{yyyy}/{mm}/{dd}/{resource}-{ulid}.parquet`

## 元数据
- `x-amz-meta-hash`：内容哈希
- `x-amz-meta-created-at`：ISO 时间戳


