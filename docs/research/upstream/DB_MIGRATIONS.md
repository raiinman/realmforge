# Upstream Database / Migration Inventory

**Upstream:** `wowemulation-dev/tavern`  
**Revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Migration count:** 25 (`0001`–`0025`)

This document answers two different questions:

1. What database state does the interim Gate expect?
2. Which of that state should Realmforge actually own long term?

Those answers are deliberately not the same.

---

# 1. Migration ledger

| Migration | Main effect | Realmforge disposition |
|---|---|---|
| `0001_init.sql` | enables `citext`; creates `accounts`, `credentials` | KEEP NOW; replace with Realmforge identity projection later |
| `0002_oauth.sql` | creates `oauth_clients`, `authorization_codes`, `refresh_tokens`, `sessions`, `service_tickets`, `signing_keys`, `account_licenses` | KEEP NOW; standards pieces later REWRITE |
| `0003_seed_oauth_clients.sql` | seeds observed OAuth clients | KEEP only for compatible consumers; independently inventory IDs |
| `0004_game_accounts_and_shapass.sql` | adds `credentials.sha_pass_hash`; creates `game_accounts` | KEEP NOW / P0 compatibility |
| `0005_phoenix_client_secret.sql` | adjusts/seeds Phoenix desktop-client OAuth secret state | CAPTURE consumer before long-term retention |
| `0006_account_profile_fields.sql` | adds name, birth date, mobile, country profile fields | mostly DO NOT make canonical; product-owned identity later |
| `0007_account_address.sql` | adds postal-address fields | DEFER / upstream account UI only |
| `0008_device_authorizations.sql` | creates RFC 8628 device-code state | optional; REWRITE from standard if needed |
| `0009_bgs_sessions.sql` | creates persistent BGS SSO/session-restore state | KEEP NOW / P0 |
| `0010_bgs_sessions_client_info.sql` | adds build/platform/locale to BGS sessions | KEEP NOW / important compatibility metadata |
| `0011_game_account_status.sql` | adds suspended/banned/time-expiry state to `game_accounts` | KEEP NOW if clients consume status; map from Realmforge policy later |
| `0012_account_authenticator.sql` | adds `accounts.has_authenticator` | DEFER unless compatible 2FA required |
| `0013_device_auth_fk.sql` | adds account/client FKs to device authorization | optional standards implementation detail |
| `0014_catalog_schema.sql` | creates catalog/license/product/rule tables and seeds WoW product/license data | DO NOT IMPORT unless entitlement/catalog feature is explicitly needed |
| `0015_countries.sql` | creates `countries`, `country_adulthood` and seeds 244 rows | DO NOT IMPORT for Gate critical path |
| `0016_communication_preferences.sql` | creates account communication-preference state | DO NOT IMPORT unless product feature |
| `0017_privacy_settings.sql` | creates privacy-settings state | DO NOT IMPORT unless product feature |
| `0018_account_connections.sql` | creates third-party account-connection state | DO NOT IMPORT unless product feature |
| `0019_local_oauth_client.sql` | seeds local account-management OAuth client | keep only while upstream account UI is used |
| `0020_locales_privacy_email.sql` | adds locale/reference and privacy/email support state | UI/product-only unless a client consumes it |
| `0021_countries_alpha2.sql` | extends country reference data with alpha-2 values | DO NOT IMPORT for critical path |
| `0022_license_game_account.sql` | scopes licenses to specific game accounts; changes uniqueness/FK model | KEEP only if entitlement replies require it |
| `0023_birth_date_mandatory.sql` | forces/backfills DOB with `1990-01-01` | **DO NOT inherit as Realmforge identity policy** |
| `0024_tos_acceptance.sql` | adds ToS acceptance/version fields | compatibility-only if 1.13.x legal gate requires it |
| `0025_bgs_session_key.sql` | persists BGS session key for restore + realm join | KEEP NOW / P0 |

---

# 2. Base schema from `0001`

## `accounts`

Pinned base fields:

```text
id BIGSERIAL PK
email CITEXT UNIQUE NOT NULL
email_verified BOOLEAN
battletag TEXT
country_code CHAR(3)
region SMALLINT
locale TEXT
created_at
updated_at
```

Later migrations expand this row with product/account-site fields including:

