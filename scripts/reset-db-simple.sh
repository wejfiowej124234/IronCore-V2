#!/usr/bin/env bash
# 简单重置数据库脚本 - 通过环境变量触发
# ⚠️ 警告：这会删除所有数据！仅用于开发环境

set -euo pipefail

RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo ""
echo -e "${RED}⚠️  ⚠️  ⚠️  警告：这将删除所有数据库数据！${NC}"
echo -e "${RED}⚠️  仅用于开发环境！生产环境请勿使用！${NC}"
echo ""

read -p "确认要重置数据库吗？输入 'YES' 继续: " confirm

if [[ "$confirm" != "YES" ]]; then
    echo -e "${YELLOW}❌ 操作已取消${NC}"
    exit 0
fi

echo ""
echo -e "${GREEN}🧹 正在重置数据库...${NC}"
echo ""

# 设置环境变量触发重置
export RESET_DB=true

# 获取脚本所在目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# 运行后端（会自动重置数据库）
echo -e "${GREEN}🚀 启动后端并重置数据库...${NC}"
echo "    注意：后端启动后会自动重置数据库并退出"
echo ""

cargo run

echo ""
echo -e "${GREEN}✅ 数据库重置完成！${NC}"
echo ""

