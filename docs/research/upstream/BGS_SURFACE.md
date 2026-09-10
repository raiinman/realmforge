# Upstream BGS Surface — Pinned Intake Map

**Upstream:** `wowemulation-dev/tavern`  
**Revision:** `6f9158670ee7666bfae2be58b291dafcb45f12e7`  
**Status:** code-level inventory only; every protocol claim still requires independent Realmforge evidence before becoming support authority.

BGS is the highest-value interim subsystem and the highest-risk future replacement.

---

# 1. Transport generations observed in upstream

The pinned source contains support concepts for:

- raw TCP BGS transport used by the 1.13.x line,
- HTTP/WebSocket BGS transport for later/desktop flows,
- protobuf RPC framing,
- v1 and v2 service hashes,
- protocol-generation-dependent auth/session routing,
- optional TLS on the raw BGS listener.

The documentation is internally inconsistent about exact v2 completion. Realmforge therefore keeps:

`BGS_V2_SUPPORT = CONFLICT`

until reproduced with target clients.

---

# 2. Service hash catalog in pinned source

These are **upstream constants**, useful for interim operation and research hypotheses. They do not become clean-room authority until independently correlated.

## BGS v1 / Classic 1.13.2 family

| Service descriptor | Upstream constant | Hash |
|---|---|---:|
| `bnet.protocol.connection.ConnectionService` | `CONNECTION_SERVICE_V1` | `0x65446991` |
| `bnet.protocol.authentication.AuthenticationServer` | `AUTHENTICATION_SERVER_V1` | `0x0DECFC01` |
| `bnet.protocol.authentication.AuthenticationClient` | `AUTHENTICATION_CLIENT_V1` | `0x71240E35` |
| `bnet.protocol.game_utilities.GameUtilities` | `GAME_UTILITIES_V1` | `0x3FC1274D` |
| `bnet.protocol.account.AccountService` | `ACCOUNT_SERVICE_V1` | `0x62DA0891` |
| `bnet.protocol.account.AccountNotify` | `ACCOUNT_NOTIFY_V1` | `0x54DFDA17` |
| `bnet.protocol.challenge.ChallengeNotify` | `CHALLENGE_NOTIFY_V1` | `0xBBDA171F` |
| `bnet.protocol.user_manager.UserManagerService` | `USER_MANAGER_SERVICE_V1` | `0x3E19268A` |
| `bnet.protocol.user_manager.UserManagerNotify` | `USER_MANAGER_NOTIFY_V1` | `0xBC872C22` |
| `bnet.protocol.friends.FriendsService` | `FRIENDS_SERVICE_V1` | `0xA3DDB1BD` |
| `bnet.protocol.friends.FriendsNotify` | `FRIENDS_NOTIFY_V1` | `0x6F259A13` |
| `bnet.protocol.presence.PresenceService` | `PRESENCE_SERVICE_V1` | `0xFA0796FF` |
| `bnet.protocol.presence.v1.PresenceListener` | `PRESENCE_LISTENER_V1` | `0x890AB85F` |
| `bnet.protocol.report.ReportService` | `REPORT_SERVICE_V1` | `0x7CAF61C9` |
| `bnet.protocol.resources.Resources` | `RESOURCES_V1` | `0xECBE75BA` |
| `bnet.protocol.club.v1.ClubMembershipService` | `CLUB_MEMBERSHIP_SERVICE_V1` | `0x94B94786` |
| `bnet.protocol.club.v1.ClubMembershipListener` | `CLUB_MEMBERSHIP_LISTENER_V1` | `0x2B34597B` |

## BGS v2 / desktop-era descriptors in upstream

| Service | Hash |
|---|---:|
| ConnectionService | `0x65446991` |
| AuthenticationService | `0x7F55071D` |
| AuthenticationListener | `0x55776BBA` |
| GameUtilitiesService | `0xA64D6CAC` |
| AccountService | `0x62DA0891` |
| AccountListener | `0x98F9BD46` |
| SessionService | `0x7E9859A3` |
| SessionListener | `0xBC27D188` |

## Client-v2-specific descriptors in upstream

| Service | Hash |
|---|---:|
| `authentication.v2.client.AuthenticationService` | `0xC02F8216` |
| `authentication.v2.client.AuthenticationListener` | `0x9DA8116B` |
| `session.v2.client.SessionService` | `0x7B37D770` |

