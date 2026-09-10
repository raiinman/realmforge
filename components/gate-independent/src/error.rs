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
    InvalidSessionId,
    SessionClosed,
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
            Self::InvalidSessionId => write!(f, "session id must not be empty"),
            Self::SessionClosed => write!(f, "session is already closed"),
            Self::WorldAuthUnavailable => write!(f, "world authentication is unavailable"),
        }
    }
}

impl std::error::Error for GateError {}
