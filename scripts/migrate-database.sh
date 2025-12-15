#!/usr/bin/env bash
# 数据库迁移脚本 - 标准版本
# 使用新的标准化迁移文件

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# 获取脚本所在目录的父目录（IronCore目录）
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# 检查DATABASE_URL环境变量
if [[ -z "${DATABASE_URL:-}" ]]; then
    if [[ -f "config.toml" ]]; then
        # 从 config.toml 读取数据库 URL（匹配 [database] 部分的 url）
        DATABASE_URL=$(awk '/^\[database\]/,/^\[/ {if (/^url\s*=\s*"/) {match($0, /url\s*=\s*"([^"]+)"/, arr); print arr[1]; exit}}' config.toml)
    fi
    
    if [[ -z "${DATABASE_URL:-}" ]]; then
        echo -e "${YELLOW}[INFO]${NC} DATABASE_URL not found, using default"
        DATABASE_URL="postgresql://root@localhost:26257/ironcore?sslmode=disable"
    fi
fi

echo ""
echo "════════════════════════════════════════════════"
echo -e "  ${CYAN}🗄️  数据库迁移工具${NC}"
echo "════════════════════════════════════════════════"
echo ""
echo -e "${CYAN}[INFO]${NC} Running database migrations..."
echo -e "${CYAN}[INFO]${NC} Database URL: $DATABASE_URL"
echo -e "${CYAN}[INFO]${NC} Migrations directory: migrations"
echo ""

# 检查sqlx是否安装
if ! command -v sqlx &> /dev/null; then
    echo -e "${RED}[ERROR]${NC} sqlx-cli not found in PATH"
    echo -e "${YELLOW}[INFO]${NC} Please install: cargo install sqlx-cli"
    echo -e "${YELLOW}[INFO]${NC} Or migrations will run automatically on backend startup"
    exit 1
fi

# 使用sqlx migrate run
if sqlx migrate run --database-url "$DATABASE_URL"; then
    echo ""
    echo -e "${GREEN}[OK]${NC} ✅ Migrations completed successfully!"
    echo ""
    echo -e "${CYAN}[INFO]${NC} Migration files executed:"
    echo "   • 0001_schemas.sql - 创建 Schema"
    echo "   • 0002_core_tables.sql - 核心业务表"
    echo "   • 0003_gas_tables.sql - 费用系统表"
    echo "   • 0004_admin_tables.sql - 管理员表"
    echo "   • 0005_notify_tables.sql - 通知系统表"
    echo "   • 0006_asset_tables.sql - 资产聚合表"
    echo "   • 0007_tokens_tables.sql - 代币注册表"
    echo "   • 0008_events_tables.sql - 事件总线表"
    echo "   • 0009_fiat_tables.sql - 法币系统表"
    echo "   • 0010_constraints.sql - 外键和唯一约束"
    echo "   • 0011_indexes.sql - 索引"
    echo "   • 0012_check_constraints.sql - 检查约束"
    echo "   • 0013_initial_data.sql - 初始数据"
    exit 0
else
    echo ""
    echo -e "${YELLOW}[WARN]${NC} Migration failed (non-fatal)"
    echo -e "${YELLOW}[INFO]${NC} Backend will attempt to run migrations on startup"
    echo -e "${YELLOW}[TIP]${NC} Check database connection and ensure CockroachDB is running"
    exit 1
fi

