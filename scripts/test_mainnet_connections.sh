#!/bin/bash
# 生产环境链连接测试脚本

echo "========================================="
echo "生产环境主网连接测试"
echo "========================================="
echo ""

# 测试 Ethereum 主网
echo "1️⃣ Ethereum Mainnet (Chain ID: 1)"
echo "-----------------------------------"
BLOCK=$(curl -s -X POST https://eth.llamarpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
BLOCK_DEC=$((16#${BLOCK#0x}))
echo "✅ 区块高度: $BLOCK_DEC"

CHAIN_ID=$(curl -s -X POST https://eth.llamarpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
CHAIN_ID_DEC=$((16#${CHAIN_ID#0x}))
echo "✅ Chain ID: $CHAIN_ID_DEC"

BASE_FEE=$(curl -s -X POST https://eth.llamarpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_getBlockByNumber","params":["latest",false]}' | \
  grep -o '"baseFeePerGas":"[^"]*"' | cut -d'"' -f4)
BASE_FEE_DEC=$((16#${BASE_FEE#0x}))
BASE_FEE_GWEI=$(echo "scale=2; $BASE_FEE_DEC / 1000000000" | bc)
echo "✅ Base Fee: $BASE_FEE_GWEI Gwei"
echo ""

# 测试 BSC 主网
echo "2️⃣ BSC Mainnet (Chain ID: 56)"
echo "-----------------------------------"
BLOCK=$(curl -s -X POST https://bsc-dataseed1.binance.org \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
BLOCK_DEC=$((16#${BLOCK#0x}))
echo "✅ 区块高度: $BLOCK_DEC"

CHAIN_ID=$(curl -s -X POST https://bsc-dataseed1.binance.org \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
CHAIN_ID_DEC=$((16#${CHAIN_ID#0x}))
echo "✅ Chain ID: $CHAIN_ID_DEC"

GAS_PRICE=$(curl -s -X POST https://bsc-dataseed1.binance.org \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_gasPrice","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
GAS_PRICE_DEC=$((16#${GAS_PRICE#0x}))
GAS_PRICE_GWEI=$(echo "scale=2; $GAS_PRICE_DEC / 1000000000" | bc)
echo "✅ Gas Price: $GAS_PRICE_GWEI Gwei"
echo ""

# 测试 Polygon 主网
echo "3️⃣ Polygon Mainnet (Chain ID: 137)"
echo "-----------------------------------"
BLOCK=$(curl -s -X POST https://polygon-rpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_blockNumber","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
BLOCK_DEC=$((16#${BLOCK#0x}))
echo "✅ 区块高度: $BLOCK_DEC"

CHAIN_ID=$(curl -s -X POST https://polygon-rpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
CHAIN_ID_DEC=$((16#${CHAIN_ID#0x}))
echo "✅ Chain ID: $CHAIN_ID_DEC"

GAS_PRICE=$(curl -s -X POST https://polygon-rpc.com \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"eth_gasPrice","params":[]}' | \
  grep -o '"result":"[^"]*"' | cut -d'"' -f4)
GAS_PRICE_DEC=$((16#${GAS_PRICE#0x}))
GAS_PRICE_GWEI=$(echo "scale=2; $GAS_PRICE_DEC / 1000000000" | bc)
echo "✅ Gas Price: $GAS_PRICE_GWEI Gwei"
echo ""

# 测试后端 API
echo "4️⃣ 后端 API 测试"
echo "-----------------------------------"
echo "测试 Ethereum Gas 估算 API..."
curl -s "http://localhost:8088/api/v1/gas/estimate-all?chain=ethereum" | grep -o '"chain_id":[0-9]*'
echo ""

echo "测试 BSC Gas 估算 API..."
curl -s "http://localhost:8088/api/v1/gas/estimate-all?chain=bsc" | head -100
echo ""

echo "测试 Polygon Gas 估算 API..."
curl -s "http://localhost:8088/api/v1/gas/estimate-all?chain=polygon" | head -100
echo ""

echo "========================================="
echo "✅ 所有主网连接测试完成"
echo "========================================="
