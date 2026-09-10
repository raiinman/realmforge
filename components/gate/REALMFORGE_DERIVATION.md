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

## Initial Realmforge rebuild target

The first production slice removes Tavern's hard-coded single-realm assumptions from the BGS realm-list/join path and introduces a Realmforge-owned Gate realm registry/projection boundary.
