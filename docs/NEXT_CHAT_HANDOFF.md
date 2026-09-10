# Realmforge — Next Chat Handoff

**Repository:** `raiinman/realmforge`  
**Branch authority:** `main`  
**Current phase:** D0 — project authority / upstream capability inventory / compatibility evidence planning  
**Last updated:** 2026-09-09

## Read first

Before acting, read in this order:

1. `README.md`
2. `docs/PROJECT_CHARTER.md`
3. `docs/DECISIONS.md`
4. `docs/ARCHITECTURE.md`
5. `docs/LEGAL_AND_SOURCE_BOUNDARY.md`
6. `docs/reconstruction/TAVERN_EXIT_LEDGER.md`
7. `docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`
8. `docs/research/UPSTREAM_CAPABILITY_INVENTORY.md`
9. `docs/ROADMAP.md`

Do not replace these with chat memory.

## Owner intent

Realmforge is intended to become a much larger self-hosted realm platform, not a cosmetic fork of an authentication service.

The owner explicitly wants to reuse as much useful existing implementation as legally permitted **now**, while preserving a detailed enough source-independent ledger to independently rebuild important inherited pieces later.

The reconstruction requirement is strict:

> A future implementation engineer should be able to close the inherited source and still know what must be implemented, what remains unknown, how to reproduce the behavior, and how to prove compatibility.

## Locked architecture

- Realmforge owns product identity and operational UX.
- `Core` owns canonical product state.
- `Gate` contains client-facing compatibility/authentication concerns.
- `Forge` owns realm lifecycle/orchestration.
- `Bridge` owns emulator adapters.
- `Client` owns local client discovery/configuration/launch.
- `Console` owns administrator UX.
- Compatibility wire formats must not become Core's canonical model.
- Emulator-specific database/config assumptions must stay behind adapters.

## Important license correction

Pinned upstream `wowemulation-dev/tavern` revision:

`6f9158670ee7666bfae2be58b291dafcb45f12e7`

The pinned workspace declares:

`AGPL-3.0-only`

Do not repeat the earlier conversational shorthand `AGPL-3.0-or-later` as repository fact.

## Upstream state discovered so far

Pinned upstream workspace contains:

```text
crates/
  tavern-account
  tavern-bgs
  tavern-core
  tavern-db
  tavern-oauth
  tavern-observability

bin/
  account-server
  bgs-server
  oauth-server
```

High-value interim areas:

- BGS protocol/transport/dispatch,
- account/game-client login,
- OAuth proprietary compatibility,
- shared session/ticket plumbing,
- Postgres storage needed by Gate,
- TLS/config/observability needed to run Gate.

Avoid importing useless surface merely for file count:

- branding assets,
- commerce stubs,
- fake phone/captcha flows,
- unrelated visual account UI,
- social placeholders unless a real target consumes them.

## Critical upstream conflicts/gaps already found

### P0 / CONFLICT — BGS v2 status

`docs/plan.md` claims M0–M19 complete and later-client paths.

`docs/bgs-protocol-versions.md` still says v1 implemented / v2 planned.

No support claim until independently tested.

### P0 / CONFLICT — GenerateAuthToken

Plan calls it a stub; pinned BGS source dispatches v2 token-generation requests to an active handler.

Black-box it.

### P0 — Session resume/restore

Pinned BGS code contains a TODO around SSO token handling for RestoreSession dequeue behavior.

### P0 — MarkSessionAlive

Documented stub/deferred. Must capture real client behavior.

### P0 — browser SSO ticket entry

`GET /login/ticket-login` is documented as observed but not implemented.

### P0 — browser SSO generator

`GET /login/sso/generate` is documented as not implemented.

### P0 — world/realm server half

Upstream stops after pre-realm-join handoff. Realmforge needs an actual adapter/integration through realm-side authentication and gameplay.

### P1 — optional GameUtilities

`GetPlayerVariables` / `GetAchievementsFile` are no-op/unverified.

### P2 / DEFER by default

- friends/presence/clubs,
- commerce/wallet/store,
- phone verification,
- captcha,
- full 2FA compatibility,
- other non-game-critical account surfaces.

## What has NOT happened yet

Do not claim otherwise:

- No Tavern source has been imported into Realmforge yet.
- No production Realmforge runtime exists.
- No Realmforge-wide license has been selected.
- No emulator adapter has been selected.
- No real target client has been independently captured by Realmforge.
- No client build is officially supported by Realmforge.
- No clean-room replacement implementation exists yet.

## Highest-value next step

Do **one deep upstream code inventory pass** before importing source.

Create/extend authority that enumerates:

1. every upstream HTTP route,
2. every BGS service hash and method pair,
3. every protobuf message actually used,
4. every database migration/table,
5. every config/environment key,
6. every listener/port,
7. every hard-coded build/version rule,
8. every TODO/FIXME/no-op/stub,
9. every realm-list/join command,
10. every test category and synthetic assumption.

Write the result back into `docs/research/UPSTREAM_CAPABILITY_INVENTORY.md` or split large appendices under `docs/research/upstream/`.

## After the deep inventory

Then perform a deliberate source-intake decision:

```text
KEEP NOW
REWRITE IMMEDIATELY
DO NOT IMPORT
CAPTURE FIRST
DEFER
```

for every major upstream subsystem.

Only then import the useful covered source under a clearly marked third-party boundary with exact provenance and required license material.

## Do not do these

- Do not rename copied code and call it clean-room.
- Do not erase legally required provenance.
- Do not let upstream database types become Realmforge Core.
- Do not claim BGS v2 is solved from documentation alone.
- Do not implement social/commerce fluff before client->realm gameplay works.
- Do not pick a tech stack merely because upstream used Rust.
- Do not select a project-wide license without owner approval.
- Do not merge undocumented architecture assumptions into authority.

## D0 completion target

D0 can close only after:

- upstream inventory is detailed enough for deliberate intake,
- client/build candidate matrix exists,
- threat model exists,
- first emulator comparison exists,
- data ownership model exists,
- major D0 ADRs exist,
- one hostile red-team pass finds no unnamed architecture hole that would force an implementation agent to improvise core policy.
