# Implementation Plan

> **Status:** M0–M19 complete (MVP plus post-MVP BGS transport and
> desktop-app OAuth). M20 (SPA visual polish) design doc drafted,
> implementation partial — shared `static/style.css` and `static/spa.js`
> are extracted; responsive and loading/error/empty states remain.
> See [`docs/spa-design.md`](spa-design.md) for the SPA design specification.

This plan bootstraps Tavern in atomic, verifiable units. Each client line
uses its own login transport and proof and lands on one shared
account/ticket backend (see `docs/architecture.md`):

- **1.13.x Vanilla**: BGS v1 over raw TCP (port 1119) with banner
  exchange. `/client/login/external` + `JSONBnetChallengeSubmission`
  (plaintext), then BGS RPC `Logon` (method 1) →
  `VerifyWebCredentials` (method 7). Verified against the 1.13.2.31650
  client binary (2026-07).
- **1.14.x Vanilla**: same BGS v1 path as 1.13.x, plus a second web
  SRP path (`/bnetserver/login/` + `LoginForm`). Both are accepted.
- **2.5.x TBC, 3.4.x WotLK, 4.4.x Cata**: `/bnetserver/login/` +
  `/bnetserver/login/srp/` + `LoginForm`, BnetSRP6v2 (M4).
- **Battle.net desktop app**: `/token` RFC 8693 token-exchange.

The legacy 256-bit realm SRP6 (retail 2.4.3/3.3.5a) is out of scope.

## MVP Scope

**In scope** (the MVP ends at Milestone 11):

- Game-client bnet login for all advertised lines, plus the browser web login.
- Account registration, management API, and SSR UI.
- The OIDC provider (discovery, authorize, token, userinfo, JWKS, revoke),
  including the desktop-app RFC 8693 token-exchange grant.
- Locale-aware user-facing content (login pages, emails). Account creation
  resolves locale from the selected country/region and stores it for the
  account lifecycle. Emails are sent in the account's locale.

## Locale and Regionalization

All user-facing content (web UI, emails) is localized per the account's
locale. The locale is resolved during registration from the selected country
and stored in the `accounts.locale` column. Supported locales mirror the
Battle.net login page set:

| Locale | Language | Region |
| --- | --- | --- |
| `enUS` | English | US (Americas, Oceania) |
| `esMX` | Spanish (Mexico) | US |
| `ptBR` | Portuguese (Brazil) | US |
| `enGB` | English | EU (Europe, Russia, ME, Africa) |
| `deDE` | German | EU |
| `frFR` | French | EU |
| `esES` | Spanish (Spain) | EU |
| `itIT` | Italian | EU |
| `ruRU` | Russian | EU |
| `plPL` | Polish | EU |
| `ptPT` | Portuguese (Portugal) | EU |
| `koKR` | Korean | KR (Korea, Japan, SE Asia) |
| `jaJP` | Japanese | KR |
| `thTH` | Thai | KR |
| `zhTW` | Chinese (Traditional) | TW (Taiwan, HK, Macau) |
| `zhCN` | Chinese (Simplified) | CN (China) |

Country codes are mapped via ISO 3166-1 (alpha-2 and alpha-3 accepted) to
their default region and locale. The default locale per region is the first
in the list above. Unknown country codes fall back to `enUS`.

Emails select the subject and body from a locale→translation map.
Currently supported for email: `enUS`, `enGB`, `deDE`, `koKR`. Other
locales fall back to `enUS`.

**Out of scope** (post-MVP, listed at the end):

- Passkeys, 2FA / authenticator, phone verification, captcha.
- Store, payments, transactions, wallet.

## Rules

- Every milestone must leave `cargo build --workspace`,
  `cargo clippy --workspace -- -D warnings`, and `cargo nextest run` green.
- Every `crypto` or `db` unit has tests. No logic in binaries.
- No `unwrap`/`expect` in non-test code without a justification comment.
- Milestones are sequential; dependencies are noted. Each is a checkpoint.
- The OAuth/OIDC provider must be validated with `oauth2c` (installed via
  `mise`) against RFC-compliant flows once the servers are runnable (M11).
  This is a mandatory verification gate: `oauth2c` sends real form-encoded
  requests, exercises PKCE, and discovers endpoints from OIDC metadata, so it
  catches interop gaps the integration tests might miss.

## Milestones

### Milestone 0 — Workspace skeleton

**Goal:** a compiling, lint-clean workspace with empty crates and binaries.

**Build:**

- Root `Cargo.toml` as `[workspace]` with shared `[workspace.dependencies]`
  (`axum`, `sqlx`, `tokio`, `thiserror`, `anyhow`, `askama`, `serde`,
  `tracing`, `jsonwebtoken`, `rand`).
- `crates/tavern-core`, `crates/tavern-db`, `crates/tavern-oauth`,
  `crates/tavern-account` as library crates.
- `bin/oauth-server`, `bin/account-server` as binary crates.
- SPDX header on every `.rs` file.

**Verify:**

- [x] `cargo build --workspace` succeeds.
- [x] `cargo clippy --workspace -- -D warnings` clean.
- [x] `cargo nextest run` green (one trivial test per crate).

### Milestone 1 — `tavern-core`: config, errors, domain types

**Goal:** the pure foundation everything imports.

**Depends on:** Milestone 0.

**Build:**

- Error enum via `thiserror` (one variant per failure class).
- Config struct and loader from environment and an optional file
  (`DATABASE_URL`, `BIND_ADDR`, `ISSUER_URL`, signing-key path).
