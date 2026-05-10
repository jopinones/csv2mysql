use crate::error::{Csv2MysqlError, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            initial_backoff_ms: 100,
        }
    }
}

impl RetryPolicy {
    pub async fn execute<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempts = 0;
        let mut backoff = self.initial_backoff_ms;

        loop {
            attempts += 1;
            
            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if attempts >= self.max_attempts || !e.is_retryable() {
                        return Err(e);
                    }

                    warn!(
                        "Intento {}/{} falló: {} - Reintentando en {} ms",
                        attempts, self.max_attempts, e, backoff
                    );

                    sleep(Duration::from_millis(backoff)).await;
                    backoff = (backoff * 2).min(5000); // Máximo 5 segundos
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let policy = RetryPolicy::default();
        let mut call_count = 0;

        let result = policy
            .execute(|| async {
                call_count += 1;
                if call_count == 1 {
                    Err(Csv2MysqlError::General("Fallo temporal".to_string()))
                } else {
                    Ok(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count, 2);
    }
}
