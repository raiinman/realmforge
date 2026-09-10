# Realmforge Project Charter

**Authority level:** D0 project intent  
**Status:** Active  
**Owner:** raiinman  
**Project:** Realmforge

## Mission

Realmforge will make self-hosted preserved MMO realms operable as a coherent platform rather than a collection of manually configured servers, databases, launchers, patches, and authentication services.

The first target is retired World of Warcraft Classic-era client compatibility, but Realmforge's control-plane architecture must not be permanently coupled to a single game, emulator core, client generation, operating system, or third-party authentication implementation.

## Product promise

For the normal administrator, Realmforge should collapse the experience to:

```text
install
  ↓
open console
  ↓
create admin
  ↓
import/detect client
  ↓
install/select realm core
  ↓
create realm
  ↓
invite players
  ↓
play
```

Advanced users may expose more controls, but the platform must not require advanced users merely to function.

## What Realmforge owns

Realmforge is intended to own the user experience and canonical orchestration around:

- platform identity,
- installation and upgrades,
- realm lifecycle,
- realm registry,
- client detection and profiles,
- health and diagnostics,
- backups and recovery,
- emulator adapters,
- administrator UI,
- launcher UX,
- invitations/community features,
- observability,
- policy and configuration,
- compatibility contracts.

Some protocol-compatibility or emulator implementations may initially be third-party components. They are dependencies, not the identity of Realmforge.

## First compatibility target

Retired Classic-generation World of Warcraft clients requiring modern Battle.net-style account/BGS infrastructure.

Target families to investigate independently:

- 1.13.x
- 1.14.x
- 2.5.x
- 3.4.x
- 4.4.x

No build is declared supported merely because another build in the same expansion works.

## Non-goals for D0

D0 does not authorize:

- claiming support that has not been reproduced,
- silently choosing an emulator core as permanent architecture,
- cloning Battle.net's entire commercial feature set,
- implementing a payment/store system without a product reason,
- copying proprietary Blizzard assets,
- treating third-party documentation as truth when real clients disagree,
- letting framework defaults decide security or protocol policy,
- calling renamed third-party code an independent implementation.

## Design principles

### 1. Evidence beats assumption

Real retired client behavior and independently generated captures outrank documentation claims.

### 2. Compatibility is an adapter problem

Game/client-specific protocol behavior belongs behind explicit compatibility contracts. Core platform state must not become a dump of one client's wire format.

### 3. One canonical realm model

Realmforge owns a normalized realm representation. Compatibility layers translate that model to specific client generations and emulator cores.

### 4. Replaceable dependencies

Any third-party subsystem important enough to threaten Realmforge's future must have:

- a named boundary,
- a replacement plan,
- a behavioral specification,
- an interoperability test suite.

### 5. Safe defaults

Homelab use should be easy without making public deployments reckless. Public exposure, remote administration, and destructive operations need explicit security controls.

### 6. No mystery automation

Every installer, migration, patch, update, backup, and realm operation must be observable and reversible where practical.

### 7. Client preservation, not client redistribution

Realmforge may detect and configure user-supplied supported clients. The project must not casually assume it can redistribute copyrighted game clients or assets.

## Architecture labels

Working names:

- **Core** — canonical control plane
- **Gate** — client-facing authentication/protocol compatibility
- **Forge** — realm lifecycle/orchestration
- **Bridge** — emulator adapter contract and implementations
- **Client** — local launcher/client-management application
- **Console** — administrator interface

These are architectural handles. They can change later without changing the system boundaries.

## Authority discipline

When a decision becomes locked, record it in `docs/DECISIONS.md` or a future ADR.

When research closes an unknown, update the relevant authority document instead of leaving contradictory handoffs around the repository.

When a new agent begins, it must read `docs/NEXT_CHAT_HANDOFF.md` and the authority documents linked there before changing design.

## Definition of success

Realmforge is successful when a technically competent user can deploy and operate multiple compatible realms without needing to understand every emulator database, client endpoint, authentication protocol, and operating-system quirk underneath the platform—and when maintainers can replace any inherited subsystem without dismantling the rest of the product.