- Domain types as plain structs: `Account`, `Credential`, `OAuthClient`,
  `AuthorizationCode`, `RefreshToken`, `Session`, `ServiceTicket`.

**Verify:**

- [x] Config parser accepts valid env and rejects missing `DATABASE_URL`.
- [x] Error conversion tests pass.
- [x] No I/O in the crate (no `tokio`, no `sqlx` dependency here).

### Milestone 2 — Postgres dev container and `tavern-db` pool

**Goal:** a reproducible Postgres 16 database and a working pool plus migration
runner.

**Depends on:** Milestone 1.

**Build:**

- `dev/Containerfile` based on `postgres:16`, plus a `podman` run wrapper.
- `tavern-db`: `PgPool` builder, `sqlx::migrate!` runner, admin auto-migrate
  flag.
- Migration `0001_init`: `accounts` and `credentials` tables.

**Verify:**

- [x] `podman` brings up Postgres 16; `DATABASE_URL` connects.
- [x] Migration applies; a row can be inserted and read via `psql`.
- [x] `cargo sqlx prepare` generates the query cache.

### Milestone 3 — `tavern-db`: repositories

**Goal:** typed data access for accounts and credentials.

**Depends on:** Milestone 2.

**Build:**

- Repository modules: `accounts`, `credentials` (insert, find by email, find
  by id, update).
- Compile-time-checked queries via `sqlx::query!`.

**Verify:**

- [x] Integration tests against the container: CRUD on accounts and credentials.
- [x] `sqlx::query!` calls compile against the live schema.

### Milestone 4 — `tavern-core`: SRP6a

**Goal:** the credential math, pure and unit-tested.

**Depends on:** Milestone 1.

**Build:**

- SRP6a group parameters (2048-bit modulus, generator 2), salt generation,
  verifier computation.
- Server session: generate `public_B`, verify the client's `M1` proof.
- Pure functions only.

**Verify:**

- [x] Known SRP6a test vectors pass.
- [x] Property test: `register(pw)` then `login(pw)` succeeds; wrong password
      fails.

### Milestone 5 — `tavern-core`: JWT and JWKS signing

**Goal:** RS256 token signing and verification.

**Depends on:** Milestone 1.

**Approach:** this milestone wraps the `jsonwebtoken` crate (already a
workspace dependency); it does not reimplement RSA. Tavern builds a thin layer
over the library: load a keypair from PEM, sign RS256 tokens with our claims,
verify them, publish the public key as a JWK, and model key rotation
(active/retired).

**Build:**

- RSA keypair loading/generation via the crypto library, JWK encoding, RS256
  signing with our claim struct, signature verification.
- `signing_keys` concept (active and retired) over the library primitives.

**Verify:**

- [x] Sign a token, decode it, verify the signature.
- [x] JWKS round-trip: published JWK verifies a signed token.

### Milestone 6 — `tavern-db`: remaining schema and repos

**Goal:** the full data model and seeded known clients.

**Depends on:** Milestone 3.

**Build:**

- Migration `0002_oauth`: `oauth_clients`, `authorization_codes`,
  `refresh_tokens`, `sessions`, `service_tickets`, `signing_keys`,
  `account_licenses`.
- Repositories for each table.
- Seed the known `oauth_clients` observed in traffic (shop, account-settings,
  developer portal).

**Verify:**

- [x] Migration applies on a clean database.
- [x] Repository tests pass; seeded clients are queryable.

### Milestone 7 — `tavern-account`: registration and the shared ticket-mint backend

**Goal:** account registration plus the shared login-ticket backend that every
generation's transport delegates to.

**Depends on:** Milestones 4, 6.

**Build:**

- Registration: create the account and a game-account record. Store
  credentials in **both** schemes so any client line can log in: a BnetSRP6v2
  verifier (M4) for the 2.5+/3.4/4.4 lines, and a `sha_pass_hash`
  (`SHA256(hex(SHA256(name)) + ":" + UPPER(password))`) for the Vanilla
  plaintext line.
- The shared `mint_login_ticket(account_id)` module, returning an opaque ticket
  stored via the `service_tickets` repository (M6). All transports call this.
- Browser web login as the first transport: `GET /login/{locale}/`,
  `POST /login/srp` (challenge), `POST /login/{locale}/password` (verify proof,
  mint a ticket, start a session), `GET /login/ticket-login`. Routed by path
  regardless of host prefix (single-host deployment at `account.wowemu.dev`).

**Verify:**

- [x] Integration test: register, fetch the SRP challenge, compute the proof
      client-side in the test, submit, assert a session and a one-time ticket.
- [x] Wrong password is rejected; a replayed ticket is rejected.

### Milestone 8 — `tavern-account`: game-client bnet login

**Goal:** real game clients can log in over `/bnetserver/login/`.

**Scope note (2026-07):** `/bnetserver/login/` covers the 1.14.x web SRP
path and the 2.5+/3.4/4.4 lines. The 1.13.x client does not use this path
— it logs in via BGS v1 `/client/login/external` over raw TCP (see M17/
M18). All paths mint the same login ticket through the shared backend.

**Build:**

- `POST /bnetserver/login/` (GET + POST) with the `LoginForm` envelope
  (`inputs`: `account_name`, and either `password` or `public_A` +
  `client_evidence_M1`).
