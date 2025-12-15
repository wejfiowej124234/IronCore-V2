//! TransFi API客户端
//!
//! TransFi是专注于中国市场的支付服务商
//! 优势：
//! - 支持支付宝/微信支付
//! - 中国用户体验优化
//! - 合规性强
//! - 费率1.5%-3.5%
//!
//! API文档: https://docs.transfi.com/

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// TransFi客户端配置
pub struct TransFiClient {
    api_key: String,
    secret: String,
    base_url: String,
    client: reqwest::Client,
}

/// TransFi报价请求
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransFiQuoteRequest {
    /// 源币种（法币）
    pub source_currency: String,
    /// 目标币种（加密货币）
    pub target_currency: String,
    /// 金额
    pub amount: String,
    /// 支付方式（alipay, wechat_pay, bank_transfer）
    pub payment_method: String,
    /// 用户国家
    pub country_code: String,
}

/// TransFi报价响应
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransFiQuoteResponse {
    /// 报价ID
    pub quote_id: String,
    /// 源金额
    pub source_amount: String,
    /// 目标金额
    pub target_amount: String,
    /// 汇率
    pub exchange_rate: String,
    /// 手续费
    pub fee: String,
    /// 网络费
    pub network_fee: String,
    /// 报价有效期（秒）
    pub valid_for_seconds: i64,
}

/// TransFi订单创建请求
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransFiOrderRequest {
    /// 报价ID
    pub quote_id: String,
    /// 钱包地址
    pub wallet_address: String,
    /// 用户信息
    pub user_info: TransFiUserInfo,
    /// 回调URL
    pub callback_url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransFiUserInfo {
    /// 用户ID
    pub user_id: String,
    /// 邮箱
    pub email: Option<String>,
    /// 手机号
    pub phone: Option<String>,
    /// 姓名
    pub name: Option<String>,
}

/// TransFi订单响应
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransFiOrderResponse {
    /// 订单ID
    pub order_id: String,
    /// 支付链接
    pub payment_url: String,
    /// 订单状态
    pub status: String,
    /// 二维码（支付宝/微信）
    pub qr_code: Option<String>,
}

impl TransFiClient {
    /// 创建新的TransFi客户端
    pub fn new(api_key: &str, secret: &str) -> Result<Self> {
        Ok(Self {
            api_key: api_key.to_string(),
            secret: secret.to_string(),
            base_url: "https://api.transfi.com/v1".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .context("Failed to create HTTP client")?,
        })
    }

    /// 获取报价
    pub async fn get_quote(&self, request: TransFiQuoteRequest) -> Result<TransFiQuoteResponse> {
        let url = format!("{}/quotes", self.base_url);

        tracing::info!(
            "🌐 调用TransFi API获取报价: {} {} → {}",
            request.amount,
            request.source_currency,
            request.target_currency
        );

        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.generate_signature(&request, timestamp)?;

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .header("X-Timestamp", timestamp.to_string())
            .header("X-Signature", signature)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to TransFi")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("❌ TransFi API错误 ({}): {}", status, error_text);
            return Err(anyhow::anyhow!("TransFi API返回错误: {}", status));
        }

        let quote = response
            .json::<TransFiQuoteResponse>()
            .await
            .context("Failed to parse TransFi response")?;

        tracing::info!(
            "✅ TransFi报价成功: {} {} → {} {}, 费用: {}",
            quote.source_amount,
            request.source_currency,
            quote.target_amount,
            request.target_currency,
            quote.fee
        );

        Ok(quote)
    }

    /// 创建订单
    pub async fn create_order(&self, request: TransFiOrderRequest) -> Result<TransFiOrderResponse> {
        let url = format!("{}/orders", self.base_url);

        tracing::info!(
            "🌐 调用TransFi API创建订单: quote_id={}, wallet={}",
            request.quote_id,
            request.wallet_address
        );

        let timestamp = chrono::Utc::now().timestamp();
        let signature = self.generate_signature(&request, timestamp)?;

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .header("X-Timestamp", timestamp.to_string())
            .header("X-Signature", signature)
            .json(&request)
            .send()
            .await
            .context("Failed to create order with TransFi")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("❌ TransFi订单创建失败 ({}): {}", status, error_text);
            return Err(anyhow::anyhow!("TransFi订单创建失败: {}", status));
        }

        let order = response
            .json::<TransFiOrderResponse>()
            .await
            .context("Failed to parse TransFi order response")?;

        tracing::info!(
            "✅ TransFi订单创建成功: order_id={}, payment_url={}",
            order.order_id,
            order.payment_url
        );

        Ok(order)
    }

    /// 生成API签名
    fn generate_signature<T: Serialize>(&self, payload: &T, timestamp: i64) -> Result<String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;

        let payload_str = serde_json::to_string(payload).context("Failed to serialize payload")?;

        let message = format!("{}|{}|{}", timestamp, self.api_key, payload_str);

        let mut mac =
            HmacSha256::new_from_slice(self.secret.as_bytes()).context("Invalid secret key")?;
        mac.update(message.as_bytes());

        Ok(hex::encode(mac.finalize().into_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要真实API key才能运行
    async fn test_transfi_quote() {
        let api_key = std::env::var("TRANSFI_API_KEY").expect("TRANSFI_API_KEY环境变量未设置");
        let secret = std::env::var("TRANSFI_SECRET").expect("TRANSFI_SECRET环境变量未设置");

        let client = TransFiClient::new(&api_key, &secret).unwrap();

        let quote = client
            .get_quote(TransFiQuoteRequest {
                source_currency: "CNY".to_string(),
                target_currency: "USDT".to_string(),
                amount: "1000".to_string(),
                payment_method: "alipay".to_string(),
                country_code: "CN".to_string(),
            })
            .await;

        assert!(quote.is_ok(), "TransFi报价请求失败: {:?}", quote.err());
    }
}
