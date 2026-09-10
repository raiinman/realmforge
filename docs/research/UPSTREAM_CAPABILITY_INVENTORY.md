# Upstream Compatibility Capability Inventory

**Status:** ACTIVE / first code-level pass  
**Pinned upstream:** `wowemulation-dev/tavern` @ `6f9158670ee7666bfae2be58b291dafcb45f12e7` (2026-09-08)  
**Upstream workspace license declaration:** `AGPL-3.0-only`  
**Purpose:** Inventory what can accelerate Realmforge now, what is missing, and what must be independently verified before it becomes authority.

## 0. Important correction

Earlier discussion loosely referred to the upstream license as "AGPL-3.0-or-later." The pinned workspace actually declares:

```text
license = "AGPL-3.0-only"
```

Realmforge must use the exact upstream licensing/provenance data, not conversational shorthand.

## 1. Pinned upstream topology

Workspace crates at the pinned revision:

```text
crates/
  tavern-account
  tavern-bgs
  tavern-core
  tavern-db
  tavern-oauth
  tavern-observability
```

Runnable services:

```text
bin/
  account-server
  bgs-server
  oauth-server
```

Major implementation dependencies include Rust/Tokio, Axum, SQLx/Postgres, Askama, JWT/RSA tooling, SRP-related big-integer/crypto dependencies, Prost protobuf, WebSocket support, Rustls, tracing and OpenTelemetry.

## 2. Upstream's own status claim

The pinned `docs/plan.md` says milestones M0–M19 are complete and M20 visual-polish work is partial.

Its advertised login paths include:

- 1.13.x: raw BGS v1/TCP with external plaintext challenge path,
- 1.14.x: same v1 path plus web SRP path,
- 2.5.x / 3.4.x / 4.4.x: `/bnetserver/login/` + SRP path,
- desktop app: RFC 8693 token exchange.

Legacy retail 2.4.3/3.3.5a realm SRP is explicitly out of scope.

**Realmforge disposition:** useful capability claim, but `UNKNOWN` until independently reproduced per target build.

## 3. Reusable interim capability buckets

### 3.1 Core/domain/config

Upstream has a shared domain/config/error crate.

Likely useful interim behavior:

- service configuration,
- account/session/ticket domain concepts,
- cryptographic support,
- signing-key handling.

Realmforge should not inherit this as canonical Core architecture. Treat it as Gate-internal interim implementation only.

### 3.2 Database

Upstream uses PostgreSQL through SQLx and owns schemas/repositories for:

- accounts,
- credentials,
- game accounts,
- OAuth clients,
- authorization codes,
- refresh tokens,
- sessions,
- service tickets,
- signing keys,
- account licenses,
- additional account-management state added in later migrations.

Useful immediately, but high migration lock-in risk if Realmforge Core reads it directly.

**Boundary rule:** integrate through Gate semantics, not direct cross-product SQL.

### 3.3 Account service

Claimed/visible capabilities include:

- account registration,
- account/game-account creation,
- SRP browser login,
- game-client bnet login,
- shared service-ticket minting,
- SSR account UI,
- localized account content,
- management API,
- account creation flow,
- email support.

### 3.4 OAuth/OIDC

Claimed/visible capabilities include:

- discovery,
- authorize,
- token,
- userinfo,
- JWKS,
- revocation,
- client credentials,
- authorization-code/PKCE implementation,
- RFC 8693 token exchange,
- desktop-app-related token behavior.

A pinned OAuth implementation note says authorization-code + PKCE is implemented but was not yet validated against `oauth2c` in that document.

**Disposition:** split standards functionality from proprietary compatibility. Standards pieces should eventually be rewritten from standards, not upstream source.

### 3.5 BGS service

Visible/claimed capabilities include:

- TCP BGS transport,
- WebSocket/protobuf support in dependencies/current plan,
- ConnectionService,
- authentication dispatch,
- account services,
- session services,
- GameUtilities realm commands,
- account-state pushes,
- realm-list/realm-join pre-world handoff,
- duplicate/disconnect handling,
- queue notifications,
- optional TLS listeners.

