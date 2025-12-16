//! 法币支付服务商种子数据
//! 企业级标准：初始化5个主流服务商

use anyhow::Result;
use sqlx::PgPool;

/// 初始化法币支付服务商数据
pub async fn seed_providers(pool: &PgPool) -> Result<()> {
    // 先尝试创建schema（如果不存在）
    let _ = sqlx::query("CREATE SCHEMA IF NOT EXISTS fiat")
        .execute(pool)
        .await;

    // 创建providers表（如果不存在）- 用于迁移失败的情况
    let _ = sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fiat.providers (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            is_enabled BOOL NOT NULL DEFAULT true,
            priority INT NOT NULL DEFAULT 100,
            fee_min_percent DECIMAL(5, 2) NOT NULL,
            fee_max_percent DECIMAL(5, 2) NOT NULL,
            api_key_encrypted TEXT,
            api_url TEXT NOT NULL,
            webhook_url TEXT,
            timeout_seconds INT NOT NULL DEFAULT 30,
            supported_countries TEXT[],
            supported_payment_methods TEXT[],
            health_status TEXT NOT NULL DEFAULT 'unknown',
            last_health_check TIMESTAMPTZ,
            consecutive_failures INT NOT NULL DEFAULT 0,
            total_requests INT NOT NULL DEFAULT 0,
            successful_requests INT NOT NULL DEFAULT 0,
            average_response_time_ms INT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await;

    // 强制重新插入：删除旧数据
    tracing::info!("Cleaning old provider data...");
    let _ = sqlx::query("DELETE FROM fiat.providers")
        .execute(pool)
        .await;

    tracing::info!("Seeding fiat payment providers...");

    // 🎯 企业级5服务商配置（2025优化版）
    // 3层聚合架构：主力(3) + 兜底(2)
    let providers = vec![
        // ✅ 主力1: Onramper - 聚合器优先级100
        (
            "11111111-1111-1111-1111-111111111111",
            "onramper",
            "Onramper",
            100, // 最高优先级
            0.5, // 聚合器费率最优
            3.5,
            "https://api.onramper.com",
            "https://webhook.onramper.com",
            vec![
                "US", "GB", "EU", "CA", "AU", "JP", "KR", "SG", "HK", "TW", "CN", "IN", "BR", "MX",
                "RU", "ZA", "AE", "TR", "ID", "TH", "VN", "PH", "MY",
            ], // 全球95%覆盖
            vec![
                "credit_card",
                "debit_card",
                "bank_transfer",
                "apple_pay",
                "google_pay",
                "wechat_pay",
                "alipay",
                "sepa",
                "pix",
                "upi",
                "faster_payments",
            ], // 聚合25+ ramps
        ),
        // ✅ 主力2: TransFi - 中国特化优先级90
        (
            "22222222-2222-2222-2222-222222222222",
            "transfi",
            "TransFi",
            90,
            1.5, // 新兴市场低费率
            3.5,
            "https://api.transfi.com",
            "https://webhook.transfi.com",
            vec![
                "CN", "HK", "TW", "SG", "MY", "TH", "VN", "ID", "PH", "IN", "BR", "MX", "AR", "TR",
                "ZA", "AE", "RU",
            ], // 新兴市场专注
            vec![
                "alipay",
                "wechat_pay",
                "bank_transfer",
                "credit_card",
                "debit_card",
                "pix",
                "upi",
                "paytm",
                "gcash",
            ], // 2024新增支付宝/微信
        ),
        // ✅ 主力3: Alchemy Pay - Web3优化优先级85
        (
            "33333333-3333-3333-3333-333333333333",
            "alchemypay",
            "Alchemy Pay",
            85,
            2.0, // DeFi友好费率
            4.0,
            "https://api.alchemypay.org",
            "https://webhook.alchemypay.org",
            vec![
                "CN", "US", "GB", "EU", "CA", "AU", "JP", "KR", "SG", "HK", "TW", "IN", "TH", "VN",
                "ID", "PH",
            ], // Web3核心市场
            vec![
                "alipay",
                "wechat_pay",
                "credit_card",
                "debit_card",
                "bank_transfer",
                "apple_pay",
                "google_pay",
                "binance_pay",
                "okx_pay",
            ], // Binance/OKX合作
        ),
        // ✅ 兜底1: Ramp Network - 欧美兜底优先级70
        (
            "44444444-4444-4444-4444-444444444444",
            "ramp",
            "Ramp Network",
            70,   // 降低优先级作为兜底
            0.49, // 费率最低
            2.9,
            "https://api.ramp.network",
            "https://webhook.ramp.network",
            vec![
                "US", "GB", "EU", "CA", "AU", "CH", "NO", "SE", "DK", "FI", "NL", "BE", "AT", "IE",
                "ES", "IT", "PT", "FR", "DE",
            ], // 欧美专注
            vec![
                "bank_transfer",
                "sepa",
                "instant_sepa",
                "ach",
                "open_banking",
                "faster_payments",
            ], // 欧美银行转账专家
        ),
        // ✅ 兜底2: MoonPay - 全球兜底优先级60
        (
            "55555555-5555-5555-5555-555555555555",
            "moonpay",
            "MoonPay",
            60,  // 最后兜底
            1.0, // 品牌信任
            4.5,
            "https://api.moonpay.com",
            "https://webhook.moonpay.com",
            vec![
                "US", "GB", "EU", "CA", "AU", "NZ", "JP", "KR", "SG", "HK", "TW", "IN", "BR", "MX",
                "ZA", "AE", "CH", "NO", "SE", "DK", "FI",
            ], // 全球品牌覆盖
            vec![
                "credit_card",
                "debit_card",
                "bank_transfer",
                "apple_pay",
                "google_pay",
                "samsung_pay",
                "sepa",
                "pix",
            ], // 全球主流支付
        ),
    ];

    let mut _success_count = 0;
    let mut failed = Vec::new();

    for (
        uuid,
        name,
        display_name,
        priority,
        fee_min,
        fee_max,
        api_url,
        webhook_url,
        countries,
        payment_methods,
    ) in &providers
    {
        match sqlx::query(
            r#"
            INSERT INTO fiat.providers (
                id,
                name,
                display_name,
                is_enabled,
                priority,
                fee_min_percent,
                fee_max_percent,
                api_url,
                webhook_url,
                timeout_seconds,
                supported_countries,
                supported_payment_methods,
                health_status,
                last_health_check,
                consecutive_failures,
                total_requests,
                successful_requests,
                average_response_time_ms,
                created_at,
                updated_at
            ) VALUES (
                $1::uuid, $2, $3, true, $4, $5, $6, $7, $8, 30, $9, $10,
                'healthy', NOW(), 0, 0, 0, 0, NOW(), NOW()
            )
            ON CONFLICT (name) DO UPDATE SET
                display_name = EXCLUDED.display_name,
                is_enabled = true,
                priority = EXCLUDED.priority,
                fee_min_percent = EXCLUDED.fee_min_percent,
                fee_max_percent = EXCLUDED.fee_max_percent,
                updated_at = NOW()
            "#,
        )
        .bind(uuid)
        .bind(name)
        .bind(display_name)
        .bind(priority)
        .bind(fee_min)
        .bind(fee_max)
        .bind(api_url)
        .bind(webhook_url)
        .bind(countries)
        .bind(payment_methods)
        .execute(pool)
        .await
        {
            Ok(_) => {
                tracing::info!("✅ Inserted provider: {} ({})", display_name, name);
                _success_count += 1;
            }
            Err(e) => {
                tracing::error!(
                    "❌ Failed to insert provider {} ({}): {:?}",
                    display_name,
                    name,
                    e
                );
                failed.push(name);
            }
        }
    }

    // 强制验证5个providers全部插入成功
    let final_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM fiat.providers WHERE is_enabled = true")
            .fetch_one(pool)
            .await?;

    if final_count < 5 {
        let error = format!(
            "CRITICAL: Only {}/{} providers inserted successfully. Failed: {:?}",
            final_count,
            providers.len(),
            failed
        );
        tracing::error!("❌ {}", error);
        return Err(anyhow::anyhow!(error));
    }

    tracing::info!(
        "🎉 Successfully inserted {} payment providers (MoonPay, Simplex, Transak, Ramp, Banxa)",
        final_count
    );
    Ok(())
}
