# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2026-09-08] — Tavern Logo in Web UI and Emails

### Added

- Branding assets in `tavern-account/static/`, derived from
  `docs/assets/tavern-logo-monochrome.svg`: `tavern-logo-light.png` (black
  glyph) and `tavern-logo-dark.png` (white glyph) for the SPA, smaller
  `email-logo-*.png` copies for MIME embedding, and `favicon.svg`. New
  embedded handlers serve `/static/tavern-logo-*.png` and `/favicon.svg`;
  every page links the favicon.
- The welcome email now embeds the Tavern emblem as two CID inline
  images (black-on-light and white-on-dark) inside a `multipart/related`
  message, rather than a text `TAVERN` wordmark or a remote `<img>`. The
  header swaps between them under `prefers-color-scheme`, so the logo is
  self-contained and works without remote images.

### Changed

- The account SPA shows the emblem (theme-aware) on the landing page
  hero, the login, creation and device card headers, and the dashboard
  top bar. Landing and email use the large emblem; cards use a compact
  mark.
- Dashboard header redesigned around the emblem: a centered crest now
  overhangs the top bar's bottom edge into the content area, with the
  wordmark and user controls kept clear. The crest scales down on
  mobile so it does not collide with the hamburger or logout.
- `docs/spa-design.md` theme-alignment table updated to reference the
  two emblem PNG variants.

## [2026-09-08] — Markdownlint Ignore for SQL Seed

### Changed

- `docs/test-accounts.sql` is now excluded via `.markdownlintignore`
  instead of a command-line negation glob, so the docs check runs as
  plain `markdownlint docs/ README.md`.

## [2026-09-08] — TLS Listeners for Account and BGS Servers

### Added

- `account-server`: optional HTTPS. Setting both `TLS_CERT_PATH` and
  `TLS_KEY_PATH` (new `tavern-core` config keys) serves the axum app over
  TLS via a custom `axum::serve::Listener` wrapping the TCP listener in
  `tokio-rustls`; unset keeps plain HTTP. The ring `CryptoProvider` is
  installed as the process default at startup.
- `bgs-server`: the 1.13.2 TCP transport (port 1119) optionally performs
  a TLS handshake before the banner exchange when `BGS_TLS_CERT` and
  `BGS_TLS_KEY` are set, mirroring TrinityCore's bnetserver. Without them
  the listener stays plaintext for the synthetic test clients. The
  connection handler is now generic over the stream type.

## [2026-09-08] — TLS Provider Pinned to ring

### Changed

- Workspace `rustls` is now built with `default-features = false` plus the
  `ring`/`logging`/`tls12`/`std` features, and `tokio-rustls` is pinned the
  same way. This drops `aws-lc-rs`/`aws-lc-sys` (and their C/CMake build)
  from the dependency tree entirely, keeping Tavern Rust-native per the
  dependency policy. `sqlx` (`tls-rustls-ring`) and `lettre`
  (`default-features = false`) already used ring, so the whole tree is now
  on a single TLS provider.

## [2026-08-12] — Realm-Join Handoff + 1.13.2 ToS Gate

### Added

- `GameUtilities` `Command_RealmJoinRequest_v1` handler: the full
  pre-realm-join handoff — `Param_ServerAddresses` (zlib-compressed
  `JSONRealmListServerIPAddresses:` JSON), `Param_JoinSecret` (32 random
  bytes), `Param_RealmJoinTicket` (account handle echoed in
  `CMSG_AUTH_SESSION`), `Param_BnetSessionKey` (the BGS session key from
  `VerifyWebCredentials`). Unknown/absent realm addresses are rejected with
  a status-1 error frame.
- Per-build realm lists: the realm catalog version follows the client build
  (`session.build`) — 1.14.0/40618 clients get `1.14.0.40618`, everything
  else `1.13.2.<build>` — so the realm never shows a version mismatch.
- Realm-list JSON corrected to the canonical `JSON.RealmList` proto field
  names (`wowRealmAddress`, `cfgTimezonesID`, `version.versionBuild`,
  `name`, …), the shape the client's protoc-gen-json parser expects.
