# Protocol Overview

Realmforge reimplements the Battle.net account and authentication surfaces a
retired WoW Classic client needs to log in. This page is the high-level map;
details live in the linked documents.

## Three authentication paths

Every client converges on the same shared account/ticket backend, but reaches
it over a different transport depending on what it is:

| Consumer | Transport | Credential proof |
| --- | --- | --- |
| Game client 1.13.x | BGS v1 raw TCP (port 1119) + `/client/login/external`, then BGS RPC `Logon` → `VerifyWebCredentials` | plaintext |
| Game client 1.14.x | `/client/login/external` (BGS v1) or HTTP `POST /bnetserver/login/` with a `LoginForm` envelope | plaintext |
| Game client 2.5.x TBC, 3.4.x WotLK, 4.4.x Cata | HTTP `POST /bnetserver/login/` with a `LoginForm` envelope | BnetSRP6v2 |
| Human in a browser | `/login/{locale}/` web flow | BnetSRP6v2 (proof computed client-side) |
| Battle.net desktop app (Phoenix) | BGS RPC, then `POST /token` RFC 8693 token-exchange | A BGS session JWT |

The in-game and browser paths both end in an opaque **service ticket**
(`ST=<region>-<hex>-<accountId>`) that the OAuth `/authorize` endpoint consumes
to mint an authorization code. The desktop app exchanges its BGS JWT for a
DPLT platform token.

## Where the details live

- [`docs/srp.md`](srp.md) — the BnetSRP6v2 parameters and byte-order
  conventions (2048-bit modulus, generator 2, PBKDF2-HMAC-SHA-512 KDF,
  SHA-256 session hashes).
- [`docs/oauth-oidc-implementation.md`](oauth-oidc-implementation.md) — the
  OAuth/OIDC endpoints, grant types, token formats, and JWT claims.
- [`docs/architecture.md`](architecture.md) — service topology, data model,
  request flow, and the game-client authentication boundary.

## Out of scope

- The legacy 256-bit realm SRP6 (retail 2.4.3/3.3.5a) — Realmforge uses the
  modern 2048-bit BnetSRP6v2, not the legacy variant.
- The world/gameplay server — realmforge completes the pre-realm-join handoff
  (`Command_RealmJoinRequest_v1` → `Param_ServerAddresses` +
  `Param_JoinSecret` + `Param_RealmJoinTicket` + `Param_BnetSessionKey`,
  world address from `REALM_ADDRESS`/`REALM_PORT`) and stops there; the
  world server is an external integration.
- Store, payments, wallet, and social surfaces — stubbed or omitted. See the
  Commerce surface section of [`docs/architecture.md`](architecture.md).
