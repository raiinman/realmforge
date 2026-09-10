# Architecture

> **Status:** Implemented (M0–M19). The design matches the running code.
> Grounded in captured network traffic and reference implementations of the
> Battle.net account and authentication protocols.

This document covers the design of the two services the README lists as in
scope: the **OAuth backend** and the **account management frontend**. It also
marks the boundary where the **game-client authentication** service plugs in,
because that service shares the same account database.

## Design Goals

- **Minimal moving parts.** One workspace, a small shared core, and one binary
  per deployable service.
- **Grounded in the real protocol.** Endpoint paths, grant types, SRP shape, and
  token semantics are taken from observed Battle.net traffic, not invented.
- **Preservation scope.** Player-facing surfaces that a retired client needs to
  log in and play are in scope. Store, payments, and transactions are stubbed or
  omitted.
- **Simple persistence.** One relational database, schema managed by SQL
  migrations, accessed through `sqlx`.
- **Multi-region deployable.** The Tavern installation must support
  regionally prefixed hosts (e.g., `us.account.wowemu.dev`,
  `kr.account.wowemu.dev`) and multi-locale login (`/login/en/`,
  `/login/ko/`, …) matching the real Battle.net regional deployment model.
  Cookie domains, SSO tokens, service tickets, and authorization codes
  carry region tags to enable cross-region interop.

## Service Topology

Tavern serves **all advertised client lines**, each logging in over the same
`/bnetserver/login/` path with a `LoginForm` envelope, differing only in the
password proof. Known-working reference forks and network captures establish
that the right shape is one service with one front-end transport, because
every login produces the same internal artifact
— an opaque login ticket that the game client submits to BGS
`VerifyWebCredentials` regardless of how the proof was verified.

| Client line | Login path + envelope | Password proof | Source |
| --- | --- | --- | --- |
| 1.13.x Vanilla | BGS v1 raw TCP + `/client/login/external` + `JSONBnetChallengeSubmission` | plaintext | 1.13.2.31650 binary RE |
| 1.14.x Vanilla | `/client/login/external` (BGS v1) or `/bnetserver/login/` + `LoginForm` | plaintext | 1.14.0.40618 binary RE |
| 2.5.x TBC, 3.4.x WotLK, 4.4.x Cata | `/bnetserver/login/` + `/bnetserver/login/srp/` + `LoginForm` | BnetSRP6v2 (M4) | known-working WotLK fork |
| Battle.net desktop app (Phoenix) | `/token` (RFC 8693 token-exchange) | n/a (subject token is a BGS JWT) | oauth-api-gateway RE |

Verified 2026-07 against the 1.13.2.31650 and 1.14.0.40618 binaries:
1.13.x logs in over BGS v1 — `/client/login/external` with a
`JSONBnetChallengeSubmission` envelope (plaintext password), then BGS RPC
`Logon` (method 1) → `VerifyWebCredentials` (method 7) over raw TCP (port
1119) with a banner exchange. 1.14.x keeps the same BGS v1 path and adds a
second web SRP path (`/bnetserver/login/` + `LoginForm`). Both lines
converge on the shared ticket backend.

Realm auth (game-server) is uniform across every line: a token+digest
handshake (`AUTH_SESSION`/`AUTH_CONTINUED_SESSION`) seeded from the bnet login
session key. No SRP runs over the realm socket for any Classic build; the
legacy 256-bit `CMSG_AUTH_SRP6_*` realm auth is retail 2.4.3/3.3.5a only and
out of scope.

A browser-facing web login (`/login/*`) is a separate consumer (humans in
browsers), served by the same account backend.

Two runtime processes share one workspace and one database:

| Process         | Replaces                  | Transports served |
| --------------- | ------------------------- | ----------------- |
| `account-server`| `account.battle.net` + bnetserver | game-client bnet login (all lines), browser `/login/*`, registration, account API, UI |
| `oauth-server`  | `oauth.battle.net`        | OAuth/OIDC (authorize, token incl. desktop-app token-exchange, userinfo, JWKS, revoke) |

The account service owns identity, credentials, the web UI, and the per-
generation game-client login transports; the OAuth service owns authorization
and token issuance (including the Gen 3 RFC 8693 token-exchange). They share
the account database and the SRP algorithm (where SRP applies). A future
`bgs-auth` binary adds the BGS v1 WSS transport and shares the same backend.

## Workspace Layout

