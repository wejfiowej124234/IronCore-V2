#!/bin/bash
# 插入平台服务费规则到数据库

# 数据库连接字符串（从.env或默认值）
DATABASE_URL="${DATABASE_URL:-postgresql://root@localhost:26257/ironcore?sslmode=disable}"

echo "🔧 正在连接数据库并插入费用规则..."
echo "数据库: $DATABASE_URL"

# 使用cockroach sql命令执行
docker exec -i cockroach1 cockroach sql --insecure --database=ironcore < migrations/insert_platform_fee_rules.sql

if [ $? -eq 0 ]; then
    echo "✅ 费用规则插入成功！"
    echo ""
    echo "已配置的费用规则："
    echo "- Swap (代币交换): 0.5%"
    echo "- Transfer (基础转账): 0.1%"
    echo "- Fiat Onramp (法币入金): 2.0%"
    echo "- Fiat Offramp (法币出金): 2.5%"
    echo "- Limit Order (限价单): 0.5%"
    echo "- Bridge (跨链桥): 1.0%"
else
    echo "❌ 插入失败，请检查数据库连接"
    exit 1
fi
