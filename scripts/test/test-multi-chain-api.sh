#!/bin/bash

# 多链钱包 API 测试脚本

BASE_URL="http://localhost:8088"

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "多链钱包 API 测试"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# 测试 1: 列出所有支持的链
echo "📋 测试 1: GET /api/chains - 列出所有支持的链"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "$BASE_URL/api/chains" | jq '.'
echo ""
echo ""

# 测试 2: 按曲线类型分组
echo "📊 测试 2: GET /api/chains/by-curve - 按曲线类型分组"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "$BASE_URL/api/chains/by-curve" | jq '.'
echo ""
echo ""

# 测试 3: 创建 ETH 钱包
echo "🔑 测试 3: POST /api/wallets/create - 创建 ETH 钱包"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
ETH_RESPONSE=$(curl -s "$BASE_URL/api/wallets/create" \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "ETH",
    "word_count": 12
  }')
echo "$ETH_RESPONSE" | jq '.'

# 提取助记词用于多链测试
MNEMONIC=$(echo "$ETH_RESPONSE" | jq -r '.mnemonic // empty')
ETH_ADDRESS=$(echo "$ETH_RESPONSE" | jq -r '.wallet.address // empty')
echo ""
echo "✅ 已创建 ETH 钱包"
echo "   地址: $ETH_ADDRESS"
echo "   助记词: $MNEMONIC"
echo ""
echo ""

# 测试 4: 从同一助记词创建多链钱包
if [ -n "$MNEMONIC" ]; then
    echo "🌐 测试 4: POST /api/wallets/create-multi - 从同一助记词创建多链钱包"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    curl -s "$BASE_URL/api/wallets/create-multi" \
      -H "Content-Type: application/json" \
      -d "{
        \"chains\": [\"ETH\", \"BSC\", \"SOL\", \"BTC\"],
        \"mnemonic\": \"$MNEMONIC\"
      }" | jq '.'
    echo ""
    echo ""
fi

# 测试 5: 验证 ETH 地址
if [ -n "$ETH_ADDRESS" ]; then
    echo "✔️  测试 5: POST /api/wallets/validate-address - 验证 ETH 地址"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    curl -s "$BASE_URL/api/wallets/validate-address" \
      -H "Content-Type: application/json" \
      -d "{
        \"chain\": \"ETH\",
        \"address\": \"$ETH_ADDRESS\"
      }" | jq '.'
    echo ""
    echo ""
fi

# 测试 6: 创建 Solana 钱包
echo "☀️  测试 6: POST /api/wallets/create - 创建 Solana 钱包"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
curl -s "$BASE_URL/api/wallets/create" \
  -H "Content-Type: application/json" \
  -d '{
    "chain": "SOL",
    "word_count": 12
  }' | jq '.'
echo ""
echo ""

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "✅ 所有测试完成!"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
