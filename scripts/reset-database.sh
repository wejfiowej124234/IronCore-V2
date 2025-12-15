#!/usr/bin/env bash
# Bash脚本：完全重置数据库（开发环境专用）
# ⚠️ 警告：这会删除所有数据！仅用于开发环境
# 使用方法: ./reset-database.sh

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
GRAY='\033[0;37m'
NC='\033[0m' # No Color

# 检查参数
FORCE=false
if [[ "${1:-}" == "--force" || "${1:-}" == "-f" ]]; then
    FORCE=true
fi

echo ""
echo "════════════════════════════════════════════════"
echo "  🗄️  CockroachDB 完全重置工具"
echo "════════════════════════════════════════════════"
echo ""
echo -e "${RED}⚠️  ⚠️  ⚠️  警告：这将删除所有数据库数据！${NC}"
echo -e "${RED}⚠️  仅用于开发环境！生产环境请勿使用！${NC}"
echo ""

if [[ "$FORCE" != "true" ]]; then
    read -p "确认要重置数据库吗？输入 'YES' 继续: " confirm
    if [[ "$confirm" != "YES" ]]; then
        echo -e "${YELLOW}❌ 操作已取消${NC}"
        exit 0
    fi
fi

echo ""
echo "════════════════════════════════════════════════"
echo "步骤 1/4: 查找并停止 CockroachDB 容器..."
echo "════════════════════════════════════════════════"

# 查找所有可能的容器名
container_names=("ironwallet-cockroachdb" "ironwallet-co" "cockroach")
found_containers=()
seen_containers=()

for name in "${container_names[@]}"; do
    containers=$(docker ps -a --filter "name=$name" --format "{{.Names}}" 2>/dev/null || true)
    if [[ -n "$containers" ]]; then
        while IFS= read -r container; do
            if [[ -n "$container" ]]; then
                # 检查是否已经处理过这个容器
                is_seen=false
                for seen in "${seen_containers[@]}"; do
                    if [[ "$seen" == "$container" ]]; then
                        is_seen=true
                        break
                    fi
                done
                
                if [[ "$is_seen" == "false" ]]; then
                    found_containers+=("$container")
                    seen_containers+=("$container")
                    echo -e "  ${GREEN}✓${NC} 找到容器: $container"
                fi
            fi
        done <<< "$containers"
    fi
done