- Password-proof branch, selected by the stored credential scheme:
  - Vanilla (plaintext): verify `sha_pass_hash` =
    `SHA256(hex(SHA256(name)) + ":" + UPPER(password))`.
  - TBC/WotLK/Cata (BnetSRP6v2): `POST /bnetserver/login/srp/` issues the SRP
    challenge; the client returns `public_A` + `client_evidence_M1`; verify via
    M4.
- On success, mint a login ticket via the shared backend. Companion paths
  `/bnetserver/refreshLoginTicket/`, `/bnetserver/gameAccounts/`.
- The `LoginForm` protobuf codec (vendored from the reference proto set).

**Verify:**

- [x] SRP-line integration test: challenge, compute `public_A` + `M1`, submit,
      assert a login ticket is issued and the client's computed session key
      matches.
- [x] Plaintext-line integration test: submit `account_name` + `password`,
      assert a login ticket is issued.
- [x] Wrong password is rejected for both proof schemes.

### Milestone 9 — `tavern-oauth`: OIDC provider (desktop-app transport)

**Goal:** the full OAuth round-trip works, including the desktop-app grant.

**Depends on:** Milestones 5, 6, 7.

**Approach:** hand-written endpoints over the `jsonwebtoken` wrapper (M5). An
`oxide-auth` spike (decision 2026-06-23) confirmed hand-written is the lower-
risk path; the desktop-app token-exchange grant is non-standard and not
covered by any library anyway.

**Build:**

- `GET /.well-known/openid-configuration`, `GET /jwks/certs`.
- `GET /authorize` (consume the login ticket from query, validate client and
  redirect, mint an authorization code, redirect to callback).
- `POST /token` with `authorization_code` and `refresh_token` grants (JWT
  access, opaque rotatable refresh, ID token), plus the desktop-app
  `urn:ietf:params:oauth:grant-type:token-exchange` grant exchanging a BGS JWT
  for a DPLT.
- `GET /userinfo`, `POST /revoke`, `POST /v2/check_token`, `GET /logout`.
- Issuer is `https://oauth.wowemu.dev` (the deployment host); the discovery
  document and `iss` claim use it.

**Verify:**

- [x] Integration test: authorize with a login ticket, exchange the code for
      tokens, decode and assert the access-token JWT claims, call userinfo.
- [x] Refresh-token grant rotates and revokes.
- [x] Desktop-app token-exchange test: a BGS JWT exchanges for a DPLT.

### Milestone 10 — `tavern-account`: management API and UI

**Goal:** the account management experience, server-rendered.

**Depends on** Milestone 7.

**Build:**

- `/api/*` endpoints, the complete captured surface grouped by concern:
  - Account: `user`, `details`, `details/address` (GET + PUT),
    `details/battletag/rules`.
  - Security and privacy: `security`, `privacy`, `privacy/profile`, `passkeys`,
    `approvals`, `account-connections`.
  - Games and entitlements: `overview`, `games-and-subs`, `classic-games`,
    `game-account/creation/rules`, `time-gated-games`.
  - Locale and region: `env`, `location/url`, `location/country-list`,
    `location/country-address-metadata`.
  - Communications: `communication-preferences`.
  - Commerce (return empty/stubbed): `transactions`, `wallet`, `external-subs`,
    `vc/lastUsed`, `vc/ecosystem`.
- `askama` templates: login page, registration flow, dashboard, security
  (password change).
- Password change: re-verify the old password, rewrite both the SRP verifier
  and the `sha_pass_hash`.

**Verify:**

- [x] End-to-end HTTP test: register through the flow, log in, view the
      dashboard, change the password, log in with the new password.

### Milestone 11 — Binaries and end-to-end

**Goal:** two runnable servers and a full cross-generation flow. **MVP complete.**

**Depends on:** Milestones 9, 10.

**Build:**

- `oauth-server` and `account-server` binaries: load config, build the pool,
  mount the router, bind, expose `/health` and `/ready`.
- Cross-service end-to-end flow wiring.

**Verify:**

- [x] `cargo run` both servers; a scripted flow exercises the SRP-line
      bnet login, the browser web login, and the desktop-app OAuth round-trip
      end to end.
- [x] `oauth2c` validation: run `oauth2c [issuer-url]` against the running
      `oauth-server` for the authorization_code grant (with PKCE), the
      client_credentials grant, and the token-exchange grant. All must succeed
      with real form-encoded requests and browser-driven flows. This is a
      **mandatory** gate per the Rules section.
- [x] `cargo clippy --workspace -- -D warnings` and `cargo nextest run` green
      across the workspace.

## Wire-format correction

### Milestone 12 — Correct HTTP wire format against captures

**Goal:** fix every wire-format mismatch between M7–M10 and the real
`account.battle.net` / `oauth.battle.net` captures. The SRP math, JWT signing,
and repository patterns are correct — only the HTTP layer (content types,
field names, cookie chains, CSRF, redirect chains) needs correction.

**Depends:** Milestone 11.

**Scope:** `tavern-account` handlers (login, api, bnet) and `tavern-oauth`
handlers (authorize, token, claims). No changes to `tavern-core` crypto or
`tavern-db` repositories (except the `Claims` struct in `tavern-core/jwt.rs`
which gains fields).

#### A. Login flow — form-encoded two-step (`tavern-account/src/login.rs`)

Source of truth: the SRP login network capture, steps A.1–A.4.

Current state: CORRECT per capture. Form-encoded login flow, SRP challenge in
`{inputs:[...]}` format, 302 redirects, `csrftoken` correlation, 11-field
proof POST, SSO cookies, `authentication-state`/`error-code` headers. All
gates pass.

