#!/usr/bin/env python3
"""Apply the first deterministic Realmforge refactor to Gate's BGS entrypoint.

This script intentionally patches only the covered working derivative under
components/gate. The pinned baseline in third_party is never edited.
"""

from pathlib import Path

PATH = Path("components/gate/bin/bgs-server/src/main.rs")
text = PATH.read_text()


def replace_once(old: str, new: str) -> None:
    global text
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected exactly one match, found {count}: {old[:100]!r}")
    text = text.replace(old, new, 1)


replace_once(
    "mod game_utilities;\nmod tcp_transport;",
    "mod game_utilities;\nmod realmforge_realms;\nmod tcp_transport;",
)

replace_once(
    '''    /// World-server address handed to clients via `Param_ServerAddresses`
    /// in the `Command_RealmJoinRequest_v1` response (env `REALM_ADDRESS`,
    /// `REALM_PORT`). The realm server itself is an external integration;
    /// tavern stops at the join handoff.
    pub realm_ip: String,
    pub realm_port: u16,
''',
    '''    /// Gate-side projection of Realmforge's realm registry. This is a
    /// temporary compatibility boundary until Core supplies the registry over
    /// an explicit service contract; it is not the canonical product model.
    pub realm_registry: realmforge_realms::RealmRegistry,
''',
)

replace_once(
    '''            let state = Arc::new(BgsState {
''',
    '''            let realm_registry = realmforge_realms::RealmRegistry::from_env()?;
            info!(
                realm_count = realm_registry.len(),
                "Realmforge Gate realm registry loaded"
            );

            let state = Arc::new(BgsState {
''',
)

replace_once(
    '''                active_accounts: dashmap::DashMap::new(),
                realm_ip: std::env::var("REALM_ADDRESS")
                    .unwrap_or_else(|_| "127.0.0.1".to_string()),
                realm_port: std::env::var("REALM_PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(8085),
                db,
''',
    '''                active_accounts: dashmap::DashMap::new(),
                realm_registry,
                db,
''',
)

replace_once(
    'let meter_provider = tavern_observability::init("bgs-server");',
    'let meter_provider = tavern_observability::init("realmforge-gate");',
)
replace_once(
    'let bgs_meter = opentelemetry::global::meter("tavern");',
    'let bgs_meter = opentelemetry::global::meter("realmforge.gate");',
)
text = text.replace('"tavern.bgs.', '"realmforge.gate.bgs.')
replace_once(
    'info!(%bind_addr, "BGS WebSocket server starting");',
    'info!(%bind_addr, "Realmforge Gate BGS WebSocket server starting");',
)

PATH.write_text(text)
