# The Tavern

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/assets/tavern-logo-monochrome.svg">
    <source media="(prefers-color-scheme: light)" srcset="docs/assets/tavern-logo-monochrome.svg">
    <img alt="The Tavern" src="docs/assets/tavern-logo-monochrome.svg" width="200">
  </picture>
</div>

A Rust replacement for Blizzard's Battle.net account services, built for the
digital preservation of retired World of Warcraft Classic client builds.

<div align="center">

[![Discord](https://img.shields.io/discord/1394228766414471219?logo=discord&style=flat-square)](https://discord.gg/Jj4uWy3DGP)
[![License](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)
![Status](https://img.shields.io/badge/status-actively_developed-brightgreen.svg)

</div>

The Tavern lets you run your own account and authentication server so that
abandoned WoW Classic clients can still log in and play. It targets the Classic
re-release client lines — Vanilla, The Burning Crusade, Wrath of the Lich King,
and Cataclysm Classic — up to defined build cutoffs (see [Supported Client
Builds](#supported-client-builds)).

The Tavern is part of the [wowemulation-dev][org] collection of free and
open-source World of Warcraft server components. It is independent of, and not
affiliated with, Blizzard Entertainment.

> **Status:** Actively developed. Three runnable services:
> br> `oauth-server`, `account-server`, and `bgs-server`. The protocol surface
> br> (browser login, game-client login, OAuth/OIDC, BGS v1/v2 transport, desktop
> br> app SSO) is functional. See [`docs/plan.md`](docs/plan.md).

## Supported Client Builds

Tavern's preservation target is the set of World of Warcraft Classic client
builds listed below. All builds in each line, up to and including the cutoff,
are in scope; later builds and other client lines are not.

| Client line           | Recreates                    | Build cutoff |
| --------------------- | ---------------------------- | ------------ |
| 1.13.x (2019 to 2021) | Vanilla, 2005 (v1.12)        | 39692        |
| 1.14.x (2021 to 2023) | Vanilla, 2005 (v1.12)        | 51535        |
| 2.5.x through 2.5.4   | The Burning Crusade, 2007    | 44833        |
| 3.4.x through 3.4.4   | Wrath of the Lich King, 2008 | 61581        |
| 4.4.x through 4.4.2   | Cataclysm, 2010              | 60895        |

Build numbers are the values reported by the client at its login screen and
catalogued at [warcraft.wiki.gg][public-builds].

## What It Preserves

The Tavern exists to keep the retired client builds listed above playable.
These builds, as originally released, have been superseded by Blizzard's
current live offerings; the Tavern preserves them.

The Tavern does not target, and is not intended for use with:

- Any build above the cutoffs listed in
  [Supported Client Builds](#supported-client-builds).
- Any client version Blizzard currently serves. As of June 2026, that includes
  Classic Era (1.15.x), the Wrath-based Titan Reforged service (3.80.x), Mists
  of Pandaria Classic (5.5.x), the Burning Crusade Classic Anniversary Edition
  (2.5.5), and retail World of Warcraft / Midnight (12.0.x).
- Live Blizzard services or any environment where Blizzard currently provides
  authentication.

The intent is preservation of retired client builds, not circumvention of any
active commercial offering.

## What It Provides

The Tavern provides three services that replace the Battle.net account,
authentication, and BGS-transport surfaces. All three share one Postgres
account database.

### oauth-server — OAuth / OIDC Provider

- Discovery document (`/.well-known/openid-configuration`)
- RS256 JWT signing with JWKS endpoint
- Authorization code flow with PKCE
- Refresh token rotation
- Client credentials grant (Basic auth)
- RFC 8693 token-exchange for the Battle.net desktop app
- `POST /sso` `client_sso` grant (Phoenix SSO)
- Device authorization grant (RFC 8628)
- Token introspection and userinfo endpoints
- Wire format validated against real Battle.net captures

### account-server — Account Management

- **Browser SRP6a login**: two-step form-encoded (`POST /login/{locale}/` +
  `/login/{locale}/password`), SSO cookies (`login.key`, `opt`, `SESSIONID`)
- **Game-client bnet login**: `LoginForm` envelope with dual proof schemes —
  BnetSRP6v2 (TBC/WotLK/Cata) and plaintext `sha_pass_hash` (Vanilla 1.13/1.14)
- **Account registration**: multi-step wizard with CSRF protection
- **Session management**: SESSIONID cookie (HttpOnly), XSRF-TOKEN double-submit
- **Management API** (`/api/*`): user profile, game accounts, environment config
- **SPA dashboard**: client-rendered UI, server-rendered login/creation forms

### bgs-server — Battle.net Game Service Transport

- **Dual protocol**: BGS v1 (raw TCP, port 1119) for 1.13.x, WebSocket BGS v2
  (port 8119) for 1.14.x+ and desktop app
- **Auto-detection**: v2 service hashes detected on first dispatch frame
- **Binary RPC framing**: 2-byte BE header size + protobuf Header + body
- **Service dispatch**: ConnectionService (Connect/ForceDisconnect),
  AuthenticationService (Logon/VerifyWebCredentials/GenerateSSOToken),
  GameUtilitiesService (realm list, character count), AccountService
- **ForceDisconnect**: session-duplicate (57) and server-shutdown (86) error
  codes with `DisconnectNotification` body
- **Login queue**: FIFO dequeue with periodic position updates (both
  protocol versions)
- **Per-account sessions**: DashMap tracking, auto-kick on duplicate login
- **Graceful shutdown**: SIGTERM handler broadcasts ForceDisconnect, drains
  connections, flushes metrics

### Performance & Observability

- OpenTelemetry metrics exported via OTLP/HTTP (request count/duration,
  SRP timing, DB pool usage)
- Kubernetes health probes (`/health`, `/ready`, `/startup`)
- DashMap concurrency (lock-free session/challenge maps, ~26% throughput
  improvement)
- `tokio::task::spawn_blocking` for 2048-bit SRP modular exponentiation
- Configurable DB pool with `DB_POOL_MAX_CONNECTIONS` env var
- In-memory map eviction: background reaper (60s) for expired entries
- Per-email rate limiting on SRP challenges
- Admission control via tokio Semaphore with 503 on overload

### Development Tools

- `uv`-managed Python project in `load-test/`
- `bgs-client.py`: end-to-end BGS test client (logon, restore, character flow)
- `bgs-queue-test.py`: concurrent login queue stress test
- `login-harness.py`: SRP login load test with latency percentiles
- `seed-loadtest-accounts.py`: mass account generator for scale testing
- `dev/srp_auth_client.py`: end-to-end SRP auth test

### Out of Scope

- Game world, realm emulation, and gameplay systems
  xPm
- Any client version Blizzard currently serves, or any build above the
  supported cutoffs

## How It Works

Tavern runs three services sharing one Postgres account database:

- `oauth-server` at `oauth.wowemu.dev` — OAuth/OIDC endpoints. Access tokens
  are RS256 JWTs. Refresh tokens are opaque and stored.
- `account-server` at `account.wowemu.dev` — Browser login, game-client bnet
  login, account registration, management API, and the dashboard SPA.
- `bgs-server` (BGS v1/v2 transport) — WebSocket RPC for the desktop app
  and Agent-driven logins, plus raw TCP for 1.13.x game clients.

## For Developers

### Tech Stack

- Language: Rust (stable, edition 2024).
- Toolchain management: [mise][mise].
- Linting: `cargo clippy` and the configured markdownlint rules.

### Development

Prerequisites are installed with mise:

```bash
mise install
# Python test tools additionally need uv
uv sync --directory load-test
```

#### Dev database (Postgres 16 in podman)

Tavern needs a PostgreSQL 16 database for development and tests. The
`dev/db.sh` wrapper runs Postgres 16 in a `podman` container with a fixed
user, password, and database, persisted in a named volume.

```bash
bash dev/db.sh up     # create + start the tavern-db container (first run)
podman start tavern-db  # start it again after a reboot
```

The container is `tavern-db`, bound to `127.0.0.1:5432`. The default
connection is `postgres://tavern:tavern@localhost:5432/tavern`.

All `dev/db.sh` subcommands:

| Command | Effect                                                            |
| ------- | ----------------------------------------------------------------- |
| `up`    | Create and start the container (creates the volume on first run). |
| `down`  | Stop and remove the container; the data volume is retained.       |
| `reset` | `down`, delete the volume, then `up` — a fresh database.          |
| `logs`  | Tail the last 50 container log lines.                             |
| `psql`  | Open a `psql` shell as the `tavern` user.                         |

`docker` is API-compatible with `podman`; the project standardizes on
`podman`. Migrations run automatically on server startup. To apply them
before compiling against a fresh database (the `sqlx::query!` macros check
the schema at compile time), run:

```bash
DATABASE_URL=postgres://tavern:tavern@localhost:5432/tavern \
  cargo sqlx migrate run --source crates/tavern-db/migrations
```

#### Run the servers

Run the servers:

```bash
# OAuth provider (oauth.wowemu.dev) — issuer must match BIND_ADDR
DATABASE_URL=postgres://tavern:tavern@localhost:5432/tavern \
  BIND_ADDR=127.0.0.1:8081 ISSUER_URL=http://127.0.0.1:8081 \
  cargo run -p oauth-server

# Account service (account.wowemu.dev) — INSECURE_COOKIES for local HTTP
INSECURE_COOKIES=1 DATABASE_URL=postgres://tavern:tavern@localhost:5432/tavern \
  cargo run -p account-server

# BGS transport (default 127.0.0.1:8119; no database needed)
cargo run -p bgs-server
```

`oauth-server` and `account-server` need `DATABASE_URL` and run migrations on
startup. `bgs-server` is stateless (sessions held in memory) and does not.

The oauth-server ships with a demo signing key at `keys/signing.pem`.
It logs a warning if used in production — see `keys/README.md` for how
to generate a production key.

Run tests and lint:

```bash
DATABASE_URL=postgres://tavern:tavern@localhost:5432/tavern cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Integration tests require a live database (they connect via `DATABASE_URL`
and run migrations on startup). Unit tests in `tavern-core` do not.

`sqlx::query!` calls are checked against `.sqlx/` (the offline query cache,
gitignored and regenerated locally). After changing a query or the schema,
regenerate it against the running database, then commit the result:

```bash
DATABASE_URL=postgres://tavern:tavern@localhost:5432/tavern \
  cargo sqlx prepare --workspace
```

### Protocol Details

For implementation details, see the documentation:

- [Architecture](docs/architecture.md)
- [Protocol Overview](docs/protocol-overview.md)
- [OAuth / OIDC Implementation](docs/oauth-oidc-implementation.md)
- [SRP6a Implementation](docs/srp.md)
- [Testing End to End](docs/testing.md)

## License

This project is for educational and preservation purposes, and licensed under
the terms of the Affero General Public License, version 3.0 or later.

[LICENSE.md](LICENSE.md) contains the full license text along with your rights
and duties.

[org]: https://github.com/wowemulation-dev
[mise]: https://mise.jdx.dev/
[public-builds]: https://warcraft.wiki.gg/wiki/Public_client_builds

---

**Note**: This project is not affiliated with Blizzard Entertainment. It is an
independent implementation based on reverse engineering by the World of Warcraft
emulation community.
