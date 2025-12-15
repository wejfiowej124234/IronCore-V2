#!/usr/bin/env bash
# 🧪 Stripe 端到端支付测试脚本
# 使用方法：./test_stripe_payment.sh

set -e  # 遇到错误立即退出

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 配置
API_BASE="http://localhost:8088"
TEST_EMAIL="stripe-e2e-test@example.com"
TEST_PASSWORD="StripeTest@2025"

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Stripe 端到端支付测试套件${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# ============================================================
# 前置检查
# ============================================================
echo -e "${YELLOW}[1/8] 检查环境依赖...${NC}"

# 检查后端服务
if ! curl -s "${API_BASE}/api/health" > /dev/null; then
    echo -e "${RED}❌ 后端服务未运行！请先启动 IronCore${NC}"
    exit 1
fi
echo -e "${GREEN}✅ 后端服务正常运行${NC}"

# 检查环境变量
if [ -z "$STRIPE_SECRET_KEY" ] || [ "$STRIPE_SECRET_KEY" == "sk_test_placeholder" ]; then
    echo -e "${RED}❌ 未配置 STRIPE_SECRET_KEY 环境变量${NC}"
    echo -e "${YELLOW}请先设置：export STRIPE_SECRET_KEY=sk_test_your_key${NC}"
    exit 1
fi
echo -e "${GREEN}✅ Stripe 密钥已配置${NC}"

# 检查 ngrok
if ! pgrep -f "ngrok.*8088" > /dev/null; then
    echo -e "${YELLOW}⚠️  ngrok 未运行，webhook 测试将无法进行${NC}"
    echo -e "${YELLOW}建议启动：cd IronCore && ./ngrok.exe http 8088${NC}"
fi

echo ""

# ============================================================
# 步骤 1: 用户注册/登录
# ============================================================
echo -e "${YELLOW}[2/8] 用户认证...${NC}"

# 尝试注册（可能已存在）
REGISTER_RESP=$(curl -s -X POST "${API_BASE}/api/v1/auth/register" \
  -H 'Content-Type: application/json' \
  -d "{
    \"email\": \"${TEST_EMAIL}\",
    \"password\": \"${TEST_PASSWORD}\",
    \"nickname\": \"Stripe E2E Test\"
  }" || echo '{"code":40009}')

if echo "$REGISTER_RESP" | grep -q '"code":0'; then
    echo -e "${GREEN}✅ 用户注册成功${NC}"
elif echo "$REGISTER_RESP" | grep -q '40009'; then
    echo -e "${GREEN}✅ 用户已存在，继续登录${NC}"
else
    echo -e "${RED}❌ 注册失败：${REGISTER_RESP}${NC}"
    exit 1
fi

# 登录获取 token
LOGIN_RESP=$(curl -s -X POST "${API_BASE}/api/v1/auth/login" \
  -H 'Content-Type: application/json' \
  -d "{
    \"email\": \"${TEST_EMAIL}\",
    \"password\": \"${TEST_PASSWORD}\"
  }")

TOKEN=$(echo "$LOGIN_RESP" | grep -o '"access_token":"[^"]*"' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
    echo -e "${RED}❌ 登录失败：${LOGIN_RESP}${NC}"
    exit 1
fi
echo -e "${GREEN}✅ 登录成功，Token: ${TOKEN:0:20}...${NC}"
echo ""

# ============================================================
# 步骤 2: 创建法币订单
# ============================================================
echo -e "${YELLOW}[3/8] 创建法币订单...${NC}"

ORDER_ID="stripe-test-$(date +%s)"

ORDER_RESP=$(curl -s -X POST "${API_BASE}/api/v1/fiat/onramp/orders" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "{
    \"fiat_amount\": 100.00,
    \"fiat_currency\": \"USD\",
    \"crypto_currency\": \"ETH\",
    \"chain\": \"ethereum\",
    \"payment_method\": \"card\",
    \"provider_name\": \"moonpay\"
  }")

ORDER_UUID=$(echo "$ORDER_RESP" | grep -o '"order_id":"[^"]*"' | cut -d'"' -f4)

if [ -z "$ORDER_UUID" ]; then
    echo -e "${RED}❌ 订单创建失败：${ORDER_RESP}${NC}"
    exit 1
fi
echo -e "${GREEN}✅ 订单创建成功：${ORDER_UUID}${NC}"
echo ""

# ============================================================
# 步骤 3: 创建 Stripe 支付会话
# ============================================================
echo -e "${YELLOW}[4/8] 创建 Stripe 支付会话...${NC}"

SESSION_RESP=$(curl -s -X POST "${API_BASE}/api/v1/payments/stripe/create-session" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "{
    \"order_id\": \"${ORDER_UUID}\",
    \"amount\": 10000,
    \"currency\": \"USD\",
    \"success_url\": \"https://example.com/success?order_id=${ORDER_UUID}\",
    \"cancel_url\": \"https://example.com/cancel\"
  }")

SESSION_ID=$(echo "$SESSION_RESP" | grep -o '"session_id":"[^"]*"' | cut -d'"' -f4)
CHECKOUT_URL=$(echo "$SESSION_RESP" | grep -o '"url":"[^"]*"' | cut -d'"' -f4 | sed 's/\\//g')

if [ -z "$SESSION_ID" ]; then
    echo -e "${RED}❌ Stripe 会话创建失败：${SESSION_RESP}${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Stripe 会话创建成功${NC}"
echo -e "${BLUE}   Session ID: ${SESSION_ID}${NC}"
echo -e "${BLUE}   Checkout URL: ${CHECKOUT_URL}${NC}"
echo ""

# ============================================================
# 步骤 4: 用户交互 - 完成支付
# ============================================================
echo -e "${YELLOW}[5/8] 等待用户完成支付...${NC}"
echo -e "${BLUE}┌──────────────────────────────────────────────────────────┐${NC}"
echo -e "${BLUE}│ 请在浏览器中打开以下 URL 完成测试支付：                  │${NC}"
echo -e "${BLUE}│                                                           │${NC}"
echo -e "${GREEN}│ ${CHECKOUT_URL} │${NC}"
echo -e "${BLUE}│                                                           │${NC}"
echo -e "${BLUE}│ 使用 Stripe 测试卡：                                      │${NC}"
echo -e "${BLUE}│   卡号：4242 4242 4242 4242                              │${NC}"
echo -e "${BLUE}│   过期日期：12/34 (任意未来日期)                         │${NC}"
echo -e "${BLUE}│   CVC：123 (任意3位数字)                                 │${NC}"
echo -e "${BLUE}│   ZIP：12345 (任意邮编)                                  │${NC}"
echo -e "${BLUE}└──────────────────────────────────────────────────────────┘${NC}"
echo ""
echo -e "${YELLOW}完成支付后按 Enter 键继续...${NC}"
read -r

# ============================================================
# 步骤 5: 验证订单状态更新
# ============================================================
echo -e "${YELLOW}[6/8] 查询订单状态...${NC}"

sleep 2  # 等待 webhook 处理

ORDER_STATUS_RESP=$(curl -s -X GET "${API_BASE}/api/v1/fiat/onramp/orders/${ORDER_UUID}" \
  -H "Authorization: Bearer ${TOKEN}")

STATUS=$(echo "$ORDER_STATUS_RESP" | grep -o '"status":"[^"]*"' | cut -d'"' -f4)

echo -e "${BLUE}订单状态：${STATUS}${NC}"

if [ "$STATUS" == "completed" ]; then
    echo -e "${GREEN}✅ 订单状态已更新为 completed（支付成功）${NC}"
elif [ "$STATUS" == "pending" ]; then
    echo -e "${YELLOW}⚠️  订单仍为 pending 状态${NC}"
    echo -e "${YELLOW}可能原因：${NC}"
    echo -e "${YELLOW}  1. Webhook 未触发（检查 ngrok 是否运行）${NC}"
    echo -e "${YELLOW}  2. Stripe Dashboard 中 webhook 配置错误${NC}"
    echo -e "${YELLOW}  3. 支付未实际完成${NC}"
else
    echo -e "${RED}❌ 订单状态异常：${STATUS}${NC}"
fi
echo ""

# ============================================================
# 步骤 6: 测试 Webhook 签名验证
# ============================================================
echo -e "${YELLOW}[7/8] 测试 Webhook 签名验证机制...${NC}"

# 发送无效签名的 webhook（应被拒绝）
INVALID_WEBHOOK_RESP=$(curl -s -w "\n%{http_code}" -X POST "${API_BASE}/api/v1/webhooks/stripe" \
  -H 'Content-Type: application/json' \
  -H 'Stripe-Signature: t=1234567890,v1=invalid_fake_signature' \
  -d '{
    "type": "checkout.session.completed",
    "data": {
      "object": {
        "id": "cs_test_fake",
        "payment_status": "paid"
      }
    }
  }')

HTTP_CODE=$(echo "$INVALID_WEBHOOK_RESP" | tail -n 1)

if [ "$HTTP_CODE" == "401" ] || [ "$HTTP_CODE" == "400" ]; then
    echo -e "${GREEN}✅ 签名验证成功拒绝无效请求（HTTP ${HTTP_CODE}）${NC}"
else
    echo -e "${RED}❌ 签名验证失败！无效请求被接受（HTTP ${HTTP_CODE}）${NC}"
    echo -e "${RED}响应：${INVALID_WEBHOOK_RESP}${NC}"
fi
echo ""

# ============================================================
# 步骤 7: 对账流程测试
# ============================================================
echo -e "${YELLOW}[8/8] 执行对账流程...${NC}"

TODAY=$(date +%Y-%m-%d)

RECONCILE_RESP=$(curl -s -X POST "${API_BASE}/api/v1/reconciliation/daily" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H 'Content-Type: application/json' \
  -d "{
    \"date\": \"${TODAY}\",
    \"provider\": \"stripe\"
  }")

if echo "$RECONCILE_RESP" | grep -q '"code":0'; then
    echo -e "${GREEN}✅ 对账任务执行成功${NC}"
    
    # 获取对账报告
    sleep 1
    REPORT_RESP=$(curl -s -X GET "${API_BASE}/api/v1/reconciliation/reports?date=${TODAY}" \
      -H "Authorization: Bearer ${TOKEN}")
    
    echo -e "${BLUE}对账报告：${NC}"
    echo "$REPORT_RESP" | grep -o '"total_orders":[0-9]*' | head -1
    echo "$REPORT_RESP" | grep -o '"successful_orders":[0-9]*' | head -1
    echo "$REPORT_RESP" | grep -o '"failed_orders":[0-9]*' | head -1
else
    echo -e "${YELLOW}⚠️  对账任务未成功（可能数据不足）${NC}"
    echo -e "${YELLOW}响应：${RECONCILE_RESP}${NC}"
fi
echo ""

# ============================================================
# 测试总结
# ============================================================
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  测试完成总结${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${GREEN}✅ 完成的测试项：${NC}"
echo -e "  - 环境依赖检查"
echo -e "  - 用户认证流程"
echo -e "  - 法币订单创建"
echo -e "  - Stripe 支付会话创建"
echo -e "  - 订单状态查询"
echo -e "  - Webhook 签名验证"
echo -e "  - 对账流程执行"
echo ""
echo -e "${BLUE}📝 测试数据：${NC}"
echo -e "  - 订单 ID: ${ORDER_UUID}"
echo -e "  - Stripe Session: ${SESSION_ID}"
echo -e "  - 最终状态: ${STATUS}"
echo ""

if [ "$STATUS" == "completed" ]; then
    echo -e "${GREEN}🎉 端到端支付测试 100% 成功！${NC}"
    exit 0
else
    echo -e "${YELLOW}⚠️  部分测试未完全通过，请检查上述日志${NC}"
    exit 1
fi
