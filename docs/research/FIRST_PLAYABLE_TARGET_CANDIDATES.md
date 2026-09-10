# Realmforge — First Playable Target Candidates

**Status:** ACTIVE DECISION SUPPORT  
**Decision:** provisional target selected; no client support claim  
**Last reviewed:** 2026-09-10

## Purpose

Realmforge needs one retired-client + world-emulator pair that can carry the independent Gate from target-client startup all the way to world entry.

This document selects the shortest evidence-backed route to that vertical slice. It does **not** declare any WoW client supported.

## Selection rule

The first target must have:

1. a specific retired client build,
2. a credible world-emulator counterpart,
3. active enough world-side development to make integration work useful,
4. a path to first-party client captures,
5. no requirement that Realmforge copy another authentication implementation,
6. a realistic route from Gate -> realm discovery -> join -> world authentication.

The exact proprietary client/Gate wire behavior remains `CAPTURE-REQUIRED` until Realmforge reproduces it.

---

## CANDIDATE A — Cataclysm Classic 4.4.2.60895 + TrinityCore `cata_classic`

**Disposition:** `PROVISIONAL FIRST TARGET`  
**Gate protocol status:** `CAPTURE-REQUIRED`  
**World-emulator status:** `CORRELATED / STRONG`  
**Realmforge support status:** `NOT SUPPORTED YET`

### Evidence

**E-CATA-001 — official project repository description / current public repository state**  
Grade: `C` correlated implementation target.  
Source: `https://github.com/TrinityCore/TrinityCore`  
Observed 2026-09-10: the public repository describes `cata classic = 4.4.2.60895`.

**E-CATA-002 — current branch exists and is active**  
Grade: `C` correlated implementation target.  
Source: `https://github.com/TrinityCore/TrinityCore/tree/cata_classic`  
Observed 2026-09-10: `cata_classic` exists. GitHub reported branch head `237a375b8ba395953097f9cbe70969d95020d30d`, authored 2026-09-09.

**E-CATA-003 — project license**  
Grade: `C`.  
Source: TrinityCore README/COPYING.  
TrinityCore identifies its license as GPL-2.0. Realmforge should treat TrinityCore as an external emulator/integration target rather than importing its source into independent Gate.

### Why this is first

The world-server half already has a current public project explicitly associated with the exact 4.4.2.60895 build. That materially reduces the risk of completing authentication work and then discovering there is no credible world counterpart.

It is also an active branch rather than an abandoned compatibility experiment.

### What this evidence does NOT prove

It does not prove:

- how build 60895 performs initial login,
- whether its Gate-side transport is raw TCP, TLS, WebSocket, HTTP, or a combination,
- exact endpoint names,
- BGS framing,
- service identifiers,
- RPC method IDs,
- SRP parameters,
- token/ticket formats,
- session lifecycle,
- realm-list wire format,
- realm-join wire format,
- realm-side authentication material expected by this exact core revision.

All of those stay `UNKNOWN` or `CAPTURE-REQUIRED` until independently established.

### Acceptance path

```text
4.4.2.60895 target client
        ↓
Realmforge-controlled first-party capture
        ↓
minimum independently authored Gate protocol adapter
        ↓
Realmforge canonical realm catalog
        ↓
WorldAuthBridge
        ↓
TrinityCore cata_classic test realm
        ↓
character screen
        ↓
world entry
```

This candidate becomes the actual first supported client only after that path is reproduced and negative-path tests pass.

---

## CANDIDATE B — Classic 1.14.0.40618 + Frostshake/TrinityCoreClassic

**Disposition:** `SECONDARY`  
**Gate protocol status:** `CAPTURE-REQUIRED`  
**World-emulator status:** `CORRELATED`  
**Realmforge support status:** `NOT SUPPORTED YET`

### Evidence

Source: `https://github.com/Frostshake/TrinityCoreClassic/tree/vanilla_classic`

Its README states:

- development branch: `vanilla_classic`,
- supported client: `classic 1.14.0.40618`,
- current focus is modern Classic 1.14.0 compatibility.

GitHub reported branch head `1fc8f1ebe3d09fcb67194b99d0281f88188b4a0e` dated 2025-07-27 when inspected 2026-09-10.

### Why second

This is a valuable modern-Classic target, but the visible branch activity is older than TrinityCore's current `cata_classic` branch. It remains a strong second compatibility lane after Realmforge proves its architecture against one end-to-end modern Classic-era client.

---

## CANDIDATE C — Classic 1.13.2.31650

**Disposition:** `RESEARCH TARGET / WORLD COUNTERPART UNRESOLVED`  
**Client-build existence:** `CORRELATED / STRONG`  
**Gate protocol status:** `CAPTURE-REQUIRED`  
**World-emulator status:** `UNKNOWN`

### Evidence

Independent public records identify build `1.13.2.31650` as a real 2019 Classic build. It also appears in wowdev build-definition data.

This makes it useful for historical protocol research and comparison.

### Why not first

Realmforge has not yet identified a comparably strong, exact-build world-emulator target for this build. Authentication research without a world-entry counterpart would violate the first-playable selection rule.

---

# Locked working decision

Until contradicted by stronger evidence:

> **Realmforge's first playable compatibility campaign targets Cataclysm Classic 4.4.2.60895 with TrinityCore `cata_classic` as the external world-emulator integration target.**

This is a target selection, not a support claim.

## Implementation boundary

TrinityCore may be inspected as an interoperability target and its public behavior/configuration may inform the Realmforge Bridge contract subject to its license.

Independent Gate must not copy authentication/protocol implementation from TrinityCore or the covered interim Gate. Target-specific Gate behavior must be grounded in Realmforge first-party fixtures or appropriate public standards.

## Immediate work order

1. Create capture card `RF-COMPAT-001` for 4.4.2.60895.
2. Record exact client executable hash when the lab client is available.
3. Capture process start, DNS, connection destinations, transport establishment, and failure behavior with no guessed server protocol.
4. Promote only reviewed `realmforge.capture.v1` evidence to implementation-ready.
5. Implement the smallest adapter justified by the first capture.
6. Repeat capture/implementation until authentication and realm list succeed.
7. Trace realm join through TrinityCore.
8. Implement a Realmforge Bridge adapter around the independently established world-auth contract.
9. Prove character screen and world entry.
10. Add negative traces before declaring build support.
