# Upstream File Manifest / Intake Classification

**Upstream:** `wowemulation-dev/tavern`  
**Pinned revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Purpose:** Decide what Realmforge should vendor, use as research input, replace, or leave behind.

Classification vocabulary:

- **VENDOR** — useful interim code; preserve upstream provenance/license.
- **VENDOR-DEPENDENCY** — needed because a VENDOR component depends on it, not because it should define Realmforge.
- **RESEARCH** — valuable technical evidence/lead; do not make it production authority.
- **REWRITE** — functionality is useful but should be first-party/standards-derived early.
- **SKIP** — no useful Realmforge product value beyond keeping a verbatim upstream archive.
- **DANGER** — never adopt literally without security/provenance review.

---

# 1. Workspace root

| Path | Class | Why |
|---|---|---|
| `Cargo.toml` | VENDOR-DEPENDENCY | pinned workspace topology/dependencies; declares `AGPL-3.0-only` |
| `Cargo.lock` | VENDOR-DEPENDENCY | reproducible interim build |
| `LICENSE.md` | **VENDOR REQUIRED** | license text/provenance |
| `README.md` | RESEARCH | upstream operating notes/identity; not Realmforge product README |
| `CHANGELOG.md` | RESEARCH | useful implementation timeline and contradictions |
| `.sqlx/` | VENDOR-DEPENDENCY | SQLx offline build cache for vendored Rust build |

Do not replace Realmforge's root Cargo/project structure with the upstream root blindly. The interim source should live under a third-party boundary or independently buildable subtree.

---

# 2. Executable services

## `bin/account-server`

| Path | Class | Notes |
|---|---|---|
| `Cargo.toml` | VENDOR | interim build |
| `src/main.rs` | VENDOR | boots account HTTP/TLS service, DB migrations, health, concurrency |

Long term: replace product/HTTP shell after required compatibility routes are specified.

## `bin/oauth-server`

| Path | Class |
|---|---|
| `Cargo.toml` | VENDOR |
| `src/main.rs` | VENDOR |

Long term: standards core can be rebuilt independently; retain proprietary compatibility shim only as needed.

## `bin/bgs-server`

| Path | Class | Notes |
|---|---|---|
| `Cargo.toml` | **VENDOR** | highest-value executable |
| `src/main.rs` | **VENDOR** | BGS state, dispatch, auth/account/session services, queues |
| `src/tcp_transport.rs` | **VENDOR** | raw BGS TCP transport |
| `src/game_utilities.rs` | **VENDOR** | realm-list/join proof-of-concept; must be refactored behind Realmforge registry |

This service should be the last major inherited subsystem we independently replace.

---

# 3. `tavern-core`

| Path | Class | Replacement direction |
|---|---|---|
| `src/config.rs` | VENDOR-DEPENDENCY | Realmforge-generated config later |
| `src/domain.rs` | VENDOR-DEPENDENCY | do not make canonical Core model |
| `src/encrypted_ticket.rs` | VENDOR / CAPTURE | ticket behavior must be independently specified |
| `src/error.rs` | VENDOR-DEPENDENCY | implementation detail |
| `src/jwt.rs` | VENDOR-DEPENDENCY / REWRITE | JWT itself standards/tooling-derived; compatibility claims need capture |
| `src/key_meta.rs` | VENDOR-DEPENDENCY | signing-key implementation detail |
| `src/srp/encoding.rs` | **VENDOR** | compatibility-sensitive |
| `src/srp/groups.rs` | **VENDOR** | compatibility-sensitive |
| `src/srp/kdf.rs` | **VENDOR** | compatibility-sensitive |
| `src/srp/mod.rs` | **VENDOR** | high-value authentication implementation |
| test key fixture | SKIP/DANGER | do not adopt sample key as product secret |

SRP code is worth keeping now but should not be used as the clean-room specification for its own replacement.

---

# 4. `tavern-db`

## Repository modules

```text
account_licenses.rs
a ccounts.rs
authorization_codes.rs
bgs_sessions.rs
credentials.rs
device_authorizations.rs
game_accounts.rs
lib.rs
oauth_clients.rs
refresh_tokens.rs
service_tickets.rs
sessions.rs
```

(`accounts.rs` is the intended path; the spacing above is only explanatory—do not create a typo in source intake.)

### Classification

