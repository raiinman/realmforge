# Upstream Configuration Surface — Pinned Intake Map

**Upstream:** `wowemulation-dev/tavern`  
**Revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`

This is an inventory of configuration knobs Realmforge must understand before vendoring the interim Gate. It is not a recommendation that Realmforge copy these names into its own first-party configuration.

---

# 1. Shared configuration object

Pinned `tavern-core/src/config.rs` reads these keys:

| Variable | Required | Default / behavior | Used for |
|---|---|---|---|
| `DATABASE_URL` | yes | none | PostgreSQL connection |
| `BIND_ADDR` | no | `127.0.0.1:8080` in shared Config | service HTTP listener |
| `ISSUER_URL` | no | `http://localhost:8080` | OAuth issuer/`iss` |
| `SIGNING_KEY_PATH` | no | `keys/signing.pem` | RSA signing key path |
| `SMTP_HOST` | no | `localhost` | outgoing account mail |
| `SMTP_PORT` | no | `1025` | SMTP port |
| `SMTP_FROM` | no | `noreply@wowemu.dev` | sender address |
| `REGION` | no | `US`; only 2-char values accepted | tickets/auth region |
| `DB_POOL_MAX_CONNECTIONS` | no | DB layer default | pool tuning |
| `DB_POOL_MIN_CONNECTIONS` | no | DB layer default | pool tuning |
| `DB_POOL_ACQUIRE_TIMEOUT_SECS` | no | DB layer default | pool tuning |
| `DB_POOL_MAX_LIFETIME_SECS` | no | DB layer default | pool tuning |
| `DB_POOL_IDLE_TIMEOUT_SECS` | no | DB layer default | pool tuning |
| `TLS_CERT_PATH` | no | unset | account/OAuth HTTPS cert |
| `TLS_KEY_PATH` | no | unset | account/OAuth HTTPS private key |

### Intake consequence

The shared config shape is too generic for Realmforge Core. Gate can initially keep it internally, but Realmforge should eventually expose explicit product configuration and generate compatibility-service configuration from that.

`DATABASE_URL` is the only key `Config::from_pairs` requires.

---

# 2. Runtime/performance knobs

| Variable | Observed behavior |
|---|---|
| `TOKIO_WORKER_THREADS` | worker count; otherwise uses available parallelism with fallback |
| `MAX_CONCURRENT_REQUESTS` | account/OAuth request semaphore, default `1024`; overload returns HTTP 503 |
| `RUST_LOG` / tracing env | consumed through tracing env-filter behavior |

Realmforge should not expose raw Tokio implementation knobs as its primary operator UX. If retained, keep them in an advanced/service-specific section.

---

# 3. Account-service-specific environment behavior

| Variable | Observed behavior | Realmforge disposition |
|---|---|---|
| `TOS_VERSION` | current required legal-version string; upstream fallback `2026-08-05` | compatibility profile, not global product truth |
| `ACCOUNT_BASE_URL` | links emitted by account server; default `http://localhost:8080` | generate from deployment config |
| `INSECURE_COOKIES` | when nonzero/present, relaxes Secure/SameSite cookie behavior for local HTTP dev | dev-only; never silently enable in public profile |

`cookie_domain` exists in upstream application state but the pinned normal constructors initialize it to an empty string. Treat any documentation calling it configurable as another behavior/configuration point to verify before depending on it.

---

# 4. OAuth-service-specific behavior

| Variable | Observed behavior | Realmforge disposition |
|---|---|---|
| `ACCOUNT_SERVER_URL` | account login UI base; default `http://localhost:8080` | generate from Gate topology |
| `ISSUER_URL` | issuer + discovery URL | explicit Gate endpoint config |

The standards-facing issuer must remain stable enough for issued tokens and discovery consumers; changing it is not merely cosmetic.

---

# 5. BGS-specific configuration

