# 🔐 IronCore-V2 生产环境配置指南

本指南面向 [IronCore-V2](IronCore-V2) 后端。

## 总览

当前后端的**运行时关键配置主要来自环境变量**（以及 `.env`），包括：数据库、Redis、JWT、区块链 RPC、跨链费率等。

仓库根目录提供了两个参考文件：

- `IronCore-V2/.env.production.example`：生产环境建议的环境变量清单（推荐）
- `IronCore-V2/config.example.toml`：`CONFIG_PATH` 配置文件示例（可选）

> 说明：如果设置 `CONFIG_PATH`，服务会在启动时读取 TOML 并构建 `state.config`。
> 但当前实现只会把 **JWT 相关字段**同步到环境变量（用于 JWT 模块读值）。
> 其他功能（例如 Gas 动态费率、部分桥接 SDK、部分 RPC 读取逻辑）仍会直接读取环境变量或使用默认值。

---

## ✅ 必配项（生产）

### 数据库

- `DATABASE_URL`（必须）

示例：

```bash
export DATABASE_URL="postgres://USER:PASSWORD@HOST:26257/ironcore?sslmode=require"
```

### JWT

JWT 模块运行时读取的环境变量：

- `JWT_SECRET`（必须，长度至少 32）
- `JWT_TOKEN_EXPIRY_SECS`（可选，默认 3600；控制 access token 过期秒数）

> 备注：refresh token 的过期时间目前在代码中固定为 30 天（2592000 秒），暂不通过环境变量配置。
> `config.toml` 里的 `jwt.token_expiry_secs` 会在设置 `CONFIG_PATH` 时同步到 `JWT_TOKEN_EXPIRY_SECS`。

示例：

```bash
export JWT_SECRET="CHANGE_ME_TO_A_RANDOM_SECRET_AT_LEAST_32_CHARS"
export JWT_TOKEN_EXPIRY_SECS="3600"
```

### Redis（建议）

- `REDIS_URL`（可选；未设置则默认 `redis://127.0.0.1:6379`）

```bash
export REDIS_URL="redis://localhost:6379"
```

### 服务监听

- `BIND_ADDR`（可选；默认 `0.0.0.0:8088`）

```bash
export BIND_ADDR="0.0.0.0:8088"
```

---

## ⛓️ 区块链 RPC（生产建议必配）

这些值用于链上查询、Gas 估算等功能。

```bash
export ETH_RPC_URL="https://eth-mainnet.g.alchemy.com/v2/YOUR_API_KEY"
export BSC_RPC_URL="https://bsc-dataseed1.binance.org"
export POLYGON_RPC_URL="https://polygon-rpc.com"
export SOLANA_RPC_URL="https://api.mainnet-beta.solana.com"
export BITCOIN_RPC_URL="https://blockstream.info/api"
export TON_RPC_URL="https://toncenter.com/api/v2/jsonRPC"

# 非EVM链动态费率（当前实现读取 *_API_URL；可选）
# - BITCOIN_API_URL：用于获取 fee-estimates（默认使用 https://blockstream.info/api）
# - TON_API_URL：用于尝试 GET {TON_API_URL}/getAddressInformation（不设置会自动降级）
export BITCOIN_API_URL="https://blockstream.info/api"
export TON_API_URL=""
```

---

## 💸 跨链费率（生产建议必配）

```bash
export BRIDGE_FEE_PERCENTAGE="0.003"       # 0.3%
export TRANSACTION_FEE_PERCENTAGE="0.001"  # 0.1%
```

> 部署形态为 Docker/Fly/K8s 时：修改环境变量通常需要**重启进程/重新部署**才会生效。

---

## 🚀 启动方式

### 方式 A：仅使用环境变量（推荐）

1) 复制示例 `.env`

```bash
cd IronCore-V2
cp .env.production.example .env
```

2) 根据你的环境编辑 `.env`（替换数据库/JWT/RPC 等）

3) 启动（开发/生产按需）

```bash
cd IronCore-V2
cargo run --release
```

### 方式 B：使用 `CONFIG_PATH`（可选）

```bash
cd IronCore-V2
cp config.example.toml config.toml

# 运行时读取 config.toml（仅在启动时加载）
CONFIG_PATH=config.toml cargo run --release
```

---

## 🧪 运行验证

- 健康检查：`GET /healthz` 或 `GET /api/health`
- OpenAPI：`GET /openapi.yaml`
- Swagger UI：`GET /docs/`

本地示例：

```bash
curl http://localhost:8088/healthz
curl http://localhost:8088/openapi.yaml
```

---

## Fly.io 注意事项

仓库内 [IronCore-V2/fly.toml](IronCore-V2/fly.toml) 已默认设置：

- `BIND_ADDR=0.0.0.0:8088`
- `SKIP_MIGRATIONS=1`（避免部署期间 migrations 阻塞健康检查）

如果你需要在生产自动跑 migrations：

- 不要设置 `SKIP_MIGRATIONS`，或显式取消
- 确保数据库在 rollout 期间可用

