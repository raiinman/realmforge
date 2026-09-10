# Performance and Scaling — Status

> **Status:** All ten remediation phases implemented and verified. The
> component-level scaling work is complete; the remaining items are
> deployment-side (transport flags, per-IP caps, PgBouncer, k8s) and are
> listed under [Remaining](#remaining). This document records what was
> done, the measured baselines, and what is still open.

This document records the concurrency analysis of the three Realmforge binaries
(`realmforge-gate-account-server`, `realmforge-gate-oauth-server`, `realmforge-gate-bgs-server`) against the target load of
5,000–15,000 concurrent players — the range modern private servers reach.

## Outcome

The SRP challenge endpoint sustains ~1,100 rps single-instance at release
(50-way concurrency, 2026-08-12); the full-login harness caps at ~22 rps
because it serializes each login into two round trips plus client-side SRP
work, not because the server is slower. The earlier "~20 rps
single-instance ceiling, SRP CPU-bound" conclusion was measured against
debug binaries and is superseded — see [Measurements](#measurements). The
5k–15k target is reached by horizontal scaling: account/oauth are stateless
and scale freely; bgs keeps long-lived sockets and needs sticky sessions.

## Implemented Phases

All phases shipped in two commits: `3b9d2e2` (phases 0–8) and `216fc73`
(phase 9). Each acceptance criterion was verified at the time and re-verified
against the running code on 2026-08-12.

### Phase 0 — Observability (OpenTelemetry) and Health Probes

- `crates/realmforge-gate-observability` provides OTLP metrics (HTTP request
  count/duration, SRP verify duration histogram, DB pool gauge) and a shared
  health router.
- `/health`, `/ready`, `/startup` on all three binaries (verified 200 on
  all; readiness checks the DB pool).
- Dev collector at `dev/otel.sh`.

### Phase 1 — Configurable, Right-Sized DB Pool

- `PoolConfig::from_env()`: `DB_POOL_MAX_CONNECTIONS`,
  `DB_POOL_MIN_CONNECTIONS`, `DB_POOL_ACQUIRE_TIMEOUT_SECS`,
  `DB_POOL_MAX_LIFETIME_SECS`, `DB_POOL_IDLE_TIMEOUT_SECS`.
- All three binaries use `connect_with(db_url, &cfg)`; legacy `connect()`
  kept for tests.

### Phase 2 — Load-Test Harness

- `load-test/login-harness.py`: concurrent SRP logins, throughput +
  latency percentiles (p50/p95/p99), JSON output.
- `load-test/bgs-queue-test.py`: BGS connection admission (direct/queued)
  and benchmark mode.

### Phase 3 — BGS Per-Connection Session Ownership

- The global `Mutex<HashMap<String, BgsSession>>` is gone; each WS and TCP
  connection task owns its `BgsSession`. Zero per-frame lock contention.
- `dispatch_frame` and all service handlers take `session: &mut BgsSession`
  directly.

### Phase 4 — Offload SRP to a Blocking Pool

- All four SRP call sites (`ServerSession::new` and `verify` in
  `login.rs` + `bnet.rs`) run via `tokio::task::spawn_blocking`.
- `Arc<AppState>` is cloned instead of the `Group` (no per-login BigInt
  allocation); join errors map to `AccountError::Internal`.
- The 2048-bit modpow no longer stalls the async runtime.

### Phase 5 — Map Eviction and Login Rate Limiting

- Background reaper (60s interval) drops expired `challenges` (120s TTL),
  `bnet_sessions` (120s TTL), and `creation_sessions` (30min TTL).
- Per-email rate limiting in `post_srp_challenge`: duplicate challenges for
  the same email within 120s are rejected.
- Acceptance re-verified live: 200/200 logins at concurrency 100, 0
  collisions.

### Phase 6 — CSPRNG Correctness

- `session_key` (64 bytes) and `sso_secret` (32 bytes) use single
  `rand::thread_rng().fill_bytes()` calls; the UUID-loop patterns are gone.

### Phase 7 — Caching Layer

- OAuth client cache (`moka::sync::Cache`, 5-min TTL) in
  `resolve_client_auth` — every `/token` request hits this path.
- JWKS precomputed at startup (`OAuthState.jwks_json`); the `jwks` handler
  serves a clone instead of re-serializing per request.

### Phase 8 — Runtime Tuning and Lock Removal

- `bnet_sessions` and `challenges` converted from `Mutex<HashMap>` to
  `dashmap::DashMap` — 26% throughput gain (16 → 20 rps), p99 latency down
  21–23%.
- Explicit `worker_threads` on all three binaries, configurable via
  `TOKIO_WORKER_THREADS` (defaults to core count).

Before/after (200 logins, 200 accounts, pool=32):

| Concurrency | Before (Mutex) | After (DashMap) |
| --- | --- | --- |
| 25 | 16.4 rps / p99 2395ms | 20.4 rps / p99 1924ms |
| 50 | 16.5 rps / p99 4523ms | 20.3 rps / p99 4100ms |
| 100 | 16.3 rps / p99 9435ms | 20.6 rps / p99 7306ms |

### Phase 9 — Admission Control and Backpressure

- `tokio::sync::Semaphore` concurrency limiter via
  `MAX_CONCURRENT_REQUESTS` (default 1024) on realmforge-gate-account-server and
  realmforge-gate-oauth-server; overloaded requests return `503 Service Unavailable`.
- Verified at the time: `MAX_CONCURRENT_REQUESTS=10` with 50 concurrent SRP
  logins → 10 in flight, 90 get 503.

## Measurements

Two numbers must be kept apart: what the load harness measures, and what the
server actually sustains. The harness (`login-harness.py`) runs each login as
two sequential HTTP round trips (challenge, then proof) plus client-side SRP
computation, and caps around 22 rps regardless of server build — it measures
the client-side handshake, not the server ceiling.

### Harness numbers (full login)

200 distinct `lt*@loadtest.local` accounts, re-run 2026-08-12:

| Build | Concurrency | rps | p50 | p95 | p99 | fails |
| --- | --- | --- | --- | --- | --- | --- |
| debug (local `cargo build`) | 10 | 18.8 | 517ms | 733ms | 802ms | 0/200 |
| debug | 50 | 19.7 | 2.38s | 3.64s | 4.17s | 0/200 |
| debug | 100 | 19.6 | 4.66s | 6.75s | 8.38s | 0/200 |
| release | 10 | 22.4 | 426ms | 694ms | 766ms | 1/200 |
| release | 50 | 22.7 | 1.77s | 2.94s | 3.25s | 1/200 |
| release | 100 | 22.7 | 1.72s | 2.72s | 3.41s | 1/200 |

### Server endpoint numbers

The SRP challenge endpoint (`POST /bnetserver/login/srp/`) at 50-way
concurrency, 200 distinct accounts:

| Build | Single request | 50-way rps |
| --- | --- | --- |
| debug | ~56ms | 108.8 |
| release | ~6ms | 1121.8 |

The release build runs the 2048-bit modpow ~21x faster (0.6ms vs 13ms debug);
the containers (`deploy/*.Containerfile`) already build with `--release`. The
single-request latency at release is dominated by the two DB lookups and the
HTTP round trip, not the modpow. The earlier "~20 rps single-instance
ceiling" and the "69ms per modpow" figure in the phased plan were measured
against debug binaries and are superseded by these numbers.

- Pool size is not the ceiling: `DB_POOL_MAX_CONNECTIONS=32` measured the
  same throughput as pool 8 in both profiles.
- BGS admission: 200 clients at 50-concurrency admitted direct (zero
  queueing) in ~2.3s (~87 connects/sec).

## Remaining

Deployment-side and optional items, in rough priority order. None block the
component-level scaling work.

- **Per-IP connection caps at accept** (Phase 9 listed them; only the global
  in-flight semaphore exists). A remote-address limiter would harden against
  socket floods.
- **`tcp_nodelay(true)`** on sockets (Phase 8 listed it; absent).
- **`max_blocking_threads` tuning** (Phase 8 listed it; tokio default 512
  applies).
- **`SO_REUSEPORT`** — optional per the plan; within-node accept spreading.
- **PgBouncer** in front of Postgres (transaction-pooling mode) — documented
  deployment posture; not required at current scale.
- **k8s deployment**: autoscaling, sticky-session load balancing for bgs,
  graceful drain on SIGTERM (bgs must stop accepting, finish in-flight
  frames, close idle sockets within the grace window).
- **OTel histogram bucket tuning** — deferred; the `with_view` API in
  `opentelemetry_sdk` is still experimental. Default second-scale boundaries
  give accurate sums/counts.

## Horizontal Scaling Constraints

- `realmforge-gate-account-server` and `realmforge-gate-oauth-server` scale freely: session state lives in
  Postgres and requests are short-lived. Round-robin or least-connections
  routing works.
- `realmforge-gate-bgs-server` holds long-lived stateful connections. Scale on active
  connection count; the load balancer must pin a player's socket to one pod
  (sticky sessions), or a pod loss drops everyone.
- Restart bursts are the worst case. Phases 4 and 5 address the login
  thundering herd; Phase 9 bounds it. At ~20 rps per instance, 3 instances ×
  20 rps = 60 rps serves 15k logins in ~4 minutes.

## Cross-References

- `docs/architecture.md` — service topology and request flow.
- `docs/testing.md` — the end-to-end test procedure (SRP, BGS, browser).
- `CHANGELOG.md` — entry for the performance and observability phases.