| Variable | Observed behavior | Default |
|---|---|---|
| `BIND_ADDR` | WebSocket listener address | service-specific runtime value; deployment example `0.0.0.0:8119` |
| `TCP_BIND_ADDR` | raw TCP/TLS listener for Classic 1.13.x | `127.0.0.1:1119` |
| `MAX_BGS_LOGINS` | active-login capacity / queue threshold | `5000` in pinned code |
| `BGS_TLS_CERT` | cert for raw TCP BGS TLS | unset => synthetic-client plaintext mode |
| `BGS_TLS_KEY` | key for raw TCP BGS TLS | unset => synthetic-client plaintext mode |
| `REALM_ADDRESS` | world/realm IP handed out by realm join | `127.0.0.1` |
| `REALM_PORT` | world/realm port handed out by realm join | `8085` |

### Important security/interop note

Pinned source explicitly distinguishes real 1.13.2 client TLS expectations from plaintext synthetic tests. A passing synthetic-client test on plaintext port 1119 is not evidence that a real unmodified target client will connect.

---

# 6. Service ports in pinned deployment docs

| Service | Default/documented listener |
|---|---|
| account-server | `8080` HTTP/HTTPS |
| oauth-server | `8081` HTTP/HTTPS |
| bgs-server WebSocket | `8119` |
| bgs-server raw TCP/TLS | `1119` |
| external realm/world | `8085` default handoff target |
| PostgreSQL | `5432` typical deployment |
| development SMTP | `1025` example |

Realmforge must treat these as configurable implementation ports, not hard-coded product invariants.

---

# 7. Files/secrets expected by upstream

## Signing key

Pinned services read an RSA PEM from `SIGNING_KEY_PATH`.

The upstream repository contains a `keys/` area including a signing PEM and metadata. **Do not treat any repository key as a Realmforge production secret.** Realmforge must generate and manage deployment-specific keys through its secret-management path.

## TLS files

Account/OAuth HTTPS:

```text
TLS_CERT_PATH
TLS_KEY_PATH
```

Raw BGS/TCP TLS:

```text
BGS_TLS_CERT
BGS_TLS_KEY
```

These are separate configuration paths in the interim implementation.

---

# 8. Configuration smells Realmforge should remove

## Shared `BIND_ADDR` defaults

A shared default of `127.0.0.1:8080` is convenient for independent binaries but insufficient as product authority when multiple services run together. Realmforge should assign endpoints from one deployment topology.

## Realm handoff is only one address

`REALM_ADDRESS` / `REALM_PORT` assumes a single hard-coded external realm. This cannot survive Realmforge's dynamic multi-realm registry.

## Region is global

A single `REGION` process variable may be sufficient for the proof-of-concept but Realmforge should determine whether region belongs to deployment, account, realm, client compatibility profile, or some combination.

## ToS version is a process string

Realmforge should not inherit a historical Battle.net legal version as product policy. If a client requires a compatibility legal gate, it belongs to that client contract.

## Insecure-cookie toggle

Developer convenience must never become a deployment footgun. Realmforge should make local HTTP mode explicit and prevent it in a public profile unless the operator deliberately overrides a warning.

## Static world address

The Gate should request a realm-join target from Core/Forge, not read a process-global `REALM_ADDRESS` forever.

---

# 9. Realmforge target configuration model

Do not expose 20 unrelated environment variables to ordinary users.

Target conceptual product configuration:

```yaml
realmforge:
  public_base_url:
  admin_bind:
  data_dir:
  database:
  security_profile: homelab | public

compatibility_gate:
  public_hostname:
  region:
  websocket:
  tcp:
  tls:

email:
  enabled:
  smtp:

operations:
  concurrency:
  database_pool:
```

Realm runtimes themselves are Core objects, so realm addresses/ports should come from the registry rather than global config.

Advanced service-specific environment values can still exist as generated internals.

---

# 10. Intake checklist

Before the interim Gate is considered reproducibly deployable under Realmforge:

- [ ] every environment key is captured in this document,
- [ ] every default is verified from pinned source,
- [ ] every secret-bearing key is identified,
- [ ] local-development insecure modes are clearly separated,
- [ ] Realmforge generates deployment-specific signing/TLS keys,
- [ ] upstream repository keys are not used as production credentials,
- [ ] single-realm env settings are replaced/projected from Core for real multi-realm use,
- [ ] startup validates contradictory/missing TLS pairs,
- [ ] health checks clearly identify bad configuration.
