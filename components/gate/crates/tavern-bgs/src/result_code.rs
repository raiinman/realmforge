// SPDX-License-Identifier: AGPL-3.0-only

//! BGS protocol result codes.
//!
//! Values from `bgs.protocol.Result` used in `DisconnectNotification.error_code`
//! and as response status codes. Sourced from the 1.13.2 binary RE at
//! `management/src/reverse-engineering/wow-classic/1.13.2/31650/bgs/tavern-interop-analysis.md` §4.7.

/// Silent disconnect — client returns to login screen without a dialog.
pub const ERROR_OK: u32 = 0;
/// Game account is banned.
pub const ERROR_GAME_ACCOUNT_BANNED: u32 = 52;
/// Game account has no remaining game time.
pub const ERROR_GAME_ACCOUNT_NO_TIME: u32 = 30;
/// Game account is suspended.
pub const ERROR_GAME_ACCOUNT_SUSPENDED: u32 = 33;
/// Another session logged in with the same account.
pub const ERROR_SESSION_DUPLICATE: u32 = 60;
/// Session was disconnected (general).
pub const ERROR_SESSION_DISCONNECTED: u32 = 61;
/// Kicked by a GM or admin.
pub const ERROR_ADMIN_KICK: u32 = 70;
/// Unplanned server maintenance.
pub const ERROR_UNPLANNED_MAINTENANCE: u32 = 71;
/// Planned server maintenance.
pub const ERROR_PLANNED_MAINTENANCE: u32 = 72;
/// Server is shutting down.
pub const ERROR_SERVER_SHUTTING_DOWN: u32 = 92;
/// Battle.net account is banned (not just a game account).
pub const ERROR_BATTLENET_ACCOUNT_BANNED: u32 = 96;