- Realm listener configuration: `REALM_ADDRESS` / `REALM_PORT` env vars
  (default `127.0.0.1:8085`) feed `Param_ServerAddresses`.
- 1.13.2 ToS (LEGAL) gate: accounts that have not accepted the current
  `TOS_VERSION` receive `authentication_state: "LEGAL"` with `next_url` +
  `legal_form` and no `login_ticket`; `POST /client/login/tos/accept`
  (JSESSIONID-bound, requires `accept_beula`/`accept_chat` = true) and
  `GET /legal/agreement/{version}` complete the flow (migration `0024`).
- `ClientRequest.client_info = 6` + `ClientInfo` proto messages matching
  the canonical `game_utilities_types.proto`.

### Changed

- `OnLogonComplete` push uses the `ON_LOGON_COMPLETE` constant (method 5,
  corrected from the shifted value 11).
- `load-test/bgs-client.py` exercises `Command_RealmJoinRequest_v1` and
  validates the handoff (ServerAddresses JSON decompression, 32-byte
  JoinSecret, ticket, session key).
- `bgs-server` keeps the 64-byte session key on the session and returns it
  as `Param_BnetSessionKey` at realm join.

## [2026-06-26] — Browser SPA + Wire-Format Corrections + BGS M17/M18

### Web UI

- SPA dashboard replacing SSR template. All data loaded via /api/ endpoints.
- Unified email-theme CSS across dashboard, landing, login, and creation pages.
  Dark mode with CSS variables matching the welcome email template.
- Login page: `<!-- prettier-ignore-start -->` prevents Askama expression
  breakage by prettier auto-formatting.
- Creation page: HTML form with multi-step wizard (country/DOB → name →
  credentials → password). CSRF token embedded in template.
- Landing page at `/` with sign-in and registration links.

### Wire-Format Corrections

- OAuth authorize two-hop: hop 1 sets `opt` cookie (not SESSIONID), hop 2 sets
  SESSIONID as session cookie, matching live Battle.net captures.
- ST query parameter: `#[serde(rename = "ST")]` for uppercase wire format.
- Consumer logic: validate client before burning one-time ST ticket.
- API response formats (UserResponse, DetailsResponse, EnvResponse,
  BootstrapResponse) match live Battle.net capture fields and casing.
- SSO cookies: added configurable `cookie_domain` to AppState.

### Bugs Fixed

- LoginForm deserialization panic: `platform_id`, `program_id`, `version`
  now default to empty string (serde(default)), fixing intermittent server
  crashes when TrinityCore srp_auth_client.py omitted these fields.
- Logout: uses actual session UUID instead of Uuid::nil().
- Dashboard redirect: unauthenticated /overview redirects to login page.

### BGS Transport (M17)

- `crates/tavern-bgs`: protobuf definitions from protobuf-decompiler output
  (compatible with 1.13.2 Vanilla through 4.4.2 Cata).
- Binary frame format: 2-byte BE header size + Header + body.
- FNV-1a 32-bit service hash verified against spec (ConnectionService
  = 0x65446991).
- `bin/bgs-server`: WebSocket server with subprotocol `v1.rpc.battle.net`,
  ConnectionService.Connect handler.

### BGS Authentication (M18)

- AuthenticationService.Logon: validates "WoW" program, generates 64-byte
  session key and login ticket, sends OnLogonComplete.
- GameUtilitiesService and AccountService stubs routed by service hash.
- Verified end-to-end with TrinityCore srp_auth_client.py.

### Tools

- `dev/srp_auth_client.py`: end-to-end SRP auth test against /bnetserver/login/
  and /bnetserver/login/srp/. Full BnetSRP6v2 handshake with M2 verification.
- `dev/fuzz.py`: input fuzzer hitting all endpoints with malformed JSON,
  oversized payloads, raw HTTP garbage, and malicious SRP proofs.

## [Unreleased]

### Added

