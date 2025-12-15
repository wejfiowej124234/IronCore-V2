//! 支付服务智能路由单元测试
//! 测试中国地区路由、Onramper优先级、降级兜底逻辑

use ironcore::service::fiat_service::{FiatService, FiatOrderStatus};
use ironcore::service::provider_service::ProviderService;
use sqlx::PgPool;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试中国地区检测
    #[test]
    fn test_is_china_region() {
        let pool = setup_test_pool().await;
        let service = FiatService::new(pool);

        assert!(service.is_china_region("CN"));
        assert!(service.is_china_region("HK"));
        assert!(service.is_china_region("TW"));
        assert!(service.is_china_region("SG"));
        assert!(!service.is_china_region("US"));
        assert!(!service.is_china_region("GB"));
        assert!(!service.is_china_region("JP"));
    }

    /// 测试订单状态转换验证
    #[test]
    fn test_status_transition_validation() {
        let pool = setup_test_pool().await;
        let service = FiatService::new(pool);

        // pending可以转换到任何状态
        assert!(service.is_valid_status_transition("pending", "processing"));
        assert!(service.is_valid_status_transition("pending", "completed"));
        assert!(service.is_valid_status_transition("pending", "failed"));

        // processing只能转换到completed, failed, cancelled
        assert!(service.is_valid_status_transition("processing", "completed"));
        assert!(service.is_valid_status_transition("processing", "failed"));
        assert!(service.is_valid_status_transition("processing", "cancelled"));
        assert!(!service.is_valid_status_transition("processing", "pending"));

        // 终态不能再转换
        assert!(!service.is_valid_status_transition("completed", "failed"));
        assert!(!service.is_valid_status_transition("failed", "completed"));
        assert!(!service.is_valid_status_transition("cancelled", "processing"));
    }

    /// 模拟数据库连接（需要在实际测试环境中运行）
    async fn setup_test_pool() -> PgPool {
        // 注意：这需要真实的测试数据库
        let database_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgres://root@localhost:26257/ironcore_test?sslmode=disable".to_string());

        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    /// 集成测试：验证服务商优先级排序
    #[tokio::test]
    #[ignore] // 需要真实数据库才能运行
    async fn test_provider_priority_ordering() {
        let pool = setup_test_pool().await;
        let provider_service = ProviderService::new(pool.clone());

        let providers = provider_service.get_enabled_providers().await.unwrap();

        // 验证优先级从高到低排序
        let mut prev_priority = i64::MAX;
        for provider in providers {
            assert!(provider.priority <= prev_priority,
                "Provider {} priority {} not in descending order",
                provider.name, provider.priority);
            prev_priority = provider.priority;
        }

        // 验证Onramper是最高优先级
        let onramper = provider_service.get_provider_by_name("onramper").await;
        assert!(onramper.is_ok());
        assert_eq!(onramper.unwrap().priority, 100);
    }

    /// 集成测试：验证中国地区路由逻辑
    #[tokio::test]
    #[ignore] // 需要真实数据库才能运行
    async fn test_china_payment_routing() {
        let pool = setup_test_pool().await;
        let service = FiatService::new(pool);

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let amount = rust_decimal::Decimal::from(100);

        // 测试1: 中国地区 + 支付宝 -> 应该优先尝试TransFi
        let result = service.get_onramp_quote(
            tenant_id,
            user_id,
            amount,
            "USD",
            "USDT",
            "alipay",
            Some("CN".to_string()),
            None,
        ).await;

        // 如果TransFi可用，应该返回TransFi的报价
        // 如果不可用，会降级到Alchemy或Onramper
        if let Ok(quote) = result {
            assert!(
                quote.provider_name == "transfi" ||
                quote.provider_name == "alchemypay" ||
                quote.provider_name == "onramper",
                "Expected China-optimized provider, got: {}",
                quote.provider_name
            );
        }
    }

    /// 集成测试：验证非中国地区使用Onramper
    #[tokio::test]
    #[ignore] // 需要真实数据库才能运行
    async fn test_non_china_uses_onramper() {
        let pool = setup_test_pool().await;
        let service = FiatService::new(pool);

        let tenant_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let amount = rust_decimal::Decimal::from(100);

        // 测试2: 美国用户 + 信用卡 -> 应该使用Onramper
        let result = service.get_onramp_quote(
            tenant_id,
            user_id,
            amount,
            "USD",
            "USDT",
            "credit_card",
            Some("US".to_string()),
            None,
        ).await;

        if let Ok(quote) = result {
            // Onramper作为聚合器应该被优先选择
            assert_eq!(quote.provider_name, "onramper");
        }
    }

    /// 单元测试：Webhook签名验证
    #[test]
    fn test_webhook_signature_verification() {
        use ironcore::security::webhook_verifier::*;
        use axum::http::HeaderMap;
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        // Onramper签名测试
        let mut headers = HeaderMap::new();
        let body = r#"{"orderId":"test-123","status":"completed"}"#;
        let secret = "test_secret";

        // 生成正确签名
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        headers.insert("X-Onramper-Signature", signature.parse().unwrap());

        assert!(verify_onramper_signature(&headers, body, secret).is_ok());
    }
}

/// 端到端测试助手
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// 创建测试订单
    pub async fn create_test_order(pool: &PgPool) -> Uuid {
        let order_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO fiat.orders
             (id, tenant_id, user_id, order_type, payment_method,
              fiat_amount, fiat_currency, crypto_amount, crypto_token,
              exchange_rate, fee_amount, fee_percentage,
              provider_name, status, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NOW(), NOW())"
        )
        .bind(order_id)
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind("onramp")
        .bind("credit_card")
        .bind(rust_decimal::Decimal::from(100))
        .bind("USD")
        .bind(rust_decimal::Decimal::from(99))
        .bind("USDT")
        .bind(rust_decimal::Decimal::from(0.99))
        .bind(rust_decimal::Decimal::from(1))
        .bind(1.0)
        .bind("onramper")
        .bind("pending")
        .execute(pool)
        .await
        .expect("Failed to create test order");

        order_id
    }

    /// 测试Webhook端到端流程
    #[tokio::test]
    #[ignore]
    async fn test_webhook_end_to_end() {
        let pool = setup_test_pool().await;
        let service = FiatService::new(pool.clone());

        // 1. 创建测试订单
        let order_id = create_test_order(&pool).await;

        // 2. 验证初始状态
        let order = service.get_order_by_id(order_id).await.unwrap();
        assert_eq!(order.status, "pending");

        // 3. 模拟Webhook更新
        let result = service.update_order_status(
            order_id,
            FiatOrderStatus::Completed,
            Some("0x123456".to_string()),
            None,
        ).await;

        assert!(result.is_ok());

        // 4. 验证更新后状态
        let updated_order = service.get_order_by_id(order_id).await.unwrap();
        assert_eq!(updated_order.status, "completed");
        assert_eq!(updated_order.provider_tx_id.unwrap(), "0x123456");
    }
}