```text
first_name
last_name
birth_date
mobile_number
country_id
street1
street2
city
state
postal_code
has_authenticator
tos_version
tos_accepted_at
```

### Realmforge consequence

Do not simply rename this table to `users` and call it our model.

This is an upstream compatibility/account-site aggregate. Realmforge's product user should be a separate canonical model, with only the compatibility fields a target client actually requires projected into Gate.

## `credentials`

Base fields:

```text
account_id PK/FK
srp_salt
srp_verifier
srp_iterations
srp_version
updated_at
```

Later addition:

```text
sha_pass_hash
```

This dual-credential design exists to serve materially different client lines from one account.

### Realmforge consequence

Long-term model should distinguish:

```text
ProductCredential
CompatibilityCredential
  - protocol_family
  - verifier_type
  - verifier_material
  - metadata/version
```

rather than forcing Realmforge's own admin/player password policy to match a historical protocol.

---

# 3. OAuth/session tables from `0002`

## `oauth_clients`

Stores:

```text
client_id PK
client_secret_hash nullable
redirect_uris[]
scopes[]
allowed_grants[]
require_2fa
```

## `authorization_codes`

Stores one-time OAuth authorization codes plus:

```text
client
account
scope
redirect URI
PKCE challenge
nonce
expiry
used flag
```

## `refresh_tokens`

Stores opaque refresh tokens with:

```text
account
client
scope
expiry
rotated_from self-reference
```

## `sessions`

Browser session backing state:

```text
UUID session_id
account
expiry
created_at
user_agent
ip
```

## `service_tickets`

One-time login/OAuth bridge tickets:

```text
st PK
account
region
expiry
used
```

## `signing_keys`

Contains **private key material in the database**:

```text
kid
private_key_pem
public_jwk JSONB
created_at
retired_at
```

This table deserves an explicit Realmforge security review before adoption. Core should not casually copy private signing keys through backups/logs/export flows.

## `account_licenses`

Begins as account+license ownership and is later extended by `0022` for nullable per-game-account scope.

---

# 4. Game accounts

`0004` creates:

```text
game_accounts
- id
- account_id
- name
- region
- created_at
UNIQUE(account_id, name)
```

`0011` adds:

```text
is_suspended
is_banned
suspension_expires
game_time_expires
```

### Realmforge consequence

A Realmforge account may need multiple game-account projections. Do not assume the human-visible game account name is a stable global identifier.

Capture per-build expectations for:

- game-account handle encoding,
- numeric ID use,
- local selection behavior,
- region relationship,
- ban/suspension/time status rendering.

---

# 5. BGS session persistence

`0009` creates:

```text
bgs_sessions
- sso_id BYTEA PK
- account_id
- battle_tag
- game_accounts JSONB
- expires_at
- created_at
```

`0010` adds:

```text
build
platform
locale
```

`0025` adds:

```text
session_key BYTEA
```

Upstream comments say the session key is persisted so restored sessions can still supply `Param_BnetSessionKey` during realm join.

### P0 security/reconstruction questions

- Is the full session key required to be persisted?
- What lifetime is required by each real client?
- Can it be encrypted at rest?
- Is the SSO ID bearer-authentication material?
- What invalidation happens on duplicate login/logout/password change?
- Does session restore need all serialized game-account state or can it re-query Core?
- What is the exact client retry/expiry behavior?

Realmforge must answer these from independent evidence before designing its permanent session store.

---

# 6. Device authorization

`0008` creates `device_authorizations`:

```text
device_code PK
user_code UNIQUE
client_id
account_id nullable
scope
expires_at
approved
created_at
```

`0013` later adds FKs to account and OAuth client.

This is mostly standards-derived RFC 8628 behavior. Realmforge can reimplement it independently from the standard if device authorization remains a product requirement.

---

# 7. Catalog / entitlement schema

`0014` creates:

```text
catalog_licenses
catalog_programs
catalog_rules
catalog_products
catalog_product_licenses
```

and seeds WoW product/license data.

It also adds a FK from `account_licenses` to catalog license IDs.

### Realmforge disposition

This is a large amount of Battle.net-account-site fidelity that is not necessary merely to host a retired private realm.

Default disposition:

`DO NOT IMPORT INTO REALMFORGE CORE`