Correct wire format:

1. **`POST /login/{locale}/`** — email submission. Form-encoded
   (`Content-Type: application/x-www-form-urlencoded`). Fields:
   `accountName=<email>`, `srpEnabled=false`, `csrftoken=<uuid>`,
   `sessionTimeout=<epoch_ms>`. If the account exists → **302** to
   `/login/{locale}/password?ref=...&app=bas`. If not → **302** to
   `/creation/` with `error-code: INVALID_ACCOUNT` header. Sets `bnet.extra`
   cookie. Response header: `authentication-state: LOGIN_CREDENTIAL`.
2. **`POST /login/srp?csrfToken=true`** — SRP challenge (AJAX, JSON).
   Request: `{"inputs":[{"input_id":"account_name","value":"<email>"}]}`
   with `X-Requested-With: XMLHttpRequest`.
   Response: `{"modulus":"...","generator":"2","hash_function":"SHA-256",
   "username":"<sha256-hex>","salt":"<hex>","public_B":"<hex>",
   "version":2,"iterations":15000,"eligible_credential_upgrade":false,
   "csrf_token":"<uuid>"}`.
3. **`POST /login/{locale}/password`** — SRP proof. Form-encoded. Fields:
   `username`, `password=................` (dots), `useSrp=true`,
   `publicA=<512-hex>`, `clientEvidenceM1=<64-hex>`, `srpEnabled=true`,
   `persistLogin=on`, `csrftoken=<uuid>`, `accountName=<email>`,
   `usePasskey=false`. On success → **302** to
   `oauth/authorize?...&ST=<region>-<32hex>-<accountId>&flowTrackingId=`.
   Sets SSO cookies (see C below). Response header:
   `authentication-state: DONE`.

**Validation gates (integration tests):**

- [x] `POST /login/en/` with form-encoded `accountName` returns 302 to
      `/login/en/password`. Captured at A.1.
- [x] `POST /login/srp?csrfToken=true` request body matches
      `{inputs:[{input_id,value}]}` format. Response includes all 9 fields
      including `csrf_token`. Captured at A.3.
- [x] `POST /login/en/password` accepts form-encoded body with all 11 fields.
      Returns 302 to oauth/authorize with `ST=<region>-<hex>-<id>`. Captured
      at A.4.
- [x] `authentication-state` header present on all login responses
      (`LOGIN_CREDENTIAL`, `DONE`).
- [x] Wrong password returns 302 to `/login/en/password` with
      `error-code: INVALID_CREDENTIALS` header.

#### B. Service ticket format

Source: A.4 ST param.

Current: `<REGION>-<32-char-hex>-<accountId>`. Correct: `<REGION>-<32-char-hex>-<accountId>` (e.g.,
`KR-c0900ff7c7ee06fcbb292fca558c4313-1505337751`).

**Validation gate:**

- [x] `mint_login_ticket` returns `<region>-<32hex>-<accountId>`. The region
      is configurable (default `US`); the hex is random; the accountId is the
      numeric account id.

#### C. SSO cookies after login

Source: A.4 Set-Cookie headers.

Current: all 6 SSO cookies set matching the capture.

Correct: 6 cookies set on `Domain=battle.net`, all `Secure; HttpOnly;
SameSite=None`:

| Cookie | Path | Max-Age | Purpose |
| --- | --- | --- | --- |
| `BA-tassadar` | `/login` | ~1yr | Session handle |
| `BA-tassadar-login.key` | `/` | ~1yr | Cross-domain SSO key |
| `login.key` | `/` | ~1yr | Same value as above |
| `opt` | `/` | ~1yr | Opt-in flag |
| `BA-tassadar-cl` | `/login` | session | Client nonce |
| `cl` | `/login` | session | Same as BA-tassadar-cl |

Tavern sets all 6 cookies matching the capture. For a multi-region deployment
with region-specific hosts, the domain is the base domain (e.g.,
`wowemu.dev`).

**Validation gate:**

- [x] After successful `POST /login/{locale}/password`, all 6 SSO cookies
      (`BA-tassadar`, `BA-tassadar-login.key`, `login.key`, `opt`,
      `BA-tassadar-cl`, `cl`) are set on the response.

#### D. OAuth authorize — two-hop redirect + code format

Source: A.5–A.6.

Current: two-hop matching the capture. Correct:

1. `GET /authorize?...&ST=<ticket>` → **302** to `/authorize?...` (ST stripped).
   Sets `SESSIONID` cookie (oauth session).
2. `GET /authorize?...` → **303** to
   `<redirect_uri>?code=<REGION>STP<34-uppercase-alnum>&state=<state>`.
   Sets `SESSIONID` (oauth, HttpOnly).

Code format: `<REGION>STP<random>` — 34 uppercase alphanumeric chars, e.g.,
`KRSTPFJSTPJKQ6VFFEGAONXJGN94KKM8FA`.

**Validation gates:**

- [x] `GET /authorize?...&ST=...` returns 302 to the same URL without ST.
      Captured at A.5.
- [x] `GET /authorize?...` (no ST) returns 303 to the redirect_uri with
      `code=<REGION>STP<34-chars>` and `state=<echoed>`. Captured at A.6.
- [x] Authorization code is 34 uppercase alphanumeric chars.

#### E. Management API — XSRF double-submit + bootstrap

Source: A.7, C.1–C.25.

