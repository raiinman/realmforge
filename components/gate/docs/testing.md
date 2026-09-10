# Testing Realmforge End to End

This document is the full verified process for testing Realmforge: from a fresh
database through the web interface and simulated game-client interactions.

## 0. Prerequisites

- `podman` (Postgres 16 container)
- `cargo` / `mise` (Rust toolchain, edition 2024)
- `uv` (Python tooling for load-test scripts)
- `playwright-cli` (browser automation, optional but recommended for UI tests)

## 1. Fresh Database

The `dev/db.sh` wrapper manages a Postgres 16 container named `realmforge-gate-db` on
`127.0.0.1:5432`.

```bash
# Reset: down, delete volume, up (a fresh empty database)
bash dev/db.sh reset

# Verify the container is up
podman ps --filter name=realmforge-gate-db --format "{{.Names}} {{.Status}}"
```

### 1.1 Migrations

```bash
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo sqlx migrate run --source crates/realmforge-gate-db/migrations
This applies all migrations (currently 25): accounts, oauth, credentials,
game accounts, bgs sessions, catalog (licenses/products/rules), countries,
privacy settings, account connections, communication preferences, locales,
email-verification tokens, countries alpha-2, game-account license
binding, mandatory birth dates, ToS acceptance, and BGS session keys.
### 1.2 Seed Data

```bash
# Reference accounts (1001-1010) with credentials, game accounts, licenses,
# regions, suspension/game-time scenarios, and authenticator flags.
podman exec -i realmforge-gate-db psql -U realmforge -d realmforge < docs/test-accounts.sql

# BGS service tickets: mint one per run with dev/ticket.sh (single-use).
# Example:
#   uv run python bgs-client.py --flow full --ticket "$(bash dev/ticket.sh 1001)"
bash dev/ticket.sh 1001  # smoke check: prints a ticket name

# Optional: load-test accounts lt{0..N-1}@loadtest.local (password: loadtest)
uv run python load-test/seed-loadtest-accounts.py --count 200 | \
  podman exec -i realmforge-gate-db psql -U realmforge -d realmforge
```

Verify the catalog:

```bash
podman exec realmforge-gate-db psql -U realmforge -d realmforge -c \
  "SELECT license_id, description FROM catalog_licenses ORDER BY license_id;"
```

Expected: 3 production retail licenses (`179712` Shadowlands, `813186`
Dragonflight, `1106089` The War Within) plus 4 base products in
`catalog_products` (WoW/retail and the three Classic variants).

## 1.3 Mail Catcher (mailcrab)

The realmforge-gate-account-server sends verification emails over SMTP. For local
testing, a mailcrab container catches them. The server's SMTP defaults
are `127.0.0.1:1025` (host) — exactly what mailcrab binds — so no
server env overrides are needed when mailcrab is running.

```bash
# Start mailcrab (SMTP 1025, web UI 1080)
bash dev/mail.sh up
# Verify both ports are bound
ss -tln | grep -E ':1025|:1080'
# Web UI: http://127.0.0.1:1080

# Stop it
bash dev/mail.sh down
```

The mailcrab REST API lists caught messages (used by scripts):

```bash
curl -sf http://127.0.0.1:1080/api/messages | python3 -c \
  "import sys,json; d=json.load(sys.stdin); print(len(d), 'emails')"
```

## 2. Reference Accounts

| Email | Password | Account ID | Country | Notes |
|---|---|---|---|---|
| `test@example.com` | `password` | 1001 | USA | Full retail licenses, active sub, US region |
| `admin@bnet.local` | `123456` | 1002 | DEU | Full retail, EU region, authenticator |
| `player@example.org` | `Correct Horse Battery Staple` | 1003 | USA | Dragonflight only, active sub |
| `empty@example.com` | (see sql) | 1007 | CHN | Retail license, trial (no game time), CN region |
| `numeric@example.com` | (see sql) | 1008 | USA | Banned game account |

All use fixed salt (32 zero bytes) and 15000 PBKDF2 iterations for SRP v2.
Each seed account carries a consistent country, region, locale, first/last
name, and a birth date over the minimum age (all adult).

## 3. Start the Servers

Three binaries share the account database. Ports:

| Service | Port | Env |
|---|---|---|
| `realmforge-gate-account-server` | 8080 | `DATABASE_URL`, `INSECURE_COOKIES` |
| `realmforge-gate-oauth-server` | 8081 | `DATABASE_URL`, `BIND_ADDR=127.0.0.1:8081` |
| `realmforge-gate-bgs-server` | 8119 (WS), 1119 (TCP) | `DATABASE_URL` |

**Important:** for local testing over plain HTTP, start `realmforge-gate-account-server` with
`INSECURE_COOKIES=1`. Without it, the SESSIONID/XSRF cookies carry the
`Secure` attribute and the browser drops them, breaking web login.

### 3.1 Build first

```bash
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo build -p realmforge-gate-account-server -p realmforge-gate-oauth-server -p realmforge-gate-bgs-server
```

### 3.2 Launch pattern (verified)

Use `nohup ... & disown` with the env vars *before* `nohup`. Do not use a
plain `&` background job: it dies when the shell exits, taking the server
down mid-test.

```bash
# Terminal 1 — account web + management API + login UI
cd /path/to/realmforge
INSECURE_COOKIES=1 DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  nohup ./target/debug/realmforge-gate-account-server > /tmp/realmforge-gate-account-server.log 2>&1 < /dev/null & disown

