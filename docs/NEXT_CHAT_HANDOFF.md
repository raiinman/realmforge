# Realmforge — Next Chat Handoff

**Repository:** `raiinman/realmforge`  
**Main authority:** `main`  
**Active branch:** `rebuild/first-playable-target`  
**Current phase:** first playable compatibility campaign  
**Last updated:** 2026-09-10

## Read first

1. `docs/DECISIONS.md`
2. `docs/research/FIRST_PLAYABLE_TARGET_CANDIDATES.md`
3. `docs/research/cards/RF-COMPAT-001-CATA-442-60895-FIRST-TRACE.md`
4. `docs/research/CAPTURE_FIXTURE_SPEC_V1.md`
5. `docs/INDEPENDENT_GATE_REBUILD.md`
6. `docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`
7. `docs/ARCHITECTURE.md`
8. `docs/LEGAL_AND_SOURCE_BOUNDARY.md`

Do not replace repository authority with chat memory.

## Highest owner authority

D-0019 remains locked by owner:

> The final Realmforge Gate is a newly authored Realmforge implementation, not a progressively renamed or disguised derivative.

Do **not** advertise this as a formal clean-room rewrite. Maintainers have previously viewed covered implementation source.

Use:

> independent reimplementation from public standards, Realmforge-owned requirements, black-box captures/fixtures, and independently documented interoperability behavior; no copying or source porting.

## Current merged implementation

The independently authored implementation lives in:

`components/gate-independent/`

Current merged capabilities include:

- realm/build semantic model with fail-closed exact-build behavior,
- identity/account/session contracts,
- `WorldAuthBridge` contract,
- OAuth authorization code flow,
- mandatory PKCE S256,
- registered exact redirect matching,
- single-use authorization codes,
- opaque access tokens,
- persistent Realmforge RS256 signing authority,
- OIDC discovery + JWKS,
- signed OIDC ID tokens with issuer/subject/audience/lifetime/optional nonce,
- standalone `realmforge-gate` HTTP process,
- Realmforge-native Argon2id browser login,
- cryptographically random browser sessions,
- `/session/login` and `/session/logout`,
- secure cookie policy,
- native password-hash helper,
- `realmforge.capture.v1` first-party evidence model and validation CLI.

The independent-source CI guard must remain active. `components/gate-independent/` must not import from or reference the covered Gate tree as implementation input.

## Recent merged PRs

- PR #9 — OIDC ID-token issuer — merged.
- PR #10 — complete OIDC authorization-code flow with signed ID tokens — merged.
- PR #11 — Realmforge-native browser login/session bootstrap — merged at `bfa3d00889e4c8336cca9942894d7d3436212fe0`.
- PR #12 — first-party compatibility capture fixture authority — merged at `a0bdad82ee9ac045be554eaa0740f1a42e6383b8`.

PR #2 remains closed without merge; do not revive the cosmetic inherited-code rename campaign.

## Current target decision

D-0020 selects the first playable vertical-slice emulator target:

- client candidate: **Cataclysm Classic 4.4.2.60895**,
- world emulator: **TrinityCore `cata_classic`**,
- status: `CANDIDATE / CAPTURE-REQUIRED`,
- Realmforge support claim: **NONE YET**.

Why: current public TrinityCore metadata explicitly maps `cata_classic` to `4.4.2.60895`, and its branch was active on 2026-09-09 when selected. This gives Realmforge a credible world-side counterpart for the first Gate → realm → world proof.

This does not make TrinityCore canonical Realmforge architecture. D-0009 still requires emulator portability behind Bridge contracts.

## Active research card

`RF-COMPAT-001-CATA-442-60895-FIRST-TRACE.md`

The next authority must come from a Realmforge-controlled target-client capture.

Do not implement a guessed Cata/BGS adapter before the card obtains implementation-ready first-party evidence.

## Capture authority

Target-specific observations use:

`realmforge.capture.v1`

Implementation-ready target-client fixtures require:

- exact build,
- executable SHA-256,
- first-party evidence grade,
- ordered observations,
- artifact hashes,
- no secret-bearing artifacts,
- completed redaction review.

Validate with:

```bash
cargo run --manifest-path components/gate-independent/Cargo.toml \
  --bin realmforge-gate-fixture-check -- manifest.json --implementation-ready
```

## Immediate execution sequence

1. Land `rebuild/first-playable-target` after review.
2. Add a generic Realmforge capture-initialization/observation utility; do not encode an assumed WoW transport.
3. Obtain a legally held 4.4.2.60895 lab client and record its executable SHA-256.
4. Run RF-COMPAT-001-A: cold process/DNS/connection capture.
5. Run RF-COMPAT-001-B only after A identifies the real endpoint behavior.
6. Promote reviewed evidence through the fixture validator.
7. Open the smallest protocol-boundary card justified by that evidence.
8. Implement only that boundary in `components/gate-independent/`.
9. Recapture and iterate through login, session, realm list and realm join.
10. Implement the TrinityCore Bridge/world-auth adapter only from established interop contracts.
11. Prove character screen + world entry.
12. Add negative/replay/failure traces before declaring 4.4.2.60895 supported.

## Covered interim code

`components/gate/` and the frozen upstream evidence tree remain historically covered interim material while they exist.

They are **not** the target implementation.

Do not:

- translate their functions into the independent tree,
- copy tests/comments/assets,
- treat constants found only there as facts,
- erase required provenance while covered code remains,
- spend substantive time cosmetically renaming them.

## Final source exit

Once independent Gate proves the launch-critical client → realm → world path and replacement coverage is sufficient:

1. remove covered Gate from the active product tree,
2. verify no active Realmforge crate imports covered implementation/assets,
3. run source/text audit and real-client regression suite,
4. preserve accurate historical provenance for commits that actually contained covered code.

A destructive Git-history rewrite is separate and requires explicit owner approval.
