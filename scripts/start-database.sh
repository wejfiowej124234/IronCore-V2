#!/usr/bin/env bash
# 启动数据库服务脚本

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# 查找 docker-compose.yml
DOCKER_COMPOSE_PATHS=(
    "$PROJECT_ROOT/ops/docker-compose.yml"
    "$(pwd)/ops/docker-compose.yml"
    "$(pwd)/../ops/docker-compose.yml"
    "$PROJECT_ROOT/../ops/docker-compose.yml"
    "./ops/docker-compose.yml"
    "../ops/docker-compose.yml"
)

DOCKER_COMPOSE_PATH=""
for path in "${DOCKER_COMPOSE_PATHS[@]}"; do
    if [[ -f "$path" ]]; then
        DOCKER_COMPOSE_PATH="$path"
        break
    fi
done

if [[ -z "$DOCKER_COMPOSE_PATH" ]]; then
    echo -e "${RED}[ERROR]${NC} 未找到 docker-compose.yml 文件"
    exit 1
fi

echo ""
echo "════════════════════════════════════════════════"
echo -e "  ${CYAN}🚀 启动数据库服务${NC}"
echo "════════════════════════════════════════════════"
echo ""
echo -e "${CYAN}[INFO]${NC} 使用 docker-compose 文件: $DOCKER_COMPOSE_PATH"
echo ""

# 检查 Docker 是否运行
if ! docker info >/dev/null 2>&1; then
    echo -e "${RED}[ERROR]${NC} Docker 未运行，请先启动 Docker Desktop"
    exit 1
fi

# 启动 CockroachDB
echo -e "${CYAN}[INFO]${NC} 启动 CockroachDB..."
cd "$(dirname "$DOCKER_COMPOSE_PATH")"
docker compose up -d cockroach

# 等待数据库就绪
echo -e "${CYAN}[INFO]${NC} 等待数据库就绪..."
sleep 5

# 检查容器状态
if docker ps --filter "name=cockroachdb" --format "{{.Names}}" | grep -q cockroachdb; then
    echo -e "${GREEN}[OK]${NC} ✅ CockroachDB 已启动"
    echo ""
    echo -e "${CYAN}[INFO]${NC} 容器状态:"
    docker ps --filter "name=cockroachdb" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
    echo ""
    echo -e "${CYAN}[INFO]${NC} 数据库 URL: postgresql://root@localhost:26257/ironcore?sslmode=disable"
    echo -e "${CYAN}[INFO]${NC} Admin UI: http://localhost:8090"
    echo ""
    echo -e "${GREEN}[OK]${NC} 现在可以运行迁移脚本了！"
    exit 0
else
    echo -e "${RED}[ERROR]${NC} CockroachDB 启动失败"
    echo -e "${YELLOW}[INFO]${NC} 检查日志: docker logs ironwallet-cockroachdb"
    exit 1
fi

