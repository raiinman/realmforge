# Realmforge Decision Log

**Purpose:** Record locked owner/architecture decisions so future agents do not reopen them casually.

## D-0001 — Project name

**Status:** LOCKED  
**Decision:** The project is named **Realmforge**.

## D-0002 — Product scope

**Status:** LOCKED  
**Decision:** Realmforge is not merely an authentication server. It is intended to become a self-hosted platform covering launcher/client management, control plane, realm orchestration, emulator adapters, compatibility services, operations, backups, and administration.

## D-0003 — First target

**Status:** LOCKED  
**Decision:** First compatibility target is retired World of Warcraft Classic-generation clients that expect modern Battle.net-style account/BGS infrastructure.

## D-0004 — Reuse policy

**Status:** LOCKED  
**Decision:** Existing third-party implementation code may be used as an interim component when its license permits it and when doing so materially accelerates Realmforge.

Third-party obligations must remain intact while derived code remains.

## D-0005 — Long-term implementation ownership

**Status:** LOCKED  
**Decision:** Realmforge must maintain a detailed source-independent reconstruction ledger sufficient to replace important inherited compatibility components later.

The desired future state is an implementation team that can finish and maintain replacements without reopening the inherited source.

## D-0006 — Fake-clean-room rejection

**Status:** LOCKED  
**Decision:** Renaming, translating, reorganizing, or heavily editing copied code is not considered independent reconstruction.

A component becomes independently owned only after the derived implementation has actually been replaced by independently implemented code and verified against external behavior.

## D-0007 — Architecture isolation

**Status:** LOCKED  
**Decision:** Client/protocol compatibility logic must not become Realmforge's canonical product model. It belongs behind Gate/compatibility contracts.

## D-0008 — Canonical realm model

**Status:** LOCKED  
**Decision:** Realmforge Core owns a normalized realm model. Emulator-specific and client-wire representations are projections/adapters.

## D-0009 — Emulator portability

**Status:** LOCKED  
**Decision:** Realmforge must use an explicit emulator adapter contract rather than permanently embedding one core's config/database assumptions into Core.

## D-0010 — No automatic project license

**Status:** LOCKED FOR D0  
**Decision:** Do not select a Realmforge-wide license without owner approval. Public repository visibility does not itself decide licensing.

## D-0011 — Evidence hierarchy

**Status:** LOCKED  
**Decision:** Independently reproduced real-client behavior outranks implementation documentation when they disagree.

## D-0012 — Support claims

**Status:** LOCKED  
**Decision:** No client build is declared supported until that build/family is tested under the Realmforge compatibility matrix. Similar builds do not inherit support automatically.

## D-0013 — D0 before production lock-in

**Status:** LOCKED  
**Decision:** Documentation/research authority comes before allowing implementation agents to fill architectural blanks with defaults.

A bounded interim Gate rebuild is allowed before D0 closes when it stays behind the already-locked Gate boundary, preserves explicit unknowns, and does not silently settle Core/Forge/Client technology choices.

## D-0014 — Missing features should be explicit

**Status:** LOCKED  
**Decision:** Unknown, deferred, conflicting, and capture-required behavior must remain visibly labeled. Agents must not silently turn uncertainty into implementation policy.

## D-0015 — Covered Gate working derivative

**Status:** LOCKED  
**Decision:** The pinned upstream Tavern snapshot under `third_party/gate-upstream/source/` remains an untouched evidence baseline. Realmforge's interim production modifications occur in `components/gate/`, which is explicitly a **covered AGPL-3.0-only derivative** of the pinned upstream implementation.

`components/gate/` must retain required upstream license/provenance. Renaming or refactoring this tree does not make it independently owned.

**Closes:** P-005.

## D-0016 — Realm registry enters Gate through a projection boundary

**Status:** LOCKED FOR INTERIM GATE  
**Decision:** Tavern's hard-coded single-realm BGS behavior is not Realmforge architecture. Gate will resolve realm-list and realm-join behavior through a Realmforge-named registry/projection layer.

Until Core exposes the canonical registry over an explicit service contract, Gate may load a temporary local projection from Realmforge-specific configuration. That temporary configuration is not the canonical Core Realm model and must remain replaceable.

## D-0017 — Unknown client builds fail closed by default

**Status:** LOCKED FOR INTERIM GATE  
**Decision:** Gate must not silently advertise a realm to an unverified client build merely by substituting that build number into a known version tuple.

Exact build profiles are required by default. An unsafe compatibility override may exist for research, but it must be opt-in and must never count as a support claim.

## D-0018 — Realmforge-prefixed Gate configuration is authoritative

**Status:** LOCKED FOR INTERIM GATE  
**Decision:** New Gate runtime configuration uses explicit `REALMFORGE_GATE_*` names. Inherited Tavern-era environment names may remain temporarily as migration aliases, but Realmforge-prefixed values take precedence and use of a legacy alias must be observable.

This configuration authority is scoped to Gate. It does not decide Core's storage technology, deployment model, or Core ↔ Gate transport.

## D-0019 — Independent Gate replacement is the primary implementation path

**Status:** LOCKED BY OWNER  
**Decision:** The end state is not a renamed or progressively disguised derivative. Realmforge will rebuild the required Gate capabilities as newly authored Realmforge implementation so the active final subsystem no longer depends on inherited implementation code.

The covered `components/gate/` tree is temporary compatibility/evidence infrastructure only. Work that merely renames, reorganizes, or cosmetically cleans it does not advance the final replacement except where needed to keep the interim system testable.

Fresh replacement implementation begins under `components/gate-independent/` and must not import from the covered Gate tree. Implementation behavior must be justified by public standards, Realmforge-owned requirements, Realmforge-owned black-box captures/fixtures, and independently documented interoperability facts.

Because project maintainers and prior agents have already viewed inherited source, this effort must not be advertised as a formal clean-room process. It is an independent reimplementation with a strict no-copy/no-port rule.

The covered Gate may be removed from the active product tree only after independently authored replacement behavior reaches verified parity for the launch-critical path.

---

# Pending decisions

These remain open and must not be filled implicitly:

| ID | Question | Status |
|---|---|---|
| P-001 | First production implementation language(s) outside the inherited Gate derivative | OPEN |
| P-002 | Desktop launcher framework | OPEN |
| P-003 | Core API framework | OPEN |
| P-004 | First emulator core/adaptor target | OPEN |
| P-005 | Interim third-party Gate import method | **CLOSED — D-0015** |
| P-006 | Whether desktop-app compatibility is launch-critical | OPEN |
| P-007 | Homelab-only versus public-hosting security profile at M1 | OPEN |
| P-008 | Realmforge-wide software license | OPEN |
| P-009 | Primary database for Core | OPEN |
| P-010 | Source repository layout once production code expands beyond Gate | OPEN |
| P-011 | Core ↔ Gate service-contract transport and versioning | OPEN |

When a pending decision closes, add a numbered locked decision above and update the handoff.
