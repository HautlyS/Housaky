use super::traits::{ChatMessage, ChatRequest, ChatResponse};
use super::Provider;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Provider wrapper enforcing a maximum number of concurrent in-flight requests.
///
/// This is used to prevent hitting per-provider/model parallelism limits and to
/// enable graceful fallback to secondary providers when configured.
pub struct LimitProvider {
    inner: Box<dyn Provider>,
    semaphore: Arc<Semaphore>,
    label: String,
    /// If true: when saturated, immediately error instead of waiting.
    fail_fast: bool,
}

impl LimitProvider {
    /// Error prefix used to signal "saturated" to callers that want immediate failover.
    pub const SATURATED_ERROR_PREFIX: &'static str = "HOUSAKY_LIMIT_SATURATED";

    pub fn new(inner: Box<dyn Provider>, max_in_flight: usize, label: impl Into<String>) -> Self {
        Self::new_with_mode(inner, max_in_flight, label, false)
    }

    pub fn new_fail_fast(
        inner: Box<dyn Provider>,
        max_in_flight: usize,
        label: impl Into<String>,
    ) -> Self {
        Self::new_with_mode(inner, max_in_flight, label, true)
    }

    fn new_with_mode(
        inner: Box<dyn Provider>,
        max_in_flight: usize,
        label: impl Into<String>,
        fail_fast: bool,
    ) -> Self {
        let max = max_in_flight.max(1);
        Self {
            inner,
            semaphore: Arc::new(Semaphore::new(max)),
            label: label.into(),
            fail_fast,
        }
    }

    fn saturated_error(&self) -> anyhow::Error {
        anyhow::anyhow!("{}:{}", Self::SATURATED_ERROR_PREFIX, self.label)
    }
}

#[async_trait]
impl Provider for LimitProvider {
    async fn chat_with_system(
        &self,
        system_prompt: Option<&str>,
        message: &str,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let permit = if self.fail_fast {
            self.semaphore
                .clone()
                .try_acquire_owned()
                .map_err(|_| self.saturated_error())?
        } else {
            self.semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| anyhow::anyhow!("{} concurrency limiter closed", self.label))?
        };
        let res = self
            .inner
            .chat_with_system(system_prompt, message, model, temperature)
            .await;
        drop(permit);
        res
    }

    async fn chat_with_history(
        &self,
        messages: &[ChatMessage],
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<String> {
        let permit = if self.fail_fast {
            self.semaphore
                .clone()
                .try_acquire_owned()
                .map_err(|_| self.saturated_error())?
        } else {
            self.semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| anyhow::anyhow!("{} concurrency limiter closed", self.label))?
        };
        let res = self.inner.chat_with_history(messages, model, temperature).await;
        drop(permit);
        res
    }

    async fn chat(
        &self,
        request: ChatRequest<'_>,
        model: &str,
        temperature: f64,
    ) -> anyhow::Result<ChatResponse> {
        let permit = if self.fail_fast {
            self.semaphore
                .clone()
                .try_acquire_owned()
                .map_err(|_| self.saturated_error())?
        } else {
            self.semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| anyhow::anyhow!("{} concurrency limiter closed", self.label))?
        };
        let res = self.inner.chat(request, model, temperature).await;
        drop(permit);
        res
    }

    fn supports_native_tools(&self) -> bool {
        self.inner.supports_native_tools()
    }

    async fn warmup(&self) -> anyhow::Result<()> {
        // Warmup should not be concurrency-limited.
        self.inner.warmup().await
    }
}
