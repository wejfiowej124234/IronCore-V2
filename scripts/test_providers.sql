-- 测试providers表查询
SELECT 
    name, 
    display_name, 
    is_enabled, 
    priority,
    fee_min_percent,
    fee_max_percent,
    array_length(supported_countries, 1) as countries_count,
    array_length(supported_payment_methods, 1) as payment_methods_count
FROM fiat.providers
WHERE is_enabled = TRUE
ORDER BY priority ASC;

-- 显示第一个provider的完整信息
SELECT * FROM fiat.providers WHERE name = 'moonpay';
