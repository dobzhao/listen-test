//! 指数退避重试工具
//!
//! 用于 LLM/TTS/STT 调用失败时的自动重试机制。

use std::future::Future;
use std::time::Duration;
use tracing::{debug, warn};

/// 通用重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub backoff_factor: f64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 500,
            backoff_factor: 2.0,
            max_delay_ms: 5000,
        }
    }
}

/// 带指数退避的重试包装器
///
/// - `op`：每次尝试执行的异步操作
/// - `should_retry`：根据错误判断是否值得重试（例如解析失败重试、网络超时重试）
/// - 返回最后一次的结果，无论成功失败
pub async fn retry_async<T, E, F, Fut, P>(
    config: &RetryConfig,
    op_name: &str,
    mut op: F,
    should_retry: P,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    P: Fn(&E) -> bool,
{
    let mut delay = config.initial_delay_ms;
    let mut last_err: Option<E> = None;

    for attempt in 0..=config.max_retries {
        match op().await {
            Ok(v) => {
                if attempt > 0 {
                    debug!(op = op_name, attempt, "重试成功");
                }
                return Ok(v);
            }
            Err(e) => {
                let retryable = should_retry(&e);
                if !retryable || attempt == config.max_retries {
                    warn!(op = op_name, attempt, "最终失败，停止重试");
                    return Err(e);
                }
                warn!(op = op_name, attempt, "操作失败，{}ms 后重试", delay);
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(delay)).await;
                delay = ((delay as f64) * config.backoff_factor) as u64;
                delay = delay.min(config.max_delay_ms);
            }
        }
    }

    Err(last_err.expect("loop above guarantees last_err Some"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_on_third_attempt() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            backoff_factor: 1.0,
            max_delay_ms: 10,
        };
        let mut count = 0;
        let result: Result<&str, &str> = retry_async(
            &config,
            "test",
            || {
                count += 1;
                async move {
                    if count < 3 { Err("fail") } else { Ok("ok") }
                }
            },
            |_| true,
        )
        .await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay_ms: 1,
            backoff_factor: 1.0,
            max_delay_ms: 10,
        };
        let mut count = 0;
        let result: Result<&str, &str> = retry_async(
            &config,
            "test",
            || {
                count += 1;
                async move { Err("always fail") }
            },
            |_| true,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(count, 3); // 1 + 2 retries
    }
}