The upstream implementation computes hashes with FNV-1a 32-bit over descriptor names. That algorithm itself is straightforward to independently validate; do not copy the lookup table as clean-room authority without confirmation from target descriptors/captures.

---

# 3. Method IDs represented in pinned source

## ConnectionService

| Method | ID | Interim status |
|---|---:|---|
| Connect | 1 | implemented upstream |
| Bind | 2 | upstream deliberately uses bindless approach |
| Echo | 3 | implemented |
| ForceDisconnect | 4 | implemented |
| KeepAlive | 5 | implemented |
| Encrypt | 6 | documented not handled |
| RequestDisconnect | 7 | documented not handled |

## Authentication

| Method | ID | Notes |
|---|---:|---|
| Logon | 1 | critical |
| GenerateSSOToken / GenerateAuthToken | 5 | **CONFLICT**: docs say stub, code dispatch exists |
| SelectGameAccount | 6 | described deprecated/not dispatched by 1.13.2 |
| VerifyWebCredentials / VerifyAuthToken | 7 | critical |
| GenerateWebCredentials | 8 | capture exact consumers |
| LogonUpdate | 10 | capture if target invokes |

## Authentication listener/push

| Method | ID | Notes |
|---|---:|---|
| OnLogonComplete | 5 | critical server push |
| OnLogonQueueUpdate | 12 | queue behavior |
| OnLogonQueueEnd | 13 | queue behavior |

Upstream comments also identify other listener IDs such as OnServerStateChange, OnMemModuleLoad, OnLogonUpdate, OnVersionInfoUpdated and OnGameAccountSelected; only the table above is directly represented as constants in `method_id.rs`.

## AccountService

| Method | ID |
|---|---:|
| ResolveAccount | 13 |
| Subscribe | 25 |
| Unsubscribe | 26 |
| GetAccountState | 30 |
| GetGameAccountState | 31 |
| GetLicenses | 32 |
| GetGameTimeRemainingInfo | 33 |
| GetGameSessionInfo | 34 |
| GetCAISInfo | 35 |
| GetAuthorizedData | 37 |
| GetSignedAccountState | 44 |

For Realmforge, each must get a consumer/build matrix. A method existing in the upstream table does not prove every client invokes it.

## SessionService

| Method | ID | Realmforge status |
|---|---:|---|
| CreateSession | 1 | CAPTURE |
| RestoreSession | 2 | **P0 CAPTURE; upstream TODO exists** |
| DestroySession | 3 | CAPTURE |

`MarkSessionAlive` is discussed elsewhere in upstream documentation as stub/deferred and is not present in the pinned `method_id.rs` session constant set. This needs direct client capture rather than guessing its ID/contract.

## GameUtilities

| Method | ID | Upstream state |
|---|---:|---|
| ProcessClientRequest | 1 | core realm flow implemented |
| GetPlayerVariables | 3 | documented no-op/unverified |
| GetAchievementsFile | 9 | documented no-op/unverified |
| RegisterUtilities | 11 | inventory/capture consumer |
| UnregisterUtilities | 12 | inventory/capture consumer |

---

# 4. GameUtilities realm command flow

Pinned source routes `ProcessClientRequest` by the first attribute whose name starts with `Command_`.

Recognized commands:

```text
Command_RealmListTicketRequest_v1
Command_RealmListRequest_v1
Command_LastCharPlayedRequest_v1
Command_RealmJoinRequest_v1
Command_CharacterListRequest_v1
```

Unknown commands currently receive an empty success response.

## RealmListTicketRequest

Observed upstream behavior:

Input candidates:

```text
Param_Identity
Param_ClientInfo
```

Output:

```text
Param_RealmListTicket
```

Pinned implementation uses literal ticket bytes `AuthRealmListTicket`.

**Realmforge disposition:** `CAPTURE-REQUIRED`.

Do not preserve the literal just because the client accepts it. Determine whether it has actual security/session semantics in each target family.

## RealmListRequest

Pinned implementation returns:

```text
Param_RealmList
Param_CharacterCountList
```

`Param_RealmList` is built as a zlib-compressed JSON payload with a type prefix:

```text
JSONRealmListUpdates:<json>
```

The upstream JSON includes fields such as:

```text
wowRealmAddress
cfgTimezonesID
populationState
cfgCategoriesID
version.versionMajor
version.versionMinor
version.versionRevision
version.versionBuild
cfgRealmsID
flags
name
cfgConfigsID
cfgLanguagesID
```

Character counts use:

