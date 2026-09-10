// SPDX-License-Identifier: AGPL-3.0-only

//! Typed runtime configuration for Realmforge Gate's BGS process.
//!
//! Realmforge-prefixed variables are authoritative. Legacy variable names from
//! the inherited implementation are accepted as migration aliases only. This
//! keeps existing development setups usable without letting those names become
//! Realmforge's long-term configuration contract.

use std::collections::HashMap;
use std::net::SocketAddr;

use anyhow::{Context, bail};

pub const ENV_DATABASE_URL: &str = "REALMFORGE_GATE_DATABASE_URL";
pub const ENV_WS_BIND: &str = "REALMFORGE_GATE_WS_BIND";
pub const ENV_TCP_BIND: &str = "REALMFORGE_GATE_TCP_BIND";
pub const ENV_MAX_LOGINS: &str = "REALMFORGE_GATE_MAX_LOGINS";
pub const ENV_WORKER_THREADS: &str = "REALMFORGE_GATE_WORKER_THREADS";
pub const ENV_TLS_CERT: &str = "REALMFORGE_GATE_TLS_CERT";
pub const ENV_TLS_KEY: &str = "REALMFORGE_GATE_TLS_KEY";

const LEGACY_DATABASE_URL: &str = "DATABASE_URL";
const LEGACY_WS_BIND: &str = "BIND_ADDR";
const LEGACY_TCP_BIND: &str = "TCP_BIND_ADDR";
const LEGACY_MAX_LOGINS: &str = "MAX_BGS_LOGINS";
const LEGACY_WORKER_THREADS: &str = "TOKIO_WORKER_THREADS";
const LEGACY_TLS_CERT: &str = "BGS_TLS_CERT";
const LEGACY_TLS_KEY: &str = "BGS_TLS_KEY";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsFiles {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone)]
pub struct GateConfig {
    pub database_url: String,
    pub ws_bind_addr: SocketAddr,
    pub tcp_bind_addr: SocketAddr,
    pub max_logins: u64,
    pub worker_threads: usize,
    pub tcp_tls: Option<TlsFiles>,
    /// Records which deprecated compatibility aliases were actually consumed
    /// so startup can make migration debt visible instead of silently hiding it.
    pub legacy_aliases_used: Vec<&'static str>,
}

impl GateConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup<F>(lookup: F) -> anyhow::Result<Self>
    where
        F: Fn(&str) -> Option<String>,
    {
        let mut legacy_aliases_used = Vec::new();

        let database_url = required_with_legacy(
            &lookup,
            ENV_DATABASE_URL,
            LEGACY_DATABASE_URL,
            &mut legacy_aliases_used,
        )?;
        if database_url.trim().is_empty() {
            bail!("{ENV_DATABASE_URL} may not be empty");
        }

        let ws_bind_addr = optional_with_legacy(
            &lookup,
            ENV_WS_BIND,
            LEGACY_WS_BIND,
            &mut legacy_aliases_used,
        )
        .unwrap_or_else(|| "127.0.0.1:8119".to_string())
        .parse::<SocketAddr>()
        .with_context(|| format!("invalid {ENV_WS_BIND}"))?;

        let tcp_bind_addr = optional_with_legacy(
            &lookup,
            ENV_TCP_BIND,
            LEGACY_TCP_BIND,
            &mut legacy_aliases_used,
        )
        .unwrap_or_else(|| "127.0.0.1:1119".to_string())
        .parse::<SocketAddr>()
        .with_context(|| format!("invalid {ENV_TCP_BIND}"))?;

        let max_logins = optional_with_legacy(
            &lookup,
            ENV_MAX_LOGINS,
            LEGACY_MAX_LOGINS,
            &mut legacy_aliases_used,
        )
        .map(|raw| {
            raw.parse::<u64>()
                .with_context(|| format!("invalid {ENV_MAX_LOGINS}"))
        })
        .transpose()?
        .unwrap_or(5_000);
        if max_logins == 0 {
            bail!("{ENV_MAX_LOGINS} must be greater than zero");
        }

        let worker_threads = optional_with_legacy(
            &lookup,
            ENV_WORKER_THREADS,
            LEGACY_WORKER_THREADS,
            &mut legacy_aliases_used,
        )
        .map(|raw| {
            raw.parse::<usize>()
                .with_context(|| format!("invalid {ENV_WORKER_THREADS}"))
        })
        .transpose()?
        .unwrap_or_else(default_worker_threads);
        if worker_threads == 0 {
            bail!("{ENV_WORKER_THREADS} must be greater than zero");
        }

        let cert = optional_with_legacy(
            &lookup,
            ENV_TLS_CERT,
            LEGACY_TLS_CERT,
            &mut legacy_aliases_used,
        );
        let key = optional_with_legacy(
            &lookup,
            ENV_TLS_KEY,
            LEGACY_TLS_KEY,
            &mut legacy_aliases_used,
        );
        let tcp_tls = match (cert, key) {
            (None, None) => None,
            (Some(cert_path), Some(key_path)) => Some(TlsFiles {
                cert_path,
                key_path,
            }),
            (Some(_), None) => bail!("{ENV_TLS_CERT} is set but {ENV_TLS_KEY} is missing"),
            (None, Some(_)) => bail!("{ENV_TLS_KEY} is set but {ENV_TLS_CERT} is missing"),
        };

        Ok(Self {
            database_url,
            ws_bind_addr,
            tcp_bind_addr,
            max_logins,
            worker_threads,
            tcp_tls,
            legacy_aliases_used,
        })
    }
}