- **WoW Classic 1.13.2 game client interop**:
  - `/client/login/external` handler in `tavern-account` accepting plaintext
    email/password login with `auth.permit`/`remember.auth.permit` response
    fields and `JSESSIONID` cookie
  - Raw TCP BGS transport on port 1119 (`bin/bgs-server/src/tcp_transport.rs`)
    with WoW banner exchange (`WORLD OF WARCRAFT CONNECTION - SERVER TO
    CLIENT - V2`) and opcode multiplexer (0x3048-0x3052)
  - Two-step BGS v1 authentication: lean `Logon` (method 1) returns `NoData`,
    `VerifyWebCredentials` (method 7) validates login tickets against
    `service_tickets` table and pushes `OnLogonComplete`
  - `VerifyWebCredentialsRequest` proto message
  - BGS v1 service hash constants (`AuthenticationServer` 0x0DECFC01,
    `AuthenticationClient` 0x71240E35, `GameUtilities` 0x3FC1274D,
    `AccountNotify` 0x54DFDA17, `ChallengeNotify` 0xBBDA171F) with FNV-1a
    verification tests
  - `LogonResult` proto field numbers corrected to BGS v1 (12 fields,
    `EntityId`-based account/game account IDs, `sso_id`/`sso_secret`/
    `restricted_mode` fields)
  - `GameUtilities::ProcessClientRequest` realm-list handler
    (`bin/bgs-server/src/game_utilities.rs`) supporting
    `Command_RealmListTicketRequest_v1` and `Command_RealmListRequest_v1`
    with zlib-compressed `JSONRealmListUpdates` response
  - `AccountService` method-ID-aware stubs (13/25-37/44) with named logging
- **WoW Classic 1.14.0 web SRP login support**:
  - `GET /login/` route handling `?externalChallenge=login&app=wow` query
    params for the 1.14.0 game client's embedded browser SRP flow
  - SRP version auto-detection: game-client SRP uses `version: 1, iterations:
    1`, browser + 2.5+/3.4/3.4 clients continue with `version: 2, iterations:
    15000`
  - `SrpChallengeRequest` accepts `program_id`/`platform_id`/`version`/
    `account_name` fields from 1.14.0 client
- **Multi-version transport coexistence**: WebSocket on port 8119 (BGS v2,
  desktop app) + raw TCP on port 1119 (BGS v1, game client), configurable via
  `TCP_BIND_ADDR` env var

### Changed

- `tavern-bgs` service hash constants renamed for clarity: v1 constants use
  `_V1` suffix, v2 constants retain original names
- All BGS handler functions return `Vec<Bytes>` — response frames are now
  written to the transport instead of being discarded
- `bgs-server` connects to Postgres (`DATABASE_URL`) for ticket validation
  in `VerifyWebCredentials`

### Fixed

- BGS response frames were serialized but never written to the WebSocket
  or TCP transport (all `handle_*` functions used `let _frame = ...`)

### Security

- `VerifyWebCredentials` atomically consumes login tickets via
  `service_tickets::consume`. Invalid, expired, or already-used tickets are
  rejected with a status 1 error frame.

### Recent (2026-07-09, post-CHANGELOG update)

- **LogonResult proto correction**: removed `sso_id`/`sso_secret` (never in
  LogonResult; they are in `GenerateSSOTokenResponse`). Added `client_id` at
  field 11 for 1.14.0 backward-compatibility. Verified against
  protobuf-decompiler extracted schemas.
- **17 v1 service hash constants**: added `ClubMembershipService`,
  `ClubMembershipListener`, `FriendsService`, `FriendsNotify`,
  `PresenceService`, `PresenceListener`, `ReportService`, `Resources`,
  `UserManagerNotify`. All recognized in dispatch table.
- **GenerateSSOToken handler** (method 5): issues 16-byte `sso_id` +
  32-byte `sso_secret` for authenticated sessions.
- **GenerateWebCredentials handler** (method 8): issues opaque WEB_TOKEN for
  launcher-login handoff (desktop app BGS v2 → game client BGS v1).
- **SelectGameAccount handler** (method 9): acknowledges game account
  selection with NoData.
