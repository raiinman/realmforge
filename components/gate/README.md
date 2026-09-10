# Realmforge Gate

Realmforge Gate is the retired-client compatibility and authentication edge for Realmforge.

This working tree is currently an AGPL-3.0-only covered derivative while inherited compatibility code remains. Product identity, package names, runtime configuration, logs, metrics, and user-facing surfaces are Realmforge-owned. Exact upstream provenance is quarantined in `REALMFORGE_DERIVATION.md` and the frozen baseline under `third_party/gate-upstream/`.

## Responsibilities

- retired-client authentication and compatibility transports,
- session/ticket handling,
- client-facing realm-list and realm-join projection,
- typed Realmforge Gate runtime configuration,
- compatibility evidence and regression fixtures.

## Non-responsibilities

Gate does not own Realmforge canonical realm state, emulator-specific world state, or the administrator control plane. Those remain behind Core/Bridge/Forge boundaries.