```text
tavern/
├── Cargo.toml                 # [workspace], shared [workspace.dependencies]
├── crates/
│   ├── tavern-core/           # domain types, errors, config, SRP6a, JWT/JWKS
│   ├── tavern-db/             # sqlx pool, embedded migrations, repositories
│   ├── tavern-oauth/          # OIDC provider: authorize, token (+exchange), JWKS
│   ├── tavern-account/        # bnet login (all lines), web login, account API, UI
│   └── tavern-bgs/            # BGS protobuf codecs, frame format, service hashes
└── bin/
    ├── oauth-server/          # composes tavern-oauth + tavern-db
    ├── account-server/        # composes tavern-account + tavern-db
    └── bgs-server/            # BGS v1 WSS transport (M17/M18)
```

Each crate has one responsibility:

- **`tavern-core`** — pure domain types (`Account`, `Credential`, `Client`,
  `Token`), the error enum (`thiserror`), config loading, and pure crypto
  helpers (SRP6a math, JWT/JWKS encoding). No I/O, fully unit-testable.
- **`tavern-db`** — owns the `sqlx` connection pool, runs migrations on startup,
  and exposes repository functions (one module per aggregate). Compile-time
  checked queries via `sqlx::query!`.
- **`tavern-oauth`** — the OIDC provider as an `axum` router: discovery,
  authorize, the token grants (including the desktop-app RFC 8693
  token-exchange), token signing, JWKS, introspection, revocation.
- **`tavern-account`** — the bnet-login service with one `/bnetserver/login/`
  transport and a password-proof branch (plaintext for Vanilla, BnetSRP6v2
  for TBC/WotLK/Cata), plus the browser web login, registration, the
  account-management API, and `askama` server-rendered UI. All login paths
  delegate to a shared ticket-mint module backed by `tavern-db` and
  `tavern-core`.

Binaries stay thin: parse config, build the pool, mount the router, serve.

## Persistence