| Area | Class |
|---|---|
| DB bootstrap/pool/migrations | VENDOR-DEPENDENCY |
| credentials/game accounts | VENDOR |
| service tickets | VENDOR |
| BGS sessions | VENDOR |
| OAuth auth codes/refresh/client rows | VENDOR for interim |
| account licenses | VENDOR only if required by interim responses |
| device authorization | VENDOR only if desktop device flow is retained |

## Migrations

All 25 migrations are needed if we vendor the pinned binaries unchanged, because SQLx queries and runtime migration state assume the schema history.

That does **not** mean every table belongs in future Realmforge.

See `DB_MIGRATIONS.md` for disposition per migration.

---

# 5. `tavern-oauth`

| Path | Class | Notes |
|---|---|---|
| `Cargo.toml` | VENDOR |
| `src/lib.rs` | VENDOR now / REWRITE later | mixed standards + proprietary compatibility |
| `tests/oauth_flow.rs` | VENDOR with covered source; RESEARCH for replacement | do not copy as clean-room tests |

Replacement research should derive generic OAuth/OIDC behavior from RFCs rather than reverse-engineering upstream code.

---

# 6. `tavern-account`

Core source paths found at pinned revision:

```text
api.rs
bnet.rs
bnet_types.rs
callback.rs
client_login.rs
creation.rs
email.rs
lib.rs
locale.rs
login.rs
registration.rs
security.rs
session.rs
ticket.rs
ui.rs
```

## High-value compatibility files

| Path | Class | Why |
|---|---|---|
| `bnet.rs` | **VENDOR** | game-client bnet login/SRP/game-account/ticket surface |
| `bnet_types.rs` | **VENDOR** | request/response types |
| `client_login.rs` | **VENDOR** | older client login/legal/authenticator paths |
| `login.rs` | **VENDOR** | browser/external SRP login semantics |
| `session.rs` | VENDOR | cookie/session helpers used by login |
| `ticket.rs` | **VENDOR** | compatibility ticket minting |
| `callback.rs` | VENDOR only while account OAuth UI is retained |
| `lib.rs` | VENDOR | router/state glue |

## Product/UI-heavy files

| Path | Class | Notes |
|---|---|---|
| `api.rs` | VENDOR only because current UI may call it; many routes should disappear later |
| `creation.rs` | VENDOR only if using upstream registration temporarily; contains fake phone/captcha behavior |
| `email.rs` | REWRITE product-owned |
| `locale.rs` | REWRITE/skip unless needed for interim UI |
| `registration.rs` | VENDOR initially; replace with Realmforge identity enrollment |
| `security.rs` | REWRITE product security; preserve protocol-required pieces separately |
| `ui.rs` | SKIP as final product UI |

## Static assets/templates

```text
static/spa.js
static/srp6a.js
static/style.css
logo assets
favicon
templates/creation
templates/dashboard
templates/landing
templates/login
```

Disposition:

- `srp6a.js`: **VENDOR if browser SRP flow needs it**.
- generic logic inside `spa.js`: VENDOR only for interim account UI.
- visual CSS/templates/logos/favicon: **SKIP AS REALMFORGE PRODUCT UI**.

If we import an exact archival snapshot, these can remain inside the clearly marked third-party tree. Realmforge Console/Client must not depend on their branding.

## Upstream account tests

```text
api_ui
bnet_login
creation_flow
locale_emails
srp_login
tos_flow
```

Class: **VENDOR with upstream source** as regression protection for the interim component.

They are not valid clean-room replacement tests because they are part of the covered implementation. Our independent harness must live elsewhere.

---

# 7. `tavern-bgs`

| Path | Class | Why |
|---|---|---|
| `proto/bgs.proto` | **VENDOR** | interim protobuf build input; replacement must independently source/verify protocol definitions |
| `src/frame.rs` | **VENDOR** | BGS framing |
| `src/method_id.rs` | **VENDOR** | service method table |
| `src/result_code.rs` | **VENDOR** | error/result behavior |
| `src/service_hash.rs` | **VENDOR** | service routing/hash table |

All are high-value for the interim Gate and high-priority independent research targets later.

---

# 8. Observability

Pinned paths:

```text
crates/tavern-observability/src/health.rs
crates/tavern-observability/src/http.rs
crates/tavern-observability/src/lib.rs
crates/tavern-observability/src/pool.rs
```

Class: `VENDOR-DEPENDENCY` now, `REWRITE` early.

There is little strategic value in keeping Realmforge tied to upstream metric names/health semantics after the interim Gate works.