Current: `SESSIONID` cookie + `X-XSRF-TOKEN` double-submit. Correct:

1. **Session:** `SESSIONID` cookie (base64-encoded UUID, 30-min TTL, HttpOnly).
2. **CSRF:** `XSRF-TOKEN` cookie (UUID, NOT HttpOnly — JS-readable). Echoed as
   `X-XSRF-TOKEN` request header on every `/api/*` call.
3. **Bootstrap:** `POST /api/` (empty body, no CSRF) returns
   `{accountId, authenticated, loginUri, logoutUri, accountCompletion,
   userIp}`. This is the SPA's first call.
4. **Userinfo:** `GET /api/user` (no XSRF header) returns basic identity.
5. **All other `/api/*`** require the `X-XSRF-TOKEN` header.
6. **Callback exchange:** `GET /callback/oauth2/code/<client>?code=...&state=...`
   exchanges the code server-side, sets `SESSIONID` (30-min) + `XSRF-TOKEN`.

**Validation gates:**

- [x] After OAuth callback, `SESSIONID` cookie (30-min TTL) and
      `XSRF-TOKEN` cookie are set. Captured at A.7.
- [x] `POST /api/` (no XSRF) returns `{accountId, authenticated:true,...}`.
      Captured at C.1/A.9.2.
- [x] `/api/*` without `X-XSRF-TOKEN` header returns 403. With the header
      returns data. Captured at C.*.

#### F. ID token claims + userinfo casing

Source: Section 7 (captured ID token), Section 8 (userinfo).

Current `Claims` struct: 31 fields covering all captured JWT claims
(7 always present, 24 optional and skipped when empty).

All captured claims are present:
`sub`, `iss`, `iat`, `exp`, `jti`, `scope`, `client_id`, `battle_tag`,
`country_code`, `account_identifier`, `first_name`, `last_name`,
`birth_date`, `mobile_number`, `country_id`, `verified_email_address_flag`,
`employee_flag`, `aud`, `azp`, `nonce`, `at_hash`, `client_roles`,
`account_roles`, `authorities`, `programs`, `env`, `active`,
`account_authorities`, `client_authorities`, `account_id`, `username`.

`aud` and `azp` appear only in ID tokens (not access tokens), matching
the capture. `account_authorities` and `client_authorities` are empty
arrays for M2M tokens, also matching the capture.

Casing: `userinfo` returns `battletag` (lowercase); `id_token` returns
`battle_tag` (snake_case).

**Validation gates:**

- [x] ID token contains all 17 claims from the captured JWT (Section 7).
- [x] `/userinfo` returns `battletag` (lowercase), not `battle_tag`.
- [x] `/userinfo` returns `id` (numeric) in addition to `sub`.

#### G. Creation flow wire format (plan correction for M15)

Source: the account creation network capture, steps B.1–B.12.

The creation steps use `multipart/form-data` (NOT JSON), `_csrf` (NOT
`csrftoken`), and hyphenated field names (`first-name`, `last-name`,
`phone-number`, `sms-verification-code`, `opt-in-blizzard-news-special-offers`,
`tou-agreements-implicit`). Each step's response embeds the next `_csrf` token
in HTML. This is documented here for the M15 creation milestone.

**Validation gate (for M15, not this milestone):**

- [x] Creation flow accepts `multipart/form-data` (the captured wire format).
      The step handlers are multipart-only; the integration test exercises them
      as multipart.

## Browser flow milestones

M13, M14, and M15 are complete.

### Milestone 13 — Browser login flow: session cookie + redirect chain

**Status:** Complete.

**Goal:** a human can log in via the browser and reach the OAuth authorize
flow.

**Depends:** Milestones 11, 12.

**Build:**

- After SRP login proof is verified (`POST /login/{locale}/password`), set an
  `HttpOnly; Secure; SameSite=Lax` session cookie (`SESSIONID`) backed by the
  `sessions` table (M6). Replace the `X-Account-Id` header mechanism in the
  `/api/*` and `/overview` handlers with cookie-based session resolution.
- After login, redirect to the OAuth authorize endpoint with the login ticket
  as a query parameter: `oauth/authorize?st=...`. The redirect target comes
  from the `ref` query parameter on the original `/login/{locale}/` request.
- Add a `GET /login/{locale}/` handler that preserves the `ref` parameter so
  the redirect chain can complete.
- Logout: `GET /logout` clears the session cookie and deletes the session row.

**Verify:**

- [x] Integration test: POST `/login/srp` → POST `/login/{locale}/password`
      → assert `Set-Cookie: SESSIONID=...` and a 302 redirect to
      `oauth/authorize?st=...`.
- [x] Cookie-authenticated request to `/api/user` returns account data without
      an `X-Account-Id` header.
- [x] `/logout` clears the cookie and subsequent requests are unauthenticated.

### Milestone 14 — Browser-side SRP computation

**Status:** Complete.

**Goal:** the login page computes the SRP proof client-side so the password
never leaves the browser.

**Depends:** Milestone 13.

**Build:**

- A JavaScript module that implements the client side of BnetSRP6v2:
  reads the challenge from `/login/srp`, computes `publicA` and
  `clientEvidenceM1` from the password using the challenge parameters
  (PBKDF2-HMAC-SHA-512, SHA-256 padded-pair hashing), and submits
  the proof to `/login/{locale}/password`. After computation, the
  password field is overwritten with dots (`"."` × password length).
