#!/bin/bash
# 数据库健康检查和自动修复脚本
# 用于确保 IronCore 后端启动前所有必要的数据已初始化

set -e

# 获取数据库连接信息（从环境变量或配置文件）
DB_URL="${DATABASE_URL:-postgres://root@localhost:26257/ironcore?sslmode=disable}"

echo "=================================================="
echo "IronCore 数据库健康检查与自动修复"
echo "=================================================="
echo ""

# 1. 检查数据库连接
echo "1️⃣  检查数据库连接..."
if psql "$DB_URL" -c "SELECT 1" > /dev/null 2>&1; then
    echo "✅ 数据库连接正常"
else
    echo "❌ 数据库连接失败，请检查 DATABASE_URL: $DB_URL"
    exit 1
fi

# 2. 检查 fiat.providers 表是否存在
echo ""
echo "2️⃣  检查 fiat.providers 表..."
TABLE_EXISTS=$(psql "$DB_URL" -tAc "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_schema='fiat' AND table_name='providers');")

if [ "$TABLE_EXISTS" != "t" ]; then
    echo "❌ fiat.providers 表不存在，请先运行迁移: cargo sqlx migrate run"
    exit 1
else
    echo "✅ fiat.providers 表存在"
fi

# 3. 检查服务商数据
echo ""
echo "3️⃣  检查服务商数据..."
PROVIDER_COUNT=$(psql "$DB_URL" -tAc "SELECT COUNT(*) FROM fiat.providers;")

if [ "$PROVIDER_COUNT" -eq 0 ]; then
    echo "⚠️  fiat.providers 表为空，正在自动初始化..."
    
    # 执行自动修复脚本
    psql "$DB_URL" -f "$(dirname "$0")/check_and_seed_providers.sql"
    
    # 再次检查
    PROVIDER_COUNT=$(psql "$DB_URL" -tAc "SELECT COUNT(*) FROM fiat.providers;")
    
    if [ "$PROVIDER_COUNT" -gt 0 ]; then
        echo "✅ 成功初始化 $PROVIDER_COUNT 个服务商"
    else
        echo "❌ 服务商初始化失败"
        exit 1
    fi
else
    echo "✅ 找到 $PROVIDER_COUNT 个服务商"
fi

# 4. 检查启用的服务商
echo ""
echo "4️⃣  检查启用的服务商..."
ENABLED_COUNT=$(psql "$DB_URL" -tAc "SELECT COUNT(*) FROM fiat.providers WHERE is_enabled = true;")

if [ "$ENABLED_COUNT" -eq 0 ]; then
    echo "⚠️  没有启用的服务商，正在启用所有服务商..."
    psql "$DB_URL" -c "UPDATE fiat.providers SET is_enabled = true, health_status = 'healthy';"
    
    ENABLED_COUNT=$(psql "$DB_URL" -tAc "SELECT COUNT(*) FROM fiat.providers WHERE is_enabled = true;")
    echo "✅ 启用了 $ENABLED_COUNT 个服务商"
else
    echo "✅ 有 $ENABLED_COUNT 个启用的服务商"
fi

# 5. 显示服务商列表
echo ""
echo "5️⃣  当前服务商状态："
echo "------------------------------------------------"
psql "$DB_URL" -c "SELECT name, display_name, is_enabled, priority, health_status FROM fiat.providers ORDER BY priority;"

echo ""
echo "=================================================="
echo "✅ 数据库健康检查完成，可以启动后端服务"
echo "=================================================="