- **Database:** PostgreSQL 16 via `sqlx` with compile-time query checking.
  Local development runs Postgres 16 in a `podman` container; see
  [Local Development](#local-development).
- **Migrations:** plain `.sql` files under `migrations/`, applied on startup by
  `sqlx::migrate!`. Versioned, forward-only.
- **Connection management:** one `PgPool` per process, injected as axum state.

## Data Model

Tables are grounded in the `LogonRecord` fields from the RE specs and the
endpoints observed in the account-management-flow capture.

- **`accounts`** — `id BIGINT PK` (numeric account id, matches the numeric `sub`
  claim), `email CITEXT UNIQUE`, `email_verified BOOL`, `battletag TEXT`,
  `country_code CHAR(3)`, `region SMALLINT`, `locale TEXT`, `created_at`,
  `updated_at`.
- **`credentials`** — `account_id BIGINT FK`, `srp_salt BYTEA`, `srp_verifier
  BYTEA`, `srp_iterations INT`, `srp_version INT`, `updated_at`. Separate from
  `accounts` so credential rotation never touches the account row. The web
  login stores an SRP6a verifier, never a reversible password hash.
- **`oauth_clients`** — `client_id TEXT PK`, `client_secret_hash TEXT` (nullable
  for public clients), `redirect_uris TEXT[]`, `scopes TEXT[]`,
  `allowed_grants TEXT[]`, `require_2fa BOOL`. Pre-seeded with the known clients
  observed in traffic (shop, account-settings, developer portal).
- **`authorization_codes`** — `code`, `client_id`, `account_id`, `scope`,
  `redirect_uri`, `code_challenge`, `nonce`, `expires_at`, `used BOOL`.
  Short-lived, single use.
- **`refresh_tokens`** — `token`, `account_id`, `client_id`, `scope`,
  `expires_at`, `rotated_from`. Opaque, rotatable, revocable.
- **`sessions`** — `session_id UUID PK`, `account_id`, `expires_at`,
  `created_at`, `user_agent`, `ip`. Backs the `SESSIONID` cookie.
- **`service_tickets`** — `st`, `account_id`, `region`, `expires_at`,
  `used BOOL`. The one-time ticket bridging SRP login into the authorize flow.
- **`signing_keys`** — `kid`, `private_key_pem`, `public_jwk JSONB`,
  `created_at`, `retired_at`. RSA key material for RS256 signing and JWKS
  rotation.
- **`account_licenses`** — `account_id`, `license_id BIGINT`, `level TEXT`,
  `granted_at`. Minimal entitlement model: "account owns game X".

A `revoked_tokens` table (by `jti`) is added if JWT access-token revocation is
required; otherwise short lifetimes replace explicit revocation.

## OAuth Provider (`tavern-oauth`)

This serves the Battle.net desktop app's (Phoenix) login: the app authenticates
via BGS, receives a JWT, then exchanges it at `/token` for a DPLT token (RFC
8693 token-exchange). It is in MVP scope.

Endpoints observed in the real service:

- `GET /.well-known/openid-configuration` — discovery document (issuer,
  endpoints, supported scopes and claims, `id_token_signing_alg` = RS256).
- `GET /authorize` — authorize endpoint. Renders consent or redirects to the
  account login; issues an authorization code.
- `POST /token` — grant handlers:
  `authorization_code`, `refresh_token`, `client_credentials`, and the
  desktop-app grant `urn:ietf:params:oauth:grant-type:token-exchange` (RFC
  8693), which exchanges a BGS JWT (`subject_token_type =
  urn:ietf:params:oauth:token-type:jwt`) for a DPLT (`requested_token_type =
  urn:blizzard:params:oauth:token-type:dplt`).
- `GET /userinfo` — OIDC userinfo claims. Requires a user-authorized token.
- `GET /jwks/certs` — public signing keys (non-retired entries).
- `POST /v2/check_token` — token introspection.
- `POST /revoke` — token revocation.
- `GET /logout` — end-session endpoint.
- `POST /device/code` plus the `device_code` grant (RFC 8628) — implemented
  (pending authorizations stored in `device_authorizations`).

**Tokens:** access tokens are RS256 JWTs (stateless, no per-token row); refresh
tokens are opaque and stored so they can be rotated and revoked. Blizzard serves
opaque user access tokens with introspection; Tavern defaults to JWT access
tokens for simplicity. See Open Decisions.

**Scopes and claims:** discovery advertises `openid` plus the account scopes
seen in traffic (`account.basic`, `account.full`, `account-settings.full`). The
ID token and userinfo carry the claims observed in the specs: `sub`, `iss`,
`aud`, `battle_tag`, `country_code`, and the standard OIDC fields.

## Account Service (`tavern-account`)

Three sub-routers, matching the captured surface:

### Login (SRP6a) — browser / web

These paths serve a human in a browser, mirroring the
`account.battle.net` web flow. The in-game client uses a different path served
by the same account service (see [Game-Client Authentication](#game-client-
authentication-boundary)).

Real `account.battle.net` serves these on a **region-prefixed host**
(`kr.account.battle.net`, `eu.account.battle.net`, …). Tavern's official
deployment is a single host — `account.wowemu.dev` — and routes these paths
regardless of host prefix, so clients that construct a regional subdomain
still reach them.

- `GET /login/{locale}/` — login page.
- `POST /login/srp` — returns the SRP challenge: `modulus`, `generator`,
  `hash_function`, `version`, `iterations`, `salt`, `public_B`, plus a CSRF
  token. This mirrors the real `/login/srp?csrfToken=true` response shape.
- `POST /login/{locale}/password` — accepts `publicA` and `clientEvidenceM1`
  (hex), verifies the SRP proof server-side, establishes the session, and issues
  a one-time service ticket that redirects back to the authorize endpoint.
- `GET /login/ticket-login` — SSO ticket-login entry. Observed in real
  captures (Playwright traces from post-creation flow); not yet implemented
  in Tavern.
- `GET /login/sso/generate` — SSO ticket generation (the server-side endpoint
  that creates the encrypted ticket consumed by `/login/ticket-login`).
  Not yet implemented; Tavern has `/login/sso` (the cross-site redirector)
  but not the generator.
- `GET /geoip`, `GET /login/sso` — region routing and desktop SSO.

SRP parameters are server-negotiated, as in the real protocol. The algorithm is
`BnetSRP6v2` (2048-bit modulus, generator 2, PBKDF2-HMAC-SHA-512 key
derivation, SHA-256 session hashes); see [`docs/srp.md`](srp.md). The same
algorithm serves both the web and the in-game flows.

### Account API

JSON endpoints backing the management UI. This is the complete surface observed
in the `account-management-flow` capture:

- Account: `/api/user`, `/api/details`, `/api/details/address` (GET + PUT),
  `/api/details/battletag/rules`
- Security and privacy: `/api/security`, `/api/privacy`, `/api/privacy/profile`,
  `/api/passkeys`, `/api/approvals`, `/api/account-connections`
- Games and entitlements: `/api/overview`, `/api/games-and-subs`,
  `/api/classic-games`, `/api/game-account/creation/rules`,
  `/api/time-gated-games`
- Locale and region: `/api/env`, `/api/location/url`,
  `/api/location/country-list`, `/api/location/country-address-metadata`
- Communications: `/api/communication-preferences`
- Commerce (returned empty / stubbed; out of scope): `/api/transactions`,
  `/api/wallet`, `/api/external-subs`, `/api/vc/lastUsed`, `/api/vc/ecosystem`

### Registration

- `GET /creation/flow/creation-full` and `/creation/flow/creation-full/back` —
  flow entry and back-step.
- `POST /creation/flow/creation-full/step/{step}` for the steps observed in the
  account-creation capture: `provide-name`, `provide-credentials`,
  `set-password`, `set-battletag`, `legal-and-opt-ins`. The anti-abuse steps
  (`phone-number-verification`, `sms-captcha-gate`) are captured but stubbed.
- `GET /creation/api/battletag-suggestion` — battle tag generator.

### UI rendering

The account dashboard is a **vanilla JavaScript SPA** (`spa.js`) loaded from an
`askama` shell template. The HTML shell (sidebar, top bar, footer) is
server-rendered; all page content is fetched from `/api/*` JSON endpoints and
rendered client-side. This keeps the account service a single Rust codebase
with no JavaScript build pipeline.

The visual design system is shared with the email templates. CSS custom
properties, typography, spacing, and component tokens are defined in
`static/style.css` and used by every page (login, creation wizard, dashboard).
See [`docs/spa-design.md`](spa-design.md) for the full design specification.

## Game-Client Authentication (Boundary)

The `bgs-server` binary handles BGS v1 WSS transport for desktop/Agent-driven
logins. The HTTP JSON login paths (below) are served by `account-server`. Both
share `tavern-db` and `tavern-core`, and both use the same BnetSRP6v2
credential.

Two transports:

1. **HTTP JSON login.** The in-game client posts to `/bnetserver/login/` and
   `/bnetserver/login/srp/` with a `Battlenet::JSON::Login::LoginForm` envelope.
   `/bnetserver/login/srp/` issues the SRP challenge (`modulus`, `generator`,
   `salt`, `iterations`, `hash_function`, `public_B`); the client returns
   `public_A` and `client_evidence_M1`; the server verifies and returns
   `evidence_M2`, the game accounts, and a `login_ticket`. Companion paths:
   `/bnetserver/refreshLoginTicket/`, `/bnetserver/gameAccounts/`.
2. **BGS v1 WSS (`bgs.protocol.authentication.v1.AuthenticationService.Logon`)
   for desktop/Agent-driven logins.** Input: `web_credentials` (the WEB_TOKEN
   JWT issued by the OAuth provider). Output: `session_key`, `sso_secret`, and
   the account's game-account handles.

Realm join: `GameUtilities::ProcessClientRequest` answers
`Command_RealmJoinRequest_v1` with the world-server connection parameters —
`Param_ServerAddresses` (zlib-compressed
`JSONRealmListServerIPAddresses:` JSON), `Param_JoinSecret` (32 random
bytes), `Param_RealmJoinTicket` (the account handle the client echoes in
`CMSG_AUTH_SESSION.RealmJoinTicket`), and `Param_BnetSessionKey` (the BGS
session key). The realm listener address comes from `REALM_ADDRESS` /
`REALM_PORT` (default `127.0.0.1:8085`). The realm server itself is an
external integration; tavern stops at the join handoff.

## End-to-End Request Flow

1. Browser requests `GET oauth/authorize?client_id=...&redirect_uri=...` and is
   redirected to `account/login/{locale}/?ref=<authorize-url>`.
2. The login page posts the account name to `account/login/srp`; the server
   returns the SRP challenge.
3. The browser computes `publicA` and `clientEvidenceM1`, posts them to
   `account/login/{locale}/password`. The server verifies the proof, starts a
   session, and issues a one-time service ticket `ST`.
4. The browser is redirected to `oauth/authorize?ST=...`. The OAuth provider
   consumes the ticket, mints an authorization code, and redirects to the
   client callback with `code`.
5. The account server exchanges the code at `oauth/token` and receives an access
   token (JWT), a refresh token, and an ID token.
6. The account server uses the access token (or its own session) to render the
   management UI via `/api/*`.
7. Game path: the desktop client holds the WEB_TOKEN JWT and calls
   `bgs-auth Logon`, which returns the `session_key` used for realm join.

## Cross-Cutting Concerns

- **Config:** loaded from environment and an optional file (`DATABASE_URL`,
  `BIND_ADDR`, `ISSUER_URL`, signing-key path). Shared loader in `tavern-core`.
- **Errors:** one error enum per crate, converted to HTTP responses in the
  service layer. No `unwrap` in non-test code without a justification comment.
- **Tracing:** `tracing` + `tracing-subscriber`, request IDs via tower
  middleware.
- **Migrations:** run on startup; an admin flag gates whether a server may
  auto-migrate in production.
- **Health:** `/health` (liveness) and `/ready` (readiness, checks the pool)
  on each binary.

## Local Development

The local development database is Postgres 16 running in a `podman` container.
This keeps the database reproducible and isolated from the host. The intended
setup:

- A `Containerfile`/Quadlet or a small `podman` run wrapper brings up Postgres
  16 with a fixed database name, user, and password for development.
- `DATABASE_URL` points at the containerized instance (default
  `postgres://tavern:tavern@localhost:5432/tavern`).
- A named volume persists data across container restarts.
- The same image is usable in CI without `podman`-specific tooling.

`docker` is API-compatible with `podman`; the project standardizes on `podman`
for local development.

### Query cache

`tavern-db` uses `sqlx::query!` macros that are checked against the database at
compile time. The checked metadata is cached in `.sqlx/` (committed) so the
workspace builds without a database. After changing a query or the schema, with
the dev database running, regenerate the cache:

```bash
DATABASE_URL=postgres://tavern:tavern@localhost:5432/tavern \
  cargo sqlx prepare --workspace
```

Commit the regenerated `.sqlx/` files. CI builds with `SQLX_OFFLINE=true`.

## Decisions

- **Official deployment hosts:** the Tavern installation runs at
  `account.wowemu.dev` (account service) and `oauth.wowemu.dev` (OAuth
  provider). These stand in for `account.battle.net` / `oauth.battle.net`.
  Real `account.battle.net` is region-prefixed (`kr.`/`eu.`/…); Tavern is a
  single host and routes by path regardless of subdomain prefix.
- **Frontend rendering:** server-rendered `askama` templates. Mirrors the
  original SSR site and keeps a single Rust codebase.
- **Database:** PostgreSQL 16. Richer column types and better concurrency than
  SQLite. Local development runs Postgres 16 in a `podman` container so the
  database is reproducible and isolated; see Local Development below. The `sqlx`
  migration layout would also support SQLite if a single-node deployment ever
  needs it.
- **User access-token format:** RS256 JWT. Stateless and simpler than Blizzard's
  opaque-plus-introspection model. Refresh tokens stay opaque and stored.

## Deferred

- Phone verification and Arkose captcha are stubbed; they are anti-abuse
  controls a private preservation server does not need.

### Commerce surface (post-MVP)

A complete Battle.net replacement eventually serves more than auth and account
management. The full API-group inventory, with the scope decision for each:

| Group | Surface | Tavern scope |
| --- | --- | --- |
| Entitlements | `POST /Client/EntitlementService/v1/GetOwnedLicenseIds` (Bearer DPLT, scope `commerce.entitlements.basic`) — the game-ownership gate | Stubbed (always-owned); see below |
| Product catalog | BGS `entitlement_configuration` catalog fragments | post-MVP |
| Shop / storefront | `ShopService` (desktop-app UI bridge) | post-MVP |
| Virtual currency / wallet | balance, spend, history | post-MVP (shape not in the RE specs) |
| Inventory | `commerce.inventory.full` scope, `InventoryController` | post-MVP (shape not in the RE specs) |
| Profile / settings | `profile.settings:read/write`, `ProfileService` | post-MVP |
| Data-sharing consent | GDPR consent flow | post-MVP |
| Agent HTTP API | localhost install/update/launch RPC (`/game`, `/install`, …) | post-MVP (bypassed when launching the client directly) |
| Social | friends, presence, chat, clubs | post-MVP |

The entitlement check is the only commerce surface that gates play, and only
when a client launches through the desktop app. A retired WoW Classic build
launched directly against a private realm bypasses it — the realm server gates
on the game account, not the desktop license API. Tavern stubs it
(always-owned via `account_licenses`) for the MVP; a faithful storefront,
wallet, catalog, and social surface are out of scope.
