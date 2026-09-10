# Realmforge Rebuild Plan

**Status:** ACTIVE  
**Current integration branch:** `rebuild/realmforge-bridge-world-auth`  
**Purpose:** Turn useful inherited compatibility behavior into a functioning Realmforge stack while steadily removing inherited product identity and implementation ownership.

## 1. Non-negotiable boundaries

- `third_party/gate-upstream/source/` is the frozen upstream evidence baseline and is not production identity.
- `components/gate/` is the current AGPL-covered working derivative until inherited internals are independently replaced.
- exact upstream naming/provenance belongs only in explicit legal/provenance authority while derived code remains.
- Realmforge Core models must never become aliases for compatibility protobufs, inherited database rows, or emulator SQL.
- emulator-specific storage/configuration stays behind Bridge.
- unknown client behavior remains explicit.
- real-client evidence outranks inherited documentation.
- covered-source refactors are not called clean-room replacements.

## 2. Rebuild sequence

### RF-G0 — Baseline and quarantine — COMPLETE

- exact upstream revision pinned,
- complete source snapshot imported,
- AGPL boundary preserved,
- snapshot manifest recorded,
- untouched build/test/database baseline passed,
- working derivative separated under `components/gate/`.

Baseline verification: `34436523285` — PASS.

### RF-G1 — Realmforge realm registry boundary — COMPLETE

Delivered:

- dynamic Realmforge Gate realm registry,
- configurable display name/address/port,
- multiple realm definitions,
- explicit build compatibility profiles,
- unknown builds fail closed by default,
- realm-list projection from registry,
- realm-join target resolution from registry,
- regression coverage,
- frozen baseline unchanged.

Verification: `34437936703` — PASS.

### RF-G2 — Active Gate product-identity exit — COMPLETE ON PR #2

The active Gate tree has completed its first inherited-product-identity removal pass:

- crates and binaries use `realmforge-gate-*` identity,
- active source/package metadata/logs/docs/templates no longer carry inherited product naming outside explicit provenance/license files,
- inherited logos were removed rather than renamed,
- intentionally blank Realmforge placeholders remain until native Realmforge visual assets are authored,
- reconstruction authority now uses `UPSTREAM_EXIT_LEDGER.md`,
- full workspace/all-target tests pass after the rename,
- a second `--locked` workspace pass succeeds,
- frozen upstream source remains unchanged.

Verification: `34439890791` — PASS.  
Open integration PR: `#2`.

This is product/package identity cleanup of covered code, not a clean-room claim. Replacement of inherited internals continues under RF-G7.

### RF-G3 — Gate configuration authority — ACTIVE

Verified BGS slice:

- typed `realmforge_config::GateConfig`,
- `REALMFORGE_GATE_*` variables authoritative,
- inherited names migration aliases only,
- alias use logged,
- listener/capacity/worker/database/TLS validation centralized.

Verification: `34438409222` — PASS.

Remaining:

- database-pool settings,
- account-server configuration,
- OAuth-server configuration,
- secret references/rotation policy,
- replace local realm JSON/file loading when Core ↔ Gate service transport closes.

### RF-G4 — Core ↔ Gate semantic contract — OPEN

Minimum semantics remain:

```text
AuthenticateIdentity
GetGameAccountProjection
ListCompatibleRealms
CreateClientSession
IssueRealmJoin
DisconnectSession
```

P-011 still blocks choosing the permanent transport/versioning mechanism. Do not let that block language-neutral contracts or test fixtures.

### RF-G5 — Bridge/world-auth completion — ACTIVE

First adapter target is locked by D-0019:

**TrinityCoreClassic 1.14.0.40618 family**.

World-auth evidence establishes the first concrete handoff:

```text
Realmforge game account
        |
        v
Bridge emulator account projection
        |
        +--> joinTicket = emulator account.username
        |
        +--> install Gate/client 64-byte session key
             into emulator account.session_key_bnet
        |
        v
Gate returns realm join response
        |
        v
40618 client connects to world server
        |
        v
world server validates CMSG_AUTH_SESSION
```

Observed emulator behavior transitions `session_key_bnet` from the 64-byte pre-world-auth BGS key to a derived 40-byte continued-session key after successful authentication. Bridge must treat preparation as a point-in-time handoff and must not continuously reconcile the original 64-byte value afterward.

Language-neutral contracts now live under `contracts/bridge/` and adapter authority under `docs/bridge/`.

Next implementation proof:

- resolve/link one test Realmforge game account to one emulator account projection,
- prepare a 64-byte world join,
- prove Gate emits the same key Bridge installed,
- prove the real 40618 client reaches character screen/world entry,
- capture reconnect/invalidation evidence.

### RF-G6 — Real-client compatibility matrix — OPEN

No build is supported merely because code or emulator documentation claims it.

For every supported build capture:

- exact binary/build identity,
- transport/login path,
- BGS service/method trace,
- realm-list fixture,
- realm-join fixture,
- world-auth success,
- disconnect/reconnect behavior,
- regression evidence.

40618 is first in line. 31650 remains research-only until separately reproduced.

### RF-G7 — Replace inherited Gate internals — ACTIVE IN PARALLEL

Continue replacing covered internals behind behavior tests. The goal is to shrink the legal/provenance quarantine until the inherited implementation can be removed entirely.

Replacement order follows architectural leverage rather than filename cosmetics. Protocol transport/RPC may remain late because it contains high compatibility value but little product policy.

## 3. First playable vertical slice

```text
Realmforge config knows one realm
        ↓
Gate authenticates a known Realmforge test account
        ↓
client receives Realmforge realm list
        ↓
client selects realm
        ↓
Bridge prepares emulator account + world-auth key
        ↓
Gate emits join ticket/session material
        ↓
emulator accepts CMSG_AUTH_SESSION
        ↓
character screen/world entry succeeds
```

That is the first meaningful success criterion. A UI loading is not.

## 4. Current attack list

1. finish Bridge semantic authority and adapter fixture for 40618,
2. implement/test a linked-account world-auth preparation path without leaking emulator SQL into Gate,
3. build the first real-client capture fixture,
4. prove login → realm list → join → world auth end to end,
5. capture failure/reconnect/invalidation semantics,
6. continue account/OAuth config migration where it supports the vertical slice,
7. continue removing inherited implementation and non-provenance naming as touched.

Do not pause the playable path for a repo-wide cosmetic sweep. Remove inherited identity and code ownership **as each area is touched**, while retaining only the minimum explicit legal/provenance record until replacement is complete.
