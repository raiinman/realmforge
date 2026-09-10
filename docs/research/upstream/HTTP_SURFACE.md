# Upstream HTTP Surface — Pinned Intake Map

**Upstream:** `wowemulation-dev/tavern`  
**Revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Status:** code-level inventory, not Realmforge support authority

This file records the HTTP routes present in the pinned interim implementation so Realmforge can deliberately decide what to import, replace, capture, or discard.

## Account service routes

### Player-facing / login

| Method | Route | Purpose | Realmforge intake |
|---|---|---|---|
| GET | `/` | upstream landing page | DO NOT IMPORT UI |
| GET/POST | `/login/{locale}/` | browser login entry/email | KEEP NOW |
| POST | `/login/srp` | browser SRP challenge | KEEP NOW / CAPTURE |
| GET/POST | `/login/{locale}/password` | password page + SRP proof | KEEP NOW / CAPTURE |
| GET | `/login/` | 1.14 external web-login entry | KEEP NOW / CAPTURE |
| GET/POST | `/bnetserver/login/` | game-client bnet login | KEEP NOW / P0 |
| POST | `/client/login/external` | 1.13.2 in-client/external auth | KEEP NOW / P0 |
| POST | `/client/login/authenticator` | 1.13.2 authenticator path | KEEP NOW only if required; protocol capture needed |
| POST | `/client/login/tos/accept` | ToS/legal acceptance | KEEP NOW if 1.13 target requires it |
| GET | `/legal/agreement/{version}` | legal agreement payload/page | COMPATIBILITY-ONLY |
| POST | `/bnetserver/login/srp/` | game-client SRP challenge | KEEP NOW / P0 |
| POST | `/bnetserver/gameAccounts/` | game-account discovery | KEEP NOW / P0 |
| POST | `/bnetserver/refreshLoginTicket/` | game login-ticket refresh | KEEP NOW / P0 |
| GET | `/geoip` | region/datacenter projection | KEEP NOW / replace later |
| GET | `/login/sso` | cross-site SSO redirector | KEEP NOW / CAPTURE |
| GET | `/device` | device authorization UI | OPTIONAL |
| GET | `/logout` | SSR logout | REPLACE UI |
| GET | `/callback/oauth2/code/account-settings` | OAuth callback | KEEP only while upstream account UI used |

### Known missing routes described by upstream docs

These are especially important because they are not merely optional Realmforge ideas; upstream documentation says they were observed/needed but are not implemented at the pinned revision:

| Route | Status | Realmforge action |
|---|---|---|
| `GET /login/ticket-login` | documented observed, missing | **P0 CAPTURE-REQUIRED** |
| `GET /login/sso/generate` | documented missing generator | **P0 CAPTURE-REQUIRED** |

The implementation must not infer these contracts from their names alone.

## Account management API

### Identity/details

| Method | Route | Intake |
|---|---|---|
| GET | `/api/user` | compatibility consumer audit |
| GET | `/api/details` | compatibility consumer audit |
| GET/PUT | `/api/details/address` | product/UI-only unless consumer proven |
| GET | `/api/details/battletag/rules` | product/UI-only unless consumer proven |
| GET | `/api/security` | product/UI-only unless consumer proven |
| POST | `/api/security/password` | REWRITE for Realmforge product identity |

### Privacy/preferences

| Method | Route | Intake |
|---|---|---|
| GET | `/api/privacy` | DEFER unless consumer proven |
| GET | `/api/privacy/profile` | DEFER |
| GET/PUT | `/api/privacy/data` | DEFER |
| GET/PUT | `/api/privacy/social` | DEFER |
| GET | `/api/communication-preferences` | DEFER |
| GET | `/api/communication-preferences/supported-locales` | DEFER/product-owned |

### Email/account support

| Method | Route | Intake |
|---|---|---|
| POST | `/api/email/confirm` | REWRITE product-owned |
| POST | `/api/email/verification` | REWRITE product-owned |
| GET | `/api/rum` | likely DO NOT IMPORT without consumer |
| GET | `/api/passkeys` | upstream surface only; implement WebAuthn independently if wanted |
| GET | `/api/approvals` | DEFER |
| GET | `/api/account-connections` | DEFER |

### Overview/game/account catalog

| Method | Route | Intake |
|---|---|---|
| GET | `/api/overview` | upstream UI support; not canonical Realmforge API |
| GET | `/api/games-and-subs` | inspect consumer; likely compatibility projection only |
| GET | `/api/classic-games` | inspect consumer |
| GET | `/api/game-account/creation/rules` | inspect consumer |
| GET | `/api/time-gated-games` | likely DEFER |
| GET | `/api/env` | upstream UI bootstrap; DO NOT make Realmforge canonical |

### Location/age/parental

