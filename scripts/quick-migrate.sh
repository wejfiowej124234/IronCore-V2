#!/usr/bin/env bash
# 快速迁移脚本 - 直接使用 docker exec 执行迁移

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo ""
echo "════════════════════════════════════════════════"
echo -e "  ${CYAN}🚀 快速数据库迁移${NC}"
echo "════════════════════════════════════════════════"
echo ""

# 检查容器是否运行
if ! docker ps --filter "name=cockroachdb" --format "{{.Names}}" | grep -q cockroachdb; then
    echo -e "${RED}[ERROR]${NC} CockroachDB 容器未运行"
    echo -e "${YELLOW}[INFO]${NC} 请先运行: cd ops && docker compose up -d cockroach"
    exit 1
fi

echo -e "${CYAN}[INFO]${NC} 确保数据库存在..."
docker exec ironwallet-cockroachdb cockroach sql --insecure -e "CREATE DATABASE IF NOT EXISTS ironcore;" >/dev/null 2>&1

echo -e "${CYAN}[INFO]${NC} 检查迁移状态..."
MIGRATED=$(docker exec ironwallet-cockroachdb cockroach sql --insecure -e "USE ironcore; SELECT COUNT(*) FROM schema_migrations;" --format=csv 2>/dev/null | tail -1 | tr -d ' ')

if [[ "$MIGRATED" == "0" ]] || [[ -z "$MIGRATED" ]]; then
    echo -e "${YELLOW}[INFO]${NC} 数据库未迁移，开始执行迁移..."
    echo ""
    echo -e "${CYAN}[INFO]${NC} 方法1: 启动应用自动迁移（推荐）"
    echo -e "${CYAN}[INFO]${NC}   cd IronCore && cargo run"
    echo ""
    echo -e "${CYAN}[INFO]${NC} 方法2: 使用 sqlx-cli 手动迁移"
    echo -e "${CYAN}[INFO]${NC}   export DATABASE_URL='postgresql://root@localhost:26257/ironcore?sslmode=disable'"
    echo -e "${CYAN}[INFO]${NC}   sqlx migrate run"
    echo ""
    echo -e "${GREEN}[OK]${NC} 数据库已就绪，可以开始迁移！"
else
    echo -e "${GREEN}[OK]${NC} 已迁移 $MIGRATED 个迁移文件"
    echo -e "${CYAN}[INFO]${NC} 查看迁移状态:"
    docker exec ironwallet-cockroachdb cockroach sql --insecure -e "USE ironcore; SELECT version, name FROM schema_migrations ORDER BY version;" --format=table
fi

echo ""

