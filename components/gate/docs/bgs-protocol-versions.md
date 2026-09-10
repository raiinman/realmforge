# BGS Protocol Version Support

> **Status:** Research. v1 (1.13.2) is implemented. v2 (1.14.0 / 2.5.1) is
> planned. The dispatch mechanism is the same; only hash values and some
> message fields differ.

Both protocol versions use protobuf framing with a `Header` containing
`service_hash` + `method_id`. The differences are:

## Service Hashes

| Service | v1 (1.13.2) | v2 (1.14.0 / 2.5.1) |
|---|---|---|
| ConnectionService | `0x65446991` | `0x65446991` (same) |
| AuthenticationService | `0x0DECFC01` | `0xc02f8216` |
| AuthenticationListener | `0x71240E35` | `0x9da8116b` |
| SessionService | `0x7E9859A3` | `0x7b37d770` |
| SessionListener | — | `0xbca72135` |
| GameUtilitiesService | `0x3FC1274D` | TBD |

## Auth Command IDs

Source: RE routing table from `bna/2.50.6/16125/bgs-service-routing.md`.

### AuthenticationService (client → server)

| Function | v1 ID | v2 ID | v2 Routing Key |
|---|---|---|---|
| Logon | 1 | 1 | `0xC02F8216C0000001` |
| VerifyWebCredentials / VerifyAuthToken | 7 | 2 | `0xC02F8216C0000002` |
| GenerateSSOToken / GenerateAuthToken | 5 | 3 | `0xC02F8216C0000003` |

### AuthenticationListener (server → client)

| Function | v1 ID | v2 ID | v2 Routing Key |
|---|---|---|---|
| OnLogonComplete | 5 | 1 | `0x9DA8116BC0000001` |
| OnLogonQueueUpdate | 12 | 2 | `0x9DA8116BC0000002` |
| OnLogonQueueEnd | 13 | 3 | `0x9DA8116BC0000003` |
| OnExternalChallenge | — | 4 | `0x9DA8116BC0000004` |

### SessionService (client → server)

| Function | v1 ID | v2 ID | v2 Routing Key |
|---|---|---|---|
| CreateSession | 1 | 1 | `0x7B37D77040000001` |
| RestoreSession | 2 | 2 | `0x7B37D77040000002` |
| DestroySession | 3 | 3 | `0x7B37D77040000003` |

## New in v2

- **`LogonRequest3`** adds `Compatibility` (u64) field.
- **`LogonResponse3`** replaces `OnLogonComplete`. Adds module data, ping
  timeout, regulator rules, account/game account metadata, logon failure
  count, and RAF data.
- **`SingleSignOnRequest3`** takes raw `SsoId` bytes and `Compatibility`,
  replacing `SessionId.instance_id` hex string.
- **`ResumeRequest`** (method 1) and **`ResumeResponse`** (method 1) for
  session resume with account name + game account region.

## Protocol Detection

The first frame (`ConnectRequest`) has the same `service_hash` in both
versions. Detection happens on the second frame (`LogonRequest` or equivalent):

- If `service_hash == 0x0DECFC01` → v1
- If `service_hash == 0xc02f8216` → v2

Set `BgsSession.protocol_version` from that point. All subsequent dispatch
and response messages use version-specific method IDs and message types.

## Implementation Plan

1. Add v2 hash constants to `service_hash.rs`.
2. Add `protocol_version: Option<u8>` to `BgsSession`.
3. Add v2 method ID constants.
4. In `dispatch_frame`, detect version on first auth frame.
5. Add v2 handler branches that call version-specific functions.
6. Add v2 proto messages (`LogonRequest3`, `LogonResponse3`, `ResumeRequest`,
   `ResumeResponse`).
7. Wire v2 response building (regulator rules, account metadata, etc.).
8. Update `handle_restore_session` to accept both v1 (instance_id hex) and
   v2 (raw SsoId bytes) formats.

