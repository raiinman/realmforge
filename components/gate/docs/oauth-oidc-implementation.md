# OAuth / OIDC Implementation

The OAuth/OIDC provider (`realmforge-gate-oauth`) is a hand-written implementation
over the `jsonwebtoken` crate, verified against real Battle.net captures.
An `oxide-auth` spike (2026-06-23) confirmed hand-written is the lower-risk
path; the desktop-app token-exchange grant is non-standard and not covered
by any library.

## Endpoints

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/.well-known/openid-configuration` | GET | OIDC Discovery |
| `/jwks/certs` | GET | JWKS (RS256 public keys) |
| `/authorize` | GET | Authorization (login ticket → code) |
| `/token` | POST | Token issuance (all grant types) |
| `/sso` | POST | `client_sso` grant (Phoenix desktop app SSO) |
| `/device/code` | POST | Device authorization (RFC 8628) |
| `/device/approve` | POST | Device approval (user enters user_code) |
| `/userinfo` | GET | OIDC userinfo claims |
| `/v2/check_token` | POST | Token introspection (RFC 7662) |
| `/revoke` | POST | Token revocation (RFC 7009) |
| `/health` | GET | Liveness check |
| `/ready` | GET | Readiness check (DB pool) |

## Grant Types

### `authorization_code` (RFC 6749 + PKCE RFC 7636)

The `/authorize` endpoint consumes a one-time service ticket (`ST=<region>-<hex>-<id>`)
issued by the account server after successful SRP login. Two-hop redirect:

1. `GET /authorize?...&ST=...` → **302** to `/authorize?...` (ST stripped, SESSIONID set)
2. `GET /authorize?...` → **303** to `<redirect_uri>?code=<REGION>STP<34chars>&state=<state>`

Code format: `<REGION>STP<34-uppercase-alphanumeric>` (e.g., `KRSTPFJSTPJKQ6VFFEGAONXJGN94KKM8FA`).

The `/token` endpoint validates the code, code challenge (S256), and redirect URI.

### `client_credentials` (RFC 6749)

M2M token for confidential clients. Requires Basic auth header.
Returns a JWT access token with client-level claims. No refresh token.

Authenticated clients: Phoenix desktop app (`a7f9b73e4e9c4e01a9c8056cddabff71`)
with hardcoded secret `2Lu0QbF5FLrQoLmIDjcPjojlEu4zirF2` (SHA-256 hashed in DB).

### `refresh_token` (RFC 6749)

Exchanges an opaque refresh token for a new access token.
Tokens are rotated on use (the old one is revoked).

### `urn:ietf:params:oauth:grant-type:token-exchange` (RFC 8693)

Desktop app M2M token exchange. Accepts `subject_token_type=
urn:ietf:params:oauth:token-type:jwt`. Exchanges a BGS session JWT for a
platform token. The subject token is validated (signature, expiry) and the
resulting token shares the same client identity. The `requested_token_type`
is validated against the spec; only
`urn:blizzard:params:oauth:token-type:dplt` is accepted.

### `client_sso` (Phoenix desktop app)

Exchanges a BGS auth token for an OAuth token without browser interaction.
`POST /sso?client_id=...&scope=...&token=...&grant_type=client_sso&token_type=jwt`.
The `token` parameter is a BNET-prefixed auth token from the BGS RPC session.

### `device_code` (RFC 8628)

Device authorization grant. `POST /device/code` creates a pending device
authorization; the user visits the verification URI, enters the `user_code`,
logs in, and approves at `POST /device/approve`; the device polls `/token`
with `grant_type=device_code` until authorization completes. Pending
authorizations are stored in the `device_authorizations` table.

## Tokens

### Access tokens

RS256 JWTs signed with the active keypair. Stateless — no per-token database row.
Lifetime: 86399 seconds (~24 hours), matching the capture.

### Refresh tokens

Opaque strings stored in the `refresh_tokens` table. Rotated on each use.
Lifetime: 30 days.

### ID tokens

Issued alongside access tokens in the `authorization_code` flow.
Same RS256 JWT format as access tokens, with `aud` and `azp` claims set
to the client ID. These claims are absent from access tokens per the capture.

## JWT Claims

The `Claims` struct (`realmforge-gate-core/src/jwt.rs`) has 31 fields covering every
claim observed in real Battle.net JWT captures (7 always present, 24
optional and skipped when empty).

### Always present

`sub`, `iss`, `iat`, `exp`, `jti`, `scope`, `client_id`

### Client-level (M2M tokens)

`client_roles`, `account_roles`, `authorities`, `programs`, `env`, `active`,
`account_authorities`, `client_authorities`

### User-level (user-authorized tokens)

`battle_tag`, `country_code`, `account_identifier`, `first_name`, `last_name`,
`birth_date`, `mobile_number`, `country_id`, `verified_email_address_flag`,
`employee_flag`, `account_id`, `username`

### ID-token-only

`aud`, `azp` — present only in ID tokens, absent from access tokens per the capture.

### Optional

`nonce`, `at_hash`

## Signing Keys

RSA 2048-bit keypairs in PKCS#8 PEM format. The server reads the active key
from `keys/signing.pem` (configurable via `SIGNING_KEY_PATH`).

The demo key at `keys/signing.pem` is committed and safe for development.
It is the same key used in the test suite. The `keys/signing.pem.meta`
sidecar contains project metadata; the server logs a warning at startup
if the environment is `demo`/`test`/`development`/`dev`.

Production deployment: generate a new key and write a corresponding `.meta`
file with `environment = "production"`.

## Error Format

All errors follow RFC 6749 §5.2:

```json
{"error": "...", "error_description": "..."}
```

HTTP status codes match the capture:

- `401` for invalid client credentials, invalid grant, invalid token
- `403` for insufficient scope (e.g., M2M token at `/userinfo`)
- `400` for malformed requests

## Validation

The OAuth provider is validated against oauth2c for real-client interop:

- [x] `client_credentials` with Basic auth — passes
- [x] `urn:ietf:params:oauth:grant-type:token-exchange` with Basic auth — passes
- [x] `client_credentials` without auth — correctly rejected (401)
- [ ] `authorization_code` + PKCE — implemented (M14/M15); not yet validated
      against `oauth2c` (no offline test harness)

All JWT claims have been verified against real Battle.net captures:
13 claims match for client_credentials tokens, all optional claims
(`account_authorities`, `client_authorities`, `account_id`, `username`)
are present with correct values.
