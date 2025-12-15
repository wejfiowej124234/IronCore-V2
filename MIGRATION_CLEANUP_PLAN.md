# 数据库迁移问题诊断报告

## 🔴 问题根因（Critical Issues）

### 1. **迁移编号冲突**
```
0033_cross_chain_transactions_enhancements.sql     (Dec 3, 3.9K)
0033_update_fiat_providers_optimization.sql        (Dec 5, 6.2K)  ❌ 重复编号！
```
- SQLx按文件名排序执行，两个0033会导致顺序混乱
- Checksum冲突：数据库记录的是第一个0033，但文件已被第二个覆盖

### 2. **Checksum不匹配**
```
Migration 20: applied vs local checksum不同
Migration 33: applied vs local checksum不同  
Migration 47: applied vs local checksum不同
```
- 原因：迁移文件被修改后，checksum变化，但数据库中已记录旧checksum
- SQLx拒绝执行，防止数据损坏

### 3. **废弃文件未清理**
```
0045_fix_transactions_schema.sql.deprecated  ❌ 应删除
```

### 4. **SKIP_MIGRATIONS=1 导致的数据缺失**
- `tokens.registry` 表为空 → Token API返回404
- 种子数据在 `0013_initial_data.sql` 中，但从未执行

---

## ✅ 根本解决方案（3选1）

### **方案A: 完全重建数据库（推荐🌟）**
**适用场景**: 开发环境，数据可丢失

```bash
# 1. 停止后端
taskkill //F //IM ironcore.exe

# 2. 删除数据库
cd /c/Users/plant/Desktop/Rust-Blockchain/ops
docker compose down -v  # 删除volumes
docker compose up -d    # 重新创建

# 3. 清理迁移文件冲突
cd ../IronCore/migrations
mv 0033_update_fiat_providers_optimization.sql 0050_update_fiat_providers_optimization.sql
rm 0045_fix_transactions_schema.sql.deprecated

# 4. 重新运行所有迁移
cd ..
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
sqlx migrate run

# 5. 启动后端（不跳过迁移）
unset SKIP_MIGRATIONS
CONFIG_PATH=config.toml cargo run --release
```

**优点**: 彻底解决，数据一致性最高  
**缺点**: 丢失现有数据（用户、钱包等）

---

### **方案B: 修复现有数据库（保留数据）**
**适用场景**: 生产环境，不能丢数据

```bash
# 1. 备份数据库
cd /c/Users/plant/Desktop/Rust-Blockchain/IronCore
pg_dump $DATABASE_URL > backup_$(date +%Y%m%d_%H%M%S).sql

# 2. 重命名冲突的迁移文件
cd migrations
mv 0033_update_fiat_providers_optimization.sql 0050_update_fiat_providers_optimization.sql
rm 0045_fix_transactions_schema.sql.deprecated

# 3. 手动修复_sqlx_migrations表
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
sqlx migrate revert  # 回退到安全点
sqlx migrate run     # 重新执行

# 4. 手动插入tokens种子数据
psql $DATABASE_URL < migrations/0013_initial_data.sql
```

**优点**: 保留现有数据  
**缺点**: 需要手动处理，风险较高

---

### **方案C: 清理checksum并继续（快速修复）**
**适用场景**: 临时开发，快速验证

```bash
# 1. 删除SQLx迁移记录表
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
psql $DATABASE_URL -c "DROP TABLE IF EXISTS _sqlx_migrations CASCADE;"

# 2. 清理冲突文件
cd /c/Users/plant/Desktop/Rust-Blockchain/IronCore/migrations
mv 0033_update_fiat_providers_optimization.sql 0050_update_fiat_providers_optimization.sql
rm 0045_fix_transactions_schema.sql.deprecated

# 3. 重新初始化迁移
cd ..
sqlx migrate run --ignore-missing

# 4. 手动补充tokens数据
sqlx database reset  # 如果表已存在但数据为空
```

**优点**: 最快  
**缺点**: 可能遗漏某些迁移

---

## 🛠️ 长期预防措施

### 1. **迁移编号规范**
```bash
# 添加新迁移时检查最大编号
cd IronCore/migrations
MAX_NUM=$(ls -1 *.sql | grep -o '^[0-9]\+' | sort -n | tail -1)
NEXT_NUM=$(printf "%04d" $((10#$MAX_NUM + 1)))
echo "下一个迁移编号: ${NEXT_NUM}"
```

### 2. **禁止修改已应用的迁移**
- 已执行的迁移文件 **禁止修改**
- 需要变更时创建新迁移（`ALTER TABLE`）

### 3. **环境变量管理**
```toml
# config.toml
[database]
skip_migrations = false  # ❌ 改为配置文件控制，不用环境变量

# 开发环境自动迁移
[dev]
auto_migrate = true

# 生产环境手动迁移
[prod]
auto_migrate = false
```

### 4. **迁移验证脚本**
```bash
# IronCore/scripts/validate_migrations.sh
#!/bin/bash
cd migrations
# 检查重复编号
if [ $(ls -1 *.sql | cut -d_ -f1 | sort | uniq -d | wc -l) -gt 0 ]; then
    echo "❌ 发现重复的迁移编号！"
    exit 1
fi
echo "✅ 迁移文件编号无冲突"
```

---

## 📊 当前状态诊断

```
✅ 已应用: 0001-0045 (除0046待定)
⚠️  冲突: 0033 (两个文件)
⚠️  Checksum不匹配: 20, 33, 47
⏳ 待定: 0046, 0048, 0049
❌ tokens.registry: 空表 (种子数据未执行)
```

---

## 🎯 推荐执行流程

**如果是开发环境且数据不重要 → 选择方案A**

```bash
# 完整命令序列
cd /c/Users/plant/Desktop/Rust-Blockchain
taskkill //F //IM ironcore.exe 2>/dev/null || true
cd ops && docker compose down -v && docker compose up -d
sleep 5
cd ../IronCore/migrations
mv 0033_update_fiat_providers_optimization.sql 0050_update_fiat_providers_optimization.sql
rm 0045_fix_transactions_schema.sql.deprecated 2>/dev/null || true
cd ..
export DATABASE_URL="postgres://root@localhost:26257/ironcore?sslmode=disable"
sqlx migrate run
unset SKIP_MIGRATIONS
CONFIG_PATH=config.toml cargo run --release
```

**如果需要保留数据 → 选择方案B（需要谨慎操作）**

---

## 💡 关键教训

1. **迁移文件命名**: 使用时间戳而非递增编号（如 `20251206_120000_add_tokens.sql`）
2. **Git管理**: 迁移文件应纳入版本控制，避免本地修改
3. **测试环境**: 先在测试库验证迁移，再应用到生产
4. **自动化**: 使用CI/CD自动检查迁移冲突

---

生成时间: 2025-12-06
状态: 待执行
