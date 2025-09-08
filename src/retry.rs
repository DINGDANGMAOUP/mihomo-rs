//! 重试机制模块
//!
//! 提供智能重试功能，支持指数退避、最大重试次数等策略。

use crate::error::{MihomoError, Result};
use crate::logger::Logger;
use std::time::Duration;
use tokio::time::sleep;

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_attempts: usize,
    /// 初始延迟时间
    pub initial_delay: Duration,
    /// 最大延迟时间
    pub max_delay: Duration,
    /// 退避倍数
    pub backoff_multiplier: f64,
    /// 抖动因子（0.0-1.0）
    pub jitter_factor: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

impl RetryPolicy {
    /// 创建新的重试策略
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// 设置初始延迟时间
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// 设置最大延迟时间
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// 设置退避倍数
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// 设置抖动因子
    pub fn with_jitter_factor(mut self, factor: f64) -> Self {
        self.jitter_factor = factor.clamp(0.0, 1.0);
        self
    }

    /// 计算延迟时间
    fn calculate_delay(&self, attempt: usize) -> Duration {
        let base_delay =
            self.initial_delay.as_millis() as f64 * self.backoff_multiplier.powi(attempt as i32);

        let max_delay_ms = self.max_delay.as_millis() as f64;
        let delay_ms = base_delay.min(max_delay_ms);

        // 添加抖动
        let jitter = delay_ms * self.jitter_factor * (rand::random::<f64>() - 0.5);
        let final_delay_ms = (delay_ms + jitter).max(0.0) as u64;

        Duration::from_millis(final_delay_ms)
    }
}

/// 重试执行器
#[derive(Debug, Clone, Default)]
pub struct RetryExecutor {
    policy: RetryPolicy,
}

impl RetryExecutor {
    /// 创建新的重试执行器
    pub fn new(policy: RetryPolicy) -> Self {
        Self { policy }
    }

    /// 执行带重试的异步操作
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..self.policy.max_attempts {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        Logger::info(&format!("操作在第 {} 次尝试后成功", attempt + 1));
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    // 检查是否可重试
                    if !error.is_retryable() {
                        Logger::warn(&format!("错误不可重试: {}", error));
                        return Err(error);
                    }

                    // 如果是最后一次尝试，直接返回错误
                    if attempt == self.policy.max_attempts - 1 {
                        Logger::error(&format!(
                            "所有重试尝试都失败了 ({} 次)",
                            self.policy.max_attempts
                        ));
                        return Err(error);
                    }

                    // 计算延迟时间并等待
                    let delay = self.policy.calculate_delay(attempt);
                    Logger::warn(&format!(
                        "第 {} 次尝试失败: {}，{:?} 后重试",
                        attempt + 1,
                        error,
                        delay
                    ));

                    sleep(delay).await;
                }
            }
        }

        // 这里不应该到达，但为了安全起见
        Err(last_error.unwrap_or_else(|| MihomoError::internal("重试执行器内部错误")))
    }

    /// 执行带重试的同步操作
    pub fn execute_sync<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        let mut last_error = None;

        for attempt in 0..self.policy.max_attempts {
            match operation() {
                Ok(result) => {
                    if attempt > 0 {
                        Logger::info(&format!("操作在第 {} 次尝试后成功", attempt + 1));
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());

                    // 检查是否可重试
                    if !error.is_retryable() {
                        Logger::warn(&format!("错误不可重试: {}", error));
                        return Err(error);
                    }

                    // 如果是最后一次尝试，直接返回错误
                    if attempt == self.policy.max_attempts - 1 {
                        Logger::error(&format!(
                            "所有重试尝试都失败了 ({} 次)",
                            self.policy.max_attempts
                        ));
                        return Err(error);
                    }

                    // 计算延迟时间并等待
                    let delay = self.policy.calculate_delay(attempt);
                    Logger::warn(&format!(
                        "第 {} 次尝试失败: {}，{:?} 后重试",
                        attempt + 1,
                        error,
                        delay
                    ));

                    std::thread::sleep(delay);
                }
            }
        }

        // 这里不应该到达，但为了安全起见
        Err(last_error.unwrap_or_else(|| MihomoError::internal("重试执行器内部错误")))
    }
}

/// 便捷函数：使用默认策略执行带重试的异步操作
pub async fn retry_async<F, Fut, T>(operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let executor = RetryExecutor::default();
    executor.execute(operation).await
}

/// 便捷函数：使用自定义策略执行带重试的异步操作
pub async fn retry_async_with_policy<F, Fut, T>(policy: RetryPolicy, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let executor = RetryExecutor::new(policy);
    executor.execute(operation).await
}

/// 便捷函数：使用默认策略执行带重试的同步操作
pub fn retry_sync<F, T>(operation: F) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let executor = RetryExecutor::default();
    executor.execute_sync(operation)
}

/// 便捷函数：使用自定义策略执行带重试的同步操作
pub fn retry_sync_with_policy<F, T>(policy: RetryPolicy, operation: F) -> Result<T>
where
    F: Fn() -> Result<T>,
{
    let executor = RetryExecutor::new(policy);
    executor.execute_sync(operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_millis(100));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert_eq!(policy.backoff_multiplier, 2.0);
        assert_eq!(policy.jitter_factor, 0.1);
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::new(5)
            .with_initial_delay(Duration::from_millis(200))
            .with_max_delay(Duration::from_secs(60))
            .with_backoff_multiplier(1.5)
            .with_jitter_factor(0.2);

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(200));
        assert_eq!(policy.max_delay, Duration::from_secs(60));
        assert_eq!(policy.backoff_multiplier, 1.5);
        assert_eq!(policy.jitter_factor, 0.2);
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let executor = RetryExecutor::default();
        let result = executor
            .execute(|| async { Ok::<i32, MihomoError>(42) })
            .await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let executor = RetryExecutor::new(RetryPolicy::new(3));

        let counter_clone = counter.clone();
        let result = executor
            .execute(move || {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(MihomoError::network("网络错误"))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let executor = RetryExecutor::default();
        let result = executor
            .execute(|| async { Err::<i32, MihomoError>(MihomoError::auth("认证错误")) })
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MihomoError::Auth(_)));
    }

    #[test]
    fn test_retry_sync_success() {
        let executor = RetryExecutor::default();
        let result = executor.execute_sync(|| Ok::<i32, MihomoError>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_convenience_functions() {
        // 测试便捷函数不会panic
        let policy = RetryPolicy::new(1);
        let _executor = RetryExecutor::new(policy);
    }
}
