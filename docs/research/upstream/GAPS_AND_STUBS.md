# Upstream Gaps / Stubs / No-Op Ledger

**Upstream:** `wowemulation-dev/tavern`  
**Pinned revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Status:** ACTIVE

This file is the "do not forget this later" ledger.

Every item below is either visibly incomplete in the pinned implementation/documentation, deliberately a no-op, outside upstream scope but required by Realmforge, or insufficiently verified for Realmforge to treat as closed.

The purpose is not to criticize the upstream project. Tavern has a narrower preservation scope than Realmforge. We care about the holes because our product has to cross them.

---

# Priority meanings

- **P0** — blocks a trustworthy client -> realm -> gameplay path or future clean replacement.
- **P1** — important for robust/polished operation but not necessarily first login.
- **P2** — optional/product-dependent.
- **P3** — explicitly deferred unless the product expands into that area.

Research status:

- `CODE-GAP` — pinned implementation is visibly incomplete.
- `NO-OP` — implementation intentionally returns success/empty/default behavior.
- `OUT-OF-SCOPE` — upstream intentionally does not implement it.
- `CAPTURE-REQUIRED` — behavior must be learned independently.
- `CONFLICT` — docs/code disagree.
- `PRODUCT-GAP` — Realmforge needs something larger than upstream scope.

---

# P0 — critical compatibility / reconstruction gaps

## GAP-P0-001 — exact supported-build truth

**Status:** `CONFLICT + CAPTURE-REQUIRED`

Upstream contains version-specific code and broader status claims, but its BGS-version documentation is stale/inconsistent.

### Missing

A first-party Realmforge matrix for exact retired builds:

```text
build
client family
transport
TLS
login flow
BGS protocol generation
required service map
realm-list format
realm-join format
realm-side auth
verified result
```

### Completion

At least one representative real client per family passes end-to-end, then exact builds are expanded individually.

---

## GAP-P0-002 — BGS v2 truth

**Status:** `CONFLICT + CAPTURE-REQUIRED`

Current source has v2/client-v2 routing; one upstream doc still says v2 planned.

### Missing

Real-client proof that later-family transport, auth, session and realm flow are correct.

---

## GAP-P0-003 — RestoreSession queue/dequeue path

**Status:** `CODE-GAP + CAPTURE-REQUIRED`

Pinned BGS queue state contains an explicit TODO for completing SSO-token use when a queued session is dequeued.

### Missing

- exact restore message,
- SSO/session token semantics,
- queued restore behavior,
- re-auth versus restore decision,
- timeout,
- post-restore session key continuity.

### Acceptance

A real client survives forced transport interruption and the tested restore paths without requiring fresh credentials where the client expects resume.

---

## GAP-P0-004 — MarkSessionAlive / session keepalive

**Status:** `CODE/DOC GAP + CAPTURE-REQUIRED`

Upstream plan calls `MarkSessionAlive` stubbed/deferred.

### Missing

- method identity per generation,
- whether real clients invoke it,
- cadence,
- payload,
- response,
- timeout consequences,
- relation to connection-level KeepAlive.

Do not confuse ConnectionService KeepAlive with SessionService keepalive.

---

## GAP-P0-005 — GenerateAuthToken semantics

**Status:** `CONFLICT + CAPTURE-REQUIRED`

Docs call it a stub; current dispatcher routes token-generation behavior.

### Missing

- exact consumer,
- token format,
- claims/opaque bytes,
- expiry,
- prefix,
- issuer/signing semantics,
- replay behavior,
- relation to OAuth access token and BGS SSO token.

---

## GAP-P0-006 — browser `/login/ticket-login`

**Status:** `CODE-GAP + CAPTURE-REQUIRED`

Upstream architecture docs say the route was observed but not implemented.

### Missing

Full independent contract:

```text
request method/path/query
producer of ticket
ticket encoding
expiry
one-time behavior
session binding
redirect target validation
cookies/session created
failure responses
```

---

