# Bridge target: TrinityCoreClassic 1.14.0.40618

**Status:** first-adapter authority / implementation contract in progress  
**Adapter family:** TrinityCoreClassic modern Classic  
**First client build:** `1.14.0.40618`  
**Observed reference line:** `Frostshake/TrinityCoreClassic` `vanilla_classic` at `1fc8f1ebe3d09fcb67194b99d0281f88188b4a0e`

## Boundary

Realmforge does not adopt the emulator auth database as product state.

Core owns the Realmforge user/game-account/realm identities. Bridge owns the projection necessary for this emulator family. Gate receives only the semantic output needed to construct the retired-client wire response.

No Gate module may issue TrinityCore SQL directly.

## Required projection

For a Realmforge game account that is allowed to enter a realm, the adapter must be able to resolve an emulator-side game-account projection with at least:

```text
Realmforge game account id
        -> emulator account id
        -> emulator account username
```

The emulator username is adapter data. It is not the Realmforge login name and must not become the canonical Realmforge credential.

The selected reference schema constrains `account.username` to 32 characters and makes it unique. Adapter-generated names therefore need a deterministic collision-safe strategy that does not expose secrets.

## PrepareWorldJoin semantics

Input from the Realmforge side:

```text
realmId
realmforgeGameAccountId
gateSessionId
clientBuild
clientPlatform (when known)
clientSessionKey (exactly 64 bytes for the 40618 path)
requestedAt
```

Bridge must:

1. resolve or create the emulator account projection,
2. verify that the projection belongs to the requested Realmforge game account and realm adapter,
3. install the 64-byte client/BGS session key into the emulator's transient world-auth field,
4. resolve the world endpoint that belongs to the selected realm runtime,
5. return the emulator lookup token as an opaque `joinTicket`,
6. return no emulator password or verifier material to Gate.

For the observed TrinityCoreClassic 40618 family, the concrete projection is:

```text
joinTicket -> account.username
clientSessionKey[64] -> account.session_key_bnet
```

The world server then reads `RealmJoinTicket`, selects the account by `username`, requires a 64-byte `session_key_bnet`, and validates the client's auth digest using that key and build-specific seed material.

## Important two-phase session-key behavior

The observed implementation uses `session_key_bnet` for two different phases:

```text
Before world auth:
    64-byte BGS/client session key

After successful world auth:
    40-byte continued-session key derived by the world server
```

This is intentional in the observed code path. Realmforge must not continuously overwrite the column with the original 64-byte key after a successful join, or it could destroy the emulator's continued-session state.

Consequently `PrepareWorldJoin` is a point-in-time handoff, not a continuously reconciled field.

## Projection creation policy

The emulator account schema also contains password/SRP fields. Realmforge must **not** copy the user's Realmforge password into them.

When Bridge must create an emulator-side account solely as a projection, credential material that is not used by the Realmforge Gate path should be adapter-generated and non-user-derived. The exact creation transaction remains implementation work until tested against a fresh reference auth database.

Where an existing emulator account is linked instead of created, Bridge must record that linkage explicitly and must not silently claim ownership of unrelated credentials.

## Failure behavior

`PrepareWorldJoin` must fail closed if any of these are true:

- realm has no selected/healthy adapter runtime,
- client build is not explicitly allowed for the realm,
- Realmforge game-account projection cannot be resolved safely,
- client session key is not the expected length for the selected compatibility profile,
- emulator auth storage cannot be updated atomically enough to make the join safe,
- world endpoint is missing or invalid.

On failure, Gate should return a typed join failure and must not fabricate a ticket.

## Invalidation

`InvalidateWorldJoin` is required even if the first adapter implementation can only do best-effort cleanup.

It is used for:

- Gate session disconnect before world auth,
- explicit kick/revocation,
- failed handoff where stale pre-auth material should not remain valid,
- future replay/TTL enforcement.

For this adapter, invalidation must be designed around the emulator's 64-byte pre-auth versus 40-byte post-auth state. A cleanup operation must not erase a successfully transitioned world session merely because the original Gate session ended.

## Character metadata capabilities

The adapter should later expose:

```text
GetCharacterCounts
GetLastPlayedCharacter
```

These are projections into Gate's realm-list response and must not be implemented as arbitrary cross-schema queries inside Gate.

They are not required to prove the first world-auth handshake, but they are required before Realmforge calls the surrounding realm-list experience complete.

## First vertical-slice proof

The adapter is not considered working until one reproducible fixture demonstrates:

```text
known Realmforge test identity
 -> Gate authentication
 -> Realmforge realm list
 -> Bridge PrepareWorldJoin
 -> client receives the same 64-byte key installed by Bridge
 -> client connects to the selected 40618 world server
 -> CMSG_AUTH_SESSION succeeds
 -> character screen or world entry succeeds
```

Capture the exact Gate request/response trace, adapter preparation evidence, emulator log evidence, client build identity, and cleanup/reconnect behavior.

## Not settled by this document

- Core <-> Gate transport/versioning (P-011),
- project-wide implementation language (P-001),
- Realmforge-wide database choice,
- a permanent fork pin,
- support for 31650 or any build other than the explicitly tested 40618 fixture,
- project-wide license.
