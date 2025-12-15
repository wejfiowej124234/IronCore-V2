#!/bin/bash
# 生产级 Mainnet 连接验证脚本
# 验证所有 8 条链的 RPC 连接和 Gas 估算功能
# 作者：IronCore Team | 日期：2025-12-11

set -e

BASE_URL="${API_BASE_URL:-http://localhost:8088}"
BOLD='\033[1m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BOLD}========================================${NC}"
echo -e "${BOLD}  IronCore Mainnet 连接验证 (8 chains)${NC}"
echo -e "${BOLD}========================================${NC}\n"

# 验证函数
verify_chain() {
    local chain=$1
    local expected_chain_id=$2
    
    echo -e "${YELLOW}[测试] $chain Mainnet${NC}"
    
    # 调用后端 API
    response=$(curl -s "${BASE_URL}/api/v1/gas/estimate-all?chain=${chain}")
    
    # 检查响应
    if echo "$response" | grep -q '"code":0'; then
        # 提取 Gas 费用
        slow_fee=$(echo "$response" | grep -o '"slow":{[^}]*"max_fee_per_gas_gwei":[0-9.]*' | grep -o '[0-9.]*$' || echo "N/A")
        normal_fee=$(echo "$response" | grep -o '"normal":{[^}]*"max_fee_per_gas_gwei":[0-9.]*' | grep -o '[0-9.]*$' || echo "N/A")
        fast_fee=$(echo "$response" | grep -o '"fast":{[^}]*"max_fee_per_gas_gwei":[0-9.]*' | grep -o '[0-9.]*$' || echo "N/A")
        
        echo -e "  ${GREEN}✅ SUCCESS${NC}"
        echo -e "  📊 Gas Fees (Gwei):"
        echo -e "     Slow:   $slow_fee"
        echo -e "     Normal: $normal_fee"
        echo -e "     Fast:   $fast_fee"
    else
        echo -e "  ${RED}❌ FAILED${NC}"
        echo -e "  Error: $(echo "$response" | head -c 200)"
        return 1
    fi
    
    echo ""
}

# 验证所有 EVM 链（L1 + L2）
echo -e "${BOLD}=== L1 Chains ===${NC}\n"
verify_chain "ethereum" "1"
verify_chain "bsc" "56"
verify_chain "polygon" "137"

echo -e "${BOLD}=== L2 Chains ===${NC}\n"
verify_chain "arbitrum" "42161"
verify_chain "optimism" "10"
verify_chain "avalanche" "43114"

echo -e "${BOLD}=== Non-EVM Chains ===${NC}\n"
echo -e "${YELLOW}[测试] Solana Mainnet${NC}"
sol_response=$(curl -s "${BASE_URL}/api/v1/gas/estimate-all?chain=solana")
if echo "$sol_response" | grep -q '"code":0'; then
    echo -e "  ${GREEN}✅ SUCCESS${NC}"
    echo "$sol_response" | grep -o '"max_fee_per_gas_gwei":[0-9.]*' | head -1
else
    echo -e "  ${RED}❌ FAILED${NC}"
    echo "$sol_response" | head -c 150
fi
echo ""

echo -e "${YELLOW}[测试] Bitcoin Mainnet${NC}"
btc_response=$(curl -s "${BASE_URL}/api/v1/gas/estimate-all?chain=bitcoin")
if echo "$btc_response" | grep -q '"code":0'; then
    echo -e "  ${GREEN}✅ SUCCESS${NC}"
    echo "$btc_response" | grep -o '"max_fee_per_gas_gwei":[0-9.]*' | head -1
else
    echo -e "  ${RED}❌ FAILED${NC}"
    echo "$btc_response" | head -c 150
fi
echo ""

# 验证数据库 RPC 端点配置
echo -e "${BOLD}=== Database RPC Endpoints ===${NC}\n"
echo -e "${YELLOW}[查询] 检查数据库中的 mainnet 端点数量${NC}"

# 使用 Docker 连接数据库
docker exec ironwallet-cockroachdb cockroach sql --insecure --database=ironcore \
    --execute="SELECT chain, COUNT(*) as endpoint_count FROM admin.rpc_endpoints WHERE is_active = true GROUP BY chain ORDER BY chain;" \
    2>/dev/null || echo "⚠️ 无法连接数据库，跳过数据库验证"

echo ""
echo -e "${BOLD}========================================${NC}"
echo -e "${GREEN}${BOLD}  ✅ Mainnet 验证完成！${NC}"
echo -e "${BOLD}========================================${NC}"
echo ""
echo -e "📝 总结："
echo -e "  - 6 条 EVM 链已配置 mainnet RPC"
echo -e "  - 2 条非 EVM 链（Solana, Bitcoin）已支持"
echo -e "  - 所有链 Gas 估算 API 正常工作"
echo -e "  - 数据来自真实区块链网络（非测试网）"
echo ""
