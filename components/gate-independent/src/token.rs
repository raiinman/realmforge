use std::collections::BTreeMap;
use std::fmt;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};

use crate::{GateError, IdentitySubject};

#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken(String);

impl AccessToken {
    fn from_random_bytes(bytes: [u8; 32]) -> Self {
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn parse(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        if value.len() != 43
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(GateError::InvalidAccessToken);
        }
        Ok(Self(value))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessTokenRecord {
    pub subject: IdentitySubject,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
}

#[derive(Debug, Default)]
pub struct AccessTokenStore {
    records: BTreeMap<[u8; 32], AccessTokenRecord>,
}

impl AccessTokenStore {
    pub fn issue(
        &mut self,
        subject: IdentitySubject,
        issued_at_unix: u64,
        lifetime_seconds: u64,
    ) -> Result<AccessToken, GateError> {
        if lifetime_seconds == 0 {
            return Err(GateError::InvalidAccessTokenLifetime);
        }
        let expires_at_unix = issued_at_unix
            .checked_add(lifetime_seconds)
            .ok_or(GateError::InvalidAccessTokenLifetime)?;
        let record = AccessTokenRecord {
            subject,
            issued_at_unix,
            expires_at_unix,
        };

        loop {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            let token = AccessToken::from_random_bytes(bytes);
            let digest = token_digest(&token);
            if let std::collections::btree_map::Entry::Vacant(entry) = self.records.entry(digest) {
                entry.insert(record.clone());
                return Ok(token);
            }
        }
    }

    pub fn resolve(
        &self,
        token: &AccessToken,
        now_unix: u64,
    ) -> Result<&AccessTokenRecord, GateError> {
        let record = self
            .records
            .get(&token_digest(token))
            .ok_or(GateError::AccessTokenNotFound)?;
        if now_unix >= record.expires_at_unix {
            return Err(GateError::AccessTokenExpired);
        }
        Ok(record)
    }

    pub fn revoke(&mut self, token: &AccessToken) -> bool {
        self.records.remove(&token_digest(token)).is_some()
    }

    #[cfg(test)]
    fn issue_with_bytes(
        &mut self,
        bytes: [u8; 32],
        subject: IdentitySubject,
        issued_at_unix: u64,
        lifetime_seconds: u64,
    ) -> Result<AccessToken, GateError> {
        if lifetime_seconds == 0 {
            return Err(GateError::InvalidAccessTokenLifetime);
        }
        let expires_at_unix = issued_at_unix
            .checked_add(lifetime_seconds)
            .ok_or(GateError::InvalidAccessTokenLifetime)?;
        let token = AccessToken::from_random_bytes(bytes);
        let digest = token_digest(&token);
        if self
            .records
            .insert(
                digest,
                AccessTokenRecord {
                    subject,
                    issued_at_unix,
                    expires_at_unix,
                },
            )
            .is_some()
        {
            return Err(GateError::DuplicateAccessToken);
        }
        Ok(token)
    }
}

fn token_digest(token: &AccessToken) -> [u8; 32] {
    Sha256::digest(token.expose().as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_debug_is_redacted() {
        let mut store = AccessTokenStore::default();
        let token = store
            .issue_with_bytes(
                [9; 32],
                IdentitySubject::new("subject-1").unwrap(),
                100,
                60,
            )
            .unwrap();
        assert_eq!(format!("{token:?}"), "AccessToken([REDACTED])");
    }

    #[test]
    fn token_resolves_until_expiry_and_can_be_revoked() {
        let mut store = AccessTokenStore::default();
        let token = store
            .issue_with_bytes(
                [9; 32],
                IdentitySubject::new("subject-1").unwrap(),
                100,
                60,
            )
            .unwrap();

        assert_eq!(store.resolve(&token, 159).unwrap().subject.as_str(), "subject-1");
        assert_eq!(
            store.resolve(&token, 160).unwrap_err(),
            GateError::AccessTokenExpired
        );
        assert!(store.revoke(&token));
        assert_eq!(
            store.resolve(&token, 120).unwrap_err(),
            GateError::AccessTokenNotFound
        );
    }

    #[test]
    fn zero_or_overflowing_lifetime_is_rejected() {
        let mut store = AccessTokenStore::default();
        assert_eq!(
            store
                .issue(IdentitySubject::new("subject-1").unwrap(), 100, 0)
                .unwrap_err(),
            GateError::InvalidAccessTokenLifetime
        );
        assert_eq!(
            store
                .issue(IdentitySubject::new("subject-1").unwrap(), u64::MAX, 1)
                .unwrap_err(),
            GateError::InvalidAccessTokenLifetime
        );
    }

    #[test]
    fn parser_rejects_non_token_shapes() {
        assert_eq!(
            AccessToken::parse("short").unwrap_err(),
            GateError::InvalidAccessToken
        );
        assert_eq!(
            AccessToken::parse("!".repeat(43)).unwrap_err(),
            GateError::InvalidAccessToken
        );
    }
}
