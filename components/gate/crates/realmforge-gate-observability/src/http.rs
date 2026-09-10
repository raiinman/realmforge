// SPDX-License-Identifier: AGPL-3.0-only

//! HTTP metrics middleware: request count and duration histograms.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::instruments;
use opentelemetry::KeyValue;

/// `axum::middleware::from_fn` handler that records per-request metrics.
///
/// Records `http.server.request.count` (with method and status) and
/// `http.server.request.duration` (with method) using the global instruments.
/// No-ops when observability is not initialized.
pub async fn metrics_middleware(req: Request, next: Next) -> Response {
    let Some(instruments) = instruments() else {
        return next.run(req).await;
    };

    let method = req.method().clone();
    let start = Instant::now();
    let resp = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();
    let status = resp.status().as_u16();

    let method_attr = [KeyValue::new(
        "http.request.method",
        method.as_str().to_string(),
    )];
    instruments.http_requests_total.add(
        1,
        &[KeyValue::new("http.response.status_code", status as i64)],
    );
    instruments
        .http_request_duration
        .record(elapsed, &method_attr);

    resp
}
