# SRP6a Implementation

> **Status:** Implemented (`tavern-core/src/srp/`, M4). Verified against the
> C++ `srp_reference` tool and a pure-Python reference reproduction for
> byte-accurate v2 verifiers. Grounded in the captured Battle.net web-login
> SRP code (`srp6a-routines.worker.js`, `srp.js`) and the documented
> parameters in `oauth-oidc-implementation.md` RE spec. The sections below
> describe the algorithm and parameters as implemented; historical
> “implementation plan” wording is retained for reference.

## Scope

Tavern's SRP6a implements the **Battle.net web-login password proof**, used by
`account.battle.net/login/srp` and `/login/{locale}/password`. It is the only
password-based credential path. The game-client BGS auth and the OAuth token
layer are separate and do not use SRP.

## Two SRP variants — Tavern uses the modern one

The WoW ecosystem has two unrelated SRP flavors. Tavern does **not** use the
legacy one:

| | Legacy realm auth (`wow_srp`) | **Battle.net web login (Tavern)** |
| --- | --- | --- |
| Prime size | 256-bit | **2048-bit** |
| Generator | 7 | **2** |
| Hash | SHA-1 | **SHA-256** |
| KDF | none (raw interleaved hash) | **PBKDF2-HMAC-SHA-256** |
| Used by | 3.3.5a-era legacy realm logon | account.battle.net web login |

`wow_srp` (the `gtker/wow_srp` crate) implements the legacy 256-bit/SHA-1
variant. It is a useful reference for SRP-6a structure and tested against real
WoW logins, but its parameters and KDF do **not** match Battle.net's modern web
login. A direct port would implement the wrong variant.

Tavern implements a local SRP6a tailored to the Battle.net web-login
parameters. `wow_srp` is consulted as a structural reference only.

## Authoritative reference found

A known-working reference implementation (the bnetserver `LoginRESTService`
and its `SRP6` crypto module) implements the exact modern variant on its
`wotlk_classic` and `cata_classic` branches. The endpoint
`POST /bnetserver/login/srp/` and `POST /bnetserver/login/` with
`JSON::Login::LoginForm` match the Battle.net wire shape (`inputs_required =
[account_name, public_A, client_evidence_M1]`). This is the byte-accurate
reference, superior to the RFC 3526 fallback and to the minified worker JS.

## Authoritative parameters (from reference implementations + captured traffic)

Server-negotiated, returned by `POST /login/srp`:

| Parameter | Value | Source |
| --- | --- | --- |
| `modulus` N (v2) | 2048-bit, `AC6BDB41...73` (hardcoded in the reference) | reference SRP6.cpp |
| `generator` g | 2 | reference SRP6.cpp |
| `hash_function` | SHA-256 | negotiated; SHA-512 also possible |
| `iterations` | 15000 (v2) | `GetXIterations()` returns 15000 |
| `version` | 2 | selects `BnetSRP6v2` |
| `username` | `SHA256(login)` as hex string | `ByteArrayToHexStr(SHA256::GetDigestOf(login))` |
| `salt` s | 32 random bytes | per-credential |
| `public_B` | server ephemeral | per login attempt |

The v1 prime differs (`BnetSRP6v1Base::N`, also 1024-bit). Tavern targets v2.

## Byte-order conventions (three distinct ones)

The reference implementation's `BigNumber`/SRP6 mixes byte orders; replicating
them exactly is
what makes verifiers interoperate. Validated against both the C++ `srp_reference`
tool and the pure-Python reference reproduction.

1. **Verifier / credential output is little-endian**, exact significant byte
   length, no padding to N's size. This is `ToByteVector` + `ByteArrayToHexStr`.
   The stored `verifier` and the hex served in challenges use this encoding.
   Internally Tavern holds the verifier as a big-endian `BigInt` (the natural
   math representation); it must reverse to little-endian when serializing to
   the wire or the `credentials` table to match clients and reference servers.
2. **v2 private exponent `x` is big-endian, signed** (the `0x80` sign hack is
   `int.from_bytes(..., signed=True)`); **v1 `x` is little-endian**. Same
   function name, opposite conventions. Tavern implements v2.
3. **All internal SRP math** (k, u, M1/M2 evidence) is **big-endian**, padded
   to the 256-byte field width (`BN_bn2binpad`, `littleEndian=false`).

The `%(N-1)` reduction is `BN_nnmod` — always non-negative — matching Python's
floored `%` and Tavern's normalize-after-mod.