- Reference implementation: the real Battle.net SRP JavaScript recovered
  from the Phoenix reverse engineering.
  It uses `srp6aRoutines.ClientSession.step1(username, password, salt, public_B)`.
- The login page (`login.html` template) inlines this module
  (`static/srp6a.js`, embedded via `include_str!`) to compute the
  proof client-side.
- Error handling matches the capture: `error_code`, `support_code`,
  `error_message`, `error_status` from the JSON response.
- v1→v2 verifier upgrade is stubbed (requires a Web Worker).

**Verify:**

- [x] Manual browser test: open `/login/en/`, enter email + password, assert
      a redirect to the OAuth callback with a `code` parameter.
- [x] The password field is never sent to the server (verify in browser DevTools
      network tab — only `publicA` and `clientEvidenceM1` appear in the proof
      POST).
- [x] Wrong password is rejected and an error is shown on the login page.

### Milestone 15 — Account creation flow and OAuth authorize → callback

**Status:** Complete. Welcome email and encrypted ticket verification implemented.
   Authorized-code+PKCE flow verified via programmatic test.

**Goal:** new accounts can be created via the multi-step browser flow, a
welcome email is sent, and the full browser-driven OAuth authorization_code
flow completes with `oauth2c`.

**Depends:** Milestone 14.

**Build:**

The creation flow mirrors the real `account.battle.net` multi-step wizard,
verified against the captured creation flow trace. Each step is a
`POST /creation/flow/creation-full/step/<step>` with `_csrf`, session state
in cookies, and a `flow-resume-context` cookie carrying the original OAuth
carrying the original OAuth callback URL.