if [[ ${#found_containers[@]} -eq 0 ]]; then
    echo -e "  ${GRAY}ℹ️  未找到运行中的容器${NC}"
else
    # 停止所有找到的容器
    for container in "${found_containers[@]}"; do
        echo -e "  ${YELLOW}🛑${NC} 停止容器: $container"
        if docker stop "$container" 2>/dev/null; then
            echo -e "    ${GREEN}✓${NC} 已停止"
        fi
    done
    
    # 删除所有找到的容器
    for container in "${found_containers[@]}"; do
        echo -e "  ${YELLOW}🗑️${NC}  删除容器: $container"
        if docker rm "$container" 2>/dev/null; then
            echo -e "    ${GREEN}✓${NC} 已删除"
        fi
    done
fi

echo ""
echo "════════════════════════════════════════════════"
echo "步骤 2/4: 查找并删除数据卷..."
echo "════════════════════════════════════════════════"

# 查找所有可能的数据卷名（先查找所有包含 crdb 的卷，避免重复）
found_volumes=()
seen_volumes=()

# 先查找所有包含 crdb 的卷
all_crdb_volumes=$(docker volume ls --filter "name=crdb" --format "{{.Name}}" 2>/dev/null || true)
if [[ -n "$all_crdb_volumes" ]]; then
    while IFS= read -r volume; do
        if [[ -n "$volume" ]]; then
            # 检查是否已经在列表中
            is_seen=false
            for seen in "${seen_volumes[@]}"; do
                if [[ "$seen" == "$volume" ]]; then
                    is_seen=true
                    break
                fi
            done
            
            if [[ "$is_seen" == "false" ]]; then
                found_volumes+=("$volume")
                seen_volumes+=("$volume")
                echo -e "  ${GREEN}✓${NC} 找到数据卷: $volume"
            fi
        fi
    done <<< "$all_crdb_volumes"
fi

# 也检查特定的卷名（以防遗漏）
volume_names=("ops_crdb-data" "ironwallet_cockroachdb_crdb-data" "crdb-data")
for name in "${volume_names[@]}"; do
    volumes=$(docker volume ls --filter "name=$name" --format "{{.Name}}" 2>/dev/null || true)
    if [[ -n "$volumes" ]]; then
        while IFS= read -r volume; do
            if [[ -n "$volume" ]]; then
                # 检查是否已经在列表中
                is_seen=false
                for seen in "${seen_volumes[@]}"; do
                    if [[ "$seen" == "$volume" ]]; then
                        is_seen=true
                        break
                    fi
                done
                
                if [[ "$is_seen" == "false" ]]; then
                    found_volumes+=("$volume")
                    seen_volumes+=("$volume")
                    echo -e "  ${GREEN}✓${NC} 找到数据卷: $volume"
                fi
            fi
        done <<< "$volumes"
    fi
done

if [[ ${#found_volumes[@]} -eq 0 ]]; then
    echo -e "  ${GRAY}ℹ️  未找到数据卷${NC}"
else
    # 删除所有找到的数据卷
    for volume in "${found_volumes[@]}"; do
        echo -e "  ${YELLOW}🗑️${NC}  删除数据卷: $volume"
        if docker volume rm "$volume" 2>/dev/null; then
            echo -e "    ${GREEN}✓${NC} 已删除"
        else
            echo -e "    ${YELLOW}⚠️${NC}  删除失败（可能正在使用中）"
        fi
    done
fi

echo ""
echo "════════════════════════════════════════════════"
echo "步骤 3/4: 重新启动 CockroachDB 容器..."
echo "════════════════════════════════════════════════"

# 获取脚本所在目录的父目录（项目根目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# 尝试多个可能的路径（从脚本位置和当前工作目录）
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

# 如果还是没找到，尝试从当前目录查找
if [[ -z "$DOCKER_COMPOSE_PATH" ]]; then
    # 尝试从当前工作目录查找
    if [[ -f "ops/docker-compose.yml" ]]; then
        DOCKER_COMPOSE_PATH="ops/docker-compose.yml"
        PROJECT_ROOT="$(pwd)"
    elif [[ -f "../ops/docker-compose.yml" ]]; then
        DOCKER_COMPOSE_PATH="../ops/docker-compose.yml"
        PROJECT_ROOT="$(cd .. && pwd)"
    fi
fi

if [[ -z "$DOCKER_COMPOSE_PATH" || ! -f "$DOCKER_COMPOSE_PATH" ]]; then
    echo -e "${RED}❌ 未找到 docker-compose.yml 文件${NC}"
    echo -e "${YELLOW}   已尝试的路径：${NC}"
    for path in "${DOCKER_COMPOSE_PATHS[@]}"; do
        echo -e "     • $path"
    done
    echo -e "${YELLOW}   请手动启动 CockroachDB 容器${NC}"
    echo -e "${YELLOW}   或确保在项目根目录运行脚本${NC}"
    exit 1
fi

echo -e "  ${CYAN}📁${NC} 项目目录: $PROJECT_ROOT"
echo -e "  ${CYAN}📄${NC} Docker Compose: $DOCKER_COMPOSE_PATH"
echo -e "  ${YELLOW}🚀${NC} 启动容器..."

# 切换到项目根目录
cd "$PROJECT_ROOT" || {
    echo -e "${RED}❌ 无法切换到项目目录: $PROJECT_ROOT${NC}"
    exit 1
}

# 确定 docker-compose.yml 的路径
# 如果找到的是绝对路径，直接使用；否则使用相对路径
if [[ "$DOCKER_COMPOSE_PATH" == /* ]]; then
    # 绝对路径
    COMPOSE_FILE="$DOCKER_COMPOSE_PATH"
    # 获取目录部分，用于 cd
    COMPOSE_DIR="$(dirname "$DOCKER_COMPOSE_PATH")"
    COMPOSE_FILE_NAME="$(basename "$DOCKER_COMPOSE_PATH")"
    
    # 切换到 compose 文件所在目录
    cd "$COMPOSE_DIR" || {
        echo -e "${RED}❌ 无法切换到 compose 目录: $COMPOSE_DIR${NC}"
        exit 1
    }
    COMPOSE_FILE="./$COMPOSE_FILE_NAME"
else
    # 相对路径
    COMPOSE_FILE="$DOCKER_COMPOSE_PATH"
fi

# 尝试不同的 docker compose 命令格式
echo -e "  ${CYAN}执行:${NC} docker compose -f $COMPOSE_FILE up -d cockroach"
if docker compose -f "$COMPOSE_FILE" up -d cockroach 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} 容器已启动"
elif docker-compose -f "$COMPOSE_FILE" up -d cockroach 2>/dev/null; then
    echo -e "  ${GREEN}✓${NC} 容器已启动（使用 docker-compose）"
else
    echo -e "  ${RED}❌${NC} 启动失败"
    echo -e "  ${YELLOW}尝试的命令: docker compose -f $COMPOSE_FILE up -d cockroach${NC}"
    echo -e "  ${YELLOW}当前目录: $(pwd)${NC}"
    echo -e "  ${YELLOW}Compose 文件: $COMPOSE_FILE${NC}"
    exit 1
fi

# 切换回项目根目录
cd "$PROJECT_ROOT" || true

echo ""
echo "════════════════════════════════════════════════"
echo "步骤 4/4: 等待数据库就绪..."
echo "════════════════════════════════════════════════"

# 等待数据库启动并检查健康状态
MAX_RETRIES=30
RETRY_COUNT=0
IS_READY=false

echo -e "  ${CYAN}⏳${NC} 等待数据库启动..."

while [[ $RETRY_COUNT -lt $MAX_RETRIES && "$IS_READY" == "false" ]]; do
    sleep 2
    RETRY_COUNT=$((RETRY_COUNT + 1))
    
    # 检查容器是否运行
    container_status=$(docker ps --filter "name=ironwallet-cockroachdb" --format "{{.Status}}" 2>/dev/null || true)
    if [[ "$container_status" == *"Up"* ]]; then
        # 尝试连接数据库
        if docker exec ironwallet-cockroachdb cockroach sql --insecure -e "SELECT 1;" >/dev/null 2>&1; then
            IS_READY=true
            echo -e "  ${GREEN}✓${NC} 数据库已就绪！"
            break
        fi
    fi
    
    if [[ $((RETRY_COUNT % 5)) -eq 0 ]]; then
        echo -e "    ${GRAY}... 等待中 ($RETRY_COUNT/$MAX_RETRIES) ...${NC}"
    fi
done

if [[ "$IS_READY" == "false" ]]; then
    echo -e "  ${YELLOW}⚠️${NC}  数据库可能未完全就绪，但容器已启动"
    echo -e "     请稍后手动检查数据库状态"
fi

echo ""
echo "════════════════════════════════════════════════"
echo -e "  ${GREEN}✅ 数据库重置完成！${NC}"
echo "════════════════════════════════════════════════"
echo ""
echo -e "${CYAN}📋 下一步操作：${NC}"
echo -e "   ${GRAY}1. 启动后端应用，迁移会自动执行${NC}"
echo -e "      命令: cargo run"
echo ""
echo -e "   ${GRAY}2. 或手动运行迁移脚本${NC}"
echo -e "      命令: ./scripts/run-migrations-cockroachdb.sh"
echo ""
echo -e "   ${GRAY}3. 检查数据库状态${NC}"
echo -e "      命令: docker ps --filter name=cockroach"
echo ""
echo -e "${CYAN}📊 数据库信息：${NC}"
echo -e "   ${GRAY}• 容器名: ironwallet-cockroachdb${NC}"
echo -e "   ${GRAY}• SQL 端口: localhost:26257${NC}"
echo -e "   ${GRAY}• Admin UI: http://localhost:8090${NC}"
echo ""

