#!/usr/bin/env python3
"""Route the covered Gate BGS process through Realmforge's typed config.

This is a bounded migration helper for `components/gate`; it never touches the
frozen third-party baseline.
"""

from pathlib import Path

PATH = Path("components/gate/bin/bgs-server/src/main.rs")
text = PATH.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one match, found {count}: {old[:120]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "mod game_utilities;\nmod realmforge_realms;\nmod tcp_transport;",
    "mod game_utilities;\nmod realmforge_config;\nmod realmforge_realms;\nmod tcp_transport;",
)

replace_once(
    '''    let workers = std::env::var("TOKIO_WORKER_THREADS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });
''',
    '''    let gate_config = realmforge_config::GateConfig::from_env()?;
    let workers = gate_config.worker_threads;
''',
)

replace_once(
    '''        .block_on(async {
''',
    '''        .block_on(async move {
''',
)

replace_once(
    '''            let meter_provider = tavern_observability::init("realmforge-gate");
            let bind_addr: SocketAddr = std::env::var("BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8119".to_string())
                .parse()
                .context("invalid BIND_ADDR")?;

            let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
''',
    '''            let meter_provider = tavern_observability::init("realmforge-gate");
            let bind_addr = gate_config.ws_bind_addr;
            let database_url = gate_config.database_url.clone();

            if !gate_config.legacy_aliases_used.is_empty() {
                warn!(
                    aliases = ?gate_config.legacy_aliases_used,
                    "legacy Gate configuration aliases are in use; migrate to REALMFORGE_GATE_* names"
                );
            }
''',
)

replace_once(
    '''                max_logins: std::env::var("MAX_BGS_LOGINS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(5000),
''',
    '''                max_logins: gate_config.max_logins,
''',
)

replace_once(
    '''            let tcp_bind_addr: SocketAddr = std::env::var("TCP_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:1119".to_string())
                .parse()
                .context("invalid TCP_BIND_ADDR")?;
            let tcp_tls = match (
                std::env::var("BGS_TLS_CERT").ok(),
                std::env::var("BGS_TLS_KEY").ok(),
            ) {
                (Some(cert), Some(key)) => Some(load_tls_server_config(&cert, &key)?),
                _ => None,
            };
''',
    '''            let tcp_bind_addr = gate_config.tcp_bind_addr;
            let tcp_tls = gate_config
                .tcp_tls
                .as_ref()
                .map(|tls| load_tls_server_config(&tls.cert_path, &tls.key_path))
                .transpose()?;
''',
)

# SocketAddr and anyhow::Context are no longer needed in main after moving
# parsing/validation into realmforge_config.
replace_once("use std::net::SocketAddr;\n", "")
replace_once("use anyhow::Context;\n", "")

PATH.write_text(text)
