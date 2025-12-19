#!/bin/bash
set -e

echo "=========================================="
echo "执行数据库迁移（临时移除SKIP_MIGRATIONS）"
echo "=========================================="
echo ""

echo "步骤1: 移除SKIP_MIGRATIONS环境变量"
flyctl secrets unset SKIP_MIGRATIONS -a oxidevault-ironcore-v2

echo ""
echo "步骤2: 等待应用重启并完成迁移（约2分钟）"
sleep 120

echo ""
echo "步骤3: 恢复SKIP_MIGRATIONS设置"
flyctl secrets set SKIP_MIGRATIONS=1 -a oxidevault-ironcore-v2

echo ""
echo "步骤4: 验证迁移状态"
echo "数据库迁移版本:"
printf "SELECT version, description FROM _sqlx_migrations ORDER BY version DESC LIMIT 5;\nSELECT COUNT(*) AS total FROM _sqlx_migrations;\n\\\\q\n" | flyctl postgres connect -a oxidevault-ironcore-v2-db -d ironcore

echo ""
echo "步骤5: 检查应用健康"
flyctl status -a oxidevault-ironcore-v2

echo ""
echo "✅ 迁移完成！"