- **LogonUpdate dispatch** (method 10): no-op for server-bound updates.
- `SelectGameAccountRequest` proto message added.
- All 8 `AuthenticationServer` methods dispatched.

### Next

- **Integration testing**: deploy and test against unmodified 1.13.2 and
  1.14.0 clients. Validate wire formats with captured traffic.

- Wire-format correction (M12): form-encoded login flow (`POST
  /login/{locale}/` + `POST /login/{locale}/password`), `{inputs:[...]}`
  SRP challenge format, csrf_token correlation, 302 redirects,
  `authentication-state`/`error-code` response headers, service ticket
  format `<REGION>-<32hex>-<accountId>`, authorization code format
  `<REGION>STP<34-uppercase-alnum>`, SSO cookies (`login.key`, `opt`),
  OAuth callback (`GET /callback/oauth2/code/account-settings`),
  `POST /api/` bootstrap, `POST /api/logout`
- Session infrastructure (M13): `SESSIONID` cookie (base64 UUID, 30-min
  TTL, HttpOnly), `XSRF-TOKEN` cookie + `X-XSRF-TOKEN` header double-submit
  CSRF, session-based `/api/*` auth replacing `X-Account-Id` header
- Phoenix client secret seeded via migration `0005` (SHA-256 hash)
- Battle.net-specific JWT claims: `client_roles`, `account_roles`,
  `authorities`, `programs`, `env`, `active`, `account_authorities`,
  `client_authorities`, `account_id`, `username` (31 claims total: 7
  always present, 24 optional)
- Token format: `token_type: "bearer"` (lowercase), `expires_in: 86399`
- Signing key metadata sidecar (`keys/signing.pem.meta`): project,
  environment, kid, purpose, created, note. Server logs a warning at
  startup for demo/test keys
- Demo signing key at `keys/signing.pem` with default path `keys/signing.pem`
- `KeyMeta` type in `tavern-core` for loading and checking key metadata
  - `oauth2c` validation for `client_credentials` and `token-exchange` grants
  - Desktop app OAuth (M19): `POST /sso` `client_sso` grant, RFC 8693
      token-exchange `requested_token_type` validation, `prompt=none` and
      `cookietoken` handling on `/authorize`
  - Device authorization (RFC 8628): `POST /device/code`, `POST /device/approve`,
      `device_code` grant at `/token`, `device_authorizations` table and
      repository, advertised in OIDC discovery
  - Key metadata tests (51 total tests)

- **Performance and observability (Phases 0-8)**:
  - `tavern-observability` crate: OpenTelemetry metrics via OTLP/HTTP export
    to a collector, exposing `http.server.request.count`,
    `http.server.request.duration`, `tavern.srp.verify.duration`, and
    `db.client.connections.usage` (per-service pool gauge). Kubernetes health
    probes (`/health`, `/ready`, `/startup`) on all three binaries.
  - `tavern-db::PoolConfig` with `from_env()`: configurable pool tuning via
    `DB_POOL_MAX_CONNECTIONS` and related env vars. Default kept at 8 for
    backward compatibility.
  - `load-test/login-harness.py`: concurrent SRP login load test measuring
    throughput and latency percentiles. `load-test/seed-loadtest-accounts.py`:
    seeded account generator for scale testing.
  - BGS per-connection session ownership: removed global `Mutex<HashMap<>>`;
    each WS and TCP connection task owns its `BgsSession`. All 17 handlers
    now take `&mut BgsSession` directly.
  - SRP offload: `ServerSession::new` and `verify` run on
    `tokio::task::spawn_blocking`, freeing the async runtime during 2048-bit
    modular exponentiation (~69ms per modpow).
  - In-memory map eviction: background reaper (60s interval) removes expired
    `challenges`, `bnet_sessions`, and `creation_sessions` entries. Per-email
    rate limiting rejects duplicate challenges.
  - CSPRNG correctness: `session_key` and `sso_secret` generation replaced
    64/32 UUID-loop patterns with single `rand::thread_rng().fill_bytes()` calls.
  - OAuth caching: `moka::sync::Cache` (5-min TTL) for client lookups on the
    `/token` hot path. JWKS precomputed at startup and served from
    `OAuthState.jwks_json`.
  - `bnet_sessions` and `challenges` maps converted from
    `std::sync::Mutex<HashMap>` to `dashmap::DashMap`, eliminating Mutex
    contention (~26% throughput improvement; 16→20 rps at 100 concurrent).
  - `dev/otel.sh` + `dev/otel-collector-config.yaml`: local OpenTelemetry
    Collector for development. `dev/db.sh` postgres image switched to
    `postgres:16` (Debian). Test client M2 hex padding fix in
    `dev/srp_auth_client.py`.
  - `docs/performance-scaling.md`: concurrency analysis, implemented phases,
    measurements, and remaining deployment-side items.

