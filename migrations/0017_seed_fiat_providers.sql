-- 初始化法币支付服务商数据
-- 企业级标准：5个主流服务商配置

-- NOTE: 生产环境可能已存在配置（例如 api_key_encrypted、自定义优先级等）。
-- 该迁移仅在缺失时插入默认服务商，不做全量清空。

-- 1. Moonpay (优先级最高，全球覆盖)
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
)
SELECT
    gen_random_uuid(),
    'moonpay',
    'MoonPay',
    true,
    1,
    1.0,
    4.5,
    'https://api.moonpay.com',
    'https://webhook.moonpay.com',
    30,
    ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'JP', 'KR', 'SG', 'HK', 'TW', 'CN'],
    ARRAY['credit_card', 'debit_card', 'bank_transfer', 'apple_pay', 'google_pay', 'samsung_pay'],
    'healthy',
    NOW(),
    0,
    0,
    0,
    0,
    NOW(),
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM fiat.providers WHERE name = 'moonpay');

-- 2. Simplex (快速支付，信用卡专家)
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
)
SELECT
    gen_random_uuid(),
    'simplex',
    'Simplex',
    true,
    2,
    3.5,
    5.0,
    'https://api.simplex.com',
    'https://webhook.simplex.com',
    30,
    ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'JP', 'KR', 'SG', 'HK', 'BR', 'MX'],
    ARRAY['credit_card', 'debit_card'],
    'healthy',
    NOW(),
    0,
    0,
    0,
    0,
    NOW(),
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM fiat.providers WHERE name = 'simplex');

-- 3. Transak (支持更多支付方式)
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
)
SELECT
    gen_random_uuid(),
    'transak',
    'Transak',
    true,
    3,
    0.99,
    5.5,
    'https://api.transak.com',
    'https://webhook.transak.com',
    30,
    ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'IN', 'BR', 'MX', 'AR', 'CO', 'CL', 'PE'],
    ARRAY['credit_card', 'debit_card', 'bank_transfer', 'apple_pay', 'google_pay', 'sepa', 'ach', 'pix'],
    'healthy',
    NOW(),
    0,
    0,
    0,
    0,
    NOW(),
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM fiat.providers WHERE name = 'transak');

-- 4. Ramp (银行转账专家，费用最低)
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
)
SELECT
    gen_random_uuid(),
    'ramp',
    'Ramp Network',
    true,
    4,
    0.49,
    2.9,
    'https://api.ramp.network',
    'https://webhook.ramp.network',
    30,
    ARRAY['US', 'GB', 'EU', 'CA', 'AU', 'CH', 'NO', 'SE', 'DK', 'FI'],
    ARRAY['bank_transfer', 'sepa', 'instant_sepa', 'ach', 'open_banking'],
    'healthy',
    NOW(),
    0,
    0,
    0,
    0,
    NOW(),
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM fiat.providers WHERE name = 'ramp');

-- 5. Banxa (澳洲本土，亚太地区强)
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
)
SELECT
    gen_random_uuid(),
    'banxa',
    'Banxa',
    true,
    5,
    1.5,
    4.0,
    'https://api.banxa.com',
    'https://webhook.banxa.com',
    30,
    ARRAY['US', 'GB', 'EU', 'AU', 'NZ', 'JP', 'SG', 'HK', 'TW', 'KR', 'MY', 'TH', 'VN', 'PH', 'ID'],
    ARRAY['credit_card', 'debit_card', 'bank_transfer', 'poli', 'payid', 'osko', 'fps'],
    'healthy',
    NOW(),
    0,
    0,
    0,
    0,
    NOW(),
    NOW()
WHERE NOT EXISTS (SELECT 1 FROM fiat.providers WHERE name = 'banxa');

-- 验证插入结果
SELECT 
    name,
    display_name,
    priority,
    fee_min_percent || '-' || fee_max_percent || '%' as fee_range,
    array_length(supported_countries, 1) as country_count,
    array_length(supported_payment_methods, 1) as payment_method_count,
    health_status,
    is_enabled
FROM fiat.providers
ORDER BY priority ASC;
