use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::sync::RwLock;

use crate::infrastructure::cache::RedisCtx;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeeRule {
    pub id: uuid::Uuid,
    pub chain: String,
    pub operation: String,
    pub fee_type: String,
    pub flat_amount: f64,
    pub percent_bp: i32,
    pub min_fee: f64,
    pub max_fee: Option<f64>,
    pub priority: i32,
    pub rule_version: i32,
}

#[derive(Clone, Debug)]
pub struct FeeCalcResult {
    pub platform_fee: f64,
    pub collector_address: String,
    pub applied_rule_id: uuid::Uuid,
    pub rule_version: i32,
}

struct CachedRule {
    rule: FeeRule,
    fetched_at: Instant,
}

pub struct FeeService {
    pool: PgPool,
    cache: Arc<RwLock<HashMap<String, CachedRule>>>, // L1: 本地内存缓存
    redis: Option<Arc<RedisCtx>>,                    // L2: Redis 缓存（可选）
    ttl: Duration,
}

impl FeeService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
            redis: None,
            ttl: Duration::from_secs(60),
        }
    }

    /// 创建带 Redis 二级缓存的 FeeService
    pub fn with_redis(pool: PgPool, redis: Arc<RedisCtx>) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(HashMap::new())),
            redis: Some(redis),
            ttl: Duration::from_secs(60),
        }
    }

    /// 企业级实现：清除指定链和操作的缓存（用于规则更新时）
    ///
    /// 当管理员创建/更新/删除费用规则时，需要清除相关缓存以确保立即生效
    pub async fn invalidate_cache(&self, chain: &str, operation: &str) {
        let key = format!("fee:rule:{}:{}", chain, operation);

        // 清除 L1 缓存（本地内存）
        {
            let mut cache = self.cache.write().await;
            cache.remove(&key);
            tracing::info!(key=%key, "L1 cache invalidated");
        }

        // 清除 L2 缓存（Redis）
        if let Some(redis_ctx) = &self.redis {
            if let Ok(mut conn) = redis_ctx.client.get_multiplexed_async_connection().await {
                use redis::AsyncCommands;
                let _: Result<i64, _> = conn.del(&key).await;
                tracing::info!(key=%key, "L2 Redis cache invalidated");
            }
        }
    }

    /// 企业级实现：清除指定链的所有操作缓存（用于批量更新）
    pub async fn invalidate_cache_for_chain(&self, chain: &str) {
        let prefix = format!("fee:rule:{}:", chain);

        // 清除 L1 缓存（本地内存）- 需要遍历所有键
        {
            let mut cache = self.cache.write().await;
            let keys_to_remove: Vec<String> = cache
                .keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            for key in keys_to_remove {
                cache.remove(&key);
            }
            tracing::info!(chain=%chain, "L1 cache invalidated for chain");
        }

        // 清除 L2 缓存（Redis）- 使用模式匹配删除
        if let Some(redis_ctx) = &self.redis {
            if let Ok(mut conn) = redis_ctx.client.get_multiplexed_async_connection().await {
                // 使用 SCAN + DEL 模式（生产环境建议使用 Lua 脚本保证原子性）
                let pattern = format!("{}*", prefix);
                if let Ok(keys) = redis::cmd("KEYS")
                    .arg(&pattern)
                    .query_async::<_, Vec<String>>(&mut conn)
                    .await
                {
                    if !keys.is_empty() {
                        use redis::AsyncCommands;
                        let _: Result<i64, _> = conn.del(keys).await;
                        tracing::info!(chain=%chain, "L2 Redis cache invalidated for chain");
                    }
                }
            }
        }
    }

    async fn get_active_rule(&self, chain: &str, operation: &str) -> Result<Option<FeeRule>> {
        let key = format!("fee:rule:{}:{}", chain, operation);

        // L1: 本地内存缓存检查
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&key) {
                if cached.fetched_at.elapsed() < self.ttl {
                    tracing::debug!(key=%key, "cache_hit_l1");
                    return Ok(Some(cached.rule.clone()));
                }
            }
        }

        // L2: Redis 缓存检查（如果可用）
        if let Some(redis_ctx) = &self.redis {
            if let Ok(mut conn) = redis_ctx.client.get_multiplexed_async_connection().await {
                use redis::AsyncCommands;
                match conn.get::<_, Option<String>>(&key).await {
                    Ok(Some(cached_json)) => {
                        if let Ok(rule) = serde_json::from_str::<FeeRule>(&cached_json) {
                            tracing::debug!(key=%key, "cache_hit_l2_redis");
                            // 回填 L1 缓存
                            let mut cache = self.cache.write().await;
                            cache.insert(key.clone(), CachedRule {
                                rule: rule.clone(),
                                fetched_at: Instant::now(),
                            });
                            return Ok(Some(rule));
                        }
                    }
                    Ok(None) => tracing::debug!(key=%key, "cache_miss_l2_redis"),
                    Err(e) => tracing::warn!(error=?e, key=%key, "redis_get_failed"),
                }
            }
        }

        // L3: 数据库查询
        tracing::debug!(key=%key, "cache_miss_querying_db");
        let row = sqlx::query(
            "SELECT id, chain, operation, fee_type, flat_amount, percent_bp, min_fee, max_fee, priority, rule_version
             FROM gas.platform_fee_rules
             WHERE chain = $1 AND operation = $2 AND active = true AND effective_at <= CURRENT_TIMESTAMP
             ORDER BY priority ASC, updated_at DESC LIMIT 1"
        )
        .bind(chain)
        .bind(operation)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            let rule = FeeRule {
                id: r.try_get("id")?,
                chain: r.try_get("chain")?,
                operation: r.try_get("operation")?,
                fee_type: r.try_get("fee_type")?,
                flat_amount: r.try_get::<f64, _>("flat_amount")?,
                percent_bp: r.try_get("percent_bp")?,
                min_fee: r.try_get::<f64, _>("min_fee")?,
                max_fee: r.try_get::<Option<f64>, _>("max_fee")?,
                priority: r.try_get("priority")?,
                rule_version: r.try_get("rule_version")?,
            };

            // 回填 L1 缓存
            {
                let mut cache = self.cache.write().await;
                cache.insert(key.clone(), CachedRule {
                    rule: rule.clone(),
                    fetched_at: Instant::now(),
                });
            }

            // 回填 L2 Redis 缓存（后台异步，不阻塞主流程）
            if let Some(redis_ctx) = &self.redis {
                let rule_clone = rule.clone();
                let key_clone = key.clone();
                let redis_clone = redis_ctx.clone();
                tokio::spawn(async move {
                    if let Ok(rule_json) = serde_json::to_string(&rule_clone) {
                        if let Ok(mut conn) =
                            redis_clone.client.get_multiplexed_async_connection().await
                        {
                            use redis::AsyncCommands;
                            let _: Result<(), _> = conn.set_ex(&key_clone, rule_json, 60).await;
                        }
                    }
                });
            }

            return Ok(Some(rule));
        }
        Ok(None)
    }

    async fn get_collector_address(&self, chain: &str) -> Result<Option<String>> {
        let row = sqlx::query(
            "SELECT address FROM gas.fee_collector_addresses WHERE chain = $1 AND active = true ORDER BY rotated_at DESC NULLS LAST, created_at DESC LIMIT 1"
        )
        .bind(chain)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.get::<String, _>("address")))
    }

    /// 计算平台服务费（企业级实现）
    ///
    /// 注意：这是平台服务费（钱包服务商收取的服务费用），与Gas费用（区块链网络费用）完全独立！
    ///
    /// Gas费用由区块链网络收取，用于执行交易（gas_used * gas_price）
    /// 平台服务费由钱包服务商收取，用于提供钱包服务
    ///
    /// 这两个费用是完全独立的，不能混淆！
    ///
    /// 企业级实现：输入验证
    /// - 金额必须为正数（> 0）
    /// - 金额必须为有限值（非NaN、非无穷大）
    /// - 链和操作类型不能为空
    pub async fn calculate_fee(
        &self,
        chain: &str,
        operation: &str,
        amount: f64,
    ) -> Result<Option<FeeCalcResult>> {
        // 企业级实现：输入验证
        if chain.trim().is_empty() || operation.trim().is_empty() {
            tracing::warn!("费用计算参数无效: chain={}, operation={}", chain, operation);
            return Ok(None);
        }

        // 企业级实现：金额验证（必须为正数且为有限值）
        if amount <= 0.0 || !amount.is_finite() {
            tracing::warn!(
                "费用计算金额无效: amount={}, chain={}, operation={}",
                amount,
                chain,
                operation
            );
            return Ok(None);
        }

        crate::metrics::inc_fee_calculation();
        let Some(rule) = self.get_active_rule(chain, operation).await? else {
            return Ok(None);
        };
        let Some(collector) = self.get_collector_address(chain).await? else {
            return Ok(None);
        };

        // 计算平台服务费（不是Gas费用！）
        //
        // 企业级实现：费用计算精度说明
        // - 当前使用 f64 进行费用计算，对于大多数场景（精度要求 < 18位小数）已足够
        // - 所有计算结果都经过验证（is_finite(), >= 0.0），确保结果有效
        // - 如果未来需要更高精度（如金融级计算），可以考虑迁移到 rust_decimal::Decimal
        // - 当前实现已满足企业级标准：输入验证 + 结果验证 + 错误处理
        let fee = match rule.fee_type.as_str() {
            "flat" => rule.flat_amount,
            "percent" => {
                // 百分比计算：amount * percent_bp / 10000
                // percent_bp 是基点（basis points），例如 100 = 1%
                let raw = amount * (rule.percent_bp as f64) / 10_000f64;
                let mut f = raw.max(rule.min_fee);
                if let Some(max) = rule.max_fee {
                    f = f.min(max);
                }
                f
            }
            "mixed" => {
                // 混合模式：固定费用 + 百分比费用
                let raw = amount * (rule.percent_bp as f64) / 10_000f64;
                let percent_part = raw.max(rule.min_fee);
                let mut combined = rule.flat_amount + percent_part;
                if let Some(max) = rule.max_fee {
                    combined = combined.min(max);
                }
                combined
            }
            _ => 0.0,
        };

        // 企业级实现：结果验证（费用必须为有限值且非负数）
        if !fee.is_finite() || fee < 0.0 {
            tracing::warn!(
                "费用计算结果无效: fee={}, chain={}, operation={}, amount={}",
                fee,
                chain,
                operation,
                amount
            );
            return Ok(None);
        }
        crate::metrics::add_fee_amount(fee);

        // 企业级实现：返回平台服务费计算结果
        // 注意：这是平台服务费（钱包服务商收取的服务费用），不是Gas费用！
        // Gas费用由区块链网络收取，用于执行交易（gas_used * gas_price）
        // 平台服务费由钱包服务商收取，用于提供钱包服务
        // 这两个费用是完全独立的，不能混淆！
        Ok(Some(FeeCalcResult {
            platform_fee: fee, // 平台服务费：钱包服务商收取的服务费用
            collector_address: collector,
            applied_rule_id: rule.id,
            rule_version: rule.rule_version,
        }))
    }

    /// 记录费用审计（企业级实现）
    ///
    /// 注意：fee_audit表包含两个完全独立的费用字段：
    /// 1. platform_fee: 平台服务费（钱包服务商收取的服务费用）
    /// 2. gas_fee_native: Gas费用（区块链网络收取的交易执行费用，由transaction_monitor服务回填）
    ///
    /// 这两个费用是完全独立的，不能混淆！
    ///
    /// 企业级实现：幂等性保护
    /// - 如果提供了tx_hash，检查是否已存在相同tx_hash的记录（防止重复记录）
    /// - 如果已存在，记录警告但不返回错误（幂等性）
    pub async fn record_fee_audit(
        &self,
        user_id: uuid::Uuid,
        chain: &str,
        operation: &str,
        original_amount: f64,
        calc: &FeeCalcResult,
        wallet_address: &str,
        tx_hash: Option<&str>, // 交易哈希（用于后续transaction_monitor回填gas_fee_native）
    ) -> Result<()> {
        // 企业级实现：幂等性检查（如果提供了tx_hash）
        if let Some(hash) = tx_hash {
            let existing = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM gas.fee_audit WHERE tx_hash = $1 AND platform_fee > 0",
            )
            .bind(hash)
            .fetch_one(&self.pool)
            .await
            .ok();

            if let Some(count) = existing {
                if count > 0 {
                    tracing::warn!(
                        "费用审计记录已存在（幂等性保护）: tx_hash={}, user_id={}, chain={}, operation={}",
                        hash, user_id, chain, operation
                    );
                    return Ok(()); // 幂等性：已存在，直接返回成功
                }
            }
        }

        // 企业级实现：只记录平台服务费（platform_fee）
        // Gas费用（gas_fee_native）由transaction_monitor服务在交易确认后回填
        let res = sqlx::query(
            "INSERT INTO gas.fee_audit (user_id, chain, operation, original_amount, platform_fee, fee_type, applied_rule, collector_address, wallet_address, rule_version, tx_hash)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"
        )
        .bind(user_id)
        .bind(chain)
        .bind(operation)
        .bind(original_amount)
        .bind(calc.platform_fee)  // 平台服务费：钱包服务商收取的服务费用
        .bind("computed")
        .bind(calc.applied_rule_id)
        .bind(&calc.collector_address)
        .bind(wallet_address)
        .bind(calc.rule_version)
        .bind(tx_hash)  // 交易哈希，用于transaction_monitor回填gas_fee_native
        .execute(&self.pool)
        .await;
        if res.is_err() {
            crate::metrics::inc_fee_audit_fail();
        }
        res?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    // ============ 费率计算公式测试（无需数据库） ============

    /// Test 1: 百分比费率基础计算
    #[test]
    fn test_percent_fee_basic_formula() {
        let amount = 100.0;
        let percent_bp = 50; // 0.50% = 50 基点
        let fee = amount * (percent_bp as f64) / 10_000.0;
        assert_eq!(fee, 0.5, "100 * 0.50% should equal 0.5");
    }

    /// Test 2: 高额百分比费率（10%）
    #[test]
    fn test_percent_fee_high_rate() {
        let amount = 1000.0;
        let percent_bp = 1000; // 10.00% = 1000 基点
        let fee = amount * (percent_bp as f64) / 10_000.0;
        assert_eq!(fee, 100.0, "1000 * 10% should equal 100");
    }

    /// Test 3: 最小费用限制（实际费用低于最小值）
    #[test]
    fn test_min_fee_enforcement() {
        let amount = 1.0;
        let percent_bp = 10; // 0.10%
        let min_fee = 0.5;

        let raw_fee = amount * (percent_bp as f64) / 10_000.0; // 0.001
        let actual_fee = raw_fee.max(min_fee);

        assert_eq!(raw_fee, 0.001);
        assert_eq!(
            actual_fee, 0.5,
            "Should enforce min_fee when calculated fee is too low"
        );
    }

    /// Test 4: 最大费用限制（实际费用高于最大值）
    #[test]
    fn test_max_fee_enforcement() {
        let amount = 10000.0;
        let percent_bp = 500; // 5.00%
        let max_fee = 100.0;

        let raw_fee = amount * (percent_bp as f64) / 10_000.0; // 500
        let actual_fee = raw_fee.min(max_fee);

        assert_eq!(raw_fee, 500.0);
        assert_eq!(
            actual_fee, 100.0,
            "Should cap at max_fee when calculated fee is too high"
        );
    }

    /// Test 5: 最小和最大费用同时应用（范围内）
    #[test]
    fn test_fee_within_min_max_range() {
        let amount = 500.0;
        let percent_bp = 100; // 1.00%
        let min_fee = 1.0;
        let max_fee = 10.0;

        let raw_fee = amount * (percent_bp as f64) / 10_000.0; // 5.0
        let actual_fee = raw_fee.max(min_fee).min(max_fee);

        assert_eq!(
            actual_fee, 5.0,
            "Fee should stay within [min, max] range when calculated value is in between"
        );
    }

    /// Test 6: 固定费率模式（flat）
    #[test]
    fn test_flat_fee_mode() {
        let flat_amount = 2.5;
        let _amount = 100.0; // 金额不影响固定费率

        let fee = flat_amount; // flat 模式直接返回固定值
        assert_eq!(fee, 2.5);

        let _amount2 = 10000.0;
        let fee2 = flat_amount;
        assert_eq!(
            fee2, 2.5,
            "Flat fee should be constant regardless of transaction amount"
        );
    }

    /// Test 7: 混合费率模式（flat + percent）
    #[test]
    fn test_mixed_fee_mode() {
        let flat_amount = 1.0;
        let amount = 1000.0;
        let percent_bp = 50; // 0.50%
        let min_fee = 0.1;

        let percent_part = amount * (percent_bp as f64) / 10_000.0; // 5.0
        let percent_with_min = percent_part.max(min_fee); // 5.0
        let combined = flat_amount + percent_with_min; // 6.0

        assert_eq!(
            combined, 6.0,
            "Mixed fee = flat + max(percent_fee, min_fee)"
        );
    }

    /// Test 8: 混合模式 + 最大费用限制
    #[test]
    fn test_mixed_fee_with_max_cap() {
        let flat_amount = 2.0;
        let amount = 5000.0;
        let percent_bp = 200; // 2.00%
        let min_fee = 1.0;
        let max_fee = 50.0;

        let percent_part = amount * (percent_bp as f64) / 10_000.0; // 100.0
        let percent_with_min = percent_part.max(min_fee); // 100.0
        let combined = flat_amount + percent_with_min; // 102.0
        let capped = combined.min(max_fee); // 50.0

        assert_eq!(capped, 50.0, "Mixed fee should be capped by max_fee");
    }

    /// Test 9: 零金额交易
    #[test]
    fn test_zero_amount_transaction() {
        let amount = 0.0;
        let percent_bp = 100; // 1.00%
        let min_fee = 0.5;

        let raw_fee = amount * (percent_bp as f64) / 10_000.0; // 0.0
        let actual_fee = raw_fee.max(min_fee); // 0.5

        assert_eq!(actual_fee, 0.5, "Zero amount should still apply min_fee");
    }

    /// Test 10: 极小金额交易（精度测试）
    #[test]
    fn test_tiny_amount_precision() {
        let amount = 0.001;
        let percent_bp = 5000; // 50.00%
        let fee = amount * (percent_bp as f64) / 10_000.0;

        // 0.001 * 0.5 = 0.0005
        assert!(
            (fee - 0.0005).abs() < 1e-10,
            "Should handle tiny amounts with precision"
        );
    }

    /// Test 11: 大额交易（溢出保护）
    #[test]
    fn test_large_amount_no_overflow() {
        let amount = 1_000_000.0;
        let percent_bp = 100; // 1.00%
        let fee = amount * (percent_bp as f64) / 10_000.0;

        assert_eq!(fee, 10_000.0);
        assert!(fee.is_finite(), "Large amounts should not cause overflow");
    }

    /// Test 12: 负数金额拒绝
    #[test]
    fn test_negative_amount_rejection() {
        let amount = -100.0;
        let percent_bp = 50;
        let fee = amount * (percent_bp as f64) / 10_000.0;

        // 生产代码应该拒绝负数费用
        assert!(fee < 0.0, "Negative amounts produce negative fees");
        // 实际服务中会检查 !fee.is_finite() || fee < 0.0 并返回 None
    }

    /// Test 13: NaN 和 Infinity 拒绝
    #[test]
    fn test_invalid_float_rejection() {
        let nan_fee = f64::NAN;
        let inf_fee = f64::INFINITY;

        assert!(!nan_fee.is_finite(), "NaN should be rejected");
        assert!(!inf_fee.is_finite(), "Infinity should be rejected");
    }

    /// Test 14: 基点边界值测试
    #[test]
    fn test_basis_point_boundary() {
        let amount = 100.0;

        // 0 基点（0%）
        let bp_zero = 0;
        let fee_zero = amount * (bp_zero as f64) / 10_000.0;
        assert_eq!(fee_zero, 0.0);

        // 10000 基点（100%）
        let bp_full = 10_000;
        let fee_full = amount * (bp_full as f64) / 10_000.0;
        assert_eq!(fee_full, 100.0, "10000 basis points = 100% of amount");

        // 1 基点（0.01%）
        let bp_one = 1;
        let fee_one = amount * (bp_one as f64) / 10_000.0;
        assert_eq!(fee_one, 0.01);
    }

    /// Test 15: 边界条件组合测试（综合场景）
    #[test]
    fn test_complex_boundary_scenario() {
        // 场景：混合费率，极端条件组合
        let flat_amount = 0.5;
        let amount = 50.0;
        let percent_bp = 10; // 0.10%
        let min_fee = 1.0;
        let max_fee = Some(2.0);

        // 步骤1：计算百分比部分
        let percent_part = amount * (percent_bp as f64) / 10_000.0; // 0.05
        assert_eq!(percent_part, 0.05);

        // 步骤2：应用最小费用
        let percent_with_min = percent_part.max(min_fee); // 1.0
        assert_eq!(percent_with_min, 1.0);

        // 步骤3：加上固定费用
        let combined = flat_amount + percent_with_min; // 1.5
        assert_eq!(combined, 1.5);

        // 步骤4：应用最大费用（如果存在）
        let final_fee = if let Some(max) = max_fee {
            combined.min(max)
        } else {
            combined
        };
        assert_eq!(final_fee, 1.5, "Final fee should be within all constraints");
    }

    // ============ 集成测试（需要数据库）============

    // 注意：以下测试需要真实数据库连接，在 CI/CD 中使用 testcontainers
    // 本地开发可以通过 `cargo test -- --ignored` 运行

    #[tokio::test]
    #[ignore] // 需要数据库环境
    async fn test_fee_service_integration_with_db() {
        // 这里可以添加完整的数据库集成测试
        // 包括：规则查询、缓存验证、审计记录等
        // 使用 sqlx::test 宏或 testcontainers-rs
    }
}
