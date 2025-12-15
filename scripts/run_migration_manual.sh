#!/bin/bash
# 手动运行数据库迁移脚本（适用于CockroachDB）
# 使用方法: ./run_migration_manual.sh

set -e

echo "🚀 开始运行数据库迁移..."

# 检查DATABASE_URL环境变量
if [ -z "$DATABASE_URL" ]; then
    echo "❌ 错误: DATABASE_URL环境变量未设置"
    echo "请设置: export DATABASE_URL='postgres://root@localhost:26257/ironcore?sslmode=disable'"
    exit 1
fi

echo "📋 使用数据库: $DATABASE_URL"

# 运行sqlx migrate
echo "📦 执行迁移..."
sqlx migrate run --database-url "$DATABASE_URL"

echo "✅ 迁移完成!"

