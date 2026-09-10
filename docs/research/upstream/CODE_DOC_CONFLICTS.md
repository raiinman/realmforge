# Upstream Code / Documentation Conflict Ledger

**Upstream:** `wowemulation-dev/tavern`  
**Pinned revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Status:** ACTIVE

This ledger exists because upstream documentation is useful research material but is not internally consistent enough to serve as Realmforge implementation authority.

A conflict stays `CONFLICT` until Realmforge resolves it with independent evidence. Current upstream source can tell us what the pinned implementation does, but it does not prove what a real target client requires.

---

# C-001 — BGS v2 completion status

## Documentation A

Pinned `docs/plan.md` describes the broader milestone program as M0–M19 complete and documents later Classic client login paths and BGS behavior as implemented.

## Documentation B

Pinned `docs/bgs-protocol-versions.md` begins from an older-looking status statement that v1/1.13.2 is implemented while v2/1.14.0 and 2.5.1 is planned.

The same file later contains details that imply additional v2 work exists, so the file appears to have aged unevenly rather than representing one coherent point-in-time status.

## Pinned code

The pinned code contains:

- v2/client-v2 service hashes,
- protocol-version discrimination,
- SessionService routing,
- WebSocket BGS transport,
- v2 authentication-related routing.

## Realmforge disposition

`CONFLICT / P0`

We may use the pinned implementation as interim code, but Realmforge must not advertise any v2 client build as supported until a real client from that family successfully completes the independently recorded flow.

### Resolution evidence required

For each candidate v2-family build:

```text
connect
login
authentication completion
session creation
account/game-account state
realm list
realm join
realm-side authentication
reconnect/logout
```

---

# C-002 — GenerateAuthToken / method 5

## Documentation

Pinned `docs/plan.md` describes `GenerateAuthToken` as a stub.

## Pinned code

The BGS dispatcher has active routing for the method-5 token path and sends both v1/v2 token-generation requests into `handle_generate_sso_token`-family behavior.

## Problem

"A handler exists" is not equivalent to "the target client-required behavior is correct."

The documentation may simply be stale, or the handler may be a compatibility shortcut that satisfies synthetic tests without implementing the complete real behavior.

## Realmforge disposition

`CONFLICT / P0 / CAPTURE-REQUIRED`

### Resolve

Capture an actual consumer and record:

- protocol generation,
- service hash,
- method ID,
- request message,
- preconditions,
- response token bytes/string,
- prefix,
- claims/encoding if structured,
- expiration,
- consumer of returned token,
- replay behavior,
- invalid-session behavior.

---

# C-003 — ConnectionService RequestDisconnect

## Documentation

Pinned BGS protocol documentation describes `RequestDisconnect` (ConnectionService method 7) as not handled.

## Pinned code

The current BGS dispatcher explicitly matches method 7 and routes it through the same response handler used for `KeepAlive`.

Conceptually the pinned behavior is:

```text
KeepAlive (5)        -> keep-alive handler
RequestDisconnect(7) -> keep-alive handler
```

## Why this matters

This is not a harmless documentation typo. If a real client sends method 7 expecting a disconnect negotiation, replying as though it were a keepalive could be:

- an intentional compatibility workaround,
- sufficient for the tested client,
- an implementation bug,
- dead code no supported client invokes.

We do not know which.

## Realmforge disposition

`CONFLICT / P1 / CAPTURE-REQUIRED`

### Resolve

For each target family:

1. record whether the client ever sends method 7,
2. determine under what shutdown/logout state,
3. record expected server response,
4. record whether the client then closes the socket,
5. test no-response, empty-success, explicit disconnect and current-upstream behavior.

Do not reproduce the keepalive alias in an independent implementation unless evidence supports it.

---

# C-004 — SessionService completeness

## Documentation

Upstream milestone/status text describes SessionService in conflicting ways: some areas imply v2 session behavior is complete enough for the supported flow, while the plan still calls `MarkSessionAlive` stubbed/deferred.

## Pinned code

The method table exposes:

```text
CreateSession  = 1
RestoreSession = 2
DestroySession = 3
```

and the BGS server contains persistent BGS session support.

However, a queue/session structure still carries an SSO ID with an explicit TODO associated with RestoreSession dequeue handling.

`MarkSessionAlive` is not represented in the pinned `method_id.rs` session constants already inventoried by Realmforge.

## Realmforge disposition

`CONFLICT / P0`

### Resolve

Run a state-machine campaign rather than treating individual functions as proof:

```text
cold session
create
idle
keepalive
transport drop
fast restore
delayed restore
queued restore
duplicate login
destroy/logout
server restart
```

The acceptance target is client-visible session continuity, not parity with upstream function names.

---

# C-005 — AccountService "implemented" versus stub/retail leftovers

## Documentation/code comments

