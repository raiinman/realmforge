# Realmforge — Next Chat Handoff

**Repository:** `raiinman/realmforge`  
**Main authority:** `main`  
**Active rebuild branch:** `rebuild/realmforge-gate-foundation`  
**Current phase:** D0 authority continues in parallel with the first bounded Gate rebuild  
**Last updated:** 2026-09-09

## Read first

Before acting, read in this order:

1. `README.md`
2. `docs/PROJECT_CHARTER.md`
3. `docs/DECISIONS.md`
4. `docs/ARCHITECTURE.md`
5. `docs/REALMFORGE_REBUILD_PLAN.md`
6. `docs/LEGAL_AND_SOURCE_BOUNDARY.md`
7. `components/gate/REALMFORGE_DERIVATION.md`
8. `docs/reconstruction/TAVERN_EXIT_LEDGER.md`
9. `docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`
10. `docs/research/UPSTREAM_CAPABILITY_INVENTORY.md`
11. `docs/ROADMAP.md`

Do not replace repository authority with chat memory.

## Owner intent

Realmforge is a self-hosted MMO realm platform, not a cosmetic Tavern fork and not merely an authentication server.

The owner wants to reuse useful existing implementation where legally permitted **now**, while keeping enough source-independent evidence and reconstruction authority to replace inherited compatibility code later.

The reconstruction requirement remains strict:

> A future implementation engineer should be able to close the inherited source and still know what must be implemented, what remains unknown, how to reproduce the behavior, and how to prove compatibility.

## Locked architecture

- Realmforge owns product identity and operational UX.
- `Core` owns canonical product state.
- `Gate` contains retired-client compatibility/authentication/session concerns.
- `Forge` owns realm lifecycle/orchestration.
- `Bridge` owns emulator adapters.
- `Client` owns local client discovery/configuration/launch.
- `Console` owns administrator UX.
- Compatibility wire formats must not become Core's canonical model.
- Emulator-specific database/config assumptions must stay behind adapters.

## Source and license state

Pinned upstream:

- repository: `wowemulation-dev/tavern`
- commit: `6f9158670ee7666bfae2be58b291dafcb45f12e7`
- license: `AGPL-3.0-only`

The exact upstream snapshot is now imported and frozen at:

`third_party/gate-upstream/source/`

It contains 212 tracked files / 1,562,272 bytes and remains the untouched before/after evidence baseline.

The active working derivative is:

`components/gate/`

That directory is explicitly an **AGPL-3.0-only covered derivative**. Do not describe it as clean-room or independently implemented while upstream-derived code remains.

## Verified baseline

The untouched imported snapshot has passed its reproducible CI baseline:

- account/oauth/BGS server build: PASS,
- full workspace/all-target tests: PASS,
- PostgreSQL migration round-trip: PASS,
- frozen vendor source unchanged: PASS.

Baseline workflow run: `34436523285`.

This proves the inherited implementation builds and passes its own tests. It does **not** prove real-client interoperability or establish Realmforge support for any client build.

## RF-G1 — Realm registry boundary — COMPLETE

The first actual Realmforge rebuild slice is implemented on `rebuild/realmforge-gate-foundation` and CI-green.

What changed in `components/gate`:

- introduced `realmforge_realms::RealmRegistry`,
- removed the hard-coded single `Tavern Realm` from realm-list/join behavior,
- supports multiple Gate-side realm definitions,
- realm display name/address/port are configurable,
- exact build profiles exist for the inherited 31650 and 40618 hypotheses,
- unknown client builds fail closed by default,
- an explicit research-only override exists for unknown builds,
- realm-list payloads are projected from the registry,
- realm-join resolves the selected realm from that registry,
- Gate service/metric identity began moving from Tavern to Realmforge,
- new registry/join regression tests pass,
- full locked workspace tests pass,
- the frozen upstream snapshot remains untouched.

Verified rebuild workflow run: `34437936703`.

## Important inherited behavior still visible

Do not mistake these for final Realmforge policy:

- `AuthRealmListTicket` literal remains inherited compatibility behavior,
- realm join secret remains 32 random bytes,
- account ID text remains the temporary realm-join ticket,
- character counts remain empty,
- last-character metadata remains empty,
- world/realm-server authentication is still missing,
- RestoreSession / MarkSessionAlive remain research gaps,
- BGS v2 support remains conflicting evidence,
- browser ticket SSO gaps remain open.

## What has NOT happened yet

Do not claim otherwise:

- no Realmforge-wide license has been selected,
- no emulator adapter has been selected,
- no real target client has been independently captured by Realmforge,
- no client build is officially supported by Realmforge,
- no Bridge/world-auth integration exists,
- no character-select/world-entry vertical slice exists,
- no clean-room replacement implementation exists yet,
- Core ↔ Gate transport/versioning remains undecided.

## Highest-value next work

Do not spend the next pass on a giant cosmetic rename.

Advance the playable path in this order:

1. repair and normalize baseline/rebuild evidence authority where needed,
2. finish the Gate configuration surface around the new realm registry,
3. investigate provenance of the upstream-referenced 1.13.2.31650 interoperability corpus,
4. compare first emulator adapter candidates and close the adapter decision deliberately,
5. specify Gate → Bridge/world-auth handoff semantics,
6. build the first real-client capture fixture,
7. then connect login → Realmforge realm list → join → emulator world authentication.

RF-G2 product-visible identity cleanup may proceed alongside this work when it does not delay the vertical slice.

## Do not do these

- Do not edit `third_party/gate-upstream/source/`.
- Do not rename copied code and call it clean-room.
- Do not erase legally required provenance.
- Do not let Gate/Tavern database types become Realmforge Core.
- Do not claim BGS v2 is solved from inherited documentation.
- Do not claim 31650 or 40618 support until Realmforge independently reproduces real-client behavior.
- Do not implement social/commerce fluff before client → realm gameplay works.
- Do not pick Core/Forge/Client technology merely because Gate is currently Rust.
- Do not select a project-wide license without owner approval.
- Do not merge the active rebuild branch without owner approval.

## D0 completion target

D0 still closes only after:

- upstream inventory is sufficiently complete,
- client/build candidate matrix exists,
- threat model exists,
- first emulator comparison exists,
- data ownership model exists,
- major D0 ADRs exist,
- one hostile red-team pass finds no unnamed architecture hole that would force an implementation agent to improvise core policy.
