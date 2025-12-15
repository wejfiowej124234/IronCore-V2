//! Onramper API客户端
//! 
//! Onramper是一个支付聚合器，整合了25+个fiat onramp服务商
//! 优势：
//! - 覆盖全球95%用户
//! - 自动选择最优通道
//! - 统一API接口
//! - 实时汇率和费率
//! 
//! API文档: https://docs.onramper.com/

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Onramper客户端配置
pub struct OnramperClient {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

/// 报价请求参数
#[derive(Debug, Serialize)]
pub struct QuoteParams {
    /// 法币币种（如：USD, CNY, EUR）
    pub fiat_currency: String,
    /// 加密货币币种（如：USDT, ETH, BTC）
    pub crypto_currency: String,
    /// 购买金额
    pub amount: Decimal,
    /// 支付方式（credit_card, debit_card, alipay, wechat_pay）
    pub payment_method: String,
    /// 用户国家代码（ISO 3166-1 alpha-2）
    pub country: String,
}

/// Onramper API响应 - 报价
#[derive(Debug, Deserialize)]
pub struct OnramperQuoteResponse {
    pub quotes: Vec<OnramperQuote>,
}

#[derive(Debug, Deserialize)]
pub struct OnramperQuote {
    /// 法币金额
    pub fiat_amount: String,
    /// 加密货币金额
    pub crypto_amount: String,
    /// 汇率
    pub exchange_rate: String,
    /// 总费用
    pub total_fee: String,
    /// 网络费
    pub network_fee: String,
    /// 服务费
    pub service_fee: String,
    /// 支付方式
    pub payment_method: String,
    /// 服务商名称
    pub provider_name: String,
    /// 预计到账时间（分钟）
    pub estimated_arrival_time_minutes: Option<i32>,
    /// 报价ID
    pub quote_id: String,
}

/// 订单创建请求
#[derive(Debug, Serialize)]
pub struct OrderParams {
    /// 报价ID
    pub quote_id: String,
    /// 钱包地址
    pub wallet_address: String,
    /// 用户邮箱
    pub email: Option<String>,
    /// 返回URL
    pub return_url: Option<String>,
    /// Webhook URL
    pub webhook_url: Option<String>,
}

/// Onramper订单响应
#[derive(Debug, Deserialize)]
pub struct OnramperOrderResponse {
    /// 订单ID
    pub order_id: String,
    /// 支付URL
    pub payment_url: String,
    /// 订单状态
    pub status: String,
}

impl OnramperClient {
    /// 创建新的Onramper客户端
    pub fn new(api_key: &str) -> Result<Self> {
        Ok(Self {
            api_key: api_key.to_string(),
            base_url: "https://api.onramper.com/v1".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .context("Failed to create HTTP client")?,
        })
    }

    /// 获取报价
    /// 
    /// # 示例
    /// ```rust
    /// let client = OnramperClient::new("your_api_key")?;
    /// let quote = client.get_quote(QuoteParams {
    ///     fiat_currency: "USD".to_string(),
    ///     crypto_currency: "USDT".to_string(),
    ///     amount: Decimal::from_str("100")?,
    ///     payment_method: "credit_card".to_string(),
    ///     country: "US".to_string(),
    /// }).await?;
    /// ```
    pub async fn get_quote(&self, params: QuoteParams) -> Result<OnramperQuote> {
        let url = format!("{}/transaction/buy/quotes", self.base_url);
        
        tracing::info!(
            "🌐 调用Onramper API获取报价: {} {} → {} {}",
            params.amount, params.fiat_currency,
            params.crypto_currency, params.country
        );
        
        let mut query_params = HashMap::new();
        query_params.insert("fiat", params.fiat_currency.clone());
        query_params.insert("crypto", params.crypto_currency.clone());
        query_params.insert("amount", params.amount.to_string());
        query_params.insert("paymentMethod", params.payment_method.clone());
        query_params.insert("country", params.country.clone());
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .query(&query_params)
            .send()
            .await
            .context("Failed to send request to Onramper")?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("❌ Onramper API错误 ({}): {}", status, error_text);
            return Err(anyhow::anyhow!("Onramper API返回错误: {}", status));
        }
        
        let api_response = response
            .json::<OnramperQuoteResponse>()
            .await
            .context("Failed to parse Onramper response")?;
        
        // 选择最优报价（费用最低）
        let best_quote = api_response.quotes
            .into_iter()
            .min_by_key(|q| q.total_fee.parse::<f64>().unwrap_or(f64::MAX) as i64)
            .ok_or_else(|| anyhow::anyhow!("No quotes available from Onramper"))?;
        
        tracing::info!(
            "✅ Onramper最优报价: {} {} → {} {}, 费用: {} (服务商: {})",
            best_quote.fiat_amount, params.fiat_currency,
            best_quote.crypto_amount, params.crypto_currency,
            best_quote.total_fee,
            best_quote.provider_name
        );
        
        Ok(best_quote)
    }

    /// 创建订单
    /// 
    /// # 示例
    /// ```rust
    /// let order_response = client.create_order(OrderParams {
    ///     quote_id: quote.quote_id.clone(),
    ///     wallet_address: "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string(),
    ///     email: Some("user@example.com".to_string()),
    ///     return_url: Some("https://yourapp.com/success".to_string()),
    ///     webhook_url: Some("https://yourapi.com/webhook/onramper".to_string()),
    /// }).await?;
    /// ```
    pub async fn create_order(&self, params: OrderParams) -> Result<OnramperOrderResponse> {
        let url = format!("{}/transaction/buy", self.base_url);
        
        tracing::info!(
            "🌐 调用Onramper API创建订单: quote_id={}, wallet={}",
            params.quote_id, params.wallet_address
        );
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&params)
            .send()
            .await
            .context("Failed to create order with Onramper")?;
        
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            tracing::error!("❌ Onramper订单创建失败 ({}): {}", status, error_text);
            return Err(anyhow::anyhow!("Onramper订单创建失败: {}", status));
        }
        
        let order_response = response
            .json::<OnramperOrderResponse>()
            .await
            .context("Failed to parse Onramper order response")?;
        
        tracing::info!(
            "✅ Onramper订单创建成功: order_id={}, payment_url={}",
            order_response.order_id, order_response.payment_url
        );
        
        Ok(order_response)
    }

    /// 验证Webhook签名
    pub fn verify_webhook_signature(&self, payload: &str, signature: &str, secret: &str) -> bool {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        type HmacSha256 = Hmac<Sha256>;
        
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        
        let expected_signature = hex::encode(mac.finalize().into_bytes());
        
        signature == expected_signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    #[ignore] // 需要真实API key才能运行
    async fn test_onramper_quote() {
        let api_key = std::env::var("ONRAMPER_API_KEY")
            .expect("ONRAMPER_API_KEY环境变量未设置");
        
        let client = OnramperClient::new(&api_key).unwrap();
        
        let quote = client.get_quote(QuoteParams {
            fiat_currency: "USD".to_string(),
            crypto_currency: "USDT".to_string(),
            amount: Decimal::from_str("100").unwrap(),
            payment_method: "credit_card".to_string(),
            country: "US".to_string(),
        }).await;
        
        assert!(quote.is_ok(), "Onramper报价请求失败: {:?}", quote.err());
        
        let quote = quote.unwrap();
        assert!(!quote.crypto_amount.is_empty());
        assert!(!quote.quote_id.is_empty());
    }
}
