//! Fresh Realmforge Gate foundation.
//!
//! This crate is intentionally source-independent from the covered interim
//! Gate implementation. It defines Realmforge-owned semantic contracts first;
//! protocol adapters are added only after behavior is captured independently.

mod account;
mod error;
mod identity;
mod oauth;
mod realm;
mod session;
mod world_auth;

pub use account::{AccountDirectory, AccountId, AccountRecord, AccountStatus, MemoryAccountDirectory};
pub use error::GateError;
pub use identity::{GameAccountId, GameAccountProjection, IdentitySubject};
pub use oauth::{
    AuthorizationGrant, ClientId, PkceCodeVerifier, PkceS256Challenge, RedirectUri,
};
pub use realm::{ClientBuild, RealmCatalog, RealmDescriptor, RealmEndpoint, RealmId};
pub use session::{GateSession, SessionId, SessionRegistry, SessionState};
pub use world_auth::{WorldAuthBridge, WorldAuthGrant, WorldAuthRequest};
