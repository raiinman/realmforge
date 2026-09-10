// SPDX-License-Identifier: AGPL-3.0-only

//! Configuration loading and validation.

use std::path::PathBuf;

use crate::Error;

/// Environment keys read by [`Config::from_env`].
pub const KEY_DATABASE_URL: &str = "DATABASE_URL";
pub const KEY_BIND_ADDR: &str = "BIND_ADDR";
pub const KEY_ISSUER_URL: &str = "ISSUER_URL";
pub const KEY_SIGNING_KEY_PATH: &str = "SIGNING_KEY_PATH";
pub const KEY_SMTP_HOST: &str = "SMTP_HOST";
pub const KEY_SMTP_PORT: &str = "SMTP_PORT";
pub const KEY_SMTP_FROM: &str = "SMTP_FROM";
pub const KEY_REGION: &str = "REGION";
pub const KEY_DB_POOL_MAX_CONNECTIONS: &str = "DB_POOL_MAX_CONNECTIONS";
pub const KEY_DB_POOL_MIN_CONNECTIONS: &str = "DB_POOL_MIN_CONNECTIONS";
pub const KEY_DB_POOL_ACQUIRE_TIMEOUT_SECS: &str = "DB_POOL_ACQUIRE_TIMEOUT_SECS";
pub const KEY_DB_POOL_MAX_LIFETIME_SECS: &str = "DB_POOL_MAX_LIFETIME_SECS";
pub const KEY_DB_POOL_IDLE_TIMEOUT_SECS: &str = "DB_POOL_IDLE_TIMEOUT_SECS";
pub const KEY_TLS_CERT_PATH: &str = "TLS_CERT_PATH";
pub const KEY_TLS_KEY_PATH: &str = "TLS_KEY_PATH";

/// Runtime configuration for a Tavern service.
///
/// All services share the same shape; fields a given service does not use are
/// ignored. The primary source is the process environment, parsed by the pure
/// [`Config::from_pairs`] core and surfaced through [`Config::from_env`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Postgres connection string, for example
    /// `postgres://tavern:tavern@localhost:5432/tavern`. Required.
    pub database_url: String,

    /// Socket address the HTTP listener binds to.
    pub bind_addr: String,

    /// OAuth issuer URL, used as the `iss` claim and the discovery `issuer`.
    pub issuer_url: String,

    /// Filesystem path to the RSA signing key (PEM).
    pub signing_key_path: PathBuf,
    /// SMTP server hostname for sending emails.
    pub smtp_host: String,
    /// SMTP server port.
    pub smtp_port: u16,
    /// From address for outgoing emails.
    pub smtp_from: String,
    /// 2-letter region code for authorization codes and service tickets
    /// (e.g. `US`, `KR`). Defaults to `US`.
    /// 2-letter region code for authorization codes and service tickets
    /// (e.g. `US`, `KR`). Defaults to `US`.
    pub region: String,
    /// Override for the database pool max connections (default: 8).
    pub db_pool_max_connections: Option<u32>,
    /// Override for the database pool min connections (default: 0).
    pub db_pool_min_connections: Option<u32>,
    /// Override for the database pool acquire timeout in seconds (default: 30).
    pub db_pool_acquire_timeout_secs: Option<u64>,
    /// Override for the database pool max connection lifetime in seconds
    /// (default: 1800).
    pub db_pool_max_lifetime_secs: Option<u64>,
    /// Override for the database pool idle timeout in seconds (default: 600).
    pub db_pool_idle_timeout_secs: Option<u64>,
    /// TLS certificate chain file (PEM). Both this and `tls_key_path` must be
    /// set for the HTTP listener to serve HTTPS; unset keeps plain HTTP.
    pub tls_cert_path: Option<PathBuf>,
    /// TLS private key file (PEM). Both this and `tls_cert_path` must be set
    /// for the HTTP listener to serve HTTPS; unset keeps plain HTTP.
    pub tls_key_path: Option<PathBuf>,
}

impl Config {
    const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
    const DEFAULT_ISSUER_URL: &str = "http://localhost:8080";
    const DEFAULT_SIGNING_KEY_PATH: &str = "keys/signing.pem";
    const DEFAULT_SMTP_HOST: &str = "localhost";
    const DEFAULT_SMTP_PORT: u16 = 1025;
    const DEFAULT_SMTP_FROM: &str = "noreply@wowemu.dev";
    const DEFAULT_REGION: &str = "US";