# Terminal 2 — OAuth/OIDC provider
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  BIND_ADDR=127.0.0.1:8081 ISSUER_URL=http://127.0.0.1:8081 \
  nohup ./target/debug/realmforge-gate-oauth-server > /tmp/realmforge-gate-oauth-server.log 2>&1 < /dev/null & disown

# Terminal 3 — BGS transport
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  nohup ./target/debug/realmforge-gate-bgs-server > /tmp/realmforge-gate-bgs-server.log 2>&1 < /dev/null & disown
```

### 3.3 Verify each server is up

Check the ports are actually bound before running any test:

```bash
ss -tln | grep -E ':8080|:8081|:8119|:1119'
# realmforge-gate-account-server: 127.0.0.1:8080, realmforge-gate-oauth-server: 127.0.0.1:8081,
# realmforge-gate-bgs-server: 8119 and 1119

# Health checks
curl -sf http://127.0.0.1:8080/api/location/country-list >/dev/null && echo 'account OK'
curl -sf http://127.0.0.1:8081/.well-known/openid-configuration >/dev/null && echo 'oauth OK'
```

If a server died (port not bound), check its log: `tail /tmp/<server>-server.log`.
Common causes: missing `INSECURE_COOKIES=1` (cookies dropped, web login breaks),
stale binary (rebuild), or the DB container is down (`podman ps`).

**Pitfall**: never combine `pkill -f target/debug/<server>` with the launch
in one command. `pkill -f` matches the launching shell's own command line
(it contains the same binary path), killing the shell before the server
starts — the launch silently produces no output. Kill from a separate call,
or use an exact match (`pkill -x`).

## 4. Account Web Interface

### 4.1 Headless API smoke test

```bash
# Static endpoint (no auth)
curl -sf http://localhost:8080/api/location/country-list \
  | python3 -c "import sys,json; print(f'{len(json.load(sys.stdin))} countries')"
# Expected: 244 countries

# Account endpoint (X-Account-Id header works without a session for GET /api/user)
curl -sf http://localhost:8080/api/user -H "X-Account-Id: 1001"
```

### 4.2 Full browser login (playwright-cli)

```bash
playwright-cli open --headed http://localhost:8080/login/en/
playwright-cli snapshot                      # shows Email/Phone field
playwright-cli fill e7 "test@example.com" --submit
playwright-cli snapshot                      # shows Welcome Back + Password field
playwright-cli fill f1e8 "password" --submit
# Redirects to /overview with "Tester#1001" in the header
```

The SRP login requires the Askama template expression
`{{ srp6a_js|safe }}` to render the client-side SRP library. If the login
silently returns to the password page, check the rendered page for the
literal text `srp6a_js | safe;` (broken template) instead of the JavaScript.

### 4.3 Verify the pages

| Page | Nav link | Expects |
|---|---|---|
| Overview | Overview | Account details, security status |
| Games & Subs | Games & Subs | Game account `1001#1`, World of Warcraft, US, Good, Subscription active |
| Security | Security | Authenticator/security fields |
| Account Details | Account Details | Profile fields |
| Privacy | Privacy | Communication settings (marketing off, etc.) |

```bash
playwright-cli snapshot   # inspect rendered DOM after clicking each nav link
```

### 4.4 API endpoints behind the session

The authenticated SPA calls these (each requires the `XSRF-TOKEN` cookie
and `X-XSRF-TOKEN` header, which the SPA's `api()` wrapper sets
automatically):