## GAP-P0-007 — browser `/login/sso/generate`

**Status:** `CODE-GAP + CAPTURE-REQUIRED`

Upstream documents `/login/sso` redirect behavior but says the generator route is missing.

### Missing

- caller,
- auth requirements,
- generated value,
- cryptographic properties,
- lifetime,
- intended consumer,
- relation to BGS/OAuth tickets.

---

## GAP-P0-008 — realm-side authentication

**Status:** `OUT-OF-SCOPE upstream / PRODUCT-GAP Realmforge`

Upstream stops after returning the pre-world realm-join parameters.

Realmforge has to understand and integrate the other half:

```text
realm address
join secret
realm ticket
BGS session key
        ↓
world socket
        ↓
realm auth challenge/proof
        ↓
account/game-account resolution
        ↓
world entry
```

### Missing

- exact realm opcode sequence per target family,
- digest/proof algorithm,
- which join fields are actually consumed,
- how emulator core receives/validates those fields,
- reconnect path,
- invalid-ticket error behavior,
- build mismatch behavior.

This is a hard blocker for calling Realmforge end-to-end playable.

---

## GAP-P0-009 — realm join ticket security contract

**Status:** `NO-OP/shortcut + CAPTURE-REQUIRED`

Pinned proof-of-concept sends account-ID text as the opaque realm-join ticket and a random 32-byte join secret.

That may be sufficient for the currently paired emulator/testing path, but it is not a security specification.

### Missing

- ticket entropy requirement,
- one-time use,
- expiry,
- binding to account/game account/realm/session/build,
- persistence needed by realm server,
- replay behavior,
- invalidation.

---

## GAP-P0-010 — SRP independent specification

**Status:** `CAPTURE-REQUIRED`

The upstream implementation is valuable and can be used now, but it cannot serve as its own clean-room replacement spec.

### Missing per client family

- groups,
- byte order,
- PBKDF/KDF,
- iteration count behavior,
- account normalization,
- challenge/proof encoding,
- M1/M2 formula,
- malformed-public-value handling,
- deterministic independent vectors.

---

## GAP-P0-011 — service/method consumer matrix

**Status:** `CAPTURE-REQUIRED`

The source exposes many BGS services/methods that may be:

- required,
- optional,
- retail leftovers,
- synthetic-test-only,
- dead for a given client family.

### Missing

For every exact target build:

```text
service hash
method ID
direction
observed?
preconditions
required for ordinary play?
required response?
failure behavior
```

Without this matrix we risk rebuilding unused Battle.net surface for months.

---

## GAP-P0-012 — exact negative/error semantics

**Status:** `CAPTURE-REQUIRED`

Upstream has result-code constants, but Realmforge must know what real clients do with them.

Test:

- duplicate login,
- suspended account,
- banned game account,
- no game time,
- session disconnect,
- maintenance,
- shutdown,
- malformed RPC,
- expired ticket,
- realm unavailable,
- wrong build.

---

## GAP-P0-013 — dynamic realm registry

**Status:** `PRODUCT-GAP`

Pinned GameUtilities assumes one hard-coded realm.

Realmforge requires:

```text
many realms
per-realm build compatibility
health/offline state
population
maintenance
external/internal addresses
emulator adapter ownership
character counts where available
join routing
```

The interim Gate must be modified behind an explicit Realmforge realm-provider boundary.

---

# P1 — robustness / polish gaps

## GAP-P1-001 — LastCharPlayedRequest

**Status:** `NO-OP + CAPTURE-REQUIRED`

Pinned implementation returns empty success.

Determine:

- whether supported clients call it,
- whether empty is harmless,
- response shape needed to preselect last realm/character,
- whether emulator adapter can supply it cheaply.

---

## GAP-P1-002 — CharacterListRequest

**Status:** `NO-OP + CAPTURE-REQUIRED`

Pinned implementation returns empty success.

Determine whether it is truly optional or should return real character summary data.

---

