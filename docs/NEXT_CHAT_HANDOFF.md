# Realmforge — Next Chat Handoff

**Repository:** `raiinman/realmforge`  
**Main authority:** `main`  
**Active branch:** `rebuild/realmforge-independent-gate`  
**Current phase:** independent Gate replacement  
**Last updated:** 2026-09-09

## Read first

1. `docs/DECISIONS.md`
2. `docs/INDEPENDENT_GATE_REBUILD.md`
3. `docs/ARCHITECTURE.md`
4. `docs/REALMFORGE_REBUILD_PLAN.md`
5. `docs/LEGAL_AND_SOURCE_BOUNDARY.md`
6. `docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`
7. `docs/ROADMAP.md`

Do not replace repository authority with chat memory.

## Owner clarification — highest authority

The final Realmforge Gate is to be **rebuilt as newly authored Realmforge implementation**, not obtained by progressively renaming or disguising inherited code.

D-0019 is locked by owner and supersedes cosmetic-refactor work as the primary implementation direction.

The final acceptance condition is replacement of inherited implementation code on the active product path.

## Critical wording

Do **not** advertise the new effort as a formal clean-room rewrite. Maintainers and prior agents have already viewed inherited implementation source.

Use this wording instead:

> independent reimplementation from public standards, Realmforge-owned requirements, black-box captures/fixtures, and independently documented interoperability behavior; no copying or source porting.

## Branch / PR state

- PR #1 was merged previously and established the interim Gate/evidence baseline.
- PR #2 (large inherited-product identity rename) is **closed without merge** as superseded by D-0019.
- Active work is now `rebuild/realmforge-independent-gate`.

## Covered interim code

`components/gate/` remains a covered interim implementation and must retain its applicable license/provenance while it exists.

It is no longer the target implementation.

Do not spend substantive effort making that tree look native unless required to keep an interoperability test fixture alive.

## Fresh implementation tree

Newly authored code begins at:

`components/gate-independent/`

Current independent foundation includes:

- `ClientBuild`,
- `RealmId`,
- `RealmEndpoint`,
- `RealmDescriptor`,
- `RealmCatalog`,
- exact-build fail-closed behavior,
- `IdentitySubject`,
- `GameAccountProjection`,
- `WorldAuthRequest`,
- `WorldAuthGrant`,
- `WorldAuthBridge` semantic contract,
- original unit tests for the new semantic layer.

This tree must not import from `components/gate/`.

## Implementation input rules

Allowed inputs:

- RFCs/public standards,
- Realmforge architecture and requirements,
- Realmforge-owned black-box captures,
- independently documented wire behavior,
- public emulator interoperability contracts used as behavioral targets.

Disallowed:

- translating inherited functions,
- recreating inherited module structure,
- copying tests/comments/assets/templates,
- porting source with renamed identifiers,
- treating constants found only in inherited code as facts without independent confirmation.

Unknown behavior stays `UNKNOWN` until independently captured.

## No support claims yet

Do not claim any client build is officially supported yet.

Build numbers appearing in tests are compatibility hypotheses/fixtures until Realmforge reproduces real-client behavior independently.

## Highest-value next implementation sequence

1. Finish and keep CI green for `components/gate-independent/`.
2. Split the semantic foundation into explicit identity/session/realm/world-auth modules without consulting inherited source structure.
3. Define fixture formats for black-box client observations.
4. Implement OAuth/OIDC behavior strictly from public standards.
5. Independently capture the first retired-client login/realm-list/realm-join sequence.
6. Implement the minimum protocol adapter required by that capture.
7. Select the first emulator adapter deliberately.
8. Implement `WorldAuthBridge` for that emulator using its public interoperability contract.
9. Prove login → realm list → realm join → world auth end to end.
10. Replace remaining covered Gate capabilities one behavior family at a time.

## Do not do these

- do not reopen the cosmetic rename campaign,
- do not call renamed covered code independent,
- do not erase legally required historical provenance while covered code exists,
- do not use inherited implementation as a coding template,
- do not let protocol/emulator records become Core canonical state,
- do not select a Realmforge-wide license without owner approval,
- do not claim a client build supported before independent verification.

## Final source exit

When independently authored replacement coverage is complete:

1. remove `components/gate/` from the active product tree,
2. verify no final active crate imports covered code/assets,
3. run a source/text audit plus real-client regression suite,
4. retain accurate historical provenance for commits that contained covered code.

If the owner later wants old covered commits removed from Git history too, that is a separate destructive history rewrite and requires explicit approval.