fn default_worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

fn required_with_legacy<F>(
    lookup: &F,
    primary: &'static str,
    legacy: &'static str,
    legacy_aliases_used: &mut Vec<&'static str>,
) -> anyhow::Result<String>
where
    F: Fn(&str) -> Option<String>,
{
    optional_with_legacy(lookup, primary, legacy, legacy_aliases_used)
        .with_context(|| format!("{primary} not set (legacy alias: {legacy})"))
}

fn optional_with_legacy<F>(
    lookup: &F,
    primary: &'static str,
    legacy: &'static str,
    legacy_aliases_used: &mut Vec<&'static str>,
) -> Option<String>
where
    F: Fn(&str) -> Option<String>,
{
    if let Some(value) = lookup(primary) {
        return Some(value);
    }
    let value = lookup(legacy);
    if value.is_some() {
        legacy_aliases_used.push(legacy);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_from(entries: &[(&str, &str)]) -> anyhow::Result<GateConfig> {
        let values: HashMap<String, String> = entries
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect();
        GateConfig::from_lookup(|key| values.get(key).cloned())
    }

    #[test]
    fn realmforge_names_take_precedence_over_legacy_aliases() {
        let config = config_from(&[
            (ENV_DATABASE_URL, "postgres://realmforge/new"),
            (LEGACY_DATABASE_URL, "postgres://legacy/old"),
            (ENV_WS_BIND, "127.0.0.1:9001"),
            (LEGACY_WS_BIND, "127.0.0.1:9002"),
        ])
        .unwrap();

        assert_eq!(config.database_url, "postgres://realmforge/new");
        assert_eq!(config.ws_bind_addr, "127.0.0.1:9001".parse().unwrap());
        assert!(!config.legacy_aliases_used.contains(&LEGACY_DATABASE_URL));
        assert!(!config.legacy_aliases_used.contains(&LEGACY_WS_BIND));
    }

    #[test]
    fn legacy_aliases_remain_usable_and_are_reported() {
        let config = config_from(&[
            (LEGACY_DATABASE_URL, "postgres://legacy/old"),
            (LEGACY_TCP_BIND, "127.0.0.1:21119"),
            (LEGACY_MAX_LOGINS, "123"),
        ])
        .unwrap();

        assert_eq!(config.database_url, "postgres://legacy/old");
        assert_eq!(config.tcp_bind_addr, "127.0.0.1:21119".parse().unwrap());
        assert_eq!(config.max_logins, 123);
        assert!(config.legacy_aliases_used.contains(&LEGACY_DATABASE_URL));
        assert!(config.legacy_aliases_used.contains(&LEGACY_TCP_BIND));
        assert!(config.legacy_aliases_used.contains(&LEGACY_MAX_LOGINS));
    }

    #[test]
    fn tls_requires_cert_and_key_as_a_pair() {
        let error = config_from(&[
            (ENV_DATABASE_URL, "postgres://realmforge/test"),
            (ENV_TLS_CERT, "/tmp/cert.pem"),
        ])
        .unwrap_err();
        assert!(error.to_string().contains(ENV_TLS_KEY));
    }

    #[test]
    fn rejects_zero_capacity_and_zero_workers() {
        let max_error = config_from(&[
            (ENV_DATABASE_URL, "postgres://realmforge/test"),
            (ENV_MAX_LOGINS, "0"),
        ])
        .unwrap_err();
        assert!(max_error.to_string().contains("greater than zero"));

        let worker_error = config_from(&[
            (ENV_DATABASE_URL, "postgres://realmforge/test"),
            (ENV_WORKER_THREADS, "0"),
        ])
        .unwrap_err();
        assert!(worker_error.to_string().contains("greater than zero"));
    }

    #[test]
    fn defaults_match_inherited_listener_contract() {
        let config = config_from(&[(ENV_DATABASE_URL, "postgres://realmforge/test")]).unwrap();
        assert_eq!(config.ws_bind_addr, "127.0.0.1:8119".parse().unwrap());
        assert_eq!(config.tcp_bind_addr, "127.0.0.1:1119".parse().unwrap());
        assert_eq!(config.max_logins, 5_000);
        assert!(config.tcp_tls.is_none());
    }
}
