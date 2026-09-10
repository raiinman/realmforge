# Gate Vendored Baseline — 2026-09-09

**Status:** PASS  
**Purpose:** Establish the untouched imported compatibility implementation as a reproducible regression baseline before Realmforge modifies it.

## Provenance

| Field | Value |
|---|---|
| Upstream |  |
| Upstream commit |  |
| Realmforge workflow trigger |  |
| Vendored tracked files |  |
| Vendored snapshot bytes |  |
| Rust |  |
| Cargo |  |
| PostgreSQL client |  |
| Runner |  |

## Results

| Gate | Exit code | Result |
|---|---:|---|
| Build account/oauth/BGS servers |  | PASS |
| Full workspace/all-target test suite |  | PASS |
| Explicit DB migration round-trip smoke |  | PASS |
| Vendored source unchanged by verification |  | PASS |

## Evidence logs

- 
- 
- 
- 

## Interpretation

This report proves only the state of the **pinned inherited implementation** on the recorded CI environment.

A PASS does **not** mean Realmforge has independently verified a real retired game client or that every upstream support claim is correct. Real-client interoperability remains a separate evidence campaign.

A FAIL is still useful authority: it records a reproducible inherited-baseline failure before Realmforge integration modifications. Do not silently repair the vendor tree and rewrite history; document any subsequent baseline fix as a covered-source modification.

## Modification boundary

The verification workflow intentionally performs no source edits under .

 must remain zero for this report to qualify as an untouched baseline.