For an interim verbatim Gate snapshot, migrations may be required to keep the upstream binary runnable. If so they stay confined to the third-party Gate database/schema.

Long term Realmforge needs a far simpler semantic answer:

```text
Can this player use this game/client/realm?
```

Only expand into a store/catalog if Realmforge actually develops that product feature.

---

# 8. Country/age/address data

`0015` creates and seeds:

```text
countries
country_adulthood
```

The migration says the values came from account.battle.net captures and defaults all 244 listed countries to adulthood age 18.

`0021` adds alpha-2 country support.

`0023` makes `birth_date` mandatory and backfills missing values with a fixed adult date (`1990-01-01`).

### Realmforge disposition

This is upstream account-portal emulation policy, not a valid Realmforge identity requirement.

Do not require birth dates merely because the interim schema does.

If the third-party service temporarily needs a value to function, keep that compatibility concern isolated and plan its removal.

---

# 9. Preferences/privacy/connections

Later migrations add:

```text
communication_preferences
privacy_settings
account_connections
supported_locales / related localization reference state
```

These support account-site fidelity rather than the client-to-realm critical path.

Default: `DO NOT IMPORT INTO CORE`.

If vendored upstream migrations create these tables, they remain third-party Gate implementation details until removed.

---

# 10. ToS/legal gate

`0024` adds:

```text
accounts.tos_version
accounts.tos_accepted_at
```

The migration documents an observed 1.13.2 `LEGAL` authentication state in which a login ticket must not be issued until the current agreement is accepted.

This is potentially client-critical and therefore cannot simply be deleted because Realmforge does not care about mimicking Blizzard's account website.

Required independent research:

1. Does each target 1.13 build actually enter this state?
2. What response fields are mandatory?
3. Can Realmforge provide its own agreement/version while satisfying the client state machine?
4. Is a one-time bootstrap acceptance sufficient?
5. What happens when the server never returns `LEGAL`?

Until answered: `P0/P1 depending on exact target build`.

---

# 11. Repository DB modules

Pinned `tavern-db/src/` modules:

```text
account_licenses.rs
accounts.rs
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

Notably, many later account-site tables are accessed directly through other code/queries rather than each receiving a dedicated repository module.

Realmforge should not use this module layout as a reason to mirror the schema.

---

# 12. Data ownership split for Realmforge

## Realmforge Core should ultimately own

```text
RealmforgeUser
AdminRole / RealmMembership
Realm
RealmRuntime
ClientProfile
CompatibilityProfile
EmulatorInstallation
Backup
Deployment
AuditEvent
```

## Gate should own/project only what compatibility requires

```text
CompatibilityAccountHandle
GameAccountProjection
ProtocolCredentialVerifier
CompatibilitySession
CompatibilityTicket
CompatibilityOAuthClient
CompatibilityEntitlement
```

## Emulator adapter/core owns

```text
characters
game world state
quests/items/spells/world DB
emulator-native account/session rows when unavoidable
```

The integration contract should map identity across these domains explicitly rather than sharing tables by accident.

---

# 13. Interim database intake strategy

Fastest safe first deployment:

```text
Postgres cluster
│
├── realmforge_core DB/schema      Realmforge-owned
│
└── gate_upstream DB/schema        third-party interim
```

Prefer separate databases or at least separate schemas/users.

Do **not** make Realmforge Core query upstream tables directly for business logic.

Integration should pass through Gate semantic APIs/events. That gives us a migration seam when the upstream schema is retired.

---

# 14. Migration exit checklist

Before the upstream database can disappear:

- [ ] Core identity exists independently.
- [ ] Compatibility credential format is independently specified.
- [ ] game-account handle mapping is independently specified.
- [ ] OAuth clients required by real consumers are inventoried.
- [ ] service-ticket semantics are independently specified.
- [ ] BGS SSO/session restore is independently specified.
- [ ] session-key storage/lifetime is independently specified.
- [ ] entitlement behavior needed by real clients is independently specified.
- [ ] data migration from interim account IDs to Realmforge IDs is tested.
- [ ] no Core feature reads the upstream schema.
- [ ] no required backup/restore path depends on upstream migrations.
- [ ] independent Gate passes all supported client traces after the old schema is removed.
