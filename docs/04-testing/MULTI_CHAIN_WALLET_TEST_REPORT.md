# 多链钱包 API 测试报告（归档）

> ⚠️ 本文件为历史测试报告归档。
>
> 早期版本曾包含“由后端生成助记词/私钥”的示例与旧 `/api/...` 路径，这与当前 **非托管** 架构不一致，已移除以避免误导与复制风险。

## 现行权威来源

- OpenAPI：`GET /openapi.yaml` 或 `GET /docs`
- 路由注册：`IronCore-V2/src/api/mod.rs`

## 现行 API 约定

- 业务接口统一：`/api/v1/...`
- 健康检查：`GET /api/health`

## 非托管原则（必须遵守）

- ✅ 客户端负责密钥生成、派生与签名
- ✅ 后端只接收公开信息（如地址、公钥、链标识）
- ❌ 后端不得接收/存储：`mnemonic` / `private_key` / 用户钱包密码

## 建议测试路径（示例）

- `GET /api/v1/chains`
- `POST /api/v1/auth/login` → 获取 JWT
- `POST /api/v1/wallets/batch`（携带 Bearer Token；仅提交地址/公钥等公开信息）