## Source

- WowPacketParser `BattleNet.V37165/Parsers/Authentication.cs`
- WowPacketParser `Enums/Battlenet/Command.cs`
- Tavern RE docs `tavern-interop-analysis.md`
- Tavern `service_hash.rs` (v1 hashes)

## Channel Detection

WowPacketParser uses a `BattlenetChannel` enum (byte) mapped from
`service_hash`. Both v1 and v2 embed the channel in the same Header
proto field. Detection works by reading `service_hash` on the first
non-ConnectionService frame and matching against known v1/v2 hashes.

## BGS service surface — implementation status (2026-08-01)

The 1.13.2 client's full BGS surface (verified against the client
binary, `aurora-rpc-catalog.md`) and tavern's status. Double-checked
2026-08-01 against the `bin/bgs-server` source.

| Service (v1 hash) | Direction | Tavern status |
|---|---|---|
| ConnectionService `0x65446991` | bidir | Connect (1) done; Echo (3) done (echo back); KeepAlive (5) done (success response); Encrypt (6), RequestDisconnect (7) not handled; Bind (2) skipped via `use_bindless_rpc`; ForceDisconnect (4) done (shutdown broadcast) |
| AuthenticationServer `0x0DECFC01` | C->S | Logon, SSO, VerifyWebCredentials, GenerateWebCredentials, SelectGameAccount done; LogonUpdate (10) no-op |
| AuthenticationClient `0x71240E35` | S->C | OnLogonComplete + OnLogonQueueUpdate (12) / OnLogonQueueEnd (13) done |
| GameUtilities `0x3FC1274D` | bidir | ProcessClientRequest (1) with the 5 realm commands done; GetPlayerVariables (3) / GetAchievementsFile (9) no-op stubs; Register/UnregisterUtilities (11/12) done |
| AccountService `0x62DA0891` | C->S | All methods return valid protobuf responses (ResolveAccount, GetAccountState, GetGameAccountState, GetLicenses, etc.) |
| AccountNotify `0x54DFDA17` | S->C | dispatched (logged) — server pushes OnAccountStateUpdated when subscribed |
| ChallengeNotify `0xBBDA171F` | S->C | logged only |
| UserManagerService `0x3E19268A` | C->S | logged "known v1 (not implemented)" |
| UserManagerNotify `0xBC872C22` | S->C | logged "known v1 (not implemented)" |
| FriendsService `0xA3DDB1BD` | C->S | logged "known v1 (not implemented)" |
| FriendsNotify `0x6F259A13` | S->C | logged "known v1 (not implemented)" |
| PresenceService `0xFA0796FF` | C->S | logged "known v1 (not implemented)" |
| PresenceListener `0x890AB85F` | S->C | logged "known v1 (not implemented)" |
| ReportService `0x7CAF61C9` | C->S | logged "known v1 (not implemented)" |
| Resources `0xECBE75BA` | C->S | logged "known v1 (not implemented)" |
| ClubMembershipService `0x94B94786` | C->S | logged "known v1 (not implemented)" |
| ClubMembershipListener `0x2B34597B` | S->C | logged only |
| SessionService `0x7E9859A3` | C->S | done (v2 only) |

**Priorities for a working login -> realm-list flow:**

1. ~~AccountService: return valid protobuf responses (Subscribe,
   GetAccountState with the game account, GetLicenses) and dispatch
   AccountNotify so `OnAccountStateUpdated` pushes reach the client.~~ ✓ done
2. ~~ConnectionService: respond to KeepAlive (client sends it
   periodically, verified) and Echo.~~ ✓ done
3. GameUtilities: decide whether 1.13.2 invokes GetPlayerVariables /
   GetAchievementsFile (the client has the stubs; invocation in the
   login flow is unverified).

The boundary: `Command_RealmJoinRequest_v1` returns the realm address
and join secret; the client then connects to the game server
(tavern-game, not yet implemented).
