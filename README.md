# Realmforge

**Self-hosted realm infrastructure for preserved MMO clients.**

Realmforge is a control plane, launcher, client manager, realm orchestrator, emulator-adapter layer, and compatibility-services platform intended to make running private preserved game realms feel like operating a modern self-hosted service instead of assembling a pile of unrelated tools by hand.

The first compatibility target is retired World of Warcraft Classic client infrastructure. The long-term architecture is deliberately broader than one game, one emulator, or one authentication implementation.

## Status

**Phase:** D0 — project authority / evidence campaign

Production implementation is **not yet authorized as settled architecture**. The immediate job is to document the system tightly enough that implementation agents do not fill unresolved behavior with assumptions.

The project currently has two parallel goals:

1. Use an existing compatible authentication/BGS implementation as an interim subsystem where useful.
2. Build a source-independent reconstruction authority so every inherited component can eventually be replaced by Realmforge-owned code without needing to reopen the upstream implementation.

See [`docs/reconstruction/UPSTREAM_EXIT_LEDGER.md`](docs/reconstruction/UPSTREAM_EXIT_LEDGER.md) for the current reconstruction authority.

## Product shape

```text
Realmforge
│
├── Launcher / Client Manager
├── Control Center
├── Realm Orchestrator
├── Realm Registry
├── Emulator Adapter Layer
├── Deployment / Updates
├── Backup / Restore
├── Observability
├── Community / Invite Layer
│
└── Compatibility Gateway
    ├── Identity / credentials
    ├── OAuth / OIDC
    ├── Browser login
    ├── Game-client login
    ├── BGS transport
    ├── Session state
    ├── Realm-list / realm-join handoff
    └── Desktop-app SSO
```

## Core rule

Realmforge distinguishes **behavioral compatibility** from **implementation lineage**.

A component is not considered independently owned merely because it was renamed, moved, translated to another language, or heavily modified. The replacement path is:

```text
inherited implementation
        ↓
observable behavior documented
        ↓
independent evidence captured
        ↓
source-independent specification frozen
        ↓
independent implementation written
        ↓
real-client interoperability verified
        ↓
old implementation removed
        ↓
Realmforge-owned component
```

If an implementation engineer must reopen upstream source to finish a replacement, the reconstruction package is incomplete.

## Repository authority

Read these before changing architecture:

- [`docs/PROJECT_CHARTER.md`](docs/PROJECT_CHARTER.md)
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- [`docs/ROADMAP.md`](docs/ROADMAP.md)
- [`docs/DECISIONS.md`](docs/DECISIONS.md)
- [`docs/reconstruction/UPSTREAM_EXIT_LEDGER.md`](docs/reconstruction/UPSTREAM_EXIT_LEDGER.md)
- [`docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`](docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md)
- [`docs/LEGAL_AND_SOURCE_BOUNDARY.md`](docs/LEGAL_AND_SOURCE_BOUNDARY.md)
- [`docs/NEXT_CHAT_HANDOFF.md`](docs/NEXT_CHAT_HANDOFF.md)

## Current target architecture

Realmforge should make the common case approximately:

```text
Install Realmforge
      ↓
Open Control Center
      ↓
Create administrator
      ↓
Detect/import supported retired client
      ↓
Choose/install compatible emulator adapter
      ↓
Create realm
      ↓
Invite users
      ↓
Play
```

The platform should own the operational experience even when individual compatibility or emulator components are third-party software.

## Licensing

No Realmforge-wide software license has been selected yet. Do not assume that absence of a license grants permission to reuse Realmforge code.

Third-party components retain their own licenses and obligations. See [`docs/LEGAL_AND_SOURCE_BOUNDARY.md`](docs/LEGAL_AND_SOURCE_BOUNDARY.md).

## Project identity

**Realmforge** is the product name.

Working subsystem names:

- **Gate** — protocol/authentication compatibility gateway
- **Core** — control plane and canonical state
- **Forge** — realm lifecycle/orchestration
- **Client** — launcher and local client manager
- **Console** — administrator UI
- **Bridge** — emulator adapters

These names are working architecture labels, not locked branding.
