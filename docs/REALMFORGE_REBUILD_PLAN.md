# Realmforge Rebuild Plan

**Status:** ACTIVE  
**Branch:** `rebuild/realmforge-independent-gate`  
**Purpose:** Replace the covered interim Gate with newly authored Realmforge implementation, verified from public standards and independent behavior evidence.

## 1. Non-negotiable boundaries

- `components/gate/` is temporary covered interoperability infrastructure, not the target implementation.
- `components/gate-independent/` is the fresh replacement tree.
- New implementation must not import, include, translate, or structurally port covered Gate source.
- Public standards, Realmforge-owned requirements, black-box captures, and independently documented interoperability behavior are the implementation inputs.
- Unknown client behavior remains explicit until independently captured.
- Real-client evidence outranks inherited documentation.
- This effort is not advertised as a formal clean-room rewrite because prior maintainers/agents have already viewed inherited source.
- Realmforge Core canonical state must not become a protocol or emulator schema mirror.

## 2. Rebuild sequence

### RF-I0 — Independent boundary and CI — ACTIVE

Goal: establish a source-independent implementation lane that can be audited separately.

Delivered so far:

- `docs/INDEPENDENT_GATE_REBUILD.md`,
- D-0019 owner authority,
- `components/gate-independent/`,
- fresh semantic types for builds, realms, endpoints, identities, game-account projection, and world-auth handoff,
- exact-build fail-closed catalog behavior,
- fresh unit tests,
- dedicated CI workflow.

Exit condition:

- CI green,
- no imports from `components/gate/`,
- source-independent input policy enforced.

### RF-I1 — Identity and session core

Goal: implement Realmforge-owned identity/session primitives without protocol coupling.

Work:

- identity subject model,
- game-account projection,
- session lifecycle,
- credential capability interfaces,
- token/session storage interfaces,
- explicit security/error model,
- deterministic tests.

Do not reproduce inherited database schema merely for convenience.

### RF-I2 — Standards-based web authentication

Goal: implement the browser/API auth surface from public standards.

Inputs:

- OAuth 2.0,
- OpenID Connect,
- PKCE,
- JWT/JWK,
- applicable token-exchange/device-flow standards if launch requirements need them.

Acceptance is standards conformance plus Realmforge-owned integration tests, not parity with covered source structure.

### RF-I3 — Realm registry and compatibility projection

Goal: implement the client-neutral realm catalog and build-compatibility authority needed by Gate.

Work:

- normalized realm descriptors,
- exact verified client-build compatibility,
- fail-closed unknown builds,
- realm visibility/maintenance state,
- protocol projection interface,
- tests that do not depend on covered code.

### RF-I4 — Independent client capture corpus

Goal: create Realmforge-owned evidence for the first retired client family.

For each captured flow record:

- exact client build identity,
- endpoint/transport sequence,
- request/response framing,
- service/method observations,
- realm-list behavior,
- realm-join behavior,
- disconnect/error behavior,
- world-auth inputs/outputs.

Captured facts become fixtures. Behaviors not observed remain `UNKNOWN`.

### RF-I5 — BGS transport/RPC replacement

Goal: implement only the protocol surface proven necessary by RF-I4.

Rules:

- fresh module design,
- no copied tests or function layout,
- constants require independent protocol evidence,
- unknown methods fail explicitly,
- wire fixtures are Realmforge-owned.

### RF-I6 — Realm list and realm join

Goal: reproduce the minimum real-client path from independent captures.

Acceptance:

```text
client authenticates
        ↓
client receives Realmforge realm list
        ↓
client selects verified-compatible realm
        ↓
Gate issues independently implemented join handoff
```

No support claim until the actual client reproduces the flow.

### RF-I7 — Bridge/world-auth

Goal: cross from Gate into an emulator without baking emulator schema into Gate/Core.

Work:

- deliberately select first emulator adapter,
- implement `WorldAuthBridge`,
- issue/consume join ticket and session material through the adapter contract,
- expose character-count/last-character data only if the adapter supports it,
- prove world authentication against the selected emulator.

### RF-I8 — First playable vertical slice

Success criterion:

```text
Realmforge knows one realm
        ↓
independent Gate authenticates test identity
        ↓
retired client receives realm list
        ↓
realm join succeeds
        ↓
Bridge/emulator accepts world authentication
        ↓
character screen/world entry succeeds
```

### RF-I9 — Covered Gate retirement

Goal: remove inherited implementation from the active product tree.

Required before deletion:

- launch-critical replacement parity proven by Realmforge-owned fixtures,
- real-client regression green,
- no active imports from covered code/assets,
- source/text audit performed,
- historical license/provenance records remain accurate.

Deleting the working tree does not erase historical Git commits. History rewrite, if desired later, is a separate destructive operation requiring explicit owner approval.

## 3. Interim covered code policy

The existing covered Gate may be run only when needed to:

- keep a temporary interoperability baseline available,
- compare externally observable behavior,
- support evidence collection that does not copy implementation.

Do not spend material effort on cosmetic renames, package reshuffling, or product-brand cleanup inside the covered tree unless it directly unblocks a replacement test.

PR #2 was closed without merge for this reason.

## 4. Parallel product tracks

These remain separate from protocol implementation:

```text
Core     canonical state/control API
Forge    realm lifecycle/orchestration
Bridge   emulator adapters
Client   local client discovery/configuration/launch
Console  administrator UX
```

Do not let the Gate replacement silently settle their language/framework decisions.

## 5. Current attack list

1. Keep independent Gate CI green.
2. Split the fresh semantic foundation into identity/session/realm/world-auth modules.
3. Add source-independent fixture schemas and evidence grading.
4. Implement the standards-based OAuth/OIDC core.
5. Produce the first Realmforge-owned retired-client capture.
6. Implement only the BGS surface that capture proves necessary.
7. Select and implement the first `WorldAuthBridge` adapter.
8. Reach the first client → character-screen/world-entry path.
9. Retire covered Gate behavior family by behavior family until it can leave the active tree.
