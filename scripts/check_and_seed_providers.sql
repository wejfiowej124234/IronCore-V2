-- 检查和补全法币支付服务商数据
-- 用于修复"没有可用的支付服务商"错误

-- 检查 fiat.providers 表是否有数据
DO $$
DECLARE
    provider_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO provider_count FROM fiat.providers;
    
    IF provider_count = 0 THEN
        RAISE NOTICE '警告：fiat.providers 表为空，正在初始化服务商数据...';
        
        -- 1. Moonpay (优先级最高，全球覆盖)
        INSERT INTO fiat.providers (
            id, name, display_name, is_enabled, priority,
            fee_min_percent, fee_max_percent, api_url, webhook_url, timeout_seconds,
            supported_countries, supported_payment_methods,
            health_status, last_health_check, consecutive_failures,
            total_requests, successful_requests, average_response_time_ms,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), 'moonpay', 'MoonPay', true, 1,
            1.0, 4.5, 'https://api.moonpay.com', 'https://webhook.moonpay.com', 30,
            ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'JP', 'KR', 'SG', 'HK', 'TW', 'CN'],
            ARRAY['credit_card', 'debit_card', 'bank_transfer', 'apple_pay', 'google_pay', 'samsung_pay'],
            'healthy', NOW(), 0, 0, 0, 0, NOW(), NOW()
        );

        -- 2. Simplex
        INSERT INTO fiat.providers (
            id, name, display_name, is_enabled, priority,
            fee_min_percent, fee_max_percent, api_url, webhook_url, timeout_seconds,
            supported_countries, supported_payment_methods,
            health_status, last_health_check, consecutive_failures,
            total_requests, successful_requests, average_response_time_ms,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), 'simplex', 'Simplex', true, 2,
            3.5, 5.0, 'https://api.simplex.com', 'https://webhook.simplex.com', 30,
            ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'JP', 'KR', 'SG', 'HK', 'BR', 'MX'],
            ARRAY['credit_card', 'debit_card'],
            'healthy', NOW(), 0, 0, 0, 0, NOW(), NOW()
        );

        -- 3. Transak
        INSERT INTO fiat.providers (
            id, name, display_name, is_enabled, priority,
            fee_min_percent, fee_max_percent, api_url, webhook_url, timeout_seconds,
            supported_countries, supported_payment_methods,
            health_status, last_health_check, consecutive_failures,
            total_requests, successful_requests, average_response_time_ms,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), 'transak', 'Transak', true, 3,
            0.99, 5.5, 'https://api.transak.com', 'https://webhook.transak.com', 30,
            ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'IN', 'BR', 'MX', 'AR', 'CO', 'CL', 'PE'],
            ARRAY['credit_card', 'debit_card', 'bank_transfer', 'apple_pay', 'google_pay', 'sepa', 'ach', 'pix'],
            'healthy', NOW(), 0, 0, 0, 0, NOW(), NOW()
        );

        -- 4. Ramp
        INSERT INTO fiat.providers (
            id, name, display_name, is_enabled, priority,
            fee_min_percent, fee_max_percent, api_url, webhook_url, timeout_seconds,
            supported_countries, supported_payment_methods,
            health_status, last_health_check, consecutive_failures,
            total_requests, successful_requests, average_response_time_ms,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), 'ramp', 'Ramp Network', true, 4,
            0.49, 2.9, 'https://api.ramp.network', 'https://webhook.ramp.network', 30,
            ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'CH', 'NO', 'SE', 'DK', 'FI'],
            ARRAY['bank_transfer', 'sepa', 'instant_sepa', 'ach', 'open_banking'],
            'healthy', NOW(), 0, 0, 0, 0, NOW(), NOW()
        );

        -- 5. Banxa
        INSERT INTO fiat.providers (
            id, name, display_name, is_enabled, priority,
            fee_min_percent, fee_max_percent, api_url, webhook_url, timeout_seconds,
            supported_countries, supported_payment_methods,
            health_status, last_health_check, consecutive_failures,
            total_requests, successful_requests, average_response_time_ms,
            created_at, updated_at
        ) VALUES (
            gen_random_uuid(), 'banxa', 'Banxa', true, 5,
            1.5, 4.0, 'https://api.banxa.com', 'https://webhook.banxa.com', 30,
            ARRAY['US', 'GB', 'EU', 'AU', 'NZ', 'JP', 'SG', 'HK', 'TW', 'KR', 'MY', 'TH', 'VN', 'PH', 'ID'],
            ARRAY['credit_card', 'debit_card', 'bank_transfer', 'poli', 'payid', 'osko', 'fps'],
            'healthy', NOW(), 0, 0, 0, 0, NOW(), NOW()
        );
        
        RAISE NOTICE '✅ 成功初始化 5 个法币支付服务商';
    ELSE
        RAISE NOTICE '✅ fiat.providers 表已有 % 条记录，无需初始化', provider_count;
    END IF;
END $$;

-- 显示当前服务商状态
SELECT 
    name,
    display_name,
    is_enabled,
    priority,
    fee_min_percent || '-' || fee_max_percent || '%' as fee_range,
    array_length(supported_countries, 1) as country_count,
    array_length(supported_payment_methods, 1) as payment_method_count,
    health_status
FROM fiat.providers
ORDER BY priority ASC;
