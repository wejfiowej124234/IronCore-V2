# Changelog

所有重要变更都将记录在此文件中。

本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### 计划添加
- Solana 钱包支持
- TON 钱包支持
- WebSocket 实时通知
- GraphQL API

## [0.4.0] - 2025-11-24

### 新增
- 完整的开发指南文档（4份）
  - `DEVELOPER_GUIDE.md` - 开发者完整指南
  - `DEPENDENCIES.md` - 依赖库详解
  - `DATABASE_MIGRATION_GUIDE.md` - 数据库迁移指南
  - `CONTRIBUTING.md` - 贡献规范
- 新手入门文档（4份）
  - 零基础快速上手指南
  - 29个常见问题解答
  - 完整API使用教程
  - 13个故障排查场景
- 管理员操作手册
- 业务逻辑详解文档

### 改进
- 更新文档索引（28份文档）
- 优化学习路径推荐
- 补充代码示例（50+个）

### 文档
- 新增完整文档体系（从12份增长到28份，+133%）

## [0.3.0] - 2025-11-20

### 新增
- 事件总线系统（Event Bus）
- 通知偏好管理
- 交易监控字段
- 管理员操作日志

### 改进
- 优化Gas预估API性能
- 增强RPC故障转移机制
- 改进缓存策略

### 修复
- 修复多链钱包创建时的并发问题
- 修复Gas估算缓存过期问题

### 数据库
- 新增迁移: `0010_notification_preferences.sql`
- 新增迁移: `0010_transaction_monitor_fields.sql`
- 新增迁移: `0011_event_bus.sql`
- 新增迁移: `0011_admin_api_tables.sql`

## [0.2.0] - 2025-11-15

### 新增
- 多链钱包支持（Ethereum, BSC, Polygon, Bitcoin）
- Gas管理系统
  - Gas价格缓存
  - RPC健康检查
  - 智能故障转移
- 费用审计扩展
- 资产聚合视图

### 改进
- 优化数据库索引
- 增强错误处理机制
- 改进日志记录

### 修复
- 修复钱包派生路径计算错误
- 修复链ID映射问题

### 数据库
- 新增迁移: `0004_multi_chain_wallets.sql`
- 新增迁移: `0005_asset_aggregation.sql`
- 新增迁移: `0006_notify_init.sql`
- 新增迁移: `0007_gas_admin_init.sql`
- 新增迁移: `0008_fee_audit_extend.sql`
- 新增迁移: `0009_admin_operation_log.sql`

### 安全
- 实施密码哈希（bcrypt）
- 添加JWT认证
- 实施RBAC权限控制

## [0.1.0] - 2025-11-10

### 新增
- 基础后端架构
  - Axum Web框架
  - PostgreSQL/CockroachDB数据库
  - Redis缓存层
  - Immudb审计日志
- 核心API端点
  - 用户管理
  - 钱包管理
  - 交易管理
- 基础认证授权
- 数据库迁移系统

### 数据库
- 初始化迁移: `0001_init.sql`
- 约束迁移: `0002_constraints.sql`
- 密码哈希: `0003_add_password_hash.sql`

### 文档
- 初始架构文档
- API路由映射
- 部署指南

---

## 变更类型说明

- `新增` - 新功能
- `改进` - 对现有功能的改进
- `修复` - Bug修复
- `弃用` - 即将移除的功能
- `移除` - 已移除的功能
- `安全` - 安全相关修复
- `数据库` - 数据库Schema变更
- `文档` - 文档更新

---

## 版本规范

本项目遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)：

- **主版本号（MAJOR）**: 不兼容的API变更
- **次版本号（MINOR）**: 向后兼容的新功能
- **修订号（PATCH）**: 向后兼容的问题修复

---

## 链接

- [文档索引](./docs/INDEX.md)
- [贡献指南](./docs/11-development/CONTRIBUTING.md)
- [问题追踪](https://github.com/your-org/ironforge-backend/issues)
- [发布页面](https://github.com/your-org/ironforge-backend/releases)
