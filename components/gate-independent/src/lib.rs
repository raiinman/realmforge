//! Fresh Realmforge Gate foundation.
//!
//! This crate is intentionally source-independent from the covered interim
//! Gate implementation. It defines Realmforge-owned semantic contracts first;
//! protocol adapters are added only after behavior is captured independently.

mod account;
mod authorization_code;
mod error;
mod http;
mod identity;
mod oauth;
mod oauth_client;
mod oauth_service;
mod oidc;
mod realm;
mod session;
mod signing;
mod token;
mod world_auth;

pub use account::{
    AccountDirectory, AccountId, AccountRecord, AccountStatus, MemoryAccountDirectory,
};
pub use authorization_code::{AuthorizationCode, AuthorizationCodeStore};
pub use error::GateError;
pub use http::{
    AccessTokenResponse, GATE_SESSION_COOKIE, GateHttpState, HealthResponse, gate_http_router,
    gate_http_router_with_state,
};
pub use identity::{GameAccountId, GameAccountProjection, IdentitySubject};
pub use oauth::{AuthorizationGrant, ClientId, PkceCodeVerifier, PkceS256Challenge, RedirectUri};
pub use oauth_client::{OAuthClient, OAuthClientRegistry};
pub use oauth_service::{
    AuthorizationRequest, AuthorizationResult, OAuthService, TokenExchangeRequest, TokenResponse,
};
pub use oidc::{Issuer, ProviderMetadata};
pub use realm::{ClientBuild, RealmCatalog, RealmDescriptor, RealmEndpoint, RealmId};
pub use session::{GateSession, SessionId, SessionRegistry, SessionState};
pub use signing::{JsonWebKey, JsonWebKeySet, OidcSigningAuthority};
pub use token::{AccessToken, AccessTokenRecord, AccessTokenStore};
pub use world_auth::{WorldAuthBridge, WorldAuthGrant, WorldAuthRequest};
