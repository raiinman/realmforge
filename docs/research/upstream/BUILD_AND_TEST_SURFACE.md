# Upstream Build / Test / Evidence Surface

**Upstream:** `wowemulation-dev/tavern`  
**Pinned revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Purpose:** Preserve the upstream regression baseline while keeping Realmforge's independent compatibility evidence separate.

---

# 1. Two test universes

Realmforge must maintain two explicitly different bodies of tests.

## A. Interim-source regression suite

Question answered:

> Did we preserve or intentionally change the behavior of the pinned inherited implementation?

This suite may use upstream tests, scripts, fixtures and assumptions because it is testing the inherited component itself.

## B. Realmforge independent interoperability suite

Question answered:

> Does a real target client actually require and accept this behavior?

This suite must be built from Realmforge-controlled observations, captures, standards and real target clients rather than copied upstream test assumptions.

A green A-suite does not imply a green B-suite.

---

# 2. Pinned build baseline

Workspace metadata at the pinned revision:

```text
Rust edition: 2024
rust-version: 1.97
workspace version: 0.1.0
workspace license: AGPL-3.0-only
```

Primary Rust services:

```text
account-server
oauth-server
bgs-server
```

The upstream testing guide's baseline build command is conceptually:

```text
cargo build -p account-server -p oauth-server -p bgs-server
```

with a valid PostgreSQL `DATABASE_URL` available for SQLx/runtime behavior.

The repository also carries `.sqlx/` offline query metadata used by its containerized build path.

### Realmforge baseline rule

After source intake, before changing covered source:

1. build the exact pinned snapshot,
2. apply all pinned migrations to a fresh database,
3. run upstream Rust tests,
4. run the documented synthetic smoke flows,
5. record the exact toolchain/container versions and results,
6. only then begin Realmforge integration modifications.

If the exact snapshot does not pass its own baseline, record that fact rather than silently fixing it before the baseline commit.

---

# 3. Database test baseline

Pinned testing documentation expects PostgreSQL 16 and all 25 migrations.

The upstream test setup has reference accounts and additional seed data used to drive states such as:

- ordinary active account,
- different regions,
- authenticator flag,
- different license/game-time state,
- suspended/banned scenarios,
- multiple game/account metadata cases.

Realmforge should retain the upstream seed fixtures inside the third-party archive for regression testing.

Do not reuse their test identities/passwords as Realmforge production defaults.

---

# 4. Rust integration tests present in the pinned tree

## `tavern-account`

Pinned test files:

```text
crates/tavern-account/tests/api_ui.rs
crates/tavern-account/tests/bnet_login.rs
crates/tavern-account/tests/creation_flow.rs
crates/tavern-account/tests/locale_emails.rs
crates/tavern-account/tests/srp_login.rs
crates/tavern-account/tests/tos_flow.rs
```

### What they are useful for

- account/API route regressions,
- expected upstream JSON shapes,
- bnet login flow,
- SRP server behavior,
- account-creation state machine,
- localized email behavior,
- legal/ToS flow.

### What they cannot prove independently

They are authored with the same implementation knowledge as the server. They do not prove that every asserted shape is required by a real target client.

## `tavern-oauth`

Pinned integration test:

```text
crates/tavern-oauth/tests/oauth_flow.rs
```

Useful for preserving current OAuth behavior.

Realmforge's future standards implementation must instead be tested from standards/conformance expectations plus proprietary-client interoperability.

## `tavern-core`

The tree contains test-key data and substantial unit tests embedded in source modules.

The PEM under test data is a test fixture. It must never become a production signing key.

---

# 5. Browser/account UI tests

The upstream testing guide documents optional browser automation through `playwright-cli` for:

- login pages,
- overview dashboard,
- Games & Subs,
- Security,
- Account Details,
- Privacy,
- email-verification behavior.

This is useful only while Realmforge temporarily exposes the upstream account UI.

Realmforge Console should receive its own product-level UI tests once it exists.

Do not preserve upstream visual fidelity merely because its Playwright flow passes.

---

# 6. OAuth external tooling

Upstream documentation uses `oauth2c` as an external client for at least parts of the validation matrix and warns that the configured issuer must match the discovery URL.

This is better evidence than a unit test that simply calls the implementation internally, but it still answers standards/client interoperability—not retired game-client compatibility by itself.

Realmforge future tests should include:

```text
OIDC discovery validation
authorization code + PKCE
client credentials where intentionally supported
refresh token rotation
revocation
introspection
device flow if retained
RFC 8693 exchange if desktop compatibility requires it
invalid redirect URI
invalid PKCE
expired code
reused code
wrong client
```

---

# 7. Upstream SRP development tools

Pinned tools:

```text
dev/srp_auth_client.py
dev/srp_oracle.py
```

