#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::timeout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeoutAction {
    Retry,
    Fail,
    Extend(Duration),
}

#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    pub llm_request_secs: u64,
    pub message_processing_secs: u64,
    pub connect_secs: u64,
    pub retry_count: usize,
    pub retry_delay_ms: u64,
    pub retry_backoff_factor: f64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            llm_request_secs: 120,
            message_processing_secs: 300,
            connect_secs: 10,
            retry_count: 1,
            retry_delay_ms: 1000,
            retry_backoff_factor: 2.0,
        }
    }
}

impl TimeoutConfig {
    pub fn for_local_llm() -> Self {
        Self {
            llm_request_secs: 300,
            message_processing_secs: 600,
            connect_secs: 30,
            retry_count: 2,
            retry_delay_ms: 2000,
            retry_backoff_factor: 1.5,
        }
    }

    pub fn for_cloud_llm() -> Self {
        Self {
            llm_request_secs: 120,
            message_processing_secs: 300,
            connect_secs: 10,
            retry_count: 1,
            retry_delay_ms: 1000,
            retry_backoff_factor: 2.0,
        }
    }

    pub fn llm_request_duration(&self) -> Duration {
        Duration::from_secs(self.llm_request_secs)
    }

    pub fn message_processing_duration(&self) -> Duration {
        Duration::from_secs(self.message_processing_secs)
    }

    pub fn connect_duration(&self) -> Duration {
        Duration::from_secs(self.connect_secs)
    }
}

#[derive(Debug, Clone)]
pub struct TimeoutError {
    pub is_timeout: bool,
    pub retries_attempted: usize,
    pub elapsed: Duration,
    pub message: String,
    pub last_error: Option<String>,
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_timeout {
            write!(
                f,
                "Operation timed out after {:.1}s ({} retries attempted): {}",
                self.elapsed.as_secs_f64(),
                self.retries_attempted,
                self.message
            )
        } else {
            write!(
                f,
                "Operation failed after {:.1}s ({} retries attempted): {}",
                self.elapsed.as_secs_f64(),
                self.retries_attempted,
                self.message
            )
        }
    }
}

impl std::error::Error for TimeoutError {}

impl TimeoutError {
    pub fn timeout(message: impl Into<String>, retries: usize, elapsed: Duration) -> Self {
        Self {
            is_timeout: true,
            retries_attempted: retries,
            elapsed,
            message: message.into(),
            last_error: None,
        }
    }

    pub fn failed(
        message: impl Into<String>,
        retries: usize,
        elapsed: Duration,
        last_error: Option<String>,
    ) -> Self {
        Self {
            is_timeout: false,
            retries_attempted: retries,
            elapsed,
            message: message.into(),
            last_error,
        }
    }

    pub fn user_friendly_message(&self) -> String {
        if self.is_timeout {
            format!(
                "The operation took too long and was cancelled after {:.1} seconds. {}",
                self.elapsed.as_secs_f64(),
                if self.retries_attempted > 0 {
                    format!("We tried {} time(s), but it still didn't complete in time.", self.retries_attempted + 1)
                } else {
                    "Please try again or check your connection.".to_string()
                }
            )
        } else {
            let base = format!(
                "The operation failed after {:.1} seconds.",
                self.elapsed.as_secs_f64()
            );
            if let Some(ref err) = self.last_error {
                format!("{} Error: {}", base, err)
            } else {
                base
            }
        }
    }
}

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct TimeoutHandler {
    config: TimeoutConfig,
    state: Arc<Mutex<TimeoutState>>,
}

#[derive(Debug, Clone, Default)]
struct TimeoutState {
    total_retries: usize,
    total_timeouts: usize,
}

