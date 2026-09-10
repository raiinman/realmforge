# Gate Vendored Baseline — 2026-09-09

**Status:** PASS  
**Purpose:** Establish the untouched imported compatibility implementation as a reproducible regression baseline before Realmforge modifies it.

## Provenance

| Field | Value |
|---|---|
| Upstream | `wowemulation-dev/tavern` |
| Upstream commit | `6f9158670ee7666bfae2be58b291dafcb45f12e7` |
| Realmforge workflow trigger | `cbd04793f1bf4b8d0bd7a682535eb2c5ffa3f252` |
| Vendored tracked files | `212` |
| Vendored snapshot bytes | `1562272` |
| Rust | `rustc 1.97.0 (2d8144b78 2026-07-07)` |
| Cargo | `cargo 1.97.0 (c980f4866 2026-06-30)` |
| PostgreSQL client | `psql (PostgreSQL) 16.15 (Ubuntu 16.15-1.pgdg24.04+2)` |
| Runner | `Linux runnervmlun5p 6.17.0-1022-azure #22-Ubuntu SMP Mon Jul 27 17:24:03 UTC 2026 x86_64 x86_64 x86_64 GNU/Linux` |

## Results

| Gate | Exit code | Result |
|---|---:|---|
| Build account/oauth/BGS servers | `0` | PASS |
| Full workspace/all-target test suite | `0` | PASS |
| Explicit DB migration round-trip smoke | `0` | PASS |
| Vendored source unchanged by verification | `0` | PASS |

## Evidence logs

- `docs/evidence/logs/gate-baseline/build.log`
- `docs/evidence/logs/gate-baseline/test.log`
- `docs/evidence/logs/gate-baseline/db-migration-smoke.log`
- `docs/evidence/logs/gate-baseline/vendor-diff.log`

## Interpretation

This report proves only the state of the **pinned inherited implementation** on the recorded CI environment.

A PASS does **not** mean Realmforge has independently verified a real retired game client or that every upstream support claim is correct. Real-client interoperability remains a separate evidence campaign.

A FAIL is still useful authority: it records a reproducible inherited-baseline failure before Realmforge integration modifications. Do not silently repair the vendor tree and rewrite history; document any subsequent baseline fix as a covered-source modification.

## Modification boundary

The verification workflow intentionally performs no source edits under `third_party/gate-upstream/source/`.

`vendor-diff` must remain zero for this report to qualify as an untouched baseline.
