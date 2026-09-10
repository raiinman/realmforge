// SPDX-License-Identifier: AGPL-3.0-only

//! Database pool, migrations, and repositories for Tavern.
//!
//! Backed by PostgreSQL via `sqlx`. This crate owns the connection pool and
//! the embedded migration set (under `migrations/`). Compile-time-checked
//! query repositories live in [`accounts`] and [`credentials`]; more tables
//! arrive in a later milestone. Pool and migration helpers are in [`connect`]
//! and [`run_migrations`].

pub mod account_licenses;
pub mod accounts;
pub mod authorization_codes;
pub mod bgs_sessions;
pub mod credentials;
pub mod device_authorizations;
pub mod game_accounts;
pub mod oauth_clients;
pub mod refresh_tokens;
pub mod service_tickets;
pub mod sessions;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;

/// Re-export so callers depend on one crate without a separate `sqlx` import.
pub use sqlx;

/// Errors returned by `tavern-db`.
#[derive(Debug, Error)]
pub enum DbError {
    /// A connection or query failure from `sqlx`.
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// A migration failure from `sqlx::migrate`.
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

/// Configuration for the sqlx connection pool.
///
/// Defaults mirror sqlx's own defaults, except `max_connections` which defaults
/// to 8 (the legacy Tavern value). All fields are configurable via environment
/// variables; see `tavern_core::Config`.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: std::time::Duration,
    pub max_lifetime: std::time::Duration,
    pub idle_timeout: std::time::Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 8,
            min_connections: 0,
            acquire_timeout: std::time::Duration::from_secs(30),
            max_lifetime: std::time::Duration::from_secs(1800),
            idle_timeout: std::time::Duration::from_secs(600),
        }
    }
}

impl PoolConfig {
    /// Build a pool configuration from the process environment.
    ///
    /// Reads `DB_POOL_MAX_CONNECTIONS`, `DB_POOL_MIN_CONNECTIONS`,
    /// `DB_POOL_ACQUIRE_TIMEOUT_SECS`, `DB_POOL_MAX_LIFETIME_SECS`,
    /// `DB_POOL_IDLE_TIMEOUT_SECS`. Missing or unparseable values fall back
    /// to the defaults.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Some(v) = parse_env_u32("DB_POOL_MAX_CONNECTIONS") {
            cfg.max_connections = v;
        }
        if let Some(v) = parse_env_u32("DB_POOL_MIN_CONNECTIONS") {
            cfg.min_connections = v;
        }
        if let Some(v) = parse_env_u64("DB_POOL_ACQUIRE_TIMEOUT_SECS") {
            cfg.acquire_timeout = std::time::Duration::from_secs(v);
        }
        if let Some(v) = parse_env_u64("DB_POOL_MAX_LIFETIME_SECS") {
            cfg.max_lifetime = std::time::Duration::from_secs(v);
        }
        if let Some(v) = parse_env_u64("DB_POOL_IDLE_TIMEOUT_SECS") {
            cfg.idle_timeout = std::time::Duration::from_secs(v);
        }
        cfg
    }
}

fn parse_env_u32(key: &str) -> Option<u32> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

fn parse_env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

/// Build a Postgres connection pool with explicit tuning.
pub async fn connect_with(db_url: &str, cfg: &PoolConfig) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(cfg.max_connections)
        .min_connections(cfg.min_connections)
        .acquire_timeout(cfg.acquire_timeout)
        .max_lifetime(cfg.max_lifetime)
        .idle_timeout(cfg.idle_timeout)
        .connect(db_url)
        .await?;
    Ok(pool)
}

/// Build a Postgres connection pool with legacy defaults.
///
/// Equivalent to `connect_with(db_url, &PoolConfig::default())`.
pub async fn connect(db_url: &str) -> Result<PgPool, DbError> {
    connect_with(db_url, &PoolConfig::default()).await
}

/// Apply all pending migrations from the embedded set.
///
/// Idempotent: `sqlx` records applied versions in `_sqlx_migrations`, so this
/// is safe to call on every startup. The binary layer decides whether to call
/// this (typically gated by an admin/auto-migrate flag).
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The integration tests require a live database. They are skipped when
    /// `DATABASE_URL` is unset so `cargo test` stays green without infra.
    fn db_url() -> Option<String> {
        std::env::var("DATABASE_URL")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    }

    #[tokio::test]
    async fn migrations_apply_and_round_trip() {
        let Some(url) = db_url() else {
            eprintln!("skipping: DATABASE_URL not set");
            return;
        };

        let pool = connect(&url).await.expect("connect");
        run_migrations(&pool).await.expect("migrate");

        // Use a transaction and roll back so repeated runs leave no rows.
        let mut tx = pool.begin().await.expect("begin tx");

        let (id,): (i64,) = sqlx::query_as("INSERT INTO accounts (email) VALUES ($1) RETURNING id")
            .bind("round-trip@example.com")
            .fetch_one(&mut *tx)
            .await
            .expect("insert account");

        assert!(id > 0);

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM accounts WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .expect("count");
        assert_eq!(count, 1);

        tx.rollback().await.expect("rollback");
    }
}
