// SPDX-License-Identifier: AGPL-3.0-only

//! FNV-1a 32-bit service hashing for BGS service routing.
//!
//! Contains pre-computed hashes for both BGS v1 (WoW Classic 1.13.2 game
//! client) and BGS v2 (Battle.net desktop app / Phoenix). Service hashes
//! are computed over the `descriptor_name` field in the proto service
//! definition, which can differ from the service block name.

/// Compute the FNV-1a 32-bit hash of a service descriptor name.
///
/// The hash matches the reference implementation used by the Battle.net
/// desktop app (FNV-1a, 32-bit, with FNV offset basis 0x811C9DC5 and
/// FNV prime 0x01000193).
pub fn fnv1a_32(data: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in data.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

// ---- BGS v1 (WoW Classic 1.13.2 game client) ----

/// `bnet.protocol.connection.ConnectionService` — same in v1 and v2.
pub const CONNECTION_SERVICE_V1: u32 = 0x65446991;

/// `bnet.protocol.authentication.AuthenticationServer` (not AuthenticationService).
/// The 1.13.2 game client dispatches to the "Server" descriptor name.
pub const AUTHENTICATION_SERVER_V1: u32 = 0x0DECFC01;

/// `bnet.protocol.authentication.AuthenticationClient` (not AuthenticationListener).
/// Server-to-client push service for OnLogonComplete.
pub const AUTHENTICATION_CLIENT_V1: u32 = 0x71240E35;

/// `bnet.protocol.game_utilities.GameUtilities` (not GameUtilitiesService).
pub const GAME_UTILITIES_V1: u32 = 0x3FC1274D;

/// `bnet.protocol.account.AccountService` — same in v1 and v2.
pub const ACCOUNT_SERVICE_V1: u32 = 0x62DA0891;

/// `bnet.protocol.account.AccountNotify` (not AccountListener).
pub const ACCOUNT_NOTIFY_V1: u32 = 0x54DFDA17;

/// `bnet.protocol.challenge.ChallengeNotify`.
pub const CHALLENGE_NOTIFY_V1: u32 = 0xBBDA171F;

/// `bnet.protocol.user_manager.UserManagerService`.
pub const USER_MANAGER_SERVICE_V1: u32 = 0x3E19268A;

/// `bnet.protocol.user_manager.UserManagerNotify`.
pub const USER_MANAGER_NOTIFY_V1: u32 = 0xBC872C22;

/// `bnet.protocol.friends.FriendsService`.
pub const FRIENDS_SERVICE_V1: u32 = 0xA3DDB1BD;

/// `bnet.protocol.friends.FriendsNotify`.
pub const FRIENDS_NOTIFY_V1: u32 = 0x6F259A13;

/// `bnet.protocol.presence.PresenceService`.
pub const PRESENCE_SERVICE_V1: u32 = 0xFA0796FF;

/// `bnet.protocol.presence.v1.PresenceListener`.
pub const PRESENCE_LISTENER_V1: u32 = 0x890AB85F;

/// `bnet.protocol.report.ReportService`.
pub const REPORT_SERVICE_V1: u32 = 0x7CAF61C9;

/// `bnet.protocol.resources.Resources`.
pub const RESOURCES_V1: u32 = 0xECBE75BA;

/// `bnet.protocol.club.v1.ClubMembershipService`.
pub const CLUB_MEMBERSHIP_SERVICE_V1: u32 = 0x94B94786;

/// `bnet.protocol.club.v1.ClubMembershipListener`.
pub const CLUB_MEMBERSHIP_LISTENER_V1: u32 = 0x2B34597B;

// ---- BGS v2 (Battle.net desktop app / Phoenix) ----

pub const CONNECTION_SERVICE: u32 = 0x65446991;
pub const AUTHENTICATION_SERVICE: u32 = 0x7F55071D;
pub const AUTHENTICATION_LISTENER: u32 = 0x55776BBA;
pub const GAME_UTILITIES_SERVICE: u32 = 0xA64D6CAC;
pub const ACCOUNT_SERVICE: u32 = 0x62DA0891;
pub const ACCOUNT_LISTENER: u32 = 0x98F9BD46;
pub const SESSION_SERVICE: u32 = 0x7E9859A3;
pub const SESSION_LISTENER: u32 = 0xBC27D188;

// ---- BGS v2 (1.14.0 / 2.5.1 game client) ----

/// `bnet.protocol.authentication.v2.client.AuthenticationService`
pub const AUTHENTICATION_SERVICE_V2: u32 = 0xC02F8216;
/// `bnet.protocol.authentication.v2.client.AuthenticationListener`
pub const AUTHENTICATION_LISTENER_V2: u32 = 0x9DA8116B;
/// `bnet.protocol.session.v2.client.SessionService`
pub const SESSION_SERVICE_V2: u32 = 0x7B37D770;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_service_hash_matches_spec() {
        assert_eq!(
            fnv1a_32("bnet.protocol.connection.ConnectionService"),
            0x65446991
        );
    }

    #[test]
    fn authentication_server_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.authentication.AuthenticationServer"),
            0x0DECFC01
        );
    }

    #[test]
    fn authentication_client_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.authentication.AuthenticationClient"),
            0x71240E35
        );
    }

    #[test]
    fn game_utilities_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.game_utilities.GameUtilities"),
            0x3FC1274D
        );
    }

    #[test]
    fn account_notify_v1_hash() {
        assert_eq!(fnv1a_32("bnet.protocol.account.AccountNotify"), 0x54DFDA17);
    }

    #[test]
    fn challenge_notify_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.challenge.ChallengeNotify"),
            0xBBDA171F
        );
    }

    #[test]
    fn authentication_service_v2_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.authentication.AuthenticationService"),
            0x7F55071D
        );
    }

    #[test]
    fn user_manager_notify_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.user_manager.UserManagerNotify"),
            0xBC872C22
        );
    }

    #[test]
    fn friends_service_v1_hash() {
        assert_eq!(fnv1a_32("bnet.protocol.friends.FriendsService"), 0xA3DDB1BD);
    }

    #[test]
    fn friends_notify_v1_hash() {
        assert_eq!(fnv1a_32("bnet.protocol.friends.FriendsNotify"), 0x6F259A13);
    }

    #[test]
    fn presence_service_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.presence.PresenceService"),
            0xFA0796FF
        );
    }

    #[test]
    fn presence_listener_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.presence.v1.PresenceListener"),
            0x890AB85F
        );
    }

    #[test]
    fn report_service_v1_hash() {
        assert_eq!(fnv1a_32("bnet.protocol.report.ReportService"), 0x7CAF61C9);
    }

    #[test]
    fn resources_v1_hash() {
        assert_eq!(fnv1a_32("bnet.protocol.resources.Resources"), 0xECBE75BA);
    }

    #[test]
    fn club_membership_service_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.club.v1.ClubMembershipService"),
            0x94B94786
        );
    }

    #[test]
    fn club_membership_listener_v1_hash() {
        assert_eq!(
            fnv1a_32("bnet.protocol.club.v1.ClubMembershipListener"),
            0x2B34597B
        );
    }

    #[test]
    fn known_fnv1a_vectors() {
        assert_eq!(fnv1a_32(""), 0x811c9dc5);
        assert_eq!(fnv1a_32("a"), 0xe40c292c);
        assert_eq!(fnv1a_32("foobar"), 0xbf9cf968);
    }
}
