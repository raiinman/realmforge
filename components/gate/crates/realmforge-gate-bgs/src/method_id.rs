// SPDX-License-Identifier: AGPL-3.0-only

//! BGS protocol method IDs.
//!
//! Per-service constants for the `method_id` field in the RPC header.
//! Sourced from the reverse-engineering routing table at
//! `management/src/reverse-engineering/`.

/// ConnectionService (0x65446991) — v1 and v2 identical.
pub mod connection {
    pub const CONNECT: u32 = 1;
    pub const BIND: u32 = 2;
    pub const ECHO: u32 = 3;
    pub const FORCE_DISCONNECT: u32 = 4;
    pub const KEEP_ALIVE: u32 = 5;
    pub const ENCRYPT: u32 = 6;
    pub const REQUEST_DISCONNECT: u32 = 7;
}

/// AuthenticationServer (v1: 0x0DECFC01, v2: 0xC02F8216).
pub mod authentication {
    pub const LOGON: u32 = 1;
    /// v1: GenerateSSOToken, v2: GenerateAuthToken.
    pub const GENERATE_TOKEN: u32 = 5;
    /// v1: VerifyWebCredentials, v2: VerifyAuthToken.
    pub const VERIFY_CREDENTIALS: u32 = 7;
    pub const GENERATE_WEB_CREDENTIALS: u32 = 8;
    /// v1: SelectGameAccount (method 6, deprecated) — the 1.13.2 client
    /// never dispatches it (game-account selection is local; the realm
    /// join carries the account). Kept for proto accuracy.
    pub const SELECT_GAME_ACCOUNT: u32 = 6;
    pub const LOGON_UPDATE: u32 = 10;
}

/// AuthenticationListener (v1: 0x71240E35, v2: 0x9DA8116B) — server pushes.
/// Canonical v1 ids from protobuf-decompiler 1.13.2.31650
/// authentication_service.proto AuthenticationListener: OnServerStateChange=4,
/// OnLogonComplete=5, OnMemModuleLoad=6, OnLogonUpdate=10,
/// OnVersionInfoUpdated=11, OnLogonQueueUpdate=12, OnLogonQueueEnd=13,
/// OnGameAccountSelected=14.
pub mod authentication_listener {
    pub const ON_LOGON_COMPLETE: u32 = 5;
    pub const ON_LOGON_QUEUE_UPDATE: u32 = 12;
    pub const ON_LOGON_QUEUE_END: u32 = 13;
}

/// AccountService (0x62DA0891) — v1 and v2 identical.
pub mod account {
    pub const RESOLVE_ACCOUNT: u32 = 13;
    pub const SUBSCRIBE: u32 = 25;
    pub const UNSUBSCRIBE: u32 = 26;
    pub const GET_ACCOUNT_STATE: u32 = 30;
    pub const GET_GAME_ACCOUNT_STATE: u32 = 31;
    pub const GET_LICENSES: u32 = 32;
    pub const GET_GAME_TIME_REMAINING_INFO: u32 = 33;
    pub const GET_GAME_SESSION_INFO: u32 = 34;
    pub const GET_CAIS_INFO: u32 = 35;
    pub const GET_AUTHORIZED_DATA: u32 = 37;
    pub const GET_SIGNED_ACCOUNT_STATE: u32 = 44;
}

/// SessionService (v2: 0x7B37D770) — desktop app only.
pub mod session {
    pub const CREATE_SESSION: u32 = 1;
    pub const RESTORE_SESSION: u32 = 2;
    pub const DESTROY_SESSION: u32 = 3;
}

/// GameUtilities (v1: 0x3FC1274D, v2 hash computed at runtime) — bidir.
pub mod game_utilities {
    pub const PROCESS_CLIENT_REQUEST: u32 = 1;
    pub const GET_PLAYER_VARIABLES: u32 = 3;
    pub const GET_ACHIEVEMENTS_FILE: u32 = 9;
    pub const REGISTER_UTILITIES: u32 = 11;
    pub const UNREGISTER_UTILITIES: u32 = 12;
}
