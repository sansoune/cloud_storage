use crate::Result;
use tokio::time::{sleep, Duration};

pub struct RetryConfig {
    max_retries: u32,
    initial_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_secs(1),
        }
    }
}

pub async fn with_retry<F, Fut, T>(config: &RetryConfig, operation: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempts = 0;
    let mut last_error = None;
    while attempts < config.max_retries {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                attempts += 1;
                if attempts < config.max_retries {
                    sleep(config.initial_delay * 2u32.pow(attempts)).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
