// SPDX-License-Identifier: AGPL-3.0-only

//! `oauth-server` binary.
//!
//! Replaces `oauth.battle.net`. Loads config, connects to Postgres, mounts the
//! OIDC provider router, and serves on `BIND_ADDR`.
//!
//! # Environment
//!
//! - `DATABASE_URL` — Postgres connection string (required).
//! - `BIND_ADDR` — socket address to bind (default `127.0.0.1:8080`).
//! - `ISSUER_URL` — the OAuth issuer URL (default `http://localhost:8080`).
//! - `SIGNING_KEY_PATH` — path to a PKCS#8 RSA private key PEM (default
//!   `signing.pem`).

use tavern_core::Config;

use std::sync::{Arc, OnceLock};

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use tokio::sync::Semaphore;

static CONCURRENCY_LIMIT: OnceLock<Arc<Semaphore>> = OnceLock::new();

async fn concurrency_middleware(req: Request, next: Next) -> axum::response::Response {
    match CONCURRENCY_LIMIT
        .get()
        .expect("concurrency limit not initialized")
        .try_acquire()
    {
        Ok(permit) => {
            let resp = next.run(req).await;
            drop(permit);
            resp
        }
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, "server overloaded").into_response(),
    }
}

const SIGNING_KEY_FALLBACK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../keys/signing.pem"
));

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workers = std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()?
        .block_on(async {
    let concurrency_limit: usize = std::env::var("MAX_CONCURRENT_REQUESTS").ok().and_then(|v| v.parse().ok()).unwrap_or(1024);
    CONCURRENCY_LIMIT.set(Arc::new(Semaphore::new(concurrency_limit))).expect("concurrency limit already set");
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let meter_provider = tavern_observability::init("oauth-server");
    let config = Config::from_env()?;
    tracing::info!(bind = %config.bind_addr, issuer = %config.issuer_url, "starting oauth-server");

    let pool_cfg = tavern_db::PoolConfig::from_env();
    let pool = tavern_db::connect_with(&config.database_url, &pool_cfg).await?;
    tavern_db::run_migrations(&pool).await?;

    tavern_observability::register_pool_gauge(pool.clone());
    let (health_router, health_state) =
        tavern_observability::health_router(Some(pool.clone()));
    health_state.mark_started();
    let key_pem = match std::fs::read_to_string(&config.signing_key_path) {
        Ok(pem) => pem,
        Err(e) => {
            tracing::warn!(
                path = %config.signing_key_path.display(),
                error = %e,
                "could not read signing key, using embedded test key (NOT FOR PRODUCTION)"
            );
            SIGNING_KEY_FALLBACK.to_string()
        }
    };

    // Check the key metadata sidecar for demo/test warnings.
    if let Some(meta) = tavern_core::key_meta::KeyMeta::load(&config.signing_key_path) {
        if meta.is_demo() {
            tracing::warn!(
                kid = %meta.kid,
                environment = %meta.environment,
                note = meta.note.as_deref().unwrap_or(""),
                "using a DEMO signing key — do not use in production"
            );
        } else {
            tracing::info!(
                kid = %meta.kid,
                environment = %meta.environment,
                "signing key loaded"
            );
        }
    }

    let state = tavern_oauth::oauth_state_with_region(
        pool,
        config.issuer_url.clone(),
        &key_pem,
        &config.region,
    )?;

    let app = tavern_oauth::router(state)
        .layer(axum::middleware::from_fn(
            tavern_observability::metrics_middleware,
        ))
        .merge(health_router)
        .layer(axum::middleware::from_fn(concurrency_middleware));
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).with_graceful_shutdown(tavern_observability::shutdown_signal()).await?;

    tavern_observability::shutdown(meter_provider);

    Ok(())
    })
}