- `GET /api/games-and-subs` — game accounts + subscriptions
- `GET /api/classic-games` — legacy CD-key titles
- `GET /api/overview` — security status + account summary
- `GET /api/user` — account identity
- `GET /api/privacy` / `GET /api/privacy/profile` — privacy settings
- `GET /api/communication-preferences` — comm preferences
- `GET /api/security` — authenticator, SMS, active logins

Static endpoints (no auth, DB-backed): `country-list`,
`country-age-of-adulthood-map`, `details/battletag/rules`,
`game-account/creation/rules`.

### 4.5 Email verification flow

New accounts start with `email_verified = FALSE`. The overview page shows
an "email address is unverified" banner with a Resend Verification Email
button, and the overview API exposes the state:

```bash
# 1. Make an account unverified and check the API
podman exec realmforge-gate-db psql -U realmforge -d realmforge -c \
  "UPDATE accounts SET email_verified = FALSE WHERE id = 1001;"

curl -sf http://127.0.0.1:8080/api/overview \
  -H 'X-Account-Id: 1001' \
  -H 'X-XSRF-TOKEN: test' \
  -H 'Cookie: XSRF-TOKEN=test' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['accountSecurityStatus']['emailVerified'])"
# Expected: False

# 2. Resend the verification email (mints a sealed ticket, sends via SMTP)
curl -sf -X POST http://127.0.0.1:8080/api/email/verification \
  -H 'X-Account-Id: 1001' \
  -H 'X-XSRF-TOKEN: test' \
  -H 'Cookie: XSRF-TOKEN=test' -o /dev/null -w '%{http_code}\n'
# Expected: 200

# 3. Mailcrab caught it (web UI: http://127.0.0.1:1080)
curl -sf http://127.0.0.1:1080/api/messages | python3 -c \
  "import sys,json; d=json.load(sys.stdin); print(len(d), 'emails')"
# Expected: 1+ emails

# 4. Open the verification link from the email: /overview?ticket=<sealed>
#    The dashboard consumes the ticket and marks email_verified = TRUE.
#    Verify:
podman exec realmforge-gate-db psql -U realmforge -d realmforge -c \
  "SELECT email_verified FROM accounts WHERE id = 1001;"
# Expected: t
```

The email link format is `/overview?ticket=<sealed>` (matches the real
Battle.net flow, which the browser hits after clicking the email link).

## 5. OAuth / OIDC

With `realmforge-gate-oauth-server` on 8081. **The `ISSUER_URL` must match `BIND_ADDR`**
— oauth2c and other OAuth clients validate the discovery document's
`issuer` against the URL they queried. The default issuer is
`http://localhost:8080` (the realmforge-gate-account-server port); pointing oauth2c at
8081 while the issuer says 8080 makes the discovery self-inconsistent
and oauth2c fails with `failed to parse error response`:

```bash
# Discovery document (issuer must read http://127.0.0.1:8081)
curl -sf http://127.0.0.1:8081/.well-known/openid-configuration \
  | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['issuer'])"

# Client credentials via oauth2c (Phoenix client from migration 0005)
oauth2c http://127.0.0.1:8081 \
  --client-id a7f9b73e4e9c4e01a9c8056cddabff71 \
  --client-secret 2Lu0QbF5FLrQoLmIDjcPjojlEu4zirF2 \
  --grant-type client_credentials --auth-method client_secret_basic

# Equivalent raw token request
curl -sf -X POST http://127.0.0.1:8081/token \
  -u "a7f9b73e4e9c4e01a9c8056cddabff71:2Lu0QbF5FLrQoLmIDjcPjojlEu4zirF2" \
  -d "grant_type=client_credentials"
```

The seeded clients' redirect URIs are production URLs (account.battle.net,
battlenet:// scheme). For a local `authorization_code` + PKCE run with
oauth2c, register a test client whose redirect URI points at oauth2c's
callback (default `http://localhost:9876/callback`) and pass
`--redirect-url` + `--pkce`.

Seeded OAuth clients are in `docs/test-accounts.sql` (migration 0003 seeds
the Phoenix desktop client). See `docs/oauth-oidc-implementation.md`
§Validation for the oauth2c matrix.

The browser flow connects the two servers: the web login mints a ticket
(`ST`), redirects to `realmforge-gate-oauth-server` `/authorize`, which validates the ticket
and issues an authorization code; the callback at `realmforge-gate-account-server`
`/callback/oauth2/code/account-settings` exchanges it for a management
session (SESSIONID + XSRF-TOKEN cookies).