Pinned BGS code comments state that `GetAccountState` (30) and `GetGameAccountState` (31) are the methods observed in the pre-realm flow, while a number of other AccountService methods are described as stubs or retail leftovers.

At the same time, the source has constants/routing coverage for a wider AccountService surface including licenses, game-time, session info, CAIS, authorized data and signed state.

## Problem

A broad method table can be mistaken for a claim that all methods are materially implemented and required.

## Realmforge disposition

`UNKNOWN / P1`

### Resolve

Build a consumer matrix:

| Build | Method | Actually sent? | Required for play? | Response semantics verified? |
|---|---|---|---|---|
| TBD | 13 ResolveAccount | TBD | TBD | TBD |
| TBD | 25 Subscribe | TBD | TBD | TBD |
| TBD | 30 GetAccountState | TBD | likely | TBD |
| TBD | 31 GetGameAccountState | TBD | likely | TBD |
| TBD | 32 GetLicenses | TBD | TBD | TBD |
| TBD | 33 GetGameTimeRemainingInfo | TBD | TBD | TBD |
| TBD | 34 GetGameSessionInfo | TBD | TBD | TBD |
| TBD | 35 GetCAISInfo | TBD | TBD | TBD |
| TBD | 37 GetAuthorizedData | TBD | TBD | TBD |
| TBD | 44 GetSignedAccountState | TBD | TBD | TBD |

Implement only verified consumers in the future independent Gate unless a product requirement adds more.

---

# C-006 — "verified end-to-end" language versus simulated clients

## Upstream testing language

Upstream documentation uses phrases such as verified end-to-end for flows driven by its Python SRP/BGS clients and TrinityCore-oriented development scripts.

Its testing guide explicitly distinguishes simulated game-client interactions and uses `dev/srp_auth_client.py` plus load-test BGS clients for important flows.

## Problem

Those tests are valuable regression evidence for upstream, but a synthetic client built from the same assumptions as the server cannot independently prove that a real retired Blizzard client behaves the same way.

## Realmforge disposition

`EVIDENCE-GRADE CORRECTION`

Classify upstream synthetic tests as:

`Grade C/D implementation-correlated evidence`

not Realmforge Grade A real-client evidence.

They are excellent for preserving the interim baseline. They are insufficient for declaring Realmforge support.

---

# C-007 — OAuth implementation status versus external conformance validation

## Upstream implementation

The pinned OAuth service has substantial endpoints and an integration test suite.

## Upstream documentation

The OAuth implementation notes describe authorization code + PKCE as implemented while also discussing an external `oauth2c` validation matrix that was not fully completed at the point that document was written.

The later general testing guide contains `oauth2c` usage instructions and describes client-credentials validation.

## Realmforge disposition

`STATUS-AGE CONFLICT / LOW RISK`

Realmforge should not waste time reconstructing standards behavior from the upstream implementation. Rebuild generic OAuth/OIDC behavior from the applicable RFCs and run our own conformance/interoperability tests.

Only proprietary client expectations belong in compatibility reconstruction.

---

# C-008 — One hard-coded realm versus broad preservation-platform language

## Upstream product/docs

The project is described as a Battle.net replacement/preservation service capable of supporting retired client lines.

## Pinned GameUtilities implementation

The current realm flow is still a proof-of-concept shape with:

```text
one hard-coded realm address
one hard-coded realm name
fixed realm metadata
empty character counts
static/default client-version projection
single REALM_ADDRESS / REALM_PORT process config
```

## Realmforge disposition

`MATCH AS PROOF OF CONCEPT / REJECT AS PRODUCT ARCHITECTURE`

This is exactly why Realmforge must own a canonical dynamic realm registry and make Gate only a projection layer.

---

# C-009 — Security-looking account flows that are intentionally non-security stubs

## Upstream account surface

The account creation API exposes SMS/captcha/phone-looking steps.

## Pinned implementation

At least the SMS captcha gate is explicitly stubbed to accept the flow, and phone verification behavior is intentionally non-production fidelity.

## Risk

A future integrator could see those routes in the UI/API and assume the interim service provides anti-abuse or phone-verification security.

## Realmforge disposition

`REJECT AS SECURITY CONTROL`

For homelab, omit/disable unnecessary gates cleanly.
For any public mode, implement security controls independently and name their actual guarantees.

---

# Conflict-resolution rule

When a conflict closes:

1. link the independent capture/test evidence,
2. state the exact builds affected,
3. update the relevant protocol/client contract,
4. change the disposition from `CONFLICT` to `MATCH`, `BEAT`, `REJECT` or `DEFER`,
5. leave this historical entry in place with its resolution rather than deleting the fact that the upstream evidence once disagreed.

The point is not to prove upstream wrong. The point is to prevent Realmforge from accidentally inheriting stale assertions as architecture.
