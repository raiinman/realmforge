// SPDX-License-Identifier: AGPL-3.0-only

//! Domain entities shared across Tavern services.
//!
//! Plain data structs grounded in the persistence schema (see
//! `docs/architecture.md` -> Data Model). They carry no behavior and perform no
//! I/O. Field types map to the SQL columns they will be loaded from: numeric
//! ids as `i64`, smallints as `i16`, bytea as `Vec<u8>`, timestamps as
//! `DateTime<Utc>`, and the session id as `Uuid`.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// A Battle.net account. The numeric `id` is the OAuth `sub` claim.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: i64,
    pub email: String,
    pub email_verified: bool,
    pub battletag: String,
    /// ISO 3166-1 alpha-3 country code.
    pub country_code: String,
    /// ISO 3166-1 numeric country id.
    pub country_id: Option<i32>,
    pub region: i16,
    /// Client locale, for example `enUS`.
    pub locale: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    /// ISO date (`YYYY-MM-DD`).
    pub birth_date: Option<String>,
    pub mobile_number: Option<String>,
    pub street1: Option<String>,
    pub street2: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Credential material for an account.
///
/// Stores both login schemes so any client line can authenticate against the
/// same account: the SRP6a verifier (2.5+/3.4/4.4 lines) and the plaintext
/// `sha_pass_hash` (1.13/1.14 Vanilla). Neither field stores a reversible
/// password.
#[derive(Debug, Clone)]
pub struct Credential {
    pub account_id: i64,
    pub srp_salt: Vec<u8>,
    pub srp_verifier: Vec<u8>,
    pub srp_iterations: i32,
    pub srp_version: i16,
    /// Plaintext-line credential hash: `SHA256(hex(SHA256(name)) + ":" + UPPER(password))`.
    /// Used by 1.13/1.14 Vanilla clients.
    pub sha_pass_hash: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// A registered OAuth client.
///
/// Seeded with the client ids observed in Battle.net traffic. The secret hash
/// is `None` for public clients.
#[derive(Debug, Clone)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret_hash: Option<String>,
    pub redirect_uris: Vec<String>,
    pub scopes: Vec<String>,
    pub allowed_grants: Vec<String>,
    pub require_2fa: bool,
}

/// A short-lived, single-use authorization code minted at the authorize
/// endpoint.
#[derive(Debug, Clone)]
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub account_id: i64,
    pub scope: String,
    pub redirect_uri: String,
    pub code_challenge: Option<String>,
    pub nonce: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

/// An opaque, rotatable, revocable refresh token.
#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub token: String,
    pub account_id: i64,
    pub client_id: String,
    pub scope: String,
    pub expires_at: DateTime<Utc>,
    pub rotated_from: Option<String>,
}

/// A web session backing the `SESSIONID` cookie.
#[derive(Debug, Clone)]
pub struct Session {
    pub session_id: Uuid,
    pub account_id: i64,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

/// A one-time service ticket bridging SRP login into the OAuth authorize flow.
///
/// After web login succeeds the account service issues an `ST`; the OAuth
/// authorize endpoint consumes it once to mint an authorization code.
#[derive(Debug, Clone)]
pub struct ServiceTicket {
    pub st: String,
    pub account_id: i64,
    pub region: i16,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}
