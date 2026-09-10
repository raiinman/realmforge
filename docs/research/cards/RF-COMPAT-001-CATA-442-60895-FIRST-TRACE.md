# RF-COMPAT-001 — Cataclysm Classic 4.4.2.60895 First Trace

**Priority:** P0  
**Target:** Cataclysm Classic 4.4.2.60895  
**World candidate:** TrinityCore `cata_classic`  
**Status:** `CAPTURE-REQUIRED`  
**Implementation authorized:** NO

## Purpose

Produce the first Realmforge-controlled observation of build 4.4.2.60895 from process start through its initial failed or successful authentication attempt.

This card exists to answer **what the client actually does first**. It must not be filled from covered source code or from another emulator's authentication implementation.

## Consumer

Future independent Gate protocol-adapter work for the first playable Realmforge vertical slice.

## Preconditions

- legally obtained retired 4.4.2.60895 client available in the research lab,
- executable build/version independently verified,
- executable SHA-256 recorded,
- synthetic lab account identity only,
- network capture tooling installed,
- test network topology documented,
- no production/private credentials used,
- capture bundle prepared using `realmforge.capture.v1`.

## Known

### K1 — exact world-emulator candidate

Current public TrinityCore repository metadata identifies its `cata_classic` target as `4.4.2.60895`.

Evidence grade: `C / CORRELATED`.

### K2 — current emulator branch

`TrinityCore/TrinityCore:cata_classic` is an active branch. When inspected 2026-09-10, GitHub reported branch head:

`237a375b8ba395953097f9cbe70969d95020d30d`

with commit timestamp 2026-09-09.

Evidence grade: `C / CORRELATED`.

### K3 — Realmforge capture schema exists

`realmforge.capture.v1` records exact client build/hash, artifacts, ordered observations, redaction state, and evidence grade.

Evidence grade: Realmforge authority.

## Unknown

Do **not** fill these from memory or inherited implementations:

- first DNS hostname,
- first destination address/port,
- first application transport,
- TLS behavior,
- certificate expectations,
- WebSocket presence/path/subprotocol if any,
- HTTP endpoints if any,
- RPC framing if any,
- BGS generation,
- service/method identifiers,
- login proof mechanism,
- token/ticket behavior,
- realm-list request,
- session establishment,
- reconnect behavior.

## Conflicts

Historical/interim documentation and third-party implementations make claims about multiple BGS generations. Those claims are not authority for 4.4.2.60895.

Disposition: `IGNORE FOR IMPLEMENTATION / USE ONLY TO DESIGN EXPERIMENTS`.

## Experiment RF-COMPAT-001-A — cold-start network observation

### Setup

1. Verify target executable reports version/build 4.4.2.60895.
2. Calculate executable SHA-256.
3. Start packet capture before launching the client.
4. Start any process/DNS observation tooling before launch.
5. Launch from a cold state with no Realmforge session/token assumptions.
6. Allow the client to reach its natural login/authentication failure or endpoint interaction.
7. Stop capture.
8. Redact synthetic credentials/tokens if any appeared.
9. Hash every retained artifact.
10. Write `manifest.json` and validate it with `realmforge-gate-fixture-check --implementation-ready`.

### Required observations

Record, in order:

```text
process start
DNS queries
TCP connections
TLS handshakes
HTTP requests if observable
WebSocket upgrade if observable
first application payload or opaque encrypted session boundary
client-visible result
```

### Do not

- infer a service merely from a familiar port,
- call traffic BGS until evidence supports that label,
- disable certificate checks merely to make interception easier without recording that modification,
- store real credentials,
- describe decrypted payload fields unless independently observed and preserved in an artifact,
- implement target-specific parsing before this card has implementation-ready evidence.

## Experiment RF-COMPAT-001-B — endpoint redirection probe

Run only after A establishes the client's actual destination behavior.

Goal: determine the least-invasive lab method for redirecting the retired client to a Realmforge observer without changing unrelated client behavior.

Record separately:

- redirection method,
- modified files/settings/DNS entries,
- before/after hashes where relevant,
- whether the client accepted the redirected endpoint,
- whether TLS/certificate validation blocked progression,
- exact client-visible failure state.

Any modified-client observation must be clearly labeled so it cannot be confused with an unmodified-client fact.

## Expected output

At least one bundle:

```text
captures/cata-4.4.2.60895/<capture-id>/manifest.json
```

Large packet data may live outside Git, but its SHA-256 and artifact metadata must be recorded.

## Implementation consequence

When experiment A/B produces implementation-ready evidence, create a new card describing exactly one observed protocol boundary, for example:

- TCP connection preamble,
- TLS endpoint requirement,
- WebSocket upgrade,
- HTTP login entrypoint,
- first RPC frame.

Implement only that boundary and then recapture.

Do not jump from first connection evidence to a guessed complete authentication server.

## Acceptance test

This card closes when:

- client executable SHA-256 is recorded,
- a first-party capture manifest passes the implementation-ready validator,
- initial destination/transport behavior is documented from that capture,
- client-visible result is recorded,
- all artifacts are redaction-reviewed,
- the next smallest implementation card can be written without consulting covered source.

## Evidence

### Correlated world target

- `https://github.com/TrinityCore/TrinityCore`
- `https://github.com/TrinityCore/TrinityCore/tree/cata_classic`

### Realmforge authority

- `docs/research/CAPTURE_FIXTURE_SPEC_V1.md`
- `docs/research/COMPATIBILITY_RESEARCH_CAMPAIGN.md`

## Current disposition

`CAPTURE-REQUIRED`

No 4.4.2.60895 target-specific Gate code is authorized by this card yet.
