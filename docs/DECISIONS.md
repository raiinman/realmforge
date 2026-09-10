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

## D-0014 — Missing features should be explicit

**Status:** LOCKED  
**Decision:** Unknown, deferred, conflicting, and capture-required behavior must remain visibly labeled. Agents must not silently turn uncertainty into implementation policy.

---

# Pending decisions

These remain open and must not be filled implicitly:

| ID | Question | Status |
|---|---|---|
| P-001 | First production implementation language(s) | OPEN |
| P-002 | Desktop launcher framework | OPEN |
| P-003 | Core API framework | OPEN |
| P-004 | First emulator core/adaptor target | OPEN |
| P-005 | Interim third-party Gate import method | OPEN |
| P-006 | Whether desktop-app compatibility is launch-critical | OPEN |
| P-007 | Homelab-only versus public-hosting security profile at M1 | OPEN |
| P-008 | Realmforge-wide software license | OPEN |
| P-009 | Primary database for Core | OPEN |
| P-010 | Source repository layout once production code begins | OPEN |

When a pending decision closes, add a numbered locked decision above and update the handoff.
