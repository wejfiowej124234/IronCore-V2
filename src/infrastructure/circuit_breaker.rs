//! 熔断器实现（S项修复）
//! 企业级实现：防止故障节点持续消耗资源

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use anyhow::Result;

/// 熔断器状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    /// 正常状态：所有请求通过
    Closed,
    /// 熔断状态：快速失败
    Open,
    /// 半开状态：尝试恢复
    HalfOpen,
}

/// 熔断器配置
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// 失败阈值（连续失败多少次触发熔断）
    pub failure_threshold: u32,
    /// 成功阈值（半开状态成功多少次恢复）
    pub success_threshold: u32,
    /// 超时时间（熔断后多久尝试恢复）
    pub timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout: Duration::from_secs(60),
        }
    }
}

/// 熔断器
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<Mutex<CircuitBreakerState>>,
}

#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
}

impl CircuitBreaker {
    /// 创建熔断器
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
            })),
        }
    }

    /// 使用默认配置创建
    pub fn default() -> Self {
        Self::new(CircuitBreakerConfig::default())
    }

    /// 执行受保护的操作
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, String>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        // 检查是否允许通过
        {
            let mut state = self.state.lock().unwrap();
            
            match state.state {
                CircuitState::Open => {
                    // 检查是否可以尝试恢复
                    if self.should_attempt_reset(&state) {
                        state.state = CircuitState::HalfOpen;
                        state.success_count = 0;
                        tracing::info!("Circuit breaker transitioning to HalfOpen state");
                    } else {
                        return Err("Circuit breaker is open".to_string());
                    }
                }
                _ => {}
            }
        }

        // 执行操作
        match f.await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(e.to_string())
            }
        }
    }

    /// 成功回调
    fn on_success(&self) {
        let mut state = self.state.lock().unwrap();
        
        match state.state {
            CircuitState::HalfOpen => {
                state.success_count += 1;
                if state.success_count >= self.config.success_threshold {
                    state.state = CircuitState::Closed;
                    state.failure_count = 0;
                    state.success_count = 0;
                    tracing::info!("Circuit breaker recovered to Closed state");
                }
            }
            CircuitState::Closed => {
                state.failure_count = 0;  // 重置失败计数
            }
            _ => {}
        }
    }

    /// 失败回调
    fn on_failure(&self) {
        let mut state = self.state.lock().unwrap();
        
        state.failure_count += 1;
        state.last_failure_time = Some(Instant::now());
        
        if state.state == CircuitState::HalfOpen {
            // 半开状态失败，立即重新熔断
            state.state = CircuitState::Open;
            state.success_count = 0;
            tracing::warn!("Circuit breaker re-opened due to failure in HalfOpen state");
        } else if state.failure_count >= self.config.failure_threshold {
            // 超过阈值，触发熔断
            state.state = CircuitState::Open;
            tracing::warn!(
                "Circuit breaker opened after {} failures",
                state.failure_count
            );
        }
    }

    /// 检查是否应该尝试恢复
    fn should_attempt_reset(&self, state: &CircuitBreakerState) -> bool {
        if let Some(last_failure) = state.last_failure_time {
            last_failure.elapsed() >= self.config.timeout
        } else {
            true
        }
    }

    /// 获取当前状态
    pub fn get_state(&self) -> CircuitState {
        self.state.lock().unwrap().state
    }

    /// 获取失败计数
    pub fn get_failure_count(&self) -> u32 {
        self.state.lock().unwrap().failure_count
    }

    /// 手动重置熔断器
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.state = CircuitState::Closed;
        state.failure_count = 0;
        state.success_count = 0;
        state.last_failure_time = None;
        tracing::info!("Circuit breaker manually reset");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_open() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
        };
        let cb = CircuitBreaker::new(config);

        // 触发3次失败
        for _ in 0..3 {
            let _ = cb.call(async { Err::<(), _>("error") }).await;
        }

        assert_eq!(cb.get_state(), CircuitState::Open);

        // 熔断状态应该快速失败
        let result = cb.call(async { Ok::<(), String>(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(10),
        };
        let cb = CircuitBreaker::new(config);

        // 触发熔断
        for _ in 0..2 {
            let _ = cb.call(async { Err::<(), _>("error") }).await;
        }
        assert_eq!(cb.get_state(), CircuitState::Open);

        // 等待超时
        tokio::time::sleep(Duration::from_millis(20)).await;

        // 2次成功应该恢复
        for _ in 0..2 {
            let _ = cb.call(async { Ok::<(), String>(()) }).await;
        }
        assert_eq!(cb.get_state(), CircuitState::Closed);
    }
}

