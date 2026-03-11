//! OpenTelemetry integration for distributed tracing and metrics.
//!
//! This module provides:
//! - OTLP exporter configuration for traces and metrics
//! - Tracing subscriber initialization with OpenTelemetry layer
//! - Helper functions for creating spans and recording metrics

use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;

const OTEL_SERVICE_NAME: &str = "housaky";
const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4317";

#[derive(Debug, Clone)]
pub struct Metrics {
    pub total_requests: Arc<AtomicU64>,
    pub total_tokens: Arc<AtomicU64>,
    pub total_cost_cents: Arc<AtomicU64>,
    pub total_latency_ms: Arc<AtomicU64>,
    pub successful_requests: Arc<AtomicU64>,
    pub failed_requests: Arc<AtomicU64>,
    pub start_time: Instant,
    requests_per_model: Arc<parking_lot::RwLock<std::collections::HashMap<String, u64>>>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            total_tokens: Arc::new(AtomicU64::new(0)),
            total_cost_cents: Arc::new(AtomicU64::new(0)),
            total_latency_ms: Arc::new(AtomicU64::new(0)),
            successful_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            requests_per_model: Arc::new(
                parking_lot::RwLock::new(std::collections::HashMap::new()),
            ),
        }
    }

    pub fn record_request(
        &self,
        model: &str,
        tokens: u64,
        cost_usd: f64,
        latency_ms: u64,
        success: bool,
    ) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.total_cost_cents
            .fetch_add((cost_usd * 100.0) as u64, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);

        if success {
            self.successful_requests.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed_requests.fetch_add(1, Ordering::Relaxed);
        }

        {
            let mut models = self.requests_per_model.write();
            *models.entry(model.to_string()).or_insert(0) += 1;
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let total = self.total_requests.load(Ordering::Relaxed);
        let successful = self.successful_requests.load(Ordering::Relaxed);
        let total_latency = self.total_latency_ms.load(Ordering::Relaxed);

        let avg_latency_ms = if total > 0 { total_latency / total } else { 0 };

        let success_rate = if total > 0 {
            successful as f64 / total as f64
        } else {
            0.0
        };

        let total_cost = self.total_cost_cents.load(Ordering::Relaxed) as f64 / 100.0;

        let models: std::collections::HashMap<String, u64> =
            { self.requests_per_model.read().clone() };

        serde_json::json!({
            "total_requests": total,
            "total_tokens": self.total_tokens.load(Ordering::Relaxed),
            "total_cost": total_cost,
            "avg_latency_ms": avg_latency_ms,
            "success_rate": success_rate,
            "requests_per_model": models,
        })
    }
}

static METRICS: std::sync::OnceLock<Metrics> = std::sync::OnceLock::new();

pub fn global_metrics() -> &'static Metrics {
    METRICS.get_or_init(Metrics::new)
}

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub otlp_endpoint: String,
    pub enable_tracing: bool,
    pub enable_metrics: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: OTEL_SERVICE_NAME.to_string(),
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| DEFAULT_OTLP_ENDPOINT.to_string()),
            enable_tracing: true,
            enable_metrics: true,
        }
    }
}

pub fn init_telemetry(config: &TelemetryConfig) -> anyhow::Result<()> {
    let resource = Resource::builder()
        .with_service_name(config.service_name.clone())
        .build();

    if config.enable_tracing {
        let tracer_provider = create_tracer_provider(&config.otlp_endpoint, resource.clone())?;
        let tracer = tracer_provider.tracer(config.service_name.clone());

        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        let registry = tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
            .with(telemetry_layer);

        tracing::subscriber::set_global_default(registry)
            .map_err(|e| anyhow::anyhow!("Failed to set tracing subscriber: {}", e))?;

        tracing::info!(
            "OpenTelemetry tracing initialized (endpoint: {})",
            config.otlp_endpoint
        );
    } else {
        let registry = tracing_subscriber::registry()
            .with(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()));

        tracing::subscriber::set_global_default(registry)
            .map_err(|e| anyhow::anyhow!("Failed to set tracing subscriber: {}", e))?;
    }

    Ok(())
}

fn create_tracer_provider(endpoint: &str, resource: Resource) -> anyhow::Result<SdkTracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build())
}

pub fn create_meter_provider(
    endpoint: &str,
    resource: Resource,
) -> anyhow::Result<SdkMeterProvider> {
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    Ok(SdkMeterProvider::builder()
        .with_resource(resource)
        .with_periodic_exporter(exporter)
        .build())
}

pub fn shutdown_telemetry() {
    if let Err(e) = opentelemetry::global::shutdown_tracer_provider() {
        tracing::warn!("Error shutting down tracer provider: {}", e);
    }
}

#[macro_export]
macro_rules! instrument_span {
    ($name:expr, $attrs:expr) => {{
        use tracing::info_span;
        let span = info_span!($name);
        for (k, v) in $attrs {
            span.record(k, v);
        }
        span
    }};
}

pub use tracing::info_span;
pub use tracing::instrument;
pub use tracing::span;
pub use tracing::Level;
pub use tracing::Span;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_records_request() {
        let m = Metrics::new();
        m.record_request("gpt-4", 100, 0.05, 150, true);
        m.record_request("gpt-4", 200, 0.10, 200, true);
        m.record_request("claude-3", 150, 0.03, 100, false);

        assert_eq!(m.total_requests.load(Ordering::Relaxed), 3);
        assert_eq!(m.total_tokens.load(Ordering::Relaxed), 450);
        assert_eq!(m.successful_requests.load(Ordering::Relaxed), 2);
        assert_eq!(m.failed_requests.load(Ordering::Relaxed), 1);

        let json = m.to_json();
        assert_eq!(json["total_requests"], 3);
        assert_eq!(json["total_tokens"], 450);
        assert_eq!(json["total_cost"], 0.18);
        assert_eq!(json["avg_latency_ms"], 150);
    }

    #[test]
    fn metrics_requests_per_model() {
        let m = Metrics::new();
        m.record_request("gpt-4", 100, 0.01, 100, true);
        m.record_request("gpt-4", 100, 0.01, 100, true);
        m.record_request("claude-3", 100, 0.01, 100, true);

        let json = m.to_json();
        let models = json["requests_per_model"].as_object().unwrap();
        assert_eq!(models.get("gpt-4").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(models.get("claude-3").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn telemetry_config_defaults() {
        let c = TelemetryConfig::default();
        assert_eq!(c.service_name, "housaky");
        assert!(c.enable_tracing);
        assert!(c.enable_metrics);
    }

    #[test]
    fn global_metrics_singleton() {
        let m1 = global_metrics();
        let m2 = global_metrics();
        m1.record_request("test", 1, 0.01, 1, true);
        assert_eq!(m2.total_requests.load(Ordering::Relaxed), 1);
    }
}
