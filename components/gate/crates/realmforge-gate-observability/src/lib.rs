// SPDX-License-Identifier: AGPL-3.0-only

//! Shared OpenTelemetry metrics setup and Kubernetes health probes.
//!
//! Each binary calls [`init`] at startup (after the tracing subscriber) to
//! install an OTLP-exporting meter provider, then mounts [`health_router`]
//! and the HTTP metrics layer. The provider is held for the process lifetime
//! and flushed via [`shutdown`] on exit.
//!
//! The OTLP endpoint is read from the standard `OTEL_EXPORTER_OTLP_ENDPOINT`
//! environment variable (set `http://localhost:4318` for the local dev
//! collector). The transport is OTLP/HTTP (protobuf) via the blocking client,
//! which the SDK runs on a dedicated background thread.

use std::sync::Arc;

use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram};
use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;

mod health;
mod http;
mod pool;

pub use health::{HealthState, health_router};
pub use http::metrics_middleware;
pub use pool::register_pool_gauge;

/// Shared instrument handles, available globally after [`init`].
pub struct Instruments {
    /// HTTP requests handled, with method and status attributes.
    pub http_requests_total: Counter<u64>,
    /// HTTP request duration in seconds.
    pub http_request_duration: Histogram<f64>,
    /// SRP proof verification duration in seconds.
    pub srp_verify_duration: Histogram<f64>,
}

static INSTRUMENTS: std::sync::OnceLock<Arc<Instruments>> = std::sync::OnceLock::new();

/// Access the global instruments. Returns `None` before [`init`] runs, so call
/// sites degrade to no-ops when observability is not initialized.
pub fn instruments() -> Option<&'static Arc<Instruments>> {
    INSTRUMENTS.get()
}

/// Record an SRP proof-verification duration from a start instant.
pub fn record_srp_duration(start: std::time::Instant) {
    if let Some(i) = instruments() {
        i.srp_verify_duration
            .record(start.elapsed().as_secs_f64(), &[]);
    }
}

/// Build and install the OTLP-exporting meter provider.
///
/// `service_name` identifies this process in the metrics backend. Returns the
/// provider so the caller can [`shutdown`] it on exit.
pub fn init(service_name: &str) -> SdkMeterProvider {
    let exporter = MetricExporter::builder()
        .with_http()
        .build()
        .expect("build OTLP metric exporter");

    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_owned())
                .build(),
        )
        .build();

    global::set_meter_provider(provider.clone());

    let meter = global::meter("realmforge");
    if INSTRUMENTS
        .set(Arc::new(Instruments {
            http_requests_total: meter
                .u64_counter("http.server.request.count")
                .with_unit("{request}")
                .with_description("HTTP requests handled by the server")
                .build(),
            http_request_duration: meter
                .f64_histogram("http.server.request.duration")
                .with_unit("s")
                .with_description("HTTP request duration")
                .build(),
            srp_verify_duration: meter
                .f64_histogram("realmforge.srp.verify.duration")
                .with_unit("s")
                .with_description("SRP proof verification duration")
                .build(),
        }))
        .is_err()
    {
        panic!("observability instruments already initialized");
    }

    provider
}

/// Flush pending metrics and shut down the provider. Call on process exit.
pub fn shutdown(provider: SdkMeterProvider) {
    if let Err(e) = provider.shutdown() {
        tracing::error!(error = %e, "meter provider shutdown failed");
    }
}

/// Returns a future that resolves when the process receives SIGTERM or SIGINT.
/// Use with `axum::serve(...).with_graceful_shutdown(signal)`.
pub async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
