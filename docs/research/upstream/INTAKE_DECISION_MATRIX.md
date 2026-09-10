# Interim Source Intake Decision Matrix

**Upstream:** `wowemulation-dev/tavern` @ `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Decision purpose:** maximize useful inherited functionality now without allowing the inherited implementation to define Realmforge's permanent architecture.

Decision vocabulary:

- `KEEP NOW` — vendor the implementation in the interim third-party Gate.
- `KEEP TO BUILD` — retain because the interim source requires it.
- `CAPTURE FIRST` — do not design a permanent replacement until external behavior is independently measured.
- `REWRITE EARLY` — useful behavior but low value to inherit long term.
- `DEFER` — not on the client-to-playable-realm critical path.
- `DO NOT ADOPT` — may exist in archival snapshot but must not become Realmforge product behavior.

---

# Executive decision

## Take a full pinned source snapshot

For the **interim source archive**, preserve the full pinned upstream repository rather than hand-selecting individual Rust files.

Reasons:

1. exact reproducibility,
2. migration/query cache dependencies,
3. regression tests stay matched to source,
4. build files stay matched to source,
5. provenance is obvious,
6. we do not discover six months later that an omitted helper was actually part of the inherited implementation.

## Use only a narrow runtime subset

Realmforge should initially execute only the pieces needed for:

```text
client login
  ↓
auth/session
  ↓
realm list
  ↓
realm join
```

Everything else can remain dormant archival source until a consumer is proven.

---

# Decision matrix

| Capability | Interim | Permanent Realmforge direction | Priority |
|---|---|---|---|
| BGS frame codec | KEEP NOW | independently replace | P0 replacement research |
| raw TCP BGS transport | KEEP NOW | independently replace | P0 |
| WebSocket BGS transport | KEEP NOW | verify/rebuild | P0 |
| BGS service hashing/routing | KEEP NOW | independently derive | P0 |
| BGS protobuf definitions | KEEP NOW | independently verify/source | P0 |
| AuthenticationService | KEEP NOW | capture + replace | P0 |
| AccountService BGS subset | KEEP NOW | capture only used methods + replace | P0/P1 |
| SessionService | KEEP NOW | capture + replace; current gaps | P0 |
| GameUtilities realm list | KEEP NOW | replace with dynamic Realmforge registry projection | P0 |
| GameUtilities realm join | KEEP NOW | replace after full world-side trace | P0 |
| queue system | KEEP NOW | retain semantics only if real client benefits | P1 |
| duplicate-session handling | KEEP NOW | independently verify policy | P1 |
| result/error codes | KEEP NOW | verify against client | P0/P1 |
| SRP crypto | KEEP NOW | independently reconstruct | P0 |
| browser SRP login | KEEP NOW | capture + replace | P0 |
| game bnet login | KEEP NOW | capture + replace | P0 |
| 1.13 external client login | KEEP NOW | capture + replace | P0 |
| one-time service tickets | KEEP NOW | independently specify/replace | P0 |
| BGS SSO tickets | KEEP NOW where functional | current gaps must be captured | P0 |
| OAuth authorization code/PKCE | KEEP NOW | REWRITE EARLY from standards | P1 |
| JWT/JWKS | KEEP NOW | REWRITE EARLY using first-party key policy | P1 |
| OAuth token exchange compatibility | KEEP NOW | preserve as compatibility shim after capture | P1/P0 if desktop app required |
| device authorization | dormant KEEP TO BUILD | standard rewrite if needed | P2 |
| account SSR UI | archival only | DO NOT ADOPT | — |
| Tavern branding/static visual assets | archival only | DO NOT ADOPT | — |
| upstream email UX/templates | archival only | Realmforge-owned | P2 |
| account creation flow | KEEP only if needed during interim | Realmforge-owned enrollment | P1 |
| fake SMS/captcha verification | archival only | **DO NOT ADOPT** | — |
| passkey placeholder API | archival only | independent WebAuthn if wanted | P2 |
| account address/profile fidelity | dormant | product decision | P2 |
| country/age catalog | dormant | DO NOT ADOPT without product need | P2 |
| communication preferences | dormant | product decision | P2 |
| privacy endpoint fidelity | dormant | product decision | P2 |
| third-party account connections | dormant | product decision | P2 |
| product catalog | dormant | DO NOT ADOPT by default | P2 |
| store/payments/wallet | stubbed/dormant | DEFER | P3 |
| social/friends/presence | not implemented/dormant | Realmforge product feature only if desired | P2 |
| observability crate | KEEP TO BUILD | REWRITE EARLY with Realmforge semantics | P1 |
| upstream Containerfiles | KEEP TO BUILD | Forge-owned deployment later | P1 |
| upstream load tests | KEEP for regression | build independent black-box harness | P0 research |
| upstream synthetic BGS client | KEEP for regression | never use as sole compatibility evidence | P0 research |
| upstream docs | archival/research | never product authority | — |
| upstream public signing key | archival only | **NEVER USE AS SECRET** | — |

---

# P0 source we want immediately available

The most valuable inherited implementation is approximately:

```text
bin/bgs-server/
crates/tavern-bgs/
crates/tavern-core/src/srp/
crates/tavern-account/src/bnet.rs
crates/tavern-account/src/bnet_types.rs
crates/tavern-account/src/client_login.rs
crates/tavern-account/src/login.rs
crates/tavern-account/src/ticket.rs
crates/tavern-db/ pieces those flows require
crates/tavern-oauth/ compatibility path
```

But importing only those files is **not** recommended. We should preserve the complete pinned repository under the third-party tree and only select these components in Realmforge integration.

---

# First refactors after source intake

These are legitimate Realmforge integration changes to the covered interim component. They remain covered source while derived from upstream.

## R1 — remove hard-coded single realm from runtime path

Current proof-of-concept owns a single static realm address/name/build projection.

Replace that with an internal interface such as:

```text
GateRealmProvider
  list_realms(client_profile, account)
  get_realm(realm_address)
  issue_join(realm, session)
