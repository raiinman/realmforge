# Realmforge first emulator adapter comparison

**Status:** decision support complete for first playable vertical slice  
**Scope:** first Bridge adapter only; this does not make any emulator canonical Realmforge architecture  
**Target client family:** modern Classic-generation retired clients, with 1.14.0.40618 as the first concrete world-auth target

## Decision

Select the **TrinityCoreClassic 1.14.0.40618 family** as Realmforge's first Bridge adapter target.

The first validation pin is the public `Frostshake/TrinityCoreClassic` `vanilla_classic` line observed at commit `1fc8f1ebe3d09fcb67194b99d0281f88188b4a0e`. A later maintained fork may beat that exact pin after reproducible build/database/runtime comparison, but the adapter contract targets the family and its observed 40618 world-auth semantics rather than a permanent fork brand.

This is an implementation target, **not a Realmforge support claim**. Build 40618 remains unsupported until a real client completes login -> realm list -> realm join -> world authentication -> character screen/world entry under Realmforge evidence capture.

## Why this wins the first slice

### TrinityCoreClassic 1.14.0.40618 family — BEAT / SELECT

Observed first-party repository evidence:

- the project README names Classic `1.14.0.40618` as its supported client,
- it accepts the modern world-auth path natively instead of inserting a modern-to-legacy packet translation proxy,
- `WorldSocket::HandleAuthSession` looks up the game account by the client's `RealmJoinTicket`,
- the lookup requires a 64-byte `session_key_bnet`,
- the world-auth HMAC is derived from that 64-byte key plus the client/server challenges and build-specific auth seed,
- after successful world authentication the server derives a 40-byte continued-session key and writes that back to `session_key_bnet`.

That lines up directly with Gate's existing 64-byte BGS session-key material. The remaining work is therefore an explicit Realmforge Bridge projection/handoff instead of inventing another protocol translation layer.

### VMangos + HermesProxy — REJECT for first adapter, retain as research lane

VMangos is a strong native Vanilla server, but its current README advertises original Vanilla-era clients such as 1.12.1 rather than modern Classic 1.14.x clients.

HermesProxy can bridge modern 1.14.0 traffic to a legacy 1.12.1 server, but it carries its own modern BNetServer, legacy auth client, modern world server, and legacy world client. That overlaps Realmforge Gate and places a whole second compatibility/proxy architecture between Realmforge and the emulator.

It remains valuable evidence for future legacy-core adapters, but it adds unnecessary moving parts to the first playable Realmforge vertical slice.

### Mainline emulator families without an exact 40618 path — DEFER

Any otherwise healthy emulator that does not expose an inspectable, reproducible 1.14.0.40618 world-auth path is deferred for the first slice. Realmforge is explicitly adapter-based, so this is not a permanent rejection.

## Exact world-auth facts captured for the selected family

For the observed TrinityCoreClassic line:

1. `account.username` is unique and limited to 32 characters.
2. `account.session_key_bnet` is `VARBINARY(64)`.
3. `HandleAuthSession` passes the client's `RealmJoinTicket` into the account lookup by `username`.
4. The world-auth lookup requires `LENGTH(session_key_bnet) = 64`.
5. The 64-byte key is hashed with the build-specific auth seed and challenge material to verify the client's digest.
6. After authentication, the world server derives a 40-byte continued-session key and writes it to the same `session_key_bnet` column.

Therefore Realmforge's current numeric account-id text is **not** a valid general first-adapter join ticket. Bridge must provide the emulator-specific username projection and must install the exact 64-byte client session key before Gate returns the join response.

## Consequences for Realmforge

The first Bridge contract needs semantic operations beyond lifecycle management:

```text
EnsureGameAccountProjection
PrepareWorldJoin
InvalidateWorldJoin
GetCharacterCounts        optional capability
GetLastPlayedCharacter    optional capability
```

`PrepareWorldJoin` must be adapter-specific internally but product-semantic externally. Gate must not learn TrinityCore table names or SQL columns.

The client-visible sequence becomes:

```text
Gate authenticates Realmforge identity/game account
        |
        v
Core/Bridge resolves emulator account projection
        |
        v
Bridge writes 64-byte world-auth key for that projection
        |
        v
Bridge returns opaque join ticket + endpoint
        |
        v
Gate emits Param_RealmJoinTicket / Param_BnetSessionKey
        |
        v
1.14.0.40618 client connects to world server
        |
        v
emulator validates CMSG_AUTH_SESSION
```

## Explicit unknowns before E2E completion

- exact reproducible Realmforge pin between the observed Frostshake line and later maintained forks,
- minimum safe fields needed when Realmforge creates/synchronizes the emulator-side account projection,
- character-count and last-character query strategy,
- disconnect/session invalidation behavior after failed or abandoned joins,
- reconnect behavior and the 64-byte -> 40-byte continued-session transition under a real client,
- whether any 40618 client behavior differs from inherited Gate assumptions before world connection.

Those unknowns are test work, not permission for implementation agents to invent policy.