This is the highest-value code to retain temporarily and the highest-risk code to replace later.

### 3.6 Observability

Dedicated upstream observability crate and tracing/OpenTelemetry dependencies exist.

Realmforge can use the interim implementation but should replace metrics/log semantics with product-owned operations contracts over time.

---

# 4. Confirmed missing / stubbed / partial surfaces

These are code/documentation observations from the pinned upstream revision, not guesses.

## 4.1 Browser SSO ticket entry — missing

Upstream architecture documentation states:

```text
GET /login/ticket-login
```

was observed in real captures but was not yet implemented.

**Realmforge:** `P0 / CAPTURE-REQUIRED`.

Need independent contract for:

- input ticket,
- ticket producer,
- lifetime,
- replay behavior,
- browser/session binding,
- redirect validation,
- success/failure responses.

## 4.2 Browser SSO ticket generator — missing

Upstream documentation states:

```text
GET /login/sso/generate
```

is not implemented; a cross-site `/login/sso` redirector exists but not the generator.

**Realmforge:** `P0 / CAPTURE-REQUIRED`.

## 4.3 `MarkSessionAlive` — documented stub/deferred

Upstream plan documentation calls `SessionService.MarkSessionAlive` stubbed/deferred while other documentation discusses session keepalive behavior.

**Realmforge:** `P0 / CONFLICT + CAPTURE-REQUIRED`.

Do not assume the implementation/documentation state is current. Observe actual target clients and the pinned binary behavior.

## 4.4 External challenge / 2FA — stub/deferred

`OnExternalChallenge` is documented as 2FA-related and stubbed/deferred.

**Realmforge:** `P2` for homelab; promote only if a supported client requires it or public-hosting security policy wants compatible 2FA behavior.

## 4.5 GenerateAuthToken — documentation/code conflict

The plan says `GenerateAuthToken` is a stub.

However, pinned code search shows the BGS server dispatches both v1 GenerateSSOToken and v2 GenerateAuthToken paths into `handle_generate_sso_token`.

**Realmforge:** `CONFLICT`.

Required action:

- black-box the pinned service,
- capture actual v2 client request/response,
- determine whether output semantics are correct or merely accepted by synthetic tests.

## 4.6 Session restore — incomplete TODO visible

Pinned BGS server source contains a TODO associated with SSO token handling for RestoreSession dequeue behavior.

**Realmforge:** `P0 / CAPTURE-REQUIRED`.

## 4.7 Game world server — absent

Upstream protocol overview says it completes the pre-realm-join handoff and stops; world/gameplay server integration is external. BGS-version docs refer to a not-yet-implemented `tavern-game` boundary.

**Realmforge:** not a bug for upstream, but a major product gap for us.

Realmforge must own the adapter/orchestration path through successful realm-side authentication and gameplay.

## 4.8 Social services — not implemented

Pinned BGS-version docs identify multiple known v1 services as logged/not implemented:

- UserManagerService / notify,
- FriendsService / notify,
- PresenceService / listener,
- ReportService,
- Resources,
- ClubMembershipService / listener.

**Realmforge:** mostly `DEFER` unless a chosen client or product feature proves a consumer.

## 4.9 GameUtilities optional calls — no-op/unverified

Pinned BGS docs say:

- `GetPlayerVariables` — no-op stub,
- `GetAchievementsFile` — no-op stub,
- actual 1.13.2 invocation during login is unverified.

**Realmforge:** `P1 / CAPTURE-REQUIRED`.

## 4.10 ConnectionService incomplete surface

Pinned BGS docs say:

- Connect — done,
- Echo — done,
- KeepAlive — done,
- ForceDisconnect — done,
- Encrypt — not handled,
- RequestDisconnect — not handled,
- Bind skipped via bindless RPC.

**Realmforge:** determine whether the missing methods matter for any supported target before copying complexity.

## 4.11 Commerce — stubbed/omitted

Upstream docs explicitly call store/payments/wallet/social surfaces stubbed or omitted.

Account API stubs include empty commerce responses such as:

