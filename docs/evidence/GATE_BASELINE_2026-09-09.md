# Gate Vendored Baseline — 2026-09-09

**Status:** PASS  
**Purpose:** Establish the untouched imported compatibility implementation as a reproducible regression baseline before Realmforge modifies it.

## Provenance

| Field | Value |
|---|---|
| Upstream | `wowemulation-dev/tavern` |
| Upstream commit | `6f9158670ee7666bfae2be58b291dafcb45f12e7` |
| Baseline workflow run | `34436523285` |
| Workflow trigger commit | `1a7a37f28e9095aec9391a94750ab8f569f2788d` |
| Vendored tracked files | `212` |
| Vendored snapshot bytes | `1,562,272` |
| Rust | `rustc 1.97.0 (2d8144b78 2026-07-07)` |
| Cargo | `cargo 1.97.0 (c980f4866 2026-06-30)` |
| PostgreSQL service | `16.15` |
| Runner OS | Ubuntu `24.04.4` |
| Runner image | `ubuntu-24.04` / `20260831.293.1` |

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

## Evidence repair note

The original generated markdown report contained blank table fields even though the actual workflow gates passed. The workflow used an unquoted shell heredoc containing markdown backticks; the shell interpreted the backtick-delimited text as command substitutions while writing the report.

This document repairs the **presentation layer only** from the recorded workflow run and logs. The baseline outcome itself is unchanged: all four enforced status values were `0`, the final enforcement step passed, and workflow run `34436523285` completed successfully.

The workflow generator is also corrected on the Realmforge rebuild branch so a future rerun cannot reproduce the malformed report.

## Interpretation

This report proves only the state of the **pinned inherited implementation** on the recorded CI environment.

A PASS does **not** mean Realmforge has independently verified a real retired game client or that every upstream support claim is correct. Real-client interoperability remains a separate evidence campaign.

It establishes something narrower and important: Realmforge has a known-good inherited starting point against which covered modifications can be regression-tested.

## Modification boundary

The verification workflow performed no source edits under `third_party/gate-upstream/source/`.

`vendor-diff` remained zero. The pinned source therefore qualifies as the untouched baseline used by the active `components/gate/` working derivative.