    /// Parse configuration from an iterator of `(key, value)` pairs.
    ///
    /// This is the pure, testable core: it performs no I/O. Keys are the
    /// environment variable names. Unknown keys are ignored so a shared
    /// environment cannot cause unrelated entries to be rejected.
    pub fn from_pairs<K, V, I>(pairs: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut database_url = None;
        let mut bind_addr = None;
        let mut issuer_url = None;
        let mut signing_key_path = None;
        let mut smtp_host = None;
        let mut smtp_port = None;
        let mut smtp_from = None;
        let mut region = None;
        let mut db_pool_max_connections = None;
        let mut db_pool_min_connections = None;
        let mut db_pool_acquire_timeout_secs = None;
        let mut db_pool_max_lifetime_secs = None;
        let mut db_pool_idle_timeout_secs = None;
        let mut tls_cert_path = None;
        let mut tls_key_path = None;

        for (key, value) in pairs {
            match key.as_ref() {
                KEY_DATABASE_URL => database_url = Some(value.as_ref().to_owned()),
                KEY_BIND_ADDR => bind_addr = Some(value.as_ref().to_owned()),
                KEY_ISSUER_URL => issuer_url = Some(value.as_ref().to_owned()),
                KEY_SIGNING_KEY_PATH => signing_key_path = Some(value.as_ref().to_owned()),
                KEY_SMTP_HOST => smtp_host = Some(value.as_ref().to_owned()),
                KEY_SMTP_PORT => smtp_port = Some(value.as_ref().to_owned()),
                KEY_SMTP_FROM => smtp_from = Some(value.as_ref().to_owned()),
                KEY_REGION => region = Some(value.as_ref().to_owned()),
                KEY_DB_POOL_MAX_CONNECTIONS => {
                    db_pool_max_connections = Some(value.as_ref().to_owned())
                }
                KEY_DB_POOL_MIN_CONNECTIONS => {
                    db_pool_min_connections = Some(value.as_ref().to_owned())
                }
                KEY_DB_POOL_ACQUIRE_TIMEOUT_SECS => {
                    db_pool_acquire_timeout_secs = Some(value.as_ref().to_owned())
                }
                KEY_DB_POOL_MAX_LIFETIME_SECS => {
                    db_pool_max_lifetime_secs = Some(value.as_ref().to_owned())
                }
                KEY_DB_POOL_IDLE_TIMEOUT_SECS => {
                    db_pool_idle_timeout_secs = Some(value.as_ref().to_owned())
                }
                KEY_TLS_CERT_PATH => tls_cert_path = Some(value.as_ref().to_owned()),
                KEY_TLS_KEY_PATH => tls_key_path = Some(value.as_ref().to_owned()),
                _ => {}
            }
        }

        let database_url = validate_non_empty(
            KEY_DATABASE_URL,
            database_url.ok_or(Error::MissingConfig("DATABASE_URL"))?,
        )?;
        let bind_addr = bind_addr.unwrap_or_else(|| Self::DEFAULT_BIND_ADDR.to_owned());
        let issuer_url = issuer_url.unwrap_or_else(|| Self::DEFAULT_ISSUER_URL.to_owned());
        let signing_key_path = signing_key_path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(Self::DEFAULT_SIGNING_KEY_PATH));
        let smtp_host = smtp_host.unwrap_or_else(|| Self::DEFAULT_SMTP_HOST.to_owned());
        let smtp_port = smtp_port
            .and_then(|v| v.parse().ok())
            .unwrap_or(Self::DEFAULT_SMTP_PORT);
        let smtp_from = smtp_from.unwrap_or_else(|| Self::DEFAULT_SMTP_FROM.to_owned());
        let region = region
            .filter(|r| r.len() == 2)
            .unwrap_or_else(|| Self::DEFAULT_REGION.to_owned());

