# 生产环境就绪验证报告（归档）
# Production Readiness Verification Report (Archived)

> ⚠️ 说明：本文件为历史报告归档，内容可能引用旧目录结构与旧 API 路径。
> 
> ✅ 现行权威来源请以以下为准：
> - OpenAPI：`GET /openapi.yaml` 或 `GET /docs`（Swagger UI）
> - 路由注册：`IronCore-V2/src/api/mod.rs`

---

## 现行约定（以代码为准）

- **业务接口统一前缀**：`/api/v1/...`
- **健康检查保留**：`GET /api/health`
- **监控指标**：`GET /metrics`

## 非托管安全原则

- ✅ 后端只接收客户端派生的**公开信息**（如地址、公钥、链标识等）
- ❌ 后端不接收/不存储：`mnemonic` / `private_key` / 用户钱包密码

## 如何验证“生产可用”

- 以 `GET /api/health` 确认服务可用
- 以 `GET /openapi.yaml` / `GET /docs` 核对当前所有端点与请求/响应结构
- 以对应的集成测试 / Smoke 测试脚本验证核心路径（链列表、报价、gas 估算、交易记录等）