## Algorithm (exact, from reference `BnetSRP6v2`)

### Verifier (registration)

```text
x = CalculateX(username, password, salt)    # see below
v = g^x mod N                                # standard SRP6a verifier
```

### Key derivation (`CalculateX`, v2) — PBKDF2-HMAC-SHA-512

```text
tmp = username + ":" + password              # the raw login (email) + ":" + password
dx = PBKDF2-HMAC-SHA-512(tmp, salt, 15000, 64 bytes)
x = int(dx, big-endian)                     # NB: big-endian (BN_bn2binpad, littleEndian=false)
if dx[0] & 0x80: x = x - 2^512               # interpret as SIGNED two's-complement
x = x mod (N - 1)                            # bound to the group
```

`username` here is the email/login string, **before** the `SHA256(login)` used
as the challenge `username` field. The `I` identity in the SRP session is
`SHA256(login)` (32 bytes).

### Session math

```text
b = random in [1, N-2]                       # server private ephemeral
B = (g^b + v*k) mod N                        # server public ephemeral
k = SHA256(pad256(N) || pad256(g))           # multiplier, padded-pair hash
u = SHA256(pad256(A) || pad256(B))           # scrambling parameter
S = (A * v^u)^b mod N                        # shared secret
M1 = SHA256(evidence(A) || evidence(B) || evidence(S))  # client evidence
M2 = SHA256(evidence(A) || evidence(M1) || evidence(K)) # server evidence
```

**Padded-pair hashing** (`pad256`): inputs are written big-endian and
left-padded with zero bytes to 256 bytes (the full width of the 2048-bit
modulus) before hashing. This is the `ToByteArray<256>(false)` pattern — the
`false` flag selects big-endian (`BN_bn2binpad`); the little-endian default is
never used in `BnetSRP6`.

**Evidence vector** (`GetBrokenEvidenceVector`): the same big-endian,
zero-padded-to-field-width encoding, with length `ceil(bits/8)`.

### Server verification

The server stores `(salt, verifier, iterations, version)`, never the password.
On login it receives the client's `A` and `M1`, computes its own `S` and `M1`,
and accepts only if they are equal. It rejects `A mod N == 0` and `u mod N == 0`.

## Implementation plan (Milestone 4, `tavern-core`)

Pure, no I/O, unit-tested. New modules under `tavern-core/src/`:

- `srp/mod.rs` — public API: `compute_verifier`, `ServerSession`.
- `srp/groups.rs` — the 2048-bit prime `AC6BDB41...73` (reference `N`) and
  `g = 2`.
- `srp/kdf.rs` — `compute_x(srp_username, password, salt, iterations)` via
  PBKDF2-HMAC-SHA-512, big-endian signed interpretation + `mod (N-1)`.
- `srp/encoding.rs` — big-endian encodings: fixed 256-byte padding for k/u,
  the two's-complement evidence vector for M1/M2, and the SRP username
  identity.

### Dependencies

- `sha2` (SHA-256 for k/u/M1; SHA-512 for the KDF), `pbkdf2`, `hmac`, `digest`
  — pure-Rust crypto, no system libs. Add to `tavern-core` Cargo deps (the
  crate remains pure: no tokio, no sqlx).
- Big integers: `num-bigint` (pure Rust) is sufficient at SRP proof frequency.
  Avoid `rug`/GMP to keep the build hermetic.

### Verification

- **Authoritative cross-check**: `tavern-core` computes the v2 verifier for
  all 10 reference tool combos (fixed zero salt) and asserts byte
  equality with both the C++ reference tool and the pure-Python reference
  reproduction. The reference value
  is little-endian hex, so the test reverses to big-endian before comparing as
  integers. This is the strongest available test — cross-implementation
  equality against the byte-accurate source.
- A self-consistent round-trip: an independent client code path and the server
  agree on the shared secret; wrong passwords, degenerate `A`, and replayed
  sessions are rejected.
- The padded-pair hashing and evidence encoding are big-endian, matching
  `ToByteArray<256>(false)` / `BN_bn2binpad`.

## Open question resolved

The exact served `modulus` is no longer unknown: the reference `BnetSRP6v2Base::N`
(`AC6BDB41...73`) is the canonical value and is hardcoded server-side. Tavern
uses it directly. The KDF hash is **SHA-512** (PBKDF2-HMAC-SHA-512), not
SHA-256 — the session hashes (k, u, M1) are SHA-256.
