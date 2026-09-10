// SPDX-License-Identifier: AGPL-3.0-only

//! `account-server` binary.
//!
//! Replaces `account.battle.net` + bnetserver. Loads config, connects to
//! Postgres, mounts the account service router (bnet login, web login,
//! management API, UI), and serves on `BIND_ADDR`.
//!
//! # Environment
//!
//! - `DATABASE_URL` — Postgres connection string (required).
//! - `BIND_ADDR` — socket address to bind (default `127.0.0.1:8080`).

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

/// TLS-wrapping TCP listener so `axum::serve` can serve HTTPS directly.
///
/// Accepts a TCP connection, performs the TLS handshake, and hands the
/// resulting [`TlsStream`] to axum. The client (WoW 1.13.x) dials
/// `https://<region>.wowemu.dev/client/login/external` and validates the
/// presented certificate against its embedded bundle (patched
/// `SignatureModulus` + served bundle) or the OS trust store.
struct TlsListener {
    tcp: tokio::net::TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
}

impl axum::serve::Listener for TlsListener {
    type Io = tokio_rustls::server::TlsStream<tokio::net::TcpStream>;
    type Addr = std::net::SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            let (stream, addr) = match self.tcp.accept().await {
                Ok(accepted) => accepted,
                Err(e) => {
                    tracing::warn!("TCP accept error: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    continue;
                }
            };
            match self.acceptor.accept(stream).await {
                Ok(tls_stream) => return (tls_stream, addr),
                Err(e) => {
                    tracing::warn!(%addr, "TLS handshake failed: {e}");
                    continue;
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.tcp.local_addr()
    }
}

/// Build a rustls server config from a PEM certificate chain and private key.
fn load_server_config(
    cert_path: &std::path::Path,
    key_path: &std::path::Path,
) -> anyhow::Result<rustls::ServerConfig> {
    use std::io::BufReader;

    let certs = rustls_pemfile::certs(&mut BufReader::new(std::fs::File::open(cert_path)?))
        .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(std::fs::File::open(key_path)?))?
        .ok_or_else(|| anyhow::anyhow!("no private key found in {}", key_path.display()))?;
    Ok(rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The whole dependency tree pins rustls to the ring provider (see the
    // workspace Cargo.toml); install it before any TLS use.
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install rustls CryptoProvider");
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
            let concurrency_limit: usize = std::env::var("MAX_CONCURRENT_REQUESTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1024);
            CONCURRENCY_LIMIT
                .set(Arc::new(Semaphore::new(concurrency_limit)))
                .expect("concurrency limit already set");

            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
                )
                .init();

            let meter_provider = tavern_observability::init("account-server");
            let config = Config::from_env()?;
            tracing::info!(bind = %config.bind_addr, "starting account-server");

            let pool_cfg = tavern_db::PoolConfig::from_env();
            let pool = tavern_db::connect_with(&config.database_url, &pool_cfg).await?;
            tavern_db::run_migrations(&pool).await?;

            tavern_observability::register_pool_gauge(pool.clone());
            let (health_router, health_state) =
                tavern_observability::health_router(Some(pool.clone()));
            health_state.mark_started();
            let signing_key_pem =
                std::fs::read_to_string(&config.signing_key_path).unwrap_or_else(|_| {
                    tracing::warn!(
                        "no signing key at {}, tickets will use empty key",
                        config.signing_key_path.display()
                    );
                    String::new()
                });

            let state = tavern_account::app_state_with_region(
                pool,
                signing_key_pem,
                &config.region,
                &config.smtp_host,
                config.smtp_port,
                &config.smtp_from,
            );
            tavern_account::spawn_reaper(state.clone());

            let app = tavern_account::router(state)
                .layer(axum::middleware::from_fn(
                    tavern_observability::metrics_middleware,
                ))
                .merge(health_router)
                .layer(axum::middleware::from_fn(concurrency_middleware));
            let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
            let shutdown = tavern_observability::shutdown_signal();
            match (&config.tls_cert_path, &config.tls_key_path) {
                (Some(cert), Some(key)) => {
                    let tls_config = load_server_config(cert, key)?;
                    tracing::info!("listening on {} (TLS)", config.bind_addr);
                    let tls_listener = TlsListener {
                        tcp: listener,
                        acceptor: tokio_rustls::TlsAcceptor::from(Arc::new(tls_config)),
                    };
                    axum::serve(tls_listener, app)
                        .with_graceful_shutdown(shutdown)
                        .await?;
                }
                _ => {
                    tracing::info!("listening on {}", config.bind_addr);
                    axum::serve(listener, app)
                        .with_graceful_shutdown(shutdown)
                        .await?;
                }
            }
            tavern_observability::shutdown(meter_provider);

            Ok(())
        })
}