### Changed

- `.sqlx/` added to `.gitignore`; query cache removed from tracking. CI
  builds that need offline mode should set `SQLX_OFFLINE=true` and
  regenerate the cache before building.
- Updated README status to MVP complete, added running instructions and
  current grant types
- Filled in `docs/oauth-oidc-implementation.md` with implementation details
  (endpoints, grant types, tokens, claims, signing keys, error format)
- Updated `docs/plan.md`: M0-M13 marked complete, M12/M13 verification gates
  checked, Claims struct listing updated to 31 fields, M14/M15/M17/M18/M19
  status reconciled with the running code
- Updated `docs/architecture.md` status to implemented
- Updated `docs/plan.md` to note M12 and M13 as done
- Updated CHANGELOG.md with M12-M13 entries and recent fixes

### Fixed

- Authorize endpoint: capture-accurate two-hop redirect (302 → sets
  `SESSIONID` cookie → strips ST → 303 with code + state), see
  `docs/plan.md` §D. Both hops now set `SESSIONID` cookies matching
  A.5–A.6 from the captured trace.
- Discovery document: added missing `end_session_endpoint`,
  `device_authorization_endpoint`, `subject_types_supported`; expanded
  `response_types_supported` from `["code"]` to `["code", "code id_token",
  "id_token", "token id_token"]` matching the captured OIDC configuration.
- Userinfo response: all 13 fields from the capture (`sub` as string, `id`,
  `battletag`, `first_name`, `last_name`, `email`, `mobile_number`,
  `account_identifier`, `country_id`, `birth_date`, `country_code`,
  `employee_flag`, `verified_email_address_flag`).
- Account domain type and `issue_tokens`: `first_name`, `last_name`,
  `birth_date`, `mobile_number`, `country_id` fields added to `Account`,
  migration `0006_account_profile_fields`, repository queries, and JWT
  claims propagation.
- Bootstrap response (`POST /api/`): `accountId` in camelCase (was
  snake_case), added `accountCompletion` and `userIp` fields per capture.
- Management API XSRF enforcement: all `/api/*` endpoints except
  `POST /api/` (bootstrap) and `GET /api/user` now require
  `X-XSRF-TOKEN` header matching the `XSRF-TOKEN` cookie. Returns 400
  on mismatch.
- `azp` and `aud` claims removed from access tokens per capture (present
  only in ID tokens)
- Userinfo returns 403 for M2M tokens per capture (was 401)
- Client credentials grant reads `client_id` from resolved Basic auth,
  not form body
- Token-exchange M2M path uses resolved `client_id` from Basic auth
- Migration version collision: `0006_device_authorizations.sql` renumbered
  to `0008_device_authorizations.sql`. Two migrations shared version `6`, which
  made `sqlx::migrate!` fail with `VersionMismatch(6)`.
- Creation-flow integration test now sends `multipart/form-data` (the wire
  format the step handlers expect). The prior test posted JSON to
  multipart-only handlers and failed with 400. `SQLX_OFFLINE=true` test
  compiles: the one test-side `sqlx::query!` was demoted to a runtime
  `sqlx::query`. Lint: `css` added to `markdownlint` `MD040` allowed
  languages; `dev/fuzz.py` bare `except` and `dev/` ruff findings fixed.
