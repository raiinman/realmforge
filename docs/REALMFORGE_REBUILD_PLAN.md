# Realmforge Rebuild Plan

**Status:** ACTIVE  
**Branch:** `rebuild/realmforge-gate-foundation`  
**Purpose:** Turn the useful inherited compatibility implementation into the first functioning Realmforge subsystem without letting inherited architecture define the product.

## 1. Non-negotiable boundaries

- `third_party/gate-upstream/source/` is the frozen upstream evidence baseline.
- `components/gate/` is the current AGPL-covered working derivative.
- Realmforge Core's canonical models must not become aliases for BGS protobufs, Tavern database rows, or emulator SQL.
- Unknown client behavior remains explicit.
- Real-client evidence outranks inherited documentation.
- Covered-source refactors remain covered-source refactors; they are not called clean-room replacements.

## 2. Rebuild sequence

### RF-G0 — Baseline and quarantine — COMPLETE

- exact upstream commit pinned,
- complete source snapshot imported,
- AGPL boundary preserved,
- snapshot manifest recorded,
- untouched build/test/database baseline passed,
- working derivative created separately under `components/gate/`.

### RF-G1 — Realmforge realm registry boundary — COMPLETE

Goal: remove the single hard-coded Tavern realm from BGS realm-list/join behavior.

Delivered:

- Realmforge Gate realm registry module,
- configurable display name/address/port,
- multiple realm definitions,
- explicit per-build compatibility profiles,
- unknown builds fail closed by default,
- realm-list projection generated from registry,
- realm-join target resolved from registry,
- Gate metrics/service identity began moving to Realmforge,
- regression tests preserving inherited wire behavior,
- full locked workspace tests pass,
- pinned upstream baseline remains untouched.

Verification: GitHub Actions run `34437936703` completed successfully.

### RF-G2 — Realmforge Gate identity cleanup — PARALLEL

Goal: remove product-visible Tavern identity while preserving required legal provenance.

Work:

- user-facing account/login branding,
- service/log/metric names,
- configuration names,
- deployment/container names,
- local test identities,
- documentation entrypoints.

Internal crate/module renames happen only where they improve maintainability; a giant cosmetic rename is not a milestone.

### RF-G3 — Gate configuration authority — ACTIVE

Goal: stop relying on a loose pile of inherited environment variables.

First verified slice:

- introduced typed `realmforge_config::GateConfig` for the BGS process,
- Realmforge-prefixed runtime variables are authoritative,
- inherited names remain migration aliases only,
- use of legacy aliases is visible at startup,
- listener addresses, login capacity, worker count, database URL, and TLS pair are validated centrally,
- TLS cert/key must be supplied together,
- runtime defaults preserve the inherited compatibility ports,
- configuration contract documented in `components/gate/REALMFORGE_CONFIG.md`,
- full locked workspace tests remain green,
- pinned upstream baseline remains untouched.

Verification: GitHub Actions run `34438409222` completed successfully.

Remaining RF-G3 work:

- database-pool environment names,
- account-server configuration surface,
- OAuth-server configuration surface,
- secret references/rotation policy,
- eventual replacement of local realm JSON/file loading by the Core ↔ Gate contract.

### RF-G4 — Core ↔ Gate contract

Goal: make Gate consume Realmforge semantic state rather than owning product state.

Minimum contract:

```text
AuthenticateIdentity
GetGameAccountProjection
ListCompatibleRealms
CreateClientSession
IssueRealmJoin
DisconnectSession
```

This milestone must resolve P-011 before implementation locks the transport.

### RF-G5 — Bridge/world-auth completion

Goal: cross the boundary Tavern never crossed: client login through actual world-server authentication.

Work:

- first emulator adapter selection,
- realm registration,
- join-ticket consumption,
- join-secret/session-key handoff,
- character counts / last-character integration where useful,
- end-to-end login → realm list → realm join → world auth fixture.

### RF-G6 — Real-client compatibility matrix

Goal: replace inherited support claims with Realmforge evidence.

For every supported build:

- exact binary/build identity,
- transport path,
- login path,
- BGS service/method trace,
- realm-list fixture,
- realm-join fixture,
- disconnect/error behavior,
- reconnect behavior,
- world-auth success,
- regression capture.

### RF-G7 — Replace inherited Gate internals

Goal: progressively remove upstream-derived implementation where the reconstruction ledger is sufficient.

Replacement order should be driven by architectural leverage, not cosmetics. The likely late replacement is BGS transport/RPC because it carries the most compatibility value and the least product identity.

## 3. Parallel product tracks

Gate is only one part of Realmforge. The following tracks remain separate and must not be improvised from Gate's internals:

```text
Core     canonical state/control API
Forge    realm lifecycle/orchestration
Bridge   emulator adapters
Client   local client discovery/configuration/launch
Console  administrator UX
```

Their implementation languages/frameworks remain open until their own decisions close.

## 4. First playable vertical slice

The first meaningful Realmforge success criterion is not “the UI loads.” It is:

```text
Realmforge config knows one realm
        ↓
Gate authenticates a known test account
        ↓
client receives Realmforge realm list
        ↓
client selects Realmforge realm
        ↓
Gate issues a join handoff
        ↓
Bridge/emulator accepts world authentication
        ↓
character screen/world entry succeeds
```

Everything before that should serve this path or prove a boundary needed by it.

## 5. Current attack list

RF-G1 is complete and RF-G3 has begun. The next highest-value work is:

1. investigate the upstream-referenced 1.13.2.31650 interoperability research corpus and its provenance,
2. compare first emulator adapter candidates,
3. close the first adapter decision deliberately,
4. specify the Gate → Bridge/world-auth handoff contract,
5. build the first real-client capture fixture,
6. continue account/OAuth configuration cleanup where it directly supports that path.

RF-G2 branding/identity cleanup continues in parallel, but it must not delay world-auth progress.