---

# 9. Deployment assets

```text
deploy/account-server.Containerfile
deploy/bgs-server.Containerfile
deploy/oauth-server.Containerfile
deploy/build.sh
```

Class: `VENDOR-DEPENDENCY / RESEARCH`.

Useful for reproducible first Gate build. Realmforge Forge should eventually generate/manage deployment itself.

Do not turn upstream container names, users, paths, or network names into Realmforge public architecture by accident.

---

# 10. Developer/research tools

Pinned `dev/` includes:

```text
Containerfile
db.sh
fuzz.py
mail.sh
otel-collector-config.yaml
otel.sh
srp_auth_client.py
srp_oracle.py
ticket.sh
```

Classification:

| File/group | Class |
|---|---|
| SRP oracle/client | RESEARCH + VENDOR with archive; do not use as independent clean-room oracle |
| ticket helper | RESEARCH |
| fuzz helper | RESEARCH |
| DB/mail/OTel scripts | VENDOR-DEPENDENCY only if useful for interim dev environment |

Realmforge should build its own black-box harness under a first-party research/test tree.

---

# 11. Load tests

Pinned load-test assets include:

```text
bgs-client.py
bgs-queue-test.py
bgs_proto/bgs_pb2.py
login-harness.py
seed-loadtest-accounts.py
pyproject.toml
uv.lock
```

Class: `VENDOR / RESEARCH` for upstream regression testing.

Important boundary:

Upstream synthetic clients can prove the upstream server behaves consistently with itself. They **cannot** prove compatibility with a real retired game client.

Realmforge must maintain separate first-party target-client evidence.

---

# 12. Upstream docs

Useful pinned docs include:

```text
architecture.md
bgs-protocol-versions.md
deployment.md
oauth-oidc-implementation.md
performance-scaling.md
plan.md
protocol-overview.md
spa-design.md
srp.md
test-accounts.md
test-data.md
testing.md
```

Class: `RESEARCH`.

They should be vendored with an archival source snapshot if convenient, but Realmforge architecture must not link to them as authority.

Known contradictions already prove why:

- BGS v2 status differs across docs/current implementation.
- GenerateAuthToken is described as stubbed while active dispatch code exists.
- RequestDisconnect documentation and current dispatcher behavior differ.

---

# 13. Keys/secrets

Pinned upstream contains:

```text
keys/README.md
keys/signing.pem
keys/signing.pem.meta
```

Classification:

- README/meta: `RESEARCH`.
- actual signing PEM: **DANGER / DO NOT USE AS REALMFORGE PRODUCTION SECRET**.

If an exact source snapshot includes it because it is already public upstream material, mark it unequivocally as test/public material and ensure Realmforge deployment never references it.

Realmforge must generate its own secrets.

---

# 14. `.sqlx` offline cache

The upstream deployment process explicitly relies on its committed SQLx offline cache to build without a live database.

Class: `VENDOR-DEPENDENCY` for an exact source intake.

When Realmforge begins modifying vendored SQL queries, regenerate/update the cache under the rules of the covered third-party component rather than hand-editing query metadata.

---

# 15. Minimum functional intake versus full archival intake

## Minimum functional Gate intake

```text
Cargo workspace metadata needed to build
Cargo.lock
LICENSE.md
.sqlx/
bin/account-server/
bin/oauth-server/
bin/bgs-server/
crates/tavern-core/
crates/tavern-db/
crates/tavern-account/
crates/tavern-oauth/
crates/tavern-bgs/
crates/tavern-observability/
deploy/ as needed
```

## Full archival intake

Also preserves:

```text
upstream docs
dev tools
load tests
CHANGELOG
README
static assets/templates
```

### Recommendation

For provenance and reproducibility, **take a full pinned source snapshot into the third-party area**, then make Realmforge's build select only the components we actually use.

Why: trying to cherry-pick individual files now risks missing build/test/schema dependencies and makes it harder to reproduce what we originally inherited.

The full snapshot remains clearly third-party. Realmforge's own product source lives outside it.

---

# 16. Never confuse archive with authority

A full source snapshot is valuable because it freezes the exact interim implementation.

It is **not** the Realmforge reverse-engineering specification.

The independent authority remains:

```text
docs/reconstruction/
docs/research/
future independent capture corpus
future first-party black-box tests
```

When Gate replacement begins, implementation agents should consume the independent reconstruction package, not this vendored source tree.