        Ok(Self {
            database_url,
            bind_addr,
            issuer_url,
            signing_key_path,
            smtp_host,
            smtp_port,
            smtp_from,
            region,
            db_pool_max_connections: db_pool_max_connections.and_then(|v| v.parse().ok()),
            db_pool_min_connections: db_pool_min_connections.and_then(|v| v.parse().ok()),
            db_pool_acquire_timeout_secs: db_pool_acquire_timeout_secs.and_then(|v| v.parse().ok()),
            db_pool_max_lifetime_secs: db_pool_max_lifetime_secs.and_then(|v| v.parse().ok()),
            db_pool_idle_timeout_secs: db_pool_idle_timeout_secs.and_then(|v| v.parse().ok()),
            tls_cert_path: tls_cert_path.map(PathBuf::from),
            tls_key_path: tls_key_path.map(PathBuf::from),
        })
    }

    /// Load configuration from the process environment.
    ///
    /// Reads [`KEY_DATABASE_URL`], [`KEY_BIND_ADDR`], [`KEY_ISSUER_URL`], and
    /// [`KEY_SIGNING_KEY_PATH`]. Thin wrapper around [`Config::from_pairs`].
    pub fn from_env() -> Result<Self, Error> {
        let pairs = [
            KEY_DATABASE_URL,
            KEY_BIND_ADDR,
            KEY_ISSUER_URL,
            KEY_SIGNING_KEY_PATH,
            KEY_SMTP_HOST,
            KEY_SMTP_PORT,
            KEY_SMTP_FROM,
            KEY_REGION,
            KEY_DB_POOL_MAX_CONNECTIONS,
            KEY_DB_POOL_MIN_CONNECTIONS,
            KEY_DB_POOL_ACQUIRE_TIMEOUT_SECS,
            KEY_DB_POOL_MAX_LIFETIME_SECS,
            KEY_DB_POOL_IDLE_TIMEOUT_SECS,
            KEY_TLS_CERT_PATH,
            KEY_TLS_KEY_PATH,
        ]
        .into_iter()
        .filter_map(|key| std::env::var(key).ok().map(|value| (key, value)));
        Self::from_pairs(pairs)
    }
}

fn validate_non_empty(key: &'static str, value: String) -> Result<String, Error> {
    if value.trim().is_empty() {
        return Err(Error::InvalidConfig {
            key,
            reason: "must not be empty".to_owned(),
        });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_with_only_required_key_applies_defaults() {
        let cfg = Config::from_pairs([(KEY_DATABASE_URL, "postgres://localhost/tavern")])
            .expect("valid config");
        assert_eq!(cfg.database_url, "postgres://localhost/tavern");
        assert_eq!(cfg.bind_addr, Config::DEFAULT_BIND_ADDR);
        assert_eq!(cfg.issuer_url, Config::DEFAULT_ISSUER_URL);
        assert_eq!(
            cfg.signing_key_path,
            PathBuf::from(Config::DEFAULT_SIGNING_KEY_PATH)
        );
    }

    #[test]
    fn missing_database_url_is_rejected() {
        let err = Config::from_pairs(Vec::<(&str, &str)>::new())
            .expect_err("missing DATABASE_URL should error");
        assert!(matches!(err, Error::MissingConfig("DATABASE_URL")));
        assert!(err.to_string().contains("DATABASE_URL"));
    }

    #[test]
    fn blank_database_url_is_rejected() {
        let err = Config::from_pairs([(KEY_DATABASE_URL, "   ")])
            .expect_err("blank DATABASE_URL should error");
        assert!(matches!(
            err,
            Error::InvalidConfig {
                key: "DATABASE_URL",
                ..
            }
        ));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let cfg = Config::from_pairs([
            (KEY_DATABASE_URL, "postgres://localhost/tavern"),
            ("UNRELATED", "noise"),
        ])
        .expect("valid config");
        assert_eq!(cfg.database_url, "postgres://localhost/tavern");
    }

    #[test]
    fn all_keys_override_defaults() {
        let cfg = Config::from_pairs([
            (KEY_DATABASE_URL, "postgres://localhost/tavern"),
            (KEY_BIND_ADDR, "0.0.0.0:9000"),
            (KEY_ISSUER_URL, "https://oauth.example"),
            (KEY_SIGNING_KEY_PATH, "/etc/tavern/key.pem"),
        ])
        .expect("valid config");
        assert_eq!(cfg.bind_addr, "0.0.0.0:9000");
        assert_eq!(cfg.issuer_url, "https://oauth.example");
        assert_eq!(cfg.signing_key_path, PathBuf::from("/etc/tavern/key.pem"));
    }

    #[test]
    fn tls_paths_are_optional_and_parse() {
        let cfg = Config::from_pairs([
            (KEY_DATABASE_URL, "postgres://localhost/tavern"),
            (KEY_TLS_CERT_PATH, "/etc/tavern/tls/cert.pem"),
            (KEY_TLS_KEY_PATH, "/etc/tavern/tls/key.pem"),
        ])
        .expect("valid config");
        assert_eq!(
            cfg.tls_cert_path,
            Some(PathBuf::from("/etc/tavern/tls/cert.pem"))
        );
        assert_eq!(
            cfg.tls_key_path,
            Some(PathBuf::from("/etc/tavern/tls/key.pem"))
        );

        let plain = Config::from_pairs([(KEY_DATABASE_URL, "postgres://localhost/tavern")])
            .expect("valid config");
        assert_eq!(plain.tls_cert_path, None);
        assert_eq!(plain.tls_key_path, None);
    }
}
