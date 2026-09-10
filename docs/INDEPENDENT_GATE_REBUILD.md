# Realmforge Independent Gate Rebuild

**Status:** ACTIVE  
**Branch:** `rebuild/realmforge-independent-gate`

## Goal

Replace the AGPL-covered interim Gate implementation with new Realmforge code so the final active Gate does not depend on inherited implementation code.

This is not a rename/refactor campaign. The acceptance condition is source replacement.

## Important honesty rule

The project history and prior investigation have already exposed maintainers and this ChatGPT session to the inherited implementation. We therefore do **not** label this process a formal clean-room rewrite.

Instead, the replacement is an **independent reimplementation from behavior and public specifications**:

- no copied source,
- no translated source,
- no line-by-line structural port,
- no copied tests,
- no copied comments, identifiers, templates, assets, or package layout,
- no implementation decisions justified only by the inherited source.

The inherited tree may remain quarantined only until replacement coverage is complete and license/provenance obligations for that historical code are satisfied.

## Allowed implementation inputs

Implementation work may use:

1. IETF and other public protocol standards (OAuth 2.0, OIDC, PKCE, JWT/JWK, WebSocket, TLS, HTTP),
2. Realmforge-owned requirements and architecture documents,
3. Realmforge-owned black-box captures and fixtures produced from legally obtained retired clients,
4. independently documented wire observations that describe behavior rather than copy implementation,
5. public emulator interface/database contracts used as interoperability targets, provided no source is copied,
6. public protocol schemas/specifications where reuse terms permit it.

## Disallowed implementation inputs

Do not use the inherited implementation as a coding template. In particular, do not:

- open a source file and reproduce the same function/module structure,
- rewrite source with renamed symbols,
- port tests by changing identifiers,
- copy constants unless independently confirmed as protocol facts,
- copy UI text/assets,
- copy SQL migration structure merely because the inherited implementation used it.

If a behavior is only known because of inherited code, mark it `UNKNOWN` and capture it independently before implementation.

## New replacement tree

Fresh implementation begins under:

`components/gate-independent/`

That tree must contain only newly authored Realmforge implementation and its own tests.

The existing `components/gate/` remains the covered interim implementation until replacement parity is proven. New code must not import from it.

## Replacement sequence

1. Define source-independent semantic contracts and error model.
2. Implement account/session primitives from public standards.
3. Implement realm registry and build compatibility projection from Realmforge authority.
4. Implement OAuth/OIDC surfaces from RFCs/specifications.
5. Implement BGS framing/RPC from independently captured behavior and public schema evidence.
6. Implement realm-list and realm-join behavior from Realmforge-owned captures.
7. Implement Bridge/world-auth handoff against the selected emulator adapter.
8. Run side-by-side black-box compatibility tests against the same fixtures.
9. Remove the covered interim Gate from the active product tree once every required behavior has an independently implemented replacement.

## Exit criteria

The covered interim Gate can leave the active tree only when:

- every launch-critical behavior has a replacement implementation,
- tests for the replacement were authored independently,
- real-client/fixture evidence is green,
- no replacement crate imports or includes covered Gate code/assets,
- a source audit finds no copied implementation text or inherited product assets,
- legal/provenance records for historical covered commits remain accurate.

## Repository-history note

Deleting the covered implementation from a future working tree does not erase older Git commits that contained it. If the owner later wants the repository history itself rewritten or the independent code moved to a fresh repository, that is a separate destructive repository operation and must be explicitly approved.