| Method | Route | Intake |
|---|---|---|
| GET | `/api/location/url` | DEFER |
| GET | `/api/location/country-list` | DEFER |
| GET | `/api/location/country-address-metadata` | DEFER |
| GET | `/api/location/country-age-of-adulthood-map` | DEFER |
| GET | `/api/age-verification` | DEFER |
| GET | `/api/parental-controls` | DEFER |

### Commerce stubs

| Method | Route | Upstream status | Realmforge |
|---|---|---|---|
| GET | `/api/transactions` | stub | DO NOT IMPORT unless product feature exists |
| GET | `/api/wallet` | stub | DO NOT IMPORT |
| GET | `/api/external-subs` | stub | DO NOT IMPORT |
| GET | `/api/vc/lastUsed` | stub | DO NOT IMPORT |
| GET | `/api/vc/ecosystem` | stub | DO NOT IMPORT |

### Session / account UI bootstrap

| Method | Route | Intake |
|---|---|---|
| POST | `/api/` | upstream account-SPA bootstrap | compatibility/UI-only |
| POST | `/api/logout` | upstream session logout | keep semantics, replace product API |
| GET | `/overview` | SSR dashboard | DO NOT IMPORT branding/UI long term |

### Account-creation flow

| Method | Route | Intake |
|---|---|---|
| GET | `/creation/flow/creation-full` | optional interim account UI |
| GET | `/creation/api/init` | optional interim UI |
| GET | `/creation/flow/creation-full/back` | optional interim UI |
| POST | `/creation/flow/creation-full/step/{step}` | optional interim UI; inspect security stubs |
| GET | `/creation/api/battletag-suggestion` | optional |

Important: pinned creation code has intentionally weak/fake phone/captcha handling. Those must never be mistaken for production security controls.

### Static/branding routes

```text
GET /static/style.css
GET /static/spa.js
GET /static/tavern-logo-light.png
GET /static/tavern-logo-dark.png
GET /favicon.svg
```

**Realmforge disposition:** `DO NOT IMPORT AS PRODUCT UI`.

If the entire upstream service is vendored verbatim during the interim phase, these may exist inside the third-party snapshot for source integrity, but Realmforge should not route users through Tavern branding as its final UI.

---

# OAuth/OIDC service

Pinned router exposes:

| Method | Route | Function | Realmforge plan |
|---|---|---|---|
| GET | `/.well-known/openid-configuration` | OIDC discovery | REWRITE FROM STANDARD |
| GET | `/jwks/certs` | JWKS | REWRITE FROM STANDARD |
| GET | `/authorize` | authorization endpoint | REWRITE standards core; capture compatibility quirks |
| POST | `/token` | token endpoint | REWRITE standards core; preserve proprietary exchange behavior only as needed |
| POST | `/sso` | SSO token flow | CAPTURE / compatibility shim |
| GET | `/userinfo` | OIDC userinfo | REWRITE FROM STANDARD |
| POST | `/revoke` | RFC 7009 | REWRITE FROM STANDARD |
| POST | `/v2/check_token` | token introspection | REWRITE standards core + verify client-specific shape |
| POST | `/device/code` | device authorization | REWRITE FROM STANDARD if desired |
| POST | `/device/approve` | device approval | REWRITE/product-owned |
| GET | `/logout` | OIDC/logout surface | REWRITE + compatibility test |

Upstream source states intended conformance with OAuth/OIDC-related RFCs including 6749, 7009, 7662, 7636, 8414, 8693 and 9068. Realmforge must validate standards behavior independently; upstream implementation is not the standards authority.

Proprietary/compatibility-sensitive areas include:

- client IDs expected by retired desktop/game software,
- desktop-app RFC 8693 requested token type,
- subject-token semantics,
- platform/SSO token claims,
- account-server redirects,
- cookie-assisted silent auth,
- Battle.net-specific endpoint naming and token expectations.

---

# Health/operations routes

Upstream observability router exposes:

```text
GET /health
GET /ready
GET /startup
```

Realmforge should retain the operational concepts but define first-party health semantics.

---

# BGS HTTP/WebSocket entry

The BGS server's WebSocket HTTP router exposes:

```text
GET /    -> WebSocket upgrade handler
```

The raw TCP BGS listener is not an HTTP route and is inventoried in `BGS_SURFACE.md` and `CONFIG_SURFACE.md`.

---

# Intake rule

No route enters Realmforge merely because the upstream implementation has it.

Every route must be assigned one of:

- `KEEP NOW` — required to make the interim Gate function.
- `CAPTURE FIRST` — likely compatibility-critical but externally under-specified.
- `REWRITE` — generic/product/standards behavior better owned by Realmforge.
- `DEFER` — valid but not on the client-to-playable-realm critical path.
- `DO NOT IMPORT` — branding/stub/fluff with no current consumer.

The product-critical path wins over account-site fidelity.
