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

## RF-G1 — first Realmforge production slice

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

RF-G2 focuses on product-visible Realmforge identity and RF-G3 on typed configuration authority. In parallel, the higher-value playable-path work is first-emulator selection and the Gate → Bridge/world-auth contract.
