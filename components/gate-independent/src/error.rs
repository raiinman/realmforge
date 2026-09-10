use std::fmt;

use crate::{ClientBuild, RealmId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateError {
    InvalidRealmId,
    InvalidDisplayName,
    InvalidHost,
    InvalidPort,
    DuplicateRealmId,
    NoCompatibleBuilds(RealmId),
    RealmNotFound,
    BuildNotSupported { realm: RealmId, build: ClientBuild },
    InvalidIdentitySubject,
    InvalidGameAccountId,
    InvalidAccountId,
    DuplicateAccountId,
    DuplicateIdentitySubject,
    AccountNotFound,
    AccountLocked,
    AccountDisabled,
    InvalidSessionId,
    DuplicateSessionId,
    SessionNotFound,
    SessionAlreadyAuthenticated,
    SessionClosed,
    InvalidOAuthClientId,
    OAuthClientNotFound,
    DuplicateOAuthClientId,
    OAuthClientHasNoRedirectUris,
    InvalidRedirectUri,
    RedirectUriNotRegistered,
    InvalidPkceVerifier,
    InvalidPkceChallenge,
    InvalidAuthorizationGrantLifetime,
    AuthorizationGrantConsumed,
    AuthorizationGrantExpired,
    AuthorizationCodeNotFound,
    DuplicateAuthorizationCode,
    OAuthClientMismatch,
    OAuthRedirectMismatch,
    PkceVerificationFailed,
    InvalidIssuer,
    WorldAuthUnavailable,
}

impl fmt::Display for GateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRealmId => write!(f, "realm id must not be empty"),
            Self::InvalidDisplayName => write!(f, "realm display name must not be empty"),
            Self::InvalidHost => write!(f, "realm host must not be empty"),
            Self::InvalidPort => write!(f, "realm port must be non-zero"),
            Self::DuplicateRealmId => write!(f, "realm ids must be unique"),
            Self::NoCompatibleBuilds(id) => {
                write!(f, "realm {} has no compatible client builds", id.as_str())
            }
            Self::RealmNotFound => write!(f, "realm not found"),
            Self::BuildNotSupported { realm, build } => write!(
                f,
                "client build {} is not supported by realm {}",
                build.0,
                realm.as_str()
            ),
            Self::InvalidIdentitySubject => write!(f, "identity subject must not be empty"),
            Self::InvalidGameAccountId => write!(f, "game account id must not be empty"),
            Self::InvalidAccountId => write!(f, "account id must not be empty"),
            Self::DuplicateAccountId => write!(f, "account ids must be unique"),
            Self::DuplicateIdentitySubject => write!(f, "identity subjects must be unique"),
            Self::AccountNotFound => write!(f, "account not found"),
            Self::AccountLocked => write!(f, "account is locked"),
            Self::AccountDisabled => write!(f, "account is disabled"),
            Self::InvalidSessionId => write!(f, "session id must not be empty"),
            Self::DuplicateSessionId => write!(f, "session id already exists"),
            Self::SessionNotFound => write!(f, "session not found"),
            Self::SessionAlreadyAuthenticated => {
                write!(f, "session is already authenticated as another identity")
            }
            Self::SessionClosed => write!(f, "session is already closed"),
            Self::InvalidOAuthClientId => write!(f, "OAuth client id must not be empty"),
            Self::OAuthClientNotFound => write!(f, "OAuth client not found"),
            Self::DuplicateOAuthClientId => write!(f, "OAuth client id already exists"),
            Self::OAuthClientHasNoRedirectUris => {
                write!(f, "OAuth client must register at least one redirect URI")
            }
            Self::InvalidRedirectUri => write!(f, "redirect URI must not be empty"),
            Self::RedirectUriNotRegistered => write!(f, "redirect URI is not registered"),
            Self::InvalidPkceVerifier => write!(f, "PKCE verifier is invalid"),
            Self::InvalidPkceChallenge => write!(f, "PKCE challenge is invalid"),
            Self::InvalidAuthorizationGrantLifetime => {
                write!(f, "authorization grant expiry must be after issuance")
            }
            Self::AuthorizationGrantConsumed => write!(f, "authorization grant was already used"),
            Self::AuthorizationGrantExpired => write!(f, "authorization grant expired"),
            Self::AuthorizationCodeNotFound => write!(f, "authorization code not found"),
            Self::DuplicateAuthorizationCode => write!(f, "authorization code collision"),
            Self::OAuthClientMismatch => {
                write!(f, "OAuth client does not match authorization grant")
            }
            Self::OAuthRedirectMismatch => {
                write!(f, "redirect URI does not match authorization grant")
            }
            Self::PkceVerificationFailed => write!(f, "PKCE verification failed"),
            Self::InvalidIssuer => write!(f, "OIDC issuer must not be empty"),
            Self::WorldAuthUnavailable => write!(f, "world authentication is unavailable"),
        }
    }
}

impl std::error::Error for GateError {}
