# Realmforge Roadmap

**Status:** D0 planning authority  
**Rule:** Milestones are evidence gates, not calendar promises.

## D0 — Authority and evidence

Goal: define what Realmforge is before production code hardens accidental assumptions.

Required:

- [x] Project name and charter
- [x] Initial architecture boundary
- [x] Decision log
- [x] Source/legal boundary
- [x] Reconstruction/exit ledger
- [ ] Authoritative upstream capability inventory
- [ ] Supported-client candidate matrix
- [ ] Independent protocol evidence plan
- [ ] First emulator-core comparison
- [ ] Threat model
- [ ] Data ownership model
- [ ] First ADR set
- [ ] D0 red-team pass

Exit condition:

A new implementation agent can explain Core, Gate, Forge, Bridge, Client and Console; knows which decisions are locked; knows which protocol facts remain uncertain; and cannot silently treat a third-party codebase as product architecture.

---

## M0 — Deterministic Core contracts

Goal: create the smallest Realmforge-owned control-plane kernel with no real game dependency.

Target contracts:

- canonical `Realm`
- canonical `ClientBuild`
- canonical `ClientProfile`
- canonical `EmulatorInstallation`
- canonical `RealmRuntime`
- adapter capability model
- lifecycle state machine
- health state model
- backup model
- audit event model
- secret reference model

Deliverables:

- typed domain contracts,
- deterministic serialization,
- validation rules,
- migration/version strategy,
- unit/property tests,
- no external protocol implementation required.

Exit condition:

A fake emulator adapter and fake compatibility adapter can exercise a complete create/start/list/stop realm lifecycle without special-case code.

---

## M1 — Single-node homelab skeleton

Goal: one host, one database, one Realmforge console, fake or minimal realm runtime.

Deliverables:

- Core service,
- Console shell,
- local administrator bootstrap,
- realm CRUD,
- process lifecycle manager,
- structured logs,
- health checks,
- configuration persistence,
- local backup primitives,
- install/preflight diagnostics.

No claim of real retired-client support required yet.

---

## M2 — Bridge adapter contract + first real emulator

Goal: prove Realmforge can manage an actual emulator without embedding it into Core.

Deliverables:

- adapter SDK/contract,
- one selected emulator adapter,
- detect/install/validate/configure,
- start/stop/restart,
- logs/health,
- realm registration,
- backup/restore,
- upgrade/rollback research.

Exit condition:

Realm runtime lifecycle works entirely through the adapter boundary.

---

## M3 — Client manager

Goal: make the user's local game installation a first-class managed object.

Deliverables:

- client discovery,
- exact-build identification,
- client hash/metadata record,
- profile creation,
- safe endpoint configuration,
- backup/restore of changed local settings,
- launch preflight,
- connectivity diagnostics,
- one-click launch.

Exit condition:

Client preparation is reproducible and reversible.

---

## M4 — Interim Gate integration

Goal: connect a functioning third-party-compatible authentication/BGS implementation behind Realmforge's semantic boundary.

Deliverables:

- provenance record,
- isolated deployment/service boundary,
- Core-to-Gate semantic API,
- account projection,
- realm registry projection,
- health integration,
- session observability,
- required source/license notices,
- black-box baseline capture.

Exit condition:

A target retired client can authenticate and see a Realmforge-managed test realm through the interim compatibility component.

---

## M5 — First end-to-end realm

Goal: client -> Gate -> realm list -> join -> emulator -> playable world.

Deliverables:

- complete realm-join handoff,
- account mapping,
- session material transfer,
- build validation,
- join rejection behavior,
- reconnect path,
- launch through Realmforge Client,
- operator diagnostics.

Exit condition:

A fresh administrator can create a realm and a player can reach gameplay using documented supported client/build/core combination.

---

## M6 — Independent compatibility evidence corpus

Goal: make the replacement work possible without inherited source.

Research strikes:

1. exact build/transport matrix,
2. one full successful trace per family,
3. negative login trace set,
4. session keepalive/resume,
5. duplicate login/disconnect behavior,
6. realm-list and join traces,
7. desktop-app SSO if still in scope,
8. service/method map,
9. error-code matrix,
10. token/ticket lifecycle.

Deliverables:

- versioned capture index,
- redacted fixtures,
- field tables,
- protocol state machines,
- test vectors,
- independent harnesses.

---

## M7 — Gate replacement wave 1

Goal: replace low-risk/high-certainty inherited components first.

Likely candidates:

- standards-based OAuth/OIDC core,
- product identity projection,
- account API endpoints used only by Realmforge,
- observability,
- configuration/domain layer,
- standard WebAuthn/passkey functionality if desired.

Each replacement must pass:

`SPEC-READY -> IMPLEMENTED-INDEPENDENTLY -> INTEROP-VERIFIED -> TAVERN-FREE`

---

## M8 — Gate replacement wave 2

Goal: replace protocol-specific authentication/session pieces.

Candidates:

- SRP implementation,
- browser login compatibility,
- game-client HTTP login,
- ticket generation,
- session lifecycle,
- OAuth proprietary shim.

---

## M9 — Gate replacement wave 3

Goal: replace BGS transport/RPC and realm handoff.

This is expected to be the hardest wave.

Deliverables:

- independent v1 transport,
- independent v2 transport where required,
- service dispatcher,
- auth/session services,
- realm list,
- realm join,
- push/listener behavior,
- keepalive/reconnect,
- multi-build verification.

Exit condition:

No production compatibility binary requires the interim implementation for all advertised supported builds.

---

## M10 — Multi-realm operations

Goal: make Realmforge useful as a real homelab platform.

Deliverables:

- multiple realm hosts,
- multiple emulator installations,
- maintenance mode,
- scheduled backups,
- restore verification,
- health dashboards,
- log aggregation,
- player/account overview,
- per-realm compatibility policies.

---

## M11 — Community and invitations

Potential:

- invite links/codes,
- player enrollment,
- realm membership,
- role-based access,
- presence,
- announcements,
- optional friends/community features.

Do not implement proprietary social compatibility without a demonstrated consumer.

---

## M12 — Public deployment hardening

Only if public-hosting becomes a product goal.

Potential:

- MFA,
- WebAuthn,
- abuse controls,
- distributed rate limits,
- stronger audit policy,
- HA database guidance,
- external backup storage,
- reverse-proxy profiles,
- certificate automation,
- secret rotation,
- multi-host authorization.

---

# Cross-cutting gates

Every milestone must preserve:

- provenance clarity,
- no unsupported compatibility claims,
- reversible migrations,
- actionable operator errors,
- explicit security boundaries,
- adapter isolation,
- updated handoff documentation.

## No milestone by vibes

A milestone is complete only when its acceptance evidence exists in the repository. "Looks done" is not evidence.