## GAP-P1-003 — GetPlayerVariables

**Status:** `NO-OP + UNVERIFIED CONSUMER`

Pinned BGS docs call it an empty/no-op stub and say invocation during 1.13.2 login was not verified.

Do not implement a rich response unless a real consumer is observed.

---

## GAP-P1-004 — GetAchievementsFile

**Status:** `NO-OP + UNVERIFIED CONSUMER`

Same rule as above.

---

## GAP-P1-005 — AccountService non-core methods

**Status:** `STUB/RETAIL-LEFTOVER AMBIGUITY`

Pinned BGS comments say GetAccountState/GetGameAccountState are observed in pre-realm flow and describe other methods as stubs/retail leftovers.

Need exact consumer matrix before future reimplementation.

---

## GAP-P1-006 — RequestDisconnect semantics

**Status:** `CONFLICT`

Docs say unhandled; current code routes method 7 to keepalive handler.

Must be resolved from client behavior before independent implementation.

---

## GAP-P1-007 — duplicate-session policy

**Status:** `CAPTURE-REQUIRED`

Upstream has duplicate-session bookkeeping and result codes.

Realmforge must decide/verify:

- kick old or reject new,
- per human account or game account,
- effect on world session,
- reconnect grace,
- queued sessions,
- public versus homelab policy.

---

## GAP-P1-008 — login queue correctness

**Status:** `IMPLEMENTED INTERIM / REAL-CLIENT VERIFICATION MISSING`

Upstream has queue tests and queue notifications.

Need real-client proof for:

- queue position UX,
- position updates,
- queue end,
- disconnect while queued,
- restore while queued,
- multiple accounts,
- overload behavior.

---

## GAP-P1-009 — multi-game-account behavior

**Status:** `CAPTURE-REQUIRED`

Need to verify:

- automatic selection,
- SelectGameAccount behavior where present,
- handle encoding,
- account state projection,
- per-game-account licenses/status,
- realm join mapping.

---

## GAP-P1-010 — character counts

**Status:** `NO-OP/EMPTY`

Pinned realm-list payload emits empty counts.

Realmforge should expose an optional emulator-adapter capability for character counts rather than making Gate query emulator DB schemas directly.

---

## GAP-P1-011 — realm metadata semantics

**Status:** `HARDCODED / CAPTURE-REQUIRED`

Pinned realm list fixes site/category/config/language/population/version metadata.

Need to determine which fields affect:

- client display,
- realm filtering,
- language,
- PvP/PvE labels,
- build compatibility,
- offline/locked/full state.

---

## GAP-P1-012 — IPv4/IPv6/internal/external realm addresses

**Status:** `PRODUCT-GAP`

Pinned join response is one configured IP/port.

Realmforge homelab needs to cope with:

- LAN clients,
- VPN clients,
- public WAN clients,
- NAT,
- multiple realm hosts,
- optional IPv6.

Join-address selection belongs in Realmforge networking policy.

---

## GAP-P1-013 — signing-key startup policy

**Status:** `SECURITY GAP`

Pinned account startup can continue with an empty signing-key string when the configured key file cannot be read, logging a warning.

Realmforge should treat missing required signing material as a hard startup failure in any profile that actually needs signed tokens/tickets.

Homelab convenience is not a reason to silently start with broken cryptographic state.

---

## GAP-P1-014 — OAuth external conformance matrix

**Status:** `PARTIAL / REWRITE-EARLY`

Upstream has integration tests and later docs show `oauth2c` usage, but generic OAuth/OIDC should be tested against standards independently once Realmforge rewrites it.

Keep a split test suite:

```text
standards conformance
proprietary client compatibility
```

---

## GAP-P1-015 — credential migration between protocol generations

**Status:** `CODE/DOC GAP`

Upstream documentation mentions v1→v2 verifier upgrade work as incomplete/stubbed because of browser-side implementation needs.

Realmforge must determine whether one Realmforge password/account needs to support all target verifier forms simultaneously or can generate protocol verifiers during enrollment/password change.