## 6. Simulated Game-Client Interactions

### 6.1 SRP web login (dev script)

```bash
python3 dev/srp_auth_client.py --server http://127.0.0.1:8080 \
  --email test@example.com --password password
# Expected: Authentication successful, login ticket US-<hex>-1001
```

### 6.2 BGS transport (load-test clients)

All under `load-test/`, run via `uv`:

```bash
cd load-test

# Logon only: Connect + Logon
uv run python bgs-client.py --flow logon

# Full auth: Connect + Logon + VerifyWebCredentials + account state + SSO.
# Tickets are single-use — mint a fresh one per run.
uv run python bgs-client.py --flow full --ticket "$(bash ../dev/ticket.sh 1001)"

# Restore a session with a prior SSO token
uv run python bgs-client.py --flow restore --sso-id <HEX>

# Character flow: full auth + realm list + character list
# The client replays the real login gate: GetAccountState (30) +
# GetGameAccountState (31) for the selected game account. The real
# 1.13.2 client never sends SelectGameAccount — selection is local and
# the realm join carries it.
uv run python bgs-client.py --flow character --ticket "$(bash ../dev/ticket.sh 1001)"

# Queue test: restart realmforge-gate-bgs-server with MAX_BGS_LOGINS=1, then
# run 3 clients with the first holding the slot for 3s
MAX_BGS_LOGINS=1 uv run python bgs-queue-test.py \
  --count 3 --hold-secs 3
# Benchmark: 200 clients, 50 concurrent
MAX_BGS_LOGINS=5000 uv run python bgs-queue-test.py \
  --count 200 --concurrency 50 --hold-secs 0
```

The BGS client detects `ForceDisconnect` frames (banned/suspended/expired
accounts) and prints the `bgs.protocol.Result` code with a human-readable
label instead of crashing.

### 6.3 Login enforcement scenarios

Seed accounts cover unhappy paths (see `docs/test-accounts.sql`):

| Scenario | Account | ForceDisconnect code |
|---|---|---|
| Banned | `numeric@example.com` (1008) | 52 ERROR_GAME_ACCOUNT_BANNED |
| Suspended | `guest@bnet.local` (1005) | 33 ERROR_GAME_ACCOUNT_SUSPENDED |
| Expired game time | `a@b.cd` (1006) | 30 ERROR_GAME_ACCOUNT_NO_TIME |

To exercise these with `bgs-client.py`, mint a ticket for the banned
account in question:

```bash
uv run python bgs-client.py --flow full --ticket "$(bash ../dev/ticket.sh 1008)"
# Expect: ForceDisconnect: banned (account banned)
```

### 6.4 Authenticator (2FA)

Account `1002` (`admin@bnet.local`) has `has_authenticator = TRUE`. The
login returns `AUTHENTICATOR` state; the test clients submit the placeholder
code `12345678`:

```bash
python3 dev/srp_auth_client.py --server http://127.0.0.1:8080 \
  --email admin@bnet.local --password 123456
```

## 7. Containerized Run

The same flow runs against podman containers instead of local binaries.
Build the images with `deploy/build.sh`, then follow `docs/deployment.md`:
one `realmforge-net` bridge, Postgres + mailcrab + the three servers on it,
signing key as a podman secret, and `SMTP_HOST=realmforge-mail` on the
realmforge-gate-account-server. The tests in sections 4-6 then run unchanged against the
published host ports (8080/8081/8119/1119). Verified 2026-08-04: full
browser login, pages, email verification, OAuth, SRP, BGS logon/full/
character, enforcement, and authenticator all pass against the containers.

This exposed one bug fixed in the process: `app_state_with_region`
hardcoded `smtp_host = localhost` instead of using the config, so mail
never left the container. It now takes the SMTP settings from
`Config::from_env()`.

## 8. QA Gates

```bash
# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Tests (requires the DB running and seeded)
DATABASE_URL=postgres://realmforge:realmforge@localhost:5432/realmforge \
  cargo test --workspace

# Python lint
ruff check --config load-test/pyproject.toml dev/*.py load-test/*.py

# Docs lint (line length 100, atx headers, dash bullets).
# docs/test-accounts.sql is a SQL seed file, not Markdown — excluded via
# .markdownlintignore (markdownlint-cli applies it to directory arguments).
markdownlint docs/ README.md
```

All four must pass with zero failures before committing.
