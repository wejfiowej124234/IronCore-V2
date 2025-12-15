# IronCore 数据库本地落地与迁移指南

## 1) 启动本地依赖（CockroachDB/Redis/immudb）
```bash
docker compose -f ops/docker-compose.yml up -d
```

## 2) 配置环境变量
复制 `.env.example` 为 `.env` 并根据环境修改：
```
DATABASE_URL=postgres://root@localhost:26257/ironcore?sslmode=disable
REDIS_URL=redis://localhost:6379
IMMUDB_ADDR=127.0.0.1:3322
IMMUDB_USER=immudb
IMMUDB_PASS=immudb
IMMUDB_DB=defaultdb
```

> 生产环境请启用 TLS、密码/ACL，凭证使用 Vault/KMS 管理。

## 3) 执行 SQLx 迁移
```bash
export DATABASE_URL=postgres://root@localhost:26257/ironcore?sslmode=disable
sqlx migrate run
```

## 4) 验证
- CockroachDB：`SELECT now();`、检查建表是否完成。
- Redis：设置/读取键，检查 AOF 文件与内存水位。
- immudb：写入与查询一条示例事件，校验 Proof。

## 5) 后续（可选）
- 第二阶段接入 Kafka：事件发布/消费、回放演练。
- 第三阶段接入 ClickHouse/TimescaleDB 与 S3：历史分析与归档。

---

常见问题
- 端口冲突：修改 compose 暴露端口或停止本机已有服务。
- 迁移失败：确认 `DATABASE_URL` 正确、Cockroach 节点已启动、SQL 兼容性（v23+）。
- 权限不足：生产不要使用 root，创建最小权限用户与数据库。

---

附：Kafka/ClickHouse/S3（可选二/三阶段）

1) 启动服务（compose 中已包含）
```bash
docker compose -f ops/docker-compose.yml up -d kafka clickhouse minio
```

2) Kafka 基本连通
```
PLAINTEXT @ localhost:9092
建议主题：tx.broadcast.requested / tx.broadcast.result / notify.user / report.increment
```

3) ClickHouse Web
```
http://localhost:8123  （Native: 9000）
初次可建库：CREATE DATABASE IF NOT EXISTS iron_analytics;
```

4) MinIO（S3 兼容）
```
Console: http://localhost:9001  （账号 admin / adminadmin）
S3 API:  http://localhost:9002  （Endpoint 映射）
创建 bucket：iron-archive（配置生命周期/版本化）
```