- transactions,
- wallet,
- external subscriptions,
- virtual-currency last-used/ecosystem.

**Realmforge:** `DEFER`. Build our own only when product requirements justify them.

## 4.12 Captcha/phone verification — intentionally fake/stubbed

Pinned account creation source includes:

- SMS captcha gate always accepted,
- phone verification accepting any six-digit code.

**Realmforge:** never mistake these for security features.

For homelab: disable unnecessary anti-abuse flow cleanly.
For public deployment: implement modern provider-agnostic controls independently.

## 4.13 Credential migration — partial

Pinned plan docs mention a v1→v2 verifier upgrade path as stubbed because it requires a Web Worker.

**Realmforge:** investigate only if real migration between targeted credential schemes matters.

## 4.14 OAuth validation gap

Pinned OAuth docs say authorization-code + PKCE was implemented but not yet validated with the planned external `oauth2c` harness at the point that document was written.

**Realmforge:** standards conformance must be revalidated independently anyway.

---

# 5. Major documentation conflicts

## C-001 — BGS v2 status

`docs/plan.md` claims broad M0–M19 completion and describes later-client login support.

`docs/bgs-protocol-versions.md` begins with:

```text
v1 (1.13.2) is implemented.
v2 (1.14.0 / 2.5.1) is planned.
```

The same BGS-version document later describes implementation status and v2-specific work in ways suggesting it has aged unevenly.

**Disposition:** `CONFLICT`.

No Realmforge support claim until real clients are tested.

## C-002 — GenerateAuthToken

Plan says stub; pinned server source has active dispatch to a token-generation handler.

**Disposition:** `CONFLICT`.

## C-003 — SessionService wording

BGS version status table labels SessionService "done (v2 only)" inside a section describing the 1.13.2 v1 surface.

**Disposition:** documentation is not safe as implementation authority.

---

# 6. What Realmforge should take first

Highest value interim import order:

1. BGS compatibility implementation and protobuf/service machinery.
2. Account/game-client login compatibility.
3. OAuth proprietary compatibility behavior.
4. Shared ticket/session integration required by the above.
5. Database pieces strictly required to make Gate runnable.
6. TLS/config/observability support necessary to operate Gate.

Lower-value items should not be imported merely because they exist:

- Tavern branding/UI assets,
- commerce stubs,
- fake phone/captcha verification,
- social placeholders,
- product-specific account visuals,
- unrelated documentation/style tooling.

The goal is **maximum useful protocol leverage**, not maximum file count for its own sake.

---

# 7. Provenance record for eventual import

If source is imported, pin this exact first candidate baseline unless a later review deliberately advances it:

```text
Upstream: https://github.com/wowemulation-dev/tavern
Revision: 6f9158670ee7666bfae2be58b291dafcb45f12e7
Date observed: 2026-09-09
Workspace license: AGPL-3.0-only
Upstream version: 0.1.0
Rust edition: 2024
Rust version declaration: 1.97
```

Before actual import, re-check upstream HEAD and decide whether to pin this revision or intentionally take a newer one. Record the chosen revision permanently.

---

# 8. Import boundary recommendation

Preferred conceptual placement:

```text
third_party/
  gate-upstream/
    LICENSE.md
    UPSTREAM.md
    ...covered source...
```

Realmforge-owned services remain outside that subtree.

Do not rename or erase provenance inside the covered source merely to make the repository look first-party.

The product UI does not need to advertise the upstream component, but source/license obligations remain while that component ships.

---

# 9. What is still missing from this inventory

This first pass is **not complete**. Next code-level inventory must enumerate:

- every migration and table,
- every HTTP route,
- every OAuth grant and claim,
- every BGS service hash/method pair,
- every protobuf message used,
- every realm command,
- every session state transition,
- every config/environment key,
- every network listener/port,
- every test category,
- every synthetic-client assumption,
- every TODO/FIXME/no-op path,
- every hard-coded target-build constant,
- every external hostname/cookie assumption,
- every upstream artifact we do not need.

That second pass is the highest-value next research/documentation task before source intake.