impl TimeoutHandler {
    pub fn new(config: TimeoutConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(TimeoutState::default())),
        }
    }

    pub fn config(&self) -> &TimeoutConfig {
        &self.config
    }

    pub async fn execute_with_timeout<F, T, E>(
        &self,
        duration: Duration,
        operation: F,
    ) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>> + Send,
        E: std::fmt::Display,
    {
        let start = Instant::now();

        match timeout(duration, operation).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => {
                let elapsed = start.elapsed();
                Err(TimeoutError::failed(
                    "Operation returned an error",
                    0,
                    elapsed,
                    Some(e.to_string()),
                ))
            }
            Err(_) => {
                let elapsed = start.elapsed();
                let mut state = self.state.lock().await;
                state.total_timeouts += 1;
                Err(TimeoutError::timeout(
                    "Operation exceeded time limit",
                    0,
                    elapsed,
                ))
            }
        }
    }

    pub async fn execute_with_retry<F, T, E>(
        &self,
        operation_factory: impl Fn() -> F,
    ) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>> + Send,
        E: std::fmt::Display,
    {
        self.execute_with_retry_and_duration(self.config.llm_request_duration(), operation_factory)
            .await
    }

    pub async fn execute_with_retry_and_duration<F, T, E>(
        &self,
        duration: Duration,
        operation_factory: impl Fn() -> F,
    ) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>> + Send,
        E: std::fmt::Display,
    {
        let start = Instant::now();
        let mut delay_ms = self.config.retry_delay_ms;
        let mut last_error: Option<String> = None;

        for attempt in 0..=self.config.retry_count {
            match timeout(duration, operation_factory()).await {
                Ok(Ok(result)) => {
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                    if attempt < self.config.retry_count {
                        tracing::debug!(
                            attempt = attempt + 1,
                            max_retries = self.config.retry_count,
                            delay_ms,
                            "Operation failed, retrying"
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms as f64 * self.config.retry_backoff_factor) as u64;
                    }
                }
                Err(_) => {
                    if attempt < self.config.retry_count {
                        let mut state = self.state.lock().await;
                        state.total_timeouts += 1;
                        tracing::debug!(
                            attempt = attempt + 1,
                            max_retries = self.config.retry_count,
                            delay_ms,
                            "Operation timed out, retrying"
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        delay_ms = (delay_ms as f64 * self.config.retry_backoff_factor) as u64;
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        let mut state = self.state.lock().await;
        state.total_retries += self.config.retry_count;

        if let Some(err) = last_error {
            Err(TimeoutError::failed(
                "Operation failed after all retries",
                self.config.retry_count,
                elapsed,
                Some(err),
            ))
        } else {
            Err(TimeoutError::timeout(
                "Operation timed out after all retries",
                self.config.retry_count,
                elapsed,
            ))
        }
    }

    pub async fn execute_with_policy<F, T, E>(
        &self,
        duration: Duration,
        policy: impl Fn(&TimeoutError, usize) -> TimeoutAction + Send + Sync + 'static,
        operation_factory: impl Fn() -> F,
    ) -> Result<T, TimeoutError>
    where
        F: Future<Output = Result<T, E>> + Send,
        E: std::fmt::Display,
    {
        let start = Instant::now();
        let mut current_duration = duration;
        let mut attempt = 0;
        let mut delay_ms = self.config.retry_delay_ms;
        let mut last_error: Option<String> = None;

        loop {
            match timeout(current_duration, operation_factory()).await {
                Ok(Ok(result)) => {
                    return Ok(result);
                }
                Ok(Err(e)) => {
                    last_error = Some(e.to_string());
                    let err = TimeoutError::failed(
                        "Operation failed",
                        attempt,
                        start.elapsed(),
                        last_error.clone(),
                    );

                    match policy(&err, attempt) {
                        TimeoutAction::Retry => {
                            attempt += 1;
                            if attempt > self.config.retry_count {
                                break;
                            }
                            tracing::debug!(
                                attempt,
                                max_retries = self.config.retry_count,
                                "Retrying after failure"
                            );
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            delay_ms =
                                (delay_ms as f64 * self.config.retry_backoff_factor) as u64;
                        }
                        TimeoutAction::Fail => break,
                        TimeoutAction::Extend(added) => {
                            current_duration += added;
                            attempt += 1;
                            tracing::debug!(
                                attempt,
                                extended_by_secs = added.as_secs(),
                                new_timeout_secs = current_duration.as_secs(),
                                "Extending timeout"
                            );
                        }
                    }
                }
                Err(_) => {
                    let mut state = self.state.lock().await;
                    state.total_timeouts += 1;

                    let err = TimeoutError::timeout(
                        "Operation timed out",
                        attempt,
                        start.elapsed(),
                    );

                    match policy(&err, attempt) {
                        TimeoutAction::Retry => {
                            attempt += 1;
                            if attempt > self.config.retry_count {
                                break;
                            }
                            tracing::debug!(
                                attempt,
                                max_retries = self.config.retry_count,
                                "Retrying after timeout"
                            );
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                            delay_ms =
                                (delay_ms as f64 * self.config.retry_backoff_factor) as u64;
                        }
                        TimeoutAction::Fail => break,
                        TimeoutAction::Extend(added) => {
                            current_duration += added;
                            attempt += 1;
                            tracing::debug!(
                                attempt,
                                extended_by_secs = added.as_secs(),
                                new_timeout_secs = current_duration.as_secs(),
                                "Extending timeout after timeout"
                            );
                        }
                    }
                }
            }
        }

        let elapsed = start.elapsed();
        let mut state = self.state.lock().await;
        state.total_retries += attempt;

        if let Some(err) = last_error {
            Err(TimeoutError::failed(
                "Operation failed",
                attempt,
                elapsed,
                Some(err),
            ))
        } else {
            Err(TimeoutError::timeout(
                "Operation timed out",
                attempt,
                elapsed,
            ))
        }
    }

    pub async fn stats(&self) -> TimeoutStats {
        let state = self.state.lock().await;
        TimeoutStats {
            total_retries: state.total_retries,
            total_timeouts: state.total_timeouts,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TimeoutStats {
    pub total_retries: usize,
    pub total_timeouts: usize,
}

pub fn default_retry_policy() -> impl Fn(&TimeoutError, usize) -> TimeoutAction + Send + Sync + 'static
{
    move |err: &TimeoutError, attempt: usize| {
        if err.is_timeout && attempt < 3 {
            TimeoutAction::Extend(Duration::from_secs(30))
        } else if !err.is_timeout && attempt < 2 {
            TimeoutAction::Retry
        } else {
            TimeoutAction::Fail
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_config_defaults() {
        let config = TimeoutConfig::default();
        assert_eq!(config.llm_request_secs, 120);
        assert_eq!(config.message_processing_secs, 300);
        assert_eq!(config.connect_secs, 10);
        assert_eq!(config.retry_count, 1);
        assert_eq!(config.retry_delay_ms, 1000);
        assert!((config.retry_backoff_factor - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn timeout_config_local_llm() {
        let config = TimeoutConfig::for_local_llm();
        assert_eq!(config.llm_request_secs, 300);
        assert_eq!(config.message_processing_secs, 600);
        assert_eq!(config.connect_secs, 30);
        assert_eq!(config.retry_count, 2);
        assert_eq!(config.retry_delay_ms, 2000);
        assert!((config.retry_backoff_factor - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn timeout_config_cloud_llm() {
        let config = TimeoutConfig::for_cloud_llm();
        assert_eq!(config.llm_request_secs, 120);
        assert_eq!(config.message_processing_secs, 300);
        assert_eq!(config.connect_secs, 10);
    }

    #[test]
    fn timeout_error_user_message() {
        let err = TimeoutError::timeout("test", 2, Duration::from_secs(10));
        let msg = err.user_friendly_message();
        assert!(msg.contains("10.0 seconds"));
        assert!(msg.contains("3 time(s)"));
    }

    #[test]
    fn timeout_error_display() {
        let err = TimeoutError::timeout("test timeout", 1, Duration::from_secs(5));
        assert!(err.to_string().contains("timed out"));
        assert!(err.to_string().contains("5.0s"));
    }

    #[tokio::test]
    async fn timeout_handler_execute_success() {
        let handler = TimeoutHandler::new(TimeoutConfig::default());
        let result = handler
            .execute_with_timeout::<_, String, &str>(Duration::from_secs(1), async { Ok("success".to_string()) })
            .await;
        assert_eq!(result.unwrap(), "success");
    }

    #[tokio::test]
    async fn timeout_handler_execute_timeout() {
        let handler = TimeoutHandler::new(TimeoutConfig::default());
        let result = handler
            .execute_with_timeout::<_, String, &str>(Duration::from_millis(10), async {
                tokio::time::sleep(Duration::from_secs(10)).await;
                Ok("success".to_string())
            })
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_timeout);
    }

    #[tokio::test]
    async fn timeout_handler_execute_error() {
        let handler = TimeoutHandler::new(TimeoutConfig::default());
        let result = handler
            .execute_with_timeout::<_, String, &str>(Duration::from_secs(1), async { Err::<String, &str>("failed") })
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_timeout);
        assert!(err.last_error.is_some());
    }

    #[tokio::test]
    async fn timeout_handler_retry_success() {
        let config = TimeoutConfig {
            retry_count: 2,
            retry_delay_ms: 10,
            ..TimeoutConfig::default()
        };
        let handler = TimeoutHandler::new(config);

        let attempts = Arc::new(Mutex::new(0));
        let attempts_clone = attempts.clone();

        let result = handler
            .execute_with_retry(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let mut a = attempts.lock().await;
                    *a += 1;
                    if *a < 3 {
                        Err::<String, &str>("temporary failure")
                    } else {
                        Ok("success".to_string())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(*attempts.lock().await, 3);
    }

    #[tokio::test]
    async fn timeout_handler_retry_exhausted() {
        let config = TimeoutConfig {
            retry_count: 1,
            retry_delay_ms: 10,
            ..TimeoutConfig::default()
        };
        let handler = TimeoutHandler::new(config);

        let result = handler
            .execute_with_retry::<_, String, &str>(|| async { Err::<String, &str>("always fails") })
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(!err.is_timeout);
        assert_eq!(err.retries_attempted, 1);
    }
}