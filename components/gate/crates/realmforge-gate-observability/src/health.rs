// SPDX-License-Identifier: AGPL-3.0-only

//! Kubernetes liveness/readiness/startup probe handlers.
//!
//! `health` (liveness) is a static "ok": the process responds. `startup`
//! flips green once initialization (connect, migrate) completes. `ready`
//! additionally requires the database pool to acquire a connection, so the
//! orchestrator stops routing traffic to a pod that lost its database.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;

/// Shared state for the probe handlers. `pool` is `Some` for servers that use
/// a database and `None` for `realmforge-gate-bgs-server`.
pub struct HealthState {
    pool: Option<sqlx::PgPool>,
    started: AtomicBool,
}

impl HealthState {
    /// Construct probe state. The pool is cloned cheaply (it is `Arc`-backed).
    pub fn new(pool: Option<sqlx::PgPool>) -> Self {
        Self {
            pool,
            started: AtomicBool::new(false),
        }
    }

    /// Mark initialization complete. `startup` and `ready` stay 503 until this
    /// is called.
    pub fn mark_started(&self) {
        self.started.store(true, Ordering::Release);
    }
}

/// Build a top-level router exposing `/health`, `/ready`, and `/startup`.
///
/// Returns the router (already state-applied, so it merges into any app) and
/// the [`HealthState`] handle so the caller can call [`HealthState::mark_started`]
/// after its own initialization.
pub fn health_router(pool: Option<sqlx::PgPool>) -> (Router, Arc<HealthState>) {
    let state = Arc::new(HealthState::new(pool));
    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/startup", get(startup))
        .with_state(state.clone());
    (router, state)
}

/// Liveness: the process responds.
async fn health() -> &'static str {
    "ok"
}

/// Startup: initialization (connect, migrate) is complete.
async fn startup(State(s): State<Arc<HealthState>>) -> impl IntoResponse {
    if s.started.load(Ordering::Acquire) {
        (StatusCode::OK, "started")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "starting")
    }
}

/// Readiness: the database pool can acquire a connection.
async fn ready(State(s): State<Arc<HealthState>>) -> impl IntoResponse {
    if !s.started.load(Ordering::Acquire) {
        return (StatusCode::SERVICE_UNAVAILABLE, "starting");
    }
    match &s.pool {
        None => (StatusCode::OK, "ready"),
        Some(pool) => match pool.acquire().await {
            Ok(_guard) => (StatusCode::OK, "ready"),
            Err(e) => {
                tracing::warn!(error = %e, "readiness probe: database acquire failed");
                (StatusCode::SERVICE_UNAVAILABLE, "not ready")
            }
        },
    }
}
