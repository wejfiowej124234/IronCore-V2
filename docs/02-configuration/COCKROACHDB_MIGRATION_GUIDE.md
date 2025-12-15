# 🗄️ CockroachDB 数据库迁移指南

## 数据库类型确认

✅ **当前使用**: **CockroachDB** (分布式SQL数据库)
- 协议: PostgreSQL 兼容
- 连接: 使用 `sqlx` 的 `postgres` 特性
- 端口: 默认 26257

---

## 快速开始

### 1. 设置数据库连接

```powershell
# 设置 DATABASE_URL 环境变量
$env:DATABASE_URL = "postgresql://root@localhost:26257/ironcore?sslmode=disable"

# 或者连接到远程 CockroachDB
$env:DATABASE_URL = "postgresql://user:password@your-cockroach-host:26257/ironcore?sslmode=require"
```

### 2. 应用所有迁移（推荐）

```powershell
cd IronCore

# 方法 A: 使用 Cargo SQLx（推荐）
.\apply_migrations_cargo.ps1

# 方法 B: 使用 CockroachDB CLI
.\apply_all_migrations.ps1
```

---

## 迁移文件清单

当前共有 **43 个迁移文件**，包括：

### 核心迁移 (0001-0016)
- 0001: Schema 创建
- 0002: 核心表（users, wallets, transactions）
- 0003-0013: 各功能模块表
- 0014-0016: 资产映射和限价单

### 非托管化改造 (0030-0038)
- **0030**: ⭐ 删除托管功能（删除私钥字段）
- 0031-0038: 非托管增强功能

### 新增迁移 (0039-0043)
- **0039**: ⭐ **非托管合规性检查**（您的新迁移）
- 0040-0043: 审计日志和模式修复

---

## 迁移执行方式

### 方式 1: Cargo SQLx（推荐）

**优点**: 
- 自动跟踪迁移状态
- 幂等性保证
- 校验和验证

**步骤**:
```powershell
# 1. 清除旧的迁移记录（如果数据库已清空）
cockroach sql --url=$env:DATABASE_URL -e "DROP TABLE IF EXISTS _sqlx_migrations CASCADE;"

# 2. 运行迁移
cargo sqlx migrate run

# 3. 查看状态
cargo sqlx migrate info
```

### 方式 2: 直接执行 SQL

**优点**: 
- 绕过校验和检查
- 可单独执行某个迁移
- 适合调试

**步骤**:
```powershell
# 执行单个迁移
cockroach sql --url=$env:DATABASE_URL --file=migrations/0039_non_custodial_compliance_checks.sql

# 或批量执行
Get-ChildItem migrations\*.sql | Sort-Object Name | ForEach-Object {
    cockroach sql --url=$env:DATABASE_URL --file=$_.FullName
}
```

### 方式 3: 使用应用内迁移系统

**优点**: 
- CockroachDB 优化
- 自动处理兼容性问题
- 详细日志

**代码**:
```rust
use ironcore::infrastructure::migration_cockroachdb::run_migrations_manual;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = /* 初始化连接池 */;
    run_migrations_manual(&pool).await?;
    Ok(())
}
```

---

## 验证迁移成功

### 1. 检查迁移记录

```sql
-- 查看已应用的迁移
SELECT * FROM _sqlx_migrations ORDER BY version;

-- 或
SELECT * FROM schema_migrations ORDER BY version;
```

### 2. 验证 0039 迁移

```sql
-- 检查合规性报告函数
SELECT * FROM generate_non_custodial_compliance_report();
```

预期输出:
```
category            | check_item                          | status    | details
--------------------+-------------------------------------+-----------+--------
Database Schema     | Wallets table has no custodial cols | ✅ PASS   | ...
Database Constraints| Non-custodial constraints enabled   | ✅ PASS   | ...
Data Integrity      | All wallets have valid addresses    | ✅ PASS   | ...
Dual Lock System    | Wallet unlock tokens table exists   | ✅ PASS   | ...
```

### 3. 检查关键表

```sql
-- 检查 wallets 表（不应有私钥字段）
\d wallets

-- 检查 wallet_unlock_tokens 表
\d wallet_unlock_tokens

-- 检查约束
SELECT conname, contype FROM pg_constraint 
WHERE conrelid = 'wallets'::regclass;
```

### 4. 检查审计日志

```sql
SELECT * FROM audit_logs 
WHERE event_type = 'NON_CUSTODIAL_COMPLIANCE_CHECKS_APPLIED'
ORDER BY created_at DESC LIMIT 1;
```

---

## 常见问题

### Q1: 迁移 2 校验和不匹配

**原因**: 迁移文件被修改，但数据库中记录的是旧的校验和

**解决方案**:
```powershell
# 删除迁移记录，重新应用
cockroach sql --url=$env:DATABASE_URL -e "DROP TABLE IF EXISTS _sqlx_migrations;"
cargo sqlx migrate run
```

### Q2: "already exists" 错误

**原因**: 表已存在，但迁移记录丢失

**解决方案**:
- 迁移文件使用 `IF NOT EXISTS`，可以安全重新运行
- 或手动记录迁移：
```sql
INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
VALUES (39, 'non custodial compliance checks', true, decode('...', 'hex'), 0);
```

### Q3: CockroachDB 不支持某些 PostgreSQL 特性

**已知限制**:
- ❌ EVENT TRIGGER（迁移 0030 中有，但可选）
- ❌ Advisory Locks
- ✅ 其他 PostgreSQL 特性大部分支持

**解决方案**: 迁移文件已经处理了兼容性问题

---

## 生产环境建议

### 1. 备份数据库

```bash
# CockroachDB 备份
cockroach dump ironcore --url=$DATABASE_URL > backup.sql
```

### 2. 在测试环境先验证

```powershell
# 使用测试数据库
$env:DATABASE_URL = "postgresql://root@localhost:26257/ironcore_test?sslmode=disable"
cargo sqlx migrate run
```

### 3. 使用事务（如果支持）

CockroachDB 支持事务，但某些 DDL 语句可能不支持回滚。

### 4. 监控迁移执行

```powershell
# 记录迁移日志
cargo sqlx migrate run 2>&1 | Tee-Object -FilePath migration.log
```

---

## 下一步

✅ 执行迁移脚本:
```powershell
cd IronCore
.\apply_migrations_cargo.ps1
```

✅ 验证结果:
```sql
SELECT * FROM generate_non_custodial_compliance_report();
```

✅ 启动应用:
```powershell
cargo run
```

---

*文档更新时间: 2025-12-03*