The testing guide uses `srp_auth_client.py` to simulate web/game authentication against the account server.

### Use in Realmforge

Keep these tools with the covered upstream snapshot and use them to answer:

> Does our interim Gate still satisfy its original synthetic client?

Do not use them to generate the only test vectors for an independent SRP replacement.

Independent SRP vectors must be derived separately and cross-checked against real target-client behavior.

---

# 8. Upstream BGS synthetic clients

Pinned load-test tree includes:

```text
load-test/bgs-client.py
load-test/bgs-queue-test.py
load-test/bgs_proto/bgs_pb2.py
load-test/login-harness.py
load-test/seed-loadtest-accounts.py
```

The general test guide documents BGS flows including:

```text
Connect + Logon
full auth using one-time ticket
account state
SSO/token behavior
session restore
queue/load scenarios
```

### Great for

- regression,
- deterministic concurrency,
- queue testing,
- session-state stress,
- database pool/load behavior,
- malformed/fuzzed synthetic requests,
- comparing before/after Realmforge modifications to the covered Gate.

### Not sufficient for

- exact real-client TLS behavior,
- real-client WebSocket negotiation quirks,
- hidden/optional service calls,
- actual build-family differences,
- request timing,
- reconnect timing,
- client-side error UX,
- undocumented fields the synthetic client never sends,
- whether the synthetic protobuf/schema itself omitted something.

---

# 9. Fuzzing

Pinned developer tree contains:

```text
dev/fuzz.py
```

Keep as an upstream regression/security input.

Realmforge should eventually establish separate first-party fuzz targets for:

- BGS frame parser,
- protobuf/RPC dispatcher,
- compressed realm JSON input/output helpers,
- login request parsing,
- SRP public values,
- OAuth parameter parsing,
- local Client agent API,
- emulator adapter inputs.

Fuzzing should focus on parser safety and state-machine robustness, not merely HTTP 500 avoidance.

---

# 10. Load/performance evidence

Upstream includes load-test harnesses and performance/scaling documentation.

Performance claims derived from those tools are useful capacity evidence for the pinned architecture, but Realmforge should preserve:

```text
revision
hardware/container limits
Postgres config
connection pool config
MAX_BGS_LOGINS
concurrency
TLS/no-TLS mode
scenario distribution
duration
success/error count
latency percentiles
```

without those fields, "15k logins" style numbers are anecdotes rather than repeatable capacity authority.

Realmforge should rerun performance tests after every major Gate/Core integration change rather than assuming upstream numbers survive the new topology.

---

# 11. Critical synthetic-versus-real evidence rule

Use this exact grading:

| Evidence | Grade | Can declare client support? |
|---|---|---|
| copied/upstream unit test | D/C | No |
| upstream synthetic client against upstream server | C | No |
| Realmforge synthetic client based only on upstream schema | C | No |
| external generic standards client | B/C for standard | Not game support |
| independent black-box request reproduced from our capture | A/B | Strong supporting evidence |
| real retired target client, controlled test, captured | A | Yes, for that exact tested build/scenario |
| emulator reaches world after same real-client flow | A | Yes, for end-to-end combination tested |

This prevents a classic emulator mistake: proving that our own client and our own server agree with each other and then calling that protocol compatibility.

---

# 12. Realmforge independent test repository shape

Future first-party tests should live outside `third_party/`:

```text
research/
  captures/
    INDEX.md
    manifests/
  fixtures/
    http/
    bgs/
    realm/
  harness/
    http/
    bgs/
    realm/
  client-contracts/
    1.13/
    1.14/
    2.5/
    3.4/
    4.4/

tests/
  contract/
  integration/
  e2e/
  soak/
```

Raw PCAPs may need external/artifact storage rather than normal Git history. Git should still carry hashes/manifests/redacted fixtures.

---

# 13. First baseline report format

When the pinned source snapshot is imported and executed, create:

`docs/evidence/GATE_BASELINE_<date>.md`

with:

```text
Upstream revision
Realmforge import commit
OS/container runtime
Rust version
Postgres version
Build result
Migration result
Rust test result
Account synthetic result
OAuth external-client result
BGS synthetic result
Queue test result
Known failures
Warnings
Changes made before baseline: NONE
```

The "changes before baseline" field matters. We want to know whether a failure belongs to the inherited snapshot or to us.

---

# 14. Gate modification test rule

Every modification to the covered interim Gate should answer both:

```text
UPSTREAM REGRESSION:
Did expected pinned behavior remain stable or change deliberately?

REALMFORGE INTEROP:
Did actual target-client behavior improve/remain stable?
```

Those can disagree.

If a Realmforge change breaks an upstream synthetic test but fixes independently confirmed target-client behavior, the correct action may be to deliberately retire/update the inherited regression—not revert the real fix.

Independent evidence wins.