Never require storing raw passwords to solve this.

---

# P2 — optional compatibility/product gaps

## GAP-P2-001 — OnExternalChallenge / 2FA compatibility

**Status:** `STUB/DEFERRED`

Only promote if:

- a target client requires it for normal use, or
- public-hosting product policy deliberately supports compatible challenge UX.

Realmforge's own admin MFA should use modern independent mechanisms regardless.

---

## GAP-P2-002 — passkeys

**Status:** `API SURFACE WITHOUT CORE IMPLEMENTATION`

Use WebAuthn standards for Realmforge product passkeys. Do not reconstruct generic passkey behavior from Tavern.

---

## GAP-P2-003 — phone verification

**Status:** `FAKE/STUBBED upstream`

Do not use as security.

---

## GAP-P2-004 — captcha / SMS captcha gate

**Status:** `FAKE/STUBBED upstream`

Do not use as security.

---

## GAP-P2-005 — account connections

**Status:** `ACCOUNT-SITE FIDELITY / DEFER`

Only build if Realmforge actually adds external-account linking.

---

## GAP-P2-006 — communication/privacy account-site fidelity

**Status:** `DEFER`

Realmforge should build its own privacy/settings model for its own features rather than clone a retired account website for no consumer.

---

## GAP-P2-007 — friends/presence/social

**Status:** `OUT-OF-SCOPE upstream / PRODUCT-OPTION`

Known BGS descriptors exist, but services are not materially implemented in the pinned server.

If Realmforge adds friends/presence later, prefer a Realmforge-native social model. Add proprietary BGS compatibility only where a supported client consumes it and the feature meaningfully improves play.

---

## GAP-P2-008 — desktop application compatibility

**Status:** `PRODUCT DECISION OPEN`

There is substantial OAuth/token-exchange work, but Realmforge has not yet locked whether the retired Battle.net desktop application itself is a launch requirement.

If Realmforge Client replaces the launcher experience, we may not need full desktop-app fidelity.

Do not spend P0 time on it until P-006 closes.

---

# P3 — intentionally deferred commercial/account-site scope

## GAP-P3-001 — transactions

**Status:** `STUB / DEFER`

## GAP-P3-002 — wallet

**Status:** `STUB / DEFER`

## GAP-P3-003 — virtual currency

**Status:** `STUB / DEFER`

## GAP-P3-004 — storefront / payments / refunds

**Status:** `OUT-OF-SCOPE / DEFER`

## GAP-P3-005 — subscriptions as commerce

**Status:** `DEFER`

Private-realm membership/permissions may exist independently. That does not require cloning Battle.net commerce.

---

# Upstream synthetic-test gaps

The pinned testing suite is useful but has an unavoidable blind spot: many key flows are exercised by upstream-authored Python/synthetic clients.

These tools include:

```text
dev/srp_auth_client.py
dev/srp_oracle.py
load-test/bgs-client.py
load-test/bgs-queue-test.py
load-test/login-harness.py
```

They are valuable for:

- regression,
- deterministic load generation,
- queue stress,
- malformed-request testing,
- ensuring our interim snapshot still behaves like the pinned upstream.

They do **not** independently prove:

- real client field ordering,
- exact real-client method usage,
- TLS behavior,
- retry timing,
- hidden optional calls,
- UI/error response,
- reconnect behavior,
- exact build compatibility.

Realmforge therefore requires a separate first-party evidence corpus driven by actual supported clients.

---

# Gap closure rule

A gap closes only when its authority moves somewhere permanent.

Example:

```text
GAP-P0-004 MarkSessionAlive
        ↓
Realmforge capture RF-CAP-0031
        ↓
client contract 3.4/SESSION.md
        ↓
black-box test RF-BGS-SESSION-014
        ↓
status MATCH/BEAT/REJECT
```

Do not simply delete the gap line after somebody implements something.

If the external behavior remains unverified, implementation is not closure.
