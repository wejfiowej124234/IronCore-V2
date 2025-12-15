#!/bin/bash
# 数据库健康检查和自动修复脚本（Docker版本）
# 使用 Docker exec 访问 CockroachDB

set -e

echo "=================================================="
echo "IronCore 数据库健康检查与自动修复 (Docker版)"
echo "=================================================="
echo ""

# 1. 检查 Docker 是否运行
echo "1️⃣  检查 Docker 状态..."
if ! docker ps > /dev/null 2>&1; then
    echo "❌ Docker 未运行，请先启动 Docker Desktop"
    exit 1
fi
echo "✅ Docker 正在运行"

# 2. 检查 CockroachDB 容器
echo ""
echo "2️⃣  检查 CockroachDB 容器..."
if ! docker ps | grep -q ironwallet-cockroachdb; then
    echo "❌ CockroachDB 容器未运行"
    echo "正在尝试启动..."
    docker start ironwallet-cockroachdb || {
        echo "❌ 无法启动容器，请运行: docker compose -f ops/docker-compose.yml up -d"
        exit 1
    }
    sleep 3
fi
echo "✅ CockroachDB 容器正在运行"

# 3. 检查 ironcore 数据库
echo ""
echo "3️⃣  检查 ironcore 数据库..."
DB_EXISTS=$(docker exec ironwallet-cockroachdb ./cockroach sql --insecure -e "SELECT COUNT(*) FROM [SHOW DATABASES] WHERE database_name = 'ironcore';" --format=tsv 2>/dev/null | tail -1)

if [ "$DB_EXISTS" = "0" ]; then
    echo "⚠️  ironcore 数据库不存在，正在创建..."
    docker exec ironwallet-cockroachdb ./cockroach sql --insecure -e "CREATE DATABASE IF NOT EXISTS ironcore;"
    echo "✅ 数据库创建成功"
else
    echo "✅ ironcore 数据库存在"
fi

# 4. 检查 fiat schema
echo ""
echo "4️⃣  检查 fiat schema..."
SCHEMA_EXISTS=$(docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "SELECT COUNT(*) FROM information_schema.schemata WHERE schema_name = 'fiat';" --format=tsv 2>/dev/null | tail -1)

if [ "$SCHEMA_EXISTS" = "0" ]; then
    echo "⚠️  fiat schema 不存在，请先运行数据库迁移"
    echo "运行: cd IronCore && cargo sqlx migrate run"
    exit 1
fi
echo "✅ fiat schema 存在"

# 5. 检查 fiat.providers 表
echo ""
echo "5️⃣  检查 fiat.providers 表..."
TABLE_EXISTS=$(docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = 'fiat' AND table_name = 'providers';" --format=tsv 2>/dev/null | tail -1)

if [ "$TABLE_EXISTS" = "0" ]; then
    echo "❌ fiat.providers 表不存在，请先运行迁移"
    exit 1
fi
echo "✅ fiat.providers 表存在"

# 6. 检查服务商数据
echo ""
echo "6️⃣  检查服务商数据..."
PROVIDER_COUNT=$(docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "SELECT COUNT(*) FROM fiat.providers;" --format=tsv 2>/dev/null | tail -1)

if [ "$PROVIDER_COUNT" = "0" ]; then
    echo "⚠️  fiat.providers 表为空，正在初始化..."
    
    # 通过 Docker 执行 SQL 文件
    cat "$(dirname "$0")/check_and_seed_providers.sql" | docker exec -i ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore
    
    # 再次检查
    PROVIDER_COUNT=$(docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "SELECT COUNT(*) FROM fiat.providers;" --format=tsv 2>/dev/null | tail -1)
    
    if [ "$PROVIDER_COUNT" -gt "0" ]; then
        echo "✅ 成功初始化 $PROVIDER_COUNT 个服务商"
    else
        echo "❌ 服务商初始化失败"
        exit 1
    fi
else
    echo "✅ 找到 $PROVIDER_COUNT 个服务商"
fi

# 7. 检查启用的服务商
echo ""
echo "7️⃣  检查启用的服务商..."
ENABLED_COUNT=$(docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "SELECT COUNT(*) FROM fiat.providers WHERE is_enabled = true;" --format=tsv 2>/dev/null | tail -1)

if [ "$ENABLED_COUNT" = "0" ]; then
    echo "⚠️  没有启用的服务商，正在启用..."
    docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "UPDATE fiat.providers SET is_enabled = true, health_status = 'healthy';"
    ENABLED_COUNT=$PROVIDER_COUNT
    echo "✅ 启用了 $ENABLED_COUNT 个服务商"
else
    echo "✅ 有 $ENABLED_COUNT 个启用的服务商"
fi

# 8. 显示服务商列表
echo ""
echo "8️⃣  当前服务商状态："
echo "------------------------------------------------"
docker exec ironwallet-cockroachdb ./cockroach sql --insecure --database=ironcore -e "SELECT name, display_name, is_enabled, priority, health_status FROM fiat.providers ORDER BY priority;"

echo ""
echo "=================================================="
echo "✅ 数据库健康检查完成，可以启动后端服务"
echo "=================================================="
echo ""
echo "下一步："
echo "  1. 设置环境变量: export DATABASE_URL='postgres://root@localhost:26257/ironcore?sslmode=disable'"
echo "  2. 启动后端: cargo run"
