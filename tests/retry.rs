#[cfg(test)]
mod tests {

    use std::sync::Arc;
    use std::io;
    use cloud_storage::{storage::retry::{with_retry, RetryConfig}, AppError, Result, StorageError};
    use tokio::sync::Mutex;
    use std::time::Instant;
    use tokio::time::Duration;
    struct MockOperation {
        attempts: Arc<Mutex<u32>>,
        success_after: u32,
        error_message: String,
    }

    impl MockOperation {
        fn new(success_after: u32, error_message: &str) -> Self {
            Self {
                attempts: Arc::new(Mutex::new(0)),
                success_after,
                error_message: error_message.to_string(),
            }
        }

        async fn execute<T: ToString>(&self, success_value: T) -> Result<String> {
            let mut attempts = self.attempts.lock().await;
            *attempts += 1;
            
            if *attempts > self.success_after {
                Ok(success_value.to_string())
            } else {
                Err(cloud_storage::AppError::Storage(StorageError::Io(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{} (Attempt {})", self.error_message, *attempts)
                ))))
            }
        }

        async fn get_attempts(&self) -> u32 {
            *self.attempts.lock().await
        }
    }

    #[tokio::test]
    async fn test_immediate_success() {
        let config = RetryConfig::default();
        let operation = MockOperation::new(0, "Should not see this error");

        let result = with_retry(&config, || {
            let op = &operation;
            async move { op.execute("Success!").await }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success!");
        assert_eq!(operation.get_attempts().await, 1);
    }

    #[tokio::test]
    async fn test_success_after_retries() {
        let config = RetryConfig::new(
            3,
            Duration::from_millis(50),
        );
        let operation = MockOperation::new(2, "Temporary error");

        let result = with_retry(&config, || {
            let op = &operation;
            async move { op.execute("Success after retry!").await }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success after retry!");
        assert_eq!(operation.get_attempts().await, 3);
    }

    #[tokio::test]
    async fn test_permanent_failure() {
        let config = RetryConfig::new(
            2,
            Duration::from_millis(50),
        );
        let operation = MockOperation::new(u32::MAX, "Permanent failure");

        let result = with_retry(&config, || {
            let op = &operation;
            async move { op.execute("Should not succeed").await }
        })
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Permanent failure"));
        assert_eq!(operation.get_attempts().await, 2);
    }

    #[tokio::test]
    async fn test_exponential_backoff_timing() {
        let config = RetryConfig::new(
            3,
            Duration::from_millis(100),
        );
        let operation = MockOperation::new(3, "Testing backoff");
        
        let start_time = Instant::now();
        let result = with_retry(&config, || {
            let op = &operation;
            async move { op.execute("Should not succeed").await }
        })
        .await;

        let elapsed = start_time.elapsed();
        assert!(result.is_err());
        
        assert!(elapsed.as_millis() >= 700, 
            "Expected at least 700ms delay, got {}ms", 
            elapsed.as_millis()
        );
    }

    #[tokio::test]
    async fn test_retry_with_different_error_types() {
        let config = RetryConfig::default();
        let attempt_counter = Arc::new(Mutex::new(0));
        
        let result = with_retry(&config, || {
            let counter = Arc::clone(&attempt_counter);
            async move {
                let mut attempts = counter.lock().await;
                *attempts += 1;
                match *attempts {
                    1 => Err(AppError::Storage( StorageError::Io(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Timeout error"
                    )))),
                    2 => Err(AppError::Storage(StorageError::Io(io::Error::new(
                        io::ErrorKind::ConnectionReset,
                        "Connection reset"
                    )))),
                    _ => Ok("Success!")
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success!");
        assert_eq!(*attempt_counter.lock().await, 3);
    }

    #[tokio::test]
    async fn test_custom_retry_config() {
        let config = RetryConfig::new(
            5,
            Duration::from_millis(25),
        );
        let operation = MockOperation::new(4, "Testing custom config");

        let result = with_retry(&config, || {
            let op = &operation;
            async move { op.execute("Custom config success!").await }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Custom config success!");
        assert_eq!(operation.get_attempts().await, 5);
    }
}