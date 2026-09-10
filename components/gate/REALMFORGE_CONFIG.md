# Realmforge Gate Configuration

**Status:** active interim Gate contract  
**Scope:** `components/gate/bin/realmforge-gate-bgs-server` only

This document defines the Realmforge-facing configuration names for the inherited BGS compatibility process.

Realmforge-prefixed names are authoritative. Old environment names are accepted only as migration aliases and are logged when consumed.

## Runtime configuration

| Realmforge variable | Legacy alias | Default | Meaning |
|---|---|---|---|
| `REALMFORGE_GATE_DATABASE_URL` | `DATABASE_URL` | none | Required PostgreSQL URL for the current covered Gate derivative. This is **not** a decision that Realmforge Core must use PostgreSQL. |
| `REALMFORGE_GATE_WS_BIND` | `BIND_ADDR` | `127.0.0.1:8119` | BGS WebSocket listener. |
| `REALMFORGE_GATE_TCP_BIND` | `TCP_BIND_ADDR` | `127.0.0.1:1119` | Raw TCP/TLS BGS listener used by the inherited Classic path. |
| `REALMFORGE_GATE_MAX_LOGINS` | `MAX_BGS_LOGINS` | `5000` | Concurrent/queued BGS login capacity. Must be greater than zero. |
| `REALMFORGE_GATE_WORKER_THREADS` | `TOKIO_WORKER_THREADS` | host parallelism | Tokio worker count. Must be greater than zero. |
| `REALMFORGE_GATE_TLS_CERT` | `BGS_TLS_CERT` | unset | PEM certificate path for the raw BGS TLS listener. |
| `REALMFORGE_GATE_TLS_KEY` | `BGS_TLS_KEY` | unset | PEM private-key path for the raw BGS TLS listener. |

TLS certificate and key are a pair. Supplying only one is a startup error.

## Realm projection configuration

The temporary Gate-side realm projection is loaded in this order:

1. `REALMFORGE_REALMS_JSON`
2. `REALMFORGE_REALMS_FILE`
3. a one-realm migration fallback using `REALM_ADDRESS`, `REALM_PORT`, and optional `REALMFORGE_REALM_NAME`

`REALMFORGE_REALMS_JSON` and `REALMFORGE_REALMS_FILE` are mutually exclusive.

A complete two-realm example is committed at:

`examples/realmforge-realms.example.json`

### Build safety

`REALMFORGE_ALLOW_UNKNOWN_BUILDS` defaults to false.

When false, Gate advertises only realms whose `supportedBuilds` contain the connecting build and for which an exact build profile exists.

Setting the variable true is a **research override only**. It does not make that build supported and must not be used as compatibility evidence.

## Known inherited configuration debt

The current covered derivative still consumes inherited database-pool variables through `realmforge_gate_db::PoolConfig::from_env()`:

- `DB_POOL_MAX_CONNECTIONS`
- `DB_POOL_MIN_CONNECTIONS`
- `DB_POOL_ACQUIRE_TIMEOUT_SECS`
- `DB_POOL_MAX_LIFETIME_SECS`
- `DB_POOL_IDLE_TIMEOUT_SECS`

Those remain RF-G3 work. They should eventually move behind Realmforge configuration authority without implying any decision about Core's future storage technology.

Other account/OAuth processes inside the covered derivative also retain inherited configuration names. RF-G3 is not complete until each process has an explicit Realmforge-facing configuration surface or is removed from the product path.

## Boundary rule

This file describes **Gate runtime configuration**, not Realmforge's canonical realm model.

The local JSON/file registry is temporary. Once Core ↔ Gate transport/versioning is decided, Core should provide canonical realm state and Gate should project that state into client-family-specific wire responses.