```text
JSONRealmCharacterCountList:<json>
```

**Major Realmforge change required:** pinned implementation advertises one hard-coded realm. Realmforge must translate a dynamic canonical realm registry into generation-specific payloads.

## LastCharPlayedRequest

Pinned implementation returns empty success.

Realmforge must determine whether this is sufficient or whether real emulator character metadata improves/affects client behavior.

## CharacterListRequest

Pinned implementation returns empty success.

Same disposition as above.

## RealmJoinRequest

Input:

```text
Param_RealmAddress
```

Pinned output can contain:

```text
Param_ServerAddresses
Param_JoinSecret
Param_RealmJoinTicket
Param_BnetSessionKey
```

Pinned behavior:

- rejects missing/unknown hard-coded realm address,
- serializes configured realm IP/port into compressed `JSONRealmListServerIPAddresses:` JSON,
- creates a random 32-byte join secret,
- uses account ID text as the opaque realm-join ticket,
- includes BGS session key when present.

This is exactly where Realmforge must stop trusting upstream as architectural authority.

Required independent work:

1. Capture actual client expectations for each supported generation.
2. Trace the corresponding realm-server handshake.
3. Determine join-ticket security requirements.
4. Determine whether server secret must be persisted/registered with the realm.
5. Determine session-key derivation and consumption.
6. Replace the one-realm constant with Core's canonical registry.
7. Handle IPv4/IPv6 and potentially internal/external address selection.
8. Map realm offline/full/maintenance/build mismatch to correct response behavior.

---

# 5. Hard-coded realm assumptions to eliminate

Pinned `game_utilities.rs` contains:

```text
REALM_ADDRESS = 0x01010100
Realm name = "Tavern Realm"
site/category/config/language IDs fixed
character counts empty
```

Version behavior in the pinned source special-cases build `40618` as `1.14.0.40618`, while other builds are projected as `1.13.2.<client-build>` with a default `31650`.

This is useful proof-of-concept behavior, not acceptable Realmforge architecture.

Realmforge needs an explicit per-build compatibility profile instead of "anything else gets 1.13.2".

---

# 6. Result/error codes present upstream

Pinned source includes these BGS result-code constants:

| Meaning | Value |
|---|---:|
| OK/silent disconnect | 0 |
| no game time | 30 |
| game account suspended | 33 |
| game account banned | 52 |
| duplicate session | 60 |
| session disconnected | 61 |
| admin kick | 70 |
| unplanned maintenance | 71 |
| planned maintenance | 72 |
| server shutting down | 92 |
| Battle.net account banned | 96 |

**Realmforge disposition:** treat as hypotheses until independently corroborated from client behavior/binary/protocol authority. Error semantics matter because clients often present materially different UX for different codes.

---

# 7. Known missing or questionable BGS behavior

## P0

- exact v1/v2 build matrix,
- protocol-generation detection,
- GenerateAuthToken actual semantics,
- RestoreSession behavior,
- session keepalive/MarkSessionAlive,
- realm-side authentication half,
- realm join ticket/security contract,
- exact disconnect/error mapping,
- external browser ticket SSO flow.

## P1

- character counts from real realm DB/API,
- last-character metadata,
- queue behavior under real clients,
- account-state subscriptions,
- multi-game-account selection,
- region/site/category semantics,
- reconnect after realm failure.

## P2 / likely defer

- friends,
- presence,
- clubs,
- report services,
- unused resources services,
- optional 2FA challenge compatibility.

---

# 8. Independent replacement packet

Before a BGS service can be rebuilt without recourse to upstream source, its reconstruction card must contain:

```text
service descriptor
independently verified service hash or hash derivation
method ID
request protobuf/field map
response protobuf/field map
push/listener map
transport generation
required session state
state transition
timeout/retry behavior
error/status behavior
one success fixture
at least one failure fixture
supported build list
capture IDs
acceptance test
```

If any field is missing, the correct status is `CAPTURE-REQUIRED`, not "probably the same as Tavern."

---

# 9. Intake recommendation

**KEEP NOW:** the full functioning BGS core as an isolated interim Gate dependency, because recreating its framing/auth/session/realm machinery immediately would throw away the largest amount of useful work.

**DO NOT LET IT DEFINE:** Realmforge realm state, product identity, final database model, supported-build truth, or future API architecture.

**REPLACE LAST:** BGS transport/RPC should be one of the final inherited pieces we independently replace, after we already have our own capture corpus and black-box harness.