```

The first adapter behind this interface can call Realmforge Core.

This gives us immediate multi-realm value without pretending the modified Gate is independently licensed.

## R2 — isolate compatibility account projection

Do not let Realmforge Core write directly into upstream account tables everywhere.

Define a narrow sync/provision interface:

```text
ensure_compatibility_account
ensure_game_account_projection
set_compatibility_entitlements
revoke_compatibility_session
```

## R3 — strip product routing dependence on upstream UI

Realmforge Console/Client should own normal enrollment/admin UX.

Keep upstream web endpoints only where the target client/desktop app genuinely needs their route/response behavior.

## R4 — replace upstream deployment shell

Forge should start/stop/health-check Gate as a managed component instead of telling operators to manually assemble Podman commands.

## R5 — externalize secrets

Never use the repository signing key. Realmforge generates deployment keys and injects them into Gate.

---

# Do not refactor these blindly before captures

The temptation will be to "clean up" strange protocol code. Do not do that before we know whether the strangeness is compatibility behavior.

Do not casually rewrite:

- SRP byte encoding,
- service hashes,
- protobuf field numbers,
- BGS framing,
- ticket prefixes/formats,
- cookie names/flags used by target software,
- realm-list JSON type prefixes,
- compressed blob framing,
- realm-join parameter names,
- error/status values,
- ordering of auth pushes/responses,
- session key widths,
- reconnect/session restore state.

Those all require evidence.

---

# Source-intake gate

A full pinned snapshot can be committed once the third-party provenance directory exists with:

```text
third_party/gate-upstream/
  UPSTREAM.md
  LICENSE.md
  SOURCE_REVISION
  source/...
```

`UPSTREAM.md` must record:

- upstream repo,
- exact revision,
- import date,
- exact license identifier,
- whether the snapshot is modified,
- where modifications are recorded,
- which Realmforge subsystem consumes it,
- current replacement status.

---

# After intake

The next milestone is not "start renaming Tavern things."

It is:

1. build the pinned source unchanged,
2. prove its tests pass,
3. deploy it isolated,
4. establish one baseline successful synthetic flow,
5. establish one baseline real-client flow,
6. record captures,
7. only then modify integration behavior.

That creates a baseline we can compare against instead of debugging our own changes and upstream assumptions simultaneously.