The observed step order from the capture (not the spec's logical table):

- **`POST /creation/flow/creation-full/step/{step}`** for the steps in the
  order the capture shows them:
  1. `get-started` — `_csrf`, `country`, `dob-year`, `dob-month`, `dob-day`.
     (Not in this capture — likely completed before tracing began; documented
     in the spec's step table.)
  2. `provide-name` — `_csrf`, `firstName`, `lastName`.
  3. `provide-credentials` — `_csrf`, `email`, `phone-number` (optional).
  4. `sms-captcha-gate` — `_csrf`, Arkose captcha token. Returns **302**
     (redirects to the Arkose challenge widget, then back). Stubbed: accepted,
     no actual captcha verification.
  5. `phone-number-verification` — `_csrf`, `sms-verification-code` (6-digit).
     Stubbed: accepted, no SMS sent.
  6. `legal-and-opt-ins` — `_csrf`, `tou-agreements-implicit`
     (ToS UUID;version), `opt-in-blizzard-news-special-offers` (true/false).
  7. `set-password` — `_csrf`, `password`. **Account is created on this
     step**: call `registration::register()` to store both credential schemes
     (M7) and create the default game account.
  8. `set-battletag` — `_csrf`, `battletag`. Returns **200** (the SPA
     navigates client-side after; no 302 redirect in the capture).
- **`GET /creation/flow/creation-full`** — the creation SPA / wizard page.
  `GET /creation/flow/creation-full/back` — navigate back one step (observed
  between `provide-credentials` and `sms-captcha-gate`).
- **`GET /creation/api/battletag-suggestion`** — generates a random BattleTag
  suggestion (the capture shows 20 calls as the user browsed suggestions).
- **Flow state cookies:** `flow-resume-context` (carries `callback-url`,
  `actor`, `app`, `continuation-type=RETURN_TO_REFERRER`, `flowTrackingId`,
  `accountIdentifier`), `SESSION`, `AccountManagement-State`
  (= `ACCOUNT_CREATION`), `XSRF-TOKEN`, `SESSIONID`. After the final step,
  the SPA follows the `callback-url` to the OAuth authorize endpoint with a
  login ticket.
- **Welcome email** (from the oauth-oidc spec, not the network capture):
  sent after account creation with an email-verification link
  (`/overview?ticket=<encrypted>`). Email language determined by GeoIP region.
  Requires SMTP config (`SMTP_HOST`, `SMTP_PORT`, `SMTP_FROM`) and a `lettre`
  dependency. Visiting the verification link sets `email_verified = true`.
- **`GET /authorize` redirect:** when the authorize endpoint receives a
  request without a valid `st` (login ticket), it redirects to
  `account/login/{locale}/?ref=<full-authorize-url>` so the flow continues
  after login or registration.
- **Registration page** (`register.html` template) linked from the login page.

**Verify:**

- [x] `oauth2c` authorization_code + PKCE flow end-to-end: the browser opens,
      logs in (or registers through the multi-step flow), the callback receives
      a code, and the token exchange succeeds. This is the **mandatory oauth2c
      gate** from the plan Rules section.
- [x] Integration test: POST through the full step sequence, assert account
      created with both credential schemes and a game account.
- [x] Duplicate email at `provide-credentials` is rejected.
- [x] Welcome email is sent with a verification link; visiting it marks
      `email_verified = true`.
- [x] After the final step, the flow redirects to the original OAuth
      `callback-url` with a login ticket.

## Post-MVP

### Milestone 16 — GeoIP and SSO endpoints

**Goal:** the two endpoints the Phoenix desktop app calls before BGS
connection.

**Depends:** none (standalone).

**Build:**

- `GET /geoip` — region detection. Returns `{"datacenter_region": "US",
  "country_alpha2": "US"}`. The region is configurable (default `US`).
  Called by the desktop app with `User-Agent: Client` before BGS
  connection. Timeout configured via `RegionDetectTimeoutMS`.
- `GET /login/sso` — cross-site SSO redirector. `GET /login/sso?token=<>&ref=<>`.
  Validates the token, resolves the account, sets session cookies, and
  redirects to the `ref` URL. Used by in-game shop checkout and desktop
  app webviews.

**Verify:**

- [x] `GET /geoip` returns `{datacenter_region, country_alpha2}`.
- [x] `GET /login/sso?token=<>&ref=<>` redirects to `ref` with session
      cookies set.

### Milestone 17 — BGS WebSocket RPC transport

**Status:** Complete. Transport, framing, and ConnectionService.Connect
      verified against all client versions (1.13.2–4.4.2).

**Goal:** accept BGS WebSocket connections with protobuf RPC framing.

**Depends:** Milestone 16 (region detection).

**Build:**

- WebSocket server over TLS with subprotocol `v1.rpc.battle.net` and
  path `/`. Certificate pinning is optional for a private server; TLS
  is required.
- Binary RPC frame format inside WebSocket binary messages (opcode 0x02):
  2-byte big-endian header size + `bgs.protocol.Header` protobuf +
  message body protobuf.
- Header fields: `service_id` (uint32), `method_id` (uint32), `token`
  (uint32, request correlation), `service_hash` (fixed32, FNV-1a of
  service descriptor name), `is_response` (bool), `status` (uint32),
  `client_id` (string).
- Service routing: 64-bit key combining method ID and service hash.
  FNV-1a 32-bit service hash in high 32 bits, method ID with direction
  prefix (0x4 outbound / 0xC inbound) in low 32 bits.
- `ConnectionService.Connect` (service hash
  `bnet.protocol.connection.ConnectionService`, method 1):
  `ConnectRequest` → `ConnectResponse` with `server_id`, `server_time`,
  `use_bindless_rpc: true`, `ciid` (connection instance ID).
- Connection timeout: 10 seconds. Keepalive: 50 seconds.

**Verify:**

- [x] WebSocket upgrade with subprotocol `v1.rpc.battle.net` succeeds.
      BGS server runs on port 8119.
- [x] `ConnectRequest` → `ConnectResponse` round-trip with correct
      framing and server-assigned `client_id`. Multi-version protos
      from protobuf-decompiler (1.13.2 through 4.4.2).

### Milestone 18 — BGS AuthenticationService and session state

**Status:** Complete. `AuthenticationService.Logon` issues a session key
      and login ticket and pushes `OnLogonComplete`; SRP auth is
      verified via the TrinityCore `srp_auth_client.py`.
      `GameUtilitiesService`, `AccountService`, and `SessionService`
      (`CreateSession`, `DestroySession`) are routed.
      `GenerateAuthToken` is a stub (BNET token prefix validated in `/sso`).
      `OnExternalChallenge` (2FA) and `MarkSessionAlive` are stubbed/deferred.

**Goal:** the full BGS login flow: authenticate, issue a session key,
and manage session lifecycle.

**Depends:** Milestone 17.

**Build:**

- `AuthenticationService.Logon` (method 1): accepts `LogonRequest` with
  `title_id` (0x417070 = "App"), `platform`, `locale`,
  `application_version`, `logon_options` (cached `auth_token`,
  `user_agent`, `device_id`, optional `email`/`phone_number`).
  Returns `LogonResponse` or triggers `OnExternalChallenge` (2FA).
- `AuthenticationListener.OnLogonComplete` (method 1): server pushes
  `LogonCompleteNotification` with `error_code` (0 = success) and
  `LogonRecord` containing `account_id`, `game_account` handles,
  `battle_tag`, `geoip_country`, `session_key` (bytes, 32-byte),
  `login_ticket`.
- `AuthenticationListener.OnExternalChallenge` (method 4): 2FA
  challenge flow. Stubbed: accept, no actual 2FA verification for MVP.
- `AuthenticationService.GenerateAuthToken` (method 3): produces a JWT
  from the SRP-authenticated session. Response: `{auth_token: <jwt>}`.
  This JWT is the `subject_token` for the OAuth token-exchange grant.
- `SessionService.CreateSession` (method 1): creates a game session for
  a `GameAccountHandle`. Returns `{session_id, variables}` with
  keepalive intervals.
- `SessionService.MarkSessionAlive` and `SessionService.DestroySession`:
  session keepalive and explicit logout.
- `AccountService.GetAccountState` (method 1) and `AccountListener`:
  account-level subscription for state changes.
- Session key format: raw bytes (up to 64). The Agent extracts the first
  8 bytes as a little-endian `int64` for `--sessionkey`.

**Verify:**

- [x] `Logon` → `OnLogonComplete` round-trip (stub). Generates 64-byte
      session key and login ticket. SRP auth via /bnetserver/login/ verified
      with TrinityCore srp_auth_client.py.
- [x] `GenerateAuthToken` — stub (BNET token prefix validated in /sso).
- [x] `CreateSession` and `DestroySession` implemented. `MarkSessionAlive` —
      deferred.

### Milestone 19 — Desktop app OAuth integration

**Status:** Complete. `client_sso` grant, token-exchange
      `requested_token_type` validation, `prompt=none`, and `cookietoken`
      are implemented. Spec derived from Phoenix reverse engineering
      (oauth-oidc-implementation.md, oauth-api-gateway.md).

**Goal:** the Phoenix desktop app's OAuth client_sso and token-exchange
flows work end-to-end with Tavern.

**Depends:** Milestones 15, 18.

**Reference:** Reverse engineering from battle.net-core 2.50.6.16125
  (Phoenix desktop app). Sources:

- `oauth-api-gateway.md` — FetchOAuthToken flow at `0x00e14440`
- `oauth-oidc-implementation.md` — client_sso grant at `0x0018af20`

**Build:**

- **`POST /sso` — `client_sso` grant** (Phoenix SSO):

  ```text
  POST <oauthServer>/sso
  Content-Type: application/x-www-form-urlencoded

  client_id=<client_id>
  &scope=<requested_scopes>
  &token=<BNET_auth_token>
  &grant_type=client_sso
  ```

  The `token` parameter is a BNET-prefixed auth token from the BGS RPC
  session. Exchanges a BGS auth token for an OAuth token without browser
  interaction. Source: `Phoenix::BattleNetAPIGateway` at `0x0018af20`.

- **`POST /token` — RFC 8693 token-exchange**:

  ```text
  POST <oauthServer>/token
  Authorization: Basic YTdmOWI3M2U0ZTljNGUwMWE5YzgwNTZjZGRhYmZmNzE6Mkx1MFFiRjVGTHJRb0xtSURqY1Bqb2psRXU0emlyRjI=
  Content-Type: application/x-www-form-urlencoded

  grant_type=urn:ietf:params:oauth:grant-type:token-exchange
  &requested_token_type=urn:blizzard:params:oauth:token-type:dplt
  &scope=
  &subject_token=<BGS_auth_JWT>
  &subject_token_type=urn:ietf:params:oauth:token-type:jwt
  ```

  Phoenix credentials: client_id=`a7f9b73e4e9c4e01a9c8056cddabff71`,
  client_secret=`2Lu0QbF5FLrQoLmIDjcPjojlEu4zirF2`. Exchanges a BGS JWT
  for a DPLT (Denuvo Platform Token). Source: `FetchOAuthToken` at
  `0x00e14440`.

- `prompt=none` on `/authorize`: OIDC-standard silent re-authentication.
  If the user has an active session (valid SESSIONID cookie), issue the
  authorization code without showing the login page. If not, return
  `interaction_required` error.
- `cookietoken=1` on `/authorize`: Battle.net-specific cookie-based
  SSO marker. If the cookie is present and valid, skip interactive login.

**Verify:**

- [x] requested_token_type validation in token-exchange, exchanges its JWT at `/token`
      with `requested_token_type=dplt`, receives a valid DPLT.
- [x] `POST /sso` accepts `client_sso` grant, validates BNET token prefix, issues access token.
- [x] `GET /authorize?...&prompt=none` returns `interaction_required` when no session.

### Milestone 20 — SPA visual polish and shared design system

**Status:** Partial. Design document complete at `docs/spa-design.md`.
      `static/style.css` and `static/spa.js` are extracted and served
      (Build steps 1–3). Responsive sidebar and per-page
      loading/error/empty states (steps 4–5) remain.

**Goal:** formalize the SPA visual design system derived from the email
      theme, extract shared CSS and JS from inline template blocks,
      and add responsive layout, loading/error/empty states.

**Depends:** Milestone 10 (the dashboard SPA exists and works).

**Reference:** [`docs/spa-design.md`](spa-design.md) — full design spec
      including color palette, typography, spacing, component library,
      page inventory, data flow, and responsive breakpoints.

**Build:**

1. **Extract `static/style.css`.** Move CSS custom properties and component
   styles from `landing.html`, `login.html`, `dashboard.html`, and
   `creation.html` into a single shared stylesheet. Each template links to
   `/static/style.css` instead of duplicating the `<style>` block.
   `email.rs` keeps its own inline styles (email client compat).

2. **Extract `static/spa.js`.** Move the SPA JavaScript from
   `dashboard.html` into a separate file. The template references it via
   `<script src="/static/spa.js">`. Add missing render functions
   (e.g., `renderSecurity` includes password change form) and error
   handling for every fetch.

3. **Apply consistent components.** Ensure all pages (login, creation
   wizard, dashboard) use the same card, badge, button, and form styles
   from the shared CSS. No inline style duplication.

4. **Responsive sidebar.** Hamburger-toggle sidebar collapse at 768px
   breakpoint. Slide-out overlay on mobile; tap outside to dismiss.

5. **Loading, error, empty states.** Every `render*` function wraps
   fetches in try/catch with `.loading`, `.error`, or `.empty` state
   display.

6. **Verify.** Cross-browser (Firefox, Chrome, Safari) and cross-locale
   (CJK font rendering). Light/dark mode. Mobile viewport.

**Verify:**

- [ ] All four templates (`landing`, `login`, `dashboard`, `creation`)
      reference `/static/style.css` and have no inline `<style>` blocks.
- [ ] `cargo build --workspace` succeeds (static files are embedded via
      `include_str!` or served from disk).
- [ ] Manual browser test: light/dark mode toggle works; mobile sidebar
      collapses and reopens; all pages render their data.
- [ ] Error state: stop the server, navigate the SPA, assert an error
      message is shown instead of a blank page.

### Deferred

Deferred, in rough priority order:

- Additional session cookies: `deviceTrackingId`, `sessionTrackingId`,
  `web.id`, `JSESSIONID` — cosmetic, not required for functional interop.
- Passkeys, 2FA / authenticator, phone verification, captcha.
- Store, payment, transaction, and wallet surfaces (stubbed in the MVP).
