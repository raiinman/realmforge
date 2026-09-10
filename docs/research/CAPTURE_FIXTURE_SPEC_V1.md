# Realmforge Capture Fixture Specification v1

**Schema:** `realmforge.capture.v1`  
**Status:** ACTIVE  
**Authority:** Realmforge compatibility research  

## Purpose

This format is the handoff boundary between target-client observation and protocol implementation.

A protocol implementer should be able to receive a reviewed fixture bundle and learn what the client actually did without consulting covered interim source code.

A fixture records evidence. It is not automatically permission to implement a conclusion.

Target-client-specific code may only treat a fixture as implementation authority when `realmforge-gate-fixture-check <manifest.json> --implementation-ready` succeeds.

## Bundle shape

```text
<capture-id>/
├── manifest.json
└── artifacts/
    ├── traffic.pcapng
    ├── http-transcript.json
    ├── tls-keylog.redacted.txt
    └── process-notes.txt
```

Large or sensitive artifacts do not need to live in Git. Their manifest entries and SHA-256 digests do.

## Required manifest fields

```json
{
  "schema": "realmforge.capture.v1",
  "capture_id": "classic-1.13.2.31650-login-001",
  "captured_at_utc": "2026-09-10T12:00:00Z",
  "researcher": "realmforge",
  "client": {
    "family": "classic",
    "version": "1.13.2",
    "build": 31650,
    "platform": "windows",
    "architecture": "x86_64",
    "executable_sha256": "<64 hex characters>"
  },
  "os": "Windows 10",
  "network_topology": "isolated-lab",
  "server_under_test": "realmforge-observer",
  "server_revision": "<commit or immutable observer revision>",
  "scenario": "cold login attempt",
  "expected_result": "observe client network sequence",
  "actual_result": "client initiated observed connection",
  "evidence_grade": "first_party",
  "artifacts": [],
  "observations": []
}
```

## Evidence grades

- `first_party` — Realmforge-controlled target-client observation. This is the only grade allowed to become implementation-ready for target-specific behavior.
- `standard` — public protocol/RFC authority. May directly govern standards-derived code, but does not prove proprietary target-client behavior.
- `correlated` — independent technical research or another implementation. Useful for hypotheses and experiment design.
- `upstream_lead` — historical covered-project material used only to identify a question. Never implementation-ready.
- `community_lead` — unverified community report.
- `hypothesis` — Realmforge working theory awaiting evidence.

## Artifact rules

Every artifact has:

- stable ID,
- relative path inside the capture bundle,
- media type,
- SHA-256,
- `contains_secrets`,
- redaction status.

Absolute paths and `..` traversal are invalid.

Implementation-ready bundles must contain no artifact marked as containing secrets. Every artifact must be `reviewed` or `not_required` for redaction.

Never commit:

- passwords,
- session cookies,
- OAuth access/refresh tokens,
- private signing keys,
- TLS private keys,
- personally identifying account names unless replaced by synthetic lab identities.

## Observation rules

Observations are ordered facts:

```json
{
  "sequence": 1,
  "offset_micros": 0,
  "direction": "client_to_server",
  "layer": "tcp",
  "operation": "connect",
  "endpoint": "observer.invalid:1119",
  "artifact_id": "pcap"
}
```

Supported layers are:

- `process`
- `dns`
- `tcp`
- `tls`
- `http`
- `web_socket`
- `rpc`
- `realm`
- `other`

Sequence numbers must strictly increase and time offsets must be monotonic.

An observation may reference an artifact and/or a separately calculated payload SHA-256. Decoded proprietary structures belong in notes or a future versioned decoded-artifact format; do not silently turn guesses into typed protocol authority.

## Implementation-ready gate

A target-client fixture is implementation-ready only when all of these are true:

- schema validates,
- client executable has a SHA-256,
- target build is explicit,
- evidence grade is `first_party`,
- observations are present,
- referenced artifacts exist in the manifest,
- hashes are valid,
- no artifact is marked as containing secrets,
- redaction review is complete,
- actual result is recorded.

This does **not** mean one capture proves an entire protocol. It means the recorded facts may be used as input to implementation.

## Capture progression

For one candidate client build, produce fixtures in this order:

1. process start + DNS/connect-only trace,
2. successful login trace,
3. realm-list trace,
4. realm-join trace,
5. realm-side authentication trace,
6. graceful logout,
7. dropped connection/reconnect,
8. wrong password,
9. expired/replayed ticket,
10. incompatible/offline realm.

The first playable-build contract is not accepted until login through world entry has independently reproduced fixtures plus negative-path coverage.

## Tooling

Validate ordinary schema integrity:

```bash
cargo run --manifest-path components/gate-independent/Cargo.toml \
  --bin realmforge-gate-fixture-check -- path/to/manifest.json
```

Require first-party implementation authority:

```bash
cargo run --manifest-path components/gate-independent/Cargo.toml \
  --bin realmforge-gate-fixture-check -- path/to/manifest.json --implementation-ready
```

## Versioning

Do not mutate v1 semantics after real capture bundles exist.

If the format needs an incompatible change, add `realmforge.capture.v2` and keep the v1 validator available for historical evidence.
