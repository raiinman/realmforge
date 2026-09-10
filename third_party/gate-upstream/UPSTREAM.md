# Gate Interim Upstream Provenance

**Component role:** Interim Realmforge Gate implementation / compatibility research baseline  
**Upstream repository:** `https://github.com/wowemulation-dev/tavern`  
**Pinned upstream revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Revision date:** 2026-09-08  
**Realmforge intake date:** 2026-09-09  
**Upstream workspace version:** `0.1.0`  
**Upstream declared license:** `AGPL-3.0-only`  
**Replacement status:** `TAVERN-DERIVED / IMPORTED-UNMODIFIED-BASELINE`

## Why this component exists

Realmforge needs a compatibility layer for retired World of Warcraft Classic-generation clients that expect Battle.net-style account, OAuth, BGS session, realm-list, and realm-join infrastructure.

The pinned upstream contains substantial working implementation in those areas. Realmforge will use that implementation as an interim Gate rather than immediately reproducing years of protocol work.

## Product relationship

This upstream project is **not** Realmforge's product identity.

Realmforge owns the broader control plane, realm orchestration, emulator adapters, client manager, launcher experience, administrator Console, operations, backups, and future independent compatibility implementation.

This directory exists to make the implementation lineage unambiguous while derived source remains.

## Planned source layout

```text
third_party/gate-upstream/
  UPSTREAM.md
  SOURCE_REVISION
  LICENSE.md
  source/        <- exact pinned source snapshot
```

The source snapshot should initially be preserved without cosmetic renaming. Realmforge-specific modifications to covered source must be documented and remain subject to the applicable upstream license.

## Modification log

At this provenance-file creation point:

- the exact pinned upstream working tree is vendored under `source/`, excluding only upstream `.git` metadata,
- `SNAPSHOT_MANIFEST.txt` records the upstream Git object identity of every tracked file,
- the snapshot is an unmodified baseline at intake,
- no Realmforge integration modification has yet been made to the covered source.

The Git commit containing this snapshot is the repository history commit produced by `.github/workflows/vendor-gate-upstream.yml` immediately after the workflow trigger commit.

## Intended first modifications after baseline verification

These changes are expected **after** the pinned source builds/tests as an unchanged baseline:

1. project dynamic Realmforge realm registry into GameUtilities instead of one hard-coded realm,
2. isolate Core↔Gate account provisioning behind semantic interfaces,
3. route Realmforge product UX around upstream branding/UI where protocol compatibility does not require it,
4. inject deployment-specific signing/TLS keys rather than using repository examples,
5. integrate Gate health/session telemetry into Realmforge operations.

These modifications do not make the covered Gate source independent. They remain part of the interim covered implementation until independently replaced.

## Replacement authority

See:

- `docs/reconstruction/TAVERN_EXIT_LEDGER.md`
- `docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`
- `docs/research/upstream/BGS_SURFACE.md`
- `docs/research/upstream/HTTP_SURFACE.md`
- `docs/research/upstream/DB_MIGRATIONS.md`
- `docs/research/upstream/CONFIG_SURFACE.md`
- `docs/research/upstream/INTAKE_DECISION_MATRIX.md`

## Independence gate

A future component may be called `TAVERN-FREE` only after the inherited implementation has actually been removed, its replacement was independently implemented from the source-independent reconstruction package, and real supported clients pass the interoperability matrix.
