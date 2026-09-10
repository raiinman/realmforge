# Realmforge Compatibility Research Campaign

**Status:** OPEN  
**Purpose:** Build a first-party evidence base strong enough to implement and maintain Realmforge compatibility services without relying on inherited source code.

## Campaign rule

Research is only complete when it produces implementation-grade authority.

A useful finding must answer:

- what client/build it applies to,
- what exact behavior was observed,
- how it was observed,
- what request/state caused it,
- what response/state followed,
- what remains unknown,
- what test can reproduce it,
- what Realmforge contract it affects.

## Evidence grades

### A — reproduced first-party evidence

Examples:
- Realmforge-generated packet capture,
- Realmforge-controlled black-box test,
- repeatable target-client observation,
- deterministic test vector derived independently.

### B — standards/primary technical authority

Examples:
- RFC,
- official protocol/schema publication,
- directly inspected target-client metadata where lawful.

### C — independently correlated implementation/research

Useful for hypotheses, not final authority when target behavior can be tested directly.

### D — upstream documentation

Useful as a lead and inventory source. Must not override contradictory real-client behavior.

### E — community report / unverified claim

Lead only.

## Status vocabulary

- `MATCH` — evidence confirms the current Realmforge assumption.
- `BEAT` — evidence supports a stronger/better Realmforge contract.
- `REJECT` — evidence disproves the assumption.
- `DEFER` — valid but not needed for current milestone.
- `UNKNOWN` — insufficient evidence.
- `CONFLICT` — credible sources disagree; no authority yet.
- `CAPTURE-REQUIRED` — likely behavior is known but first-party evidence is missing.

---

# Target board

## T0 — Upstream capability inventory

Goal: inventory every reusable interim compatibility capability and every visible gap.

Capture:
- service/crate boundaries,
- public interfaces,
- database ownership,
- build target claims,
- deployment requirements,
- protocol generations,
- incomplete/stubbed services,
- known deferred work,
- external dependencies.

Output:
`docs/research/UPSTREAM_CAPABILITY_INVENTORY.md`

No production import should happen before provenance is pinned to an exact revision.

---

## T1 — Client/build truth matrix

Goal: authoritative map of retired target builds.

For each candidate build:

```text
client family
exact build number
file/product version
OS tested
initial endpoint
BGS transport generation
TLS behavior
login mechanism
realm-list mechanism
realm-join mechanism
known emulator counterpart
status
```

Required result:
- resolve v1/v2 documentation conflicts,
- stop treating expansion names as protocol versions.

---

## T2 — Successful login traces

One representative build per family first, then broaden.

Capture:

```text
process start
DNS
TCP/TLS/WebSocket establishment
browser/HTTP login
SRP challenge/proof if present
OAuth/token exchange
BGS authentication
session creation
account/game-account projection
realm list
realm join
realm connection
```

Every capture gets a manifest and cryptographic hash.

---

## T3 — SRP reconstruction

For every SRP-using family determine:

- modulus,
- generator,
- hash,
- salt size,
- iteration/KDF,
- account normalization,
- integer endian rules,
- public-value encoding,
- proof formula,
- server proof,
- invalid-proof behavior.

Produce independent vectors.

---

## T4 — BGS framing and dispatch

Determine per generation:

- transport,
- connection preamble,
- framing,
- header schema,
- body schema,
- correlation token,
- service identifier/hash,
- method IDs,
- listener/push direction,
- error/status encoding,
- connection close behavior.

Produce a versioned service map.

---

## T5 — Session lifecycle

Scenarios:

- fresh login,
- idle,
- keepalive,
- graceful logout,
- dropped socket,
- fast reconnect,
- delayed reconnect,
- server restart,
- duplicate login,
- account switch.

Must resolve:

- `MarkSessionAlive`,
- resume/restore behavior,
- duplicate-session policy,
- timeout thresholds,
- session token reuse.

---

## T6 — Auth token/ticket lifecycle

Must independently identify:

- login/service ticket shape,
- generation point,
- consuming service,
- expiry,
- replay behavior,
- binding to identity/session/device,
- `GenerateAuthToken` semantics,
- OAuth relationship,
- invalid/expired behavior.

---

## T7 — Realm discovery

For each client generation:

- request shape,
- response shape,
- realm ID format,
- build filtering,
- status fields,
- population fields,
- character count,
- maintenance/full/offline behavior,
- refresh/subscription behavior.

Outcome must map cleanly from Realmforge's canonical `Realm`.

---

## T8 — Realm join and realm-side authentication

Trace end to end:

```text
BGS join request
  ↓
join response
  ↓
address/ticket/secret/session material
  ↓
realm socket
  ↓
realm auth opcode
  ↓
proof/digest
  ↓
account resolution
  ↓
world entry
```

This target is not complete if we only understand the BGS half.

---

## T9 — Negative path corpus

At minimum:

- unknown account,
- wrong password,
- malformed SRP public value,
- wrong proof,
- stale challenge,
- expired ticket,
- replayed ticket,
- invalid OAuth token,
- expired OAuth token,
- wrong client build,
- incompatible realm,
- realm offline,
- realm full,
- BGS offline,
- database outage,
- malformed RPC,
- duplicate session.

Record exact observable client behavior, not just server response.

---

## T10 — Desktop application compatibility

Only if retained as a product goal.

Cold state:
- no cookies,
- no cached token,
- no active session.

Warm state:
- valid cache,
- expired access token,
- expired BGS token,
- account switch,
- logout.

Map:
- token hierarchy,
- SSO path,
- regional routing,
- retry order,
- browser fallback.

---

# Capture discipline

Every capture must have a sidecar record:

```yaml
capture_id:
date:
researcher:
client_family:
client_build:
client_hashes:
os:
network_topology:
server_under_test:
server_revision:
scenario:
expected_result:
actual_result:
packet_capture_hash:
tls_keylog_present:
redaction_status:
notes:
```

Never commit secrets, real passwords, private signing keys, access tokens, or unredacted personal identifiers.

## Capture storage

Large packet captures should not be committed casually to Git history.

Use a manifest/index in Git and select a proper artifact/data store once corpus size is known.

---

# Research cards

Every unresolved P0/P1 behavior should receive a card under:

`docs/research/cards/`

Template:

```text
ID
TITLE
PRIORITY
TARGET BUILDS
STATUS

PURPOSE
CONSUMER
PRECONDITIONS

KNOWN
UNKNOWN
CONFLICTS

EXPERIMENT
EXPECTED OBSERVATIONS
CAPTURE REQUIREMENTS

IMPLEMENTATION CONSEQUENCE
ACCEPTANCE TEST

EVIDENCE
```

---

# First firing order

1. Upstream capability inventory
2. Exact client/build/transport matrix
3. Successful login trace for one representative target
4. Realm join through a real emulator
5. Session lifecycle
6. Negative-path corpus
7. Broaden across other client families
8. Desktop app if still required

Why this order: it prevents spending weeks cloning optional endpoints before proving the shortest path from client startup to playable realm.

---

# Campaign completion test

The compatibility campaign can close for a supported build only when:

- login flow is independently documented,
- protocol transport is independently documented,
- relevant service/method map exists,
- session lifecycle is documented,
- realm list is documented,
- realm join is documented on both sides,
- success trace exists,
- negative traces exist,
- independent implementation tests exist,
- no remaining `UNKNOWN` blocks ordinary play,
- remaining unknowns are named and explicitly deferred.

Anything less is research progress, not completion.
