# Realmforge Gate — Covered Working Derivative

This directory is the **working derivative** used to turn the pinned Tavern compatibility implementation into Realmforge Gate.

It is derived from:

- upstream: `wowemulation-dev/tavern`
- pinned commit: `6f9158670ee7666bfae2be58b291dafcb45f12e7`
- source baseline: `third_party/gate-upstream/source/`
- license: **AGPL-3.0-only**

The original pinned source remains untouched under `third_party/gate-upstream/source/` so Realmforge always has a reproducible before/after baseline.

## Rules

1. Do not describe this directory as clean-room or independently implemented while upstream-derived code remains.
2. Keep the upstream license and notices intact.
3. Record Realmforge modifications in Git history and, when material, in this file.
4. Realmforge product state must not become coupled to Tavern-specific database or wire-protocol structures.
5. New product behavior should move toward the Core/Gate contract documented in `docs/ARCHITECTURE.md`.

## RF-G1 — Realm registry boundary

**Status:** COMPLETE and CI-verified.

The first production slice removed Tavern's hard-coded single-realm assumption from the BGS realm-list/join path and introduced a Realmforge-owned **Gate projection** boundary.

Implemented changes:

- new `realmforge_realms::RealmRegistry`,
- configurable Realmforge realm identity, address, and port,
- multiple realm definitions,
- deterministic WoW wire-address generation from region/site/index/flags,
- exact client build profiles for inherited build hypotheses 31650 and 40618,
- unknown builds fail closed by default,
- explicit research-only override for unknown builds,
- BGS realm-list responses generated from the registry,
- BGS realm-join requests resolved back to the selected registry entry,
- Gate observability/service identity began moving from Tavern to Realmforge,
- example two-realm registry under `examples/realmforge-realms.example.json`,
- regression coverage for realm projection, multi-realm parsing, build gating, join response fields, and unknown realm rejection.

Verification workflow run `34437936703` completed successfully with:

- pinned upstream baseline unchanged,
- `bgs-server` tests passing,
- full locked workspace/all-target test suite passing,
- PostgreSQL 16 available for integration tests.

## RF-G3 — typed Gate configuration

**Status:** ACTIVE; first BGS process slice CI-verified.

The BGS process now routes its core runtime settings through `realmforge_config::GateConfig` instead of reading unrelated inherited environment variables throughout startup.

Realmforge-prefixed settings take precedence. Legacy names remain migration aliases only and their use is logged.

The first typed configuration slice covers:

- PostgreSQL URL for the current Gate derivative,
- WebSocket bind address,
- raw TCP/TLS bind address,
- maximum BGS login capacity,
- Tokio worker count,
- TLS certificate/key pair validation.

The contract is documented in `REALMFORGE_CONFIG.md`.

Verification workflow run `34438409222` completed successfully with the frozen upstream baseline unchanged and the full locked workspace/all-target test suite passing.

RF-G3 remains open for DB-pool variables plus account-server/OAuth-server configuration surfaces.

## Known inherited behavior still present

The following code remains upstream-derived compatibility behavior and is intentionally visible as replacement work:

- `AuthRealmListTicket` literal,
- random 32-byte realm join secret,
- account ID text used as the temporary realm-join ticket,
- inherited account/OAuth/session persistence implementation,
- inherited BGS transport/RPC/protobuf implementation,
- inherited internal crate/package names,
- empty character-count and last-character responses,
- no realm/world-server authentication consumer yet.

None of those should be promoted into Realmforge Core policy merely because Gate currently uses them.

## Next Gate work

RF-G2 continues product-visible Realmforge identity cleanup while RF-G3 continues configuration migration. The higher-value playable-path work is first-emulator selection and the Gate → Bridge/world-auth contract.
