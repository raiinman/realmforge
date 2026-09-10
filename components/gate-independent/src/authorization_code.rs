use std::collections::BTreeMap;
use std::fmt;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::{RngCore, rngs::OsRng};
use sha2::{Digest, Sha256};

use crate::{
    AuthorizationGrant, ClientId, GateError, IdentitySubject, PkceCodeVerifier, RedirectUri,
};

#[derive(Clone, PartialEq, Eq)]
pub struct AuthorizationCode(String);

impl AuthorizationCode {
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
            return Err(GateError::InvalidAuthorizationCode);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for AuthorizationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AuthorizationCode([REDACTED])")
    }
}

#[derive(Debug, Default)]
pub struct AuthorizationCodeStore {
    grants: BTreeMap<[u8; 32], AuthorizationGrant>,
}

impl AuthorizationCodeStore {
    pub fn issue(&mut self, grant: AuthorizationGrant) -> AuthorizationCode {
        loop {
            let mut bytes = [0u8; 32];
            OsRng.fill_bytes(&mut bytes);
            let code = AuthorizationCode::from_random_bytes(bytes);
            let digest = code_digest(&code);
            if let std::collections::btree_map::Entry::Vacant(entry) = self.grants.entry(digest) {
                entry.insert(grant);
                return code;
            }
        }
    }

    pub fn redeem(
        &mut self,
        code: &AuthorizationCode,
        now_unix: u64,
        client_id: &ClientId,
        redirect_uri: &RedirectUri,
        verifier: &PkceCodeVerifier,
    ) -> Result<IdentitySubject, GateError> {
        let digest = code_digest(code);
        let result = self
            .grants
            .get_mut(&digest)
            .ok_or(GateError::AuthorizationCodeNotFound)?
            .redeem(now_unix, client_id, redirect_uri, verifier);

        if result.is_ok() {
            self.grants.remove(&digest);
        }

        result
    }

    pub fn len(&self) -> usize {
        self.grants.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grants.is_empty()
    }

    #[cfg(test)]
    fn issue_with_bytes(
        &mut self,
        bytes: [u8; 32],
        grant: AuthorizationGrant,
    ) -> Result<AuthorizationCode, GateError> {
        let code = AuthorizationCode::from_random_bytes(bytes);
        let digest = code_digest(&code);
        if self.grants.insert(digest, grant).is_some() {
            return Err(GateError::DuplicateAuthorizationCode);
        }
        Ok(code)
    }
}

fn code_digest(code: &AuthorizationCode) -> [u8; 32] {
    Sha256::digest(code.as_str().as_bytes()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{IdentitySubject, PkceS256Challenge};

    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    fn grant() -> AuthorizationGrant {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        AuthorizationGrant::new(
            IdentitySubject::new("subject-1").unwrap(),
            ClientId::new("client-1").unwrap(),
            RedirectUri::new("https://client.example/callback").unwrap(),
            PkceS256Challenge::from_verifier(&verifier),
            100,
            200,
        )
        .unwrap()
    }

    #[test]
    fn issued_codes_are_not_exposed_by_debug() {
        let mut store = AuthorizationCodeStore::default();
        let code = store.issue_with_bytes([7; 32], grant()).unwrap();
        assert_eq!(format!("{code:?}"), "AuthorizationCode([REDACTED])");
        assert!(!code.as_str().is_empty());
    }

    #[test]
    fn parser_accepts_issued_shape_and_rejects_garbage() {
        let mut store = AuthorizationCodeStore::default();
        let code = store.issue_with_bytes([7; 32], grant()).unwrap();
        assert_eq!(AuthorizationCode::parse(code.as_str()).unwrap(), code);
        assert_eq!(
            AuthorizationCode::parse("short").unwrap_err(),
            GateError::InvalidAuthorizationCode
        );
        assert_eq!(
            AuthorizationCode::parse("!".repeat(43)).unwrap_err(),
            GateError::InvalidAuthorizationCode
        );
    }

    #[test]
    fn successful_redemption_removes_code() {
        let mut store = AuthorizationCodeStore::default();
        let code = store.issue_with_bytes([7; 32], grant()).unwrap();
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let client = ClientId::new("client-1").unwrap();
        let redirect = RedirectUri::new("https://client.example/callback").unwrap();

        assert_eq!(store.len(), 1);
        assert_eq!(
            store
                .redeem(&code, 150, &client, &redirect, &verifier)
                .unwrap()
                .as_str(),
            "subject-1"
        );
        assert!(store.is_empty());
        assert_eq!(
            store
                .redeem(&code, 151, &client, &redirect, &verifier)
                .unwrap_err(),
            GateError::AuthorizationCodeNotFound
        );
    }

    #[test]
    fn failed_pkce_does_not_consume_code() {
        let mut store = AuthorizationCodeStore::default();
        let code = store.issue_with_bytes([7; 32], grant()).unwrap();
        let wrong = PkceCodeVerifier::new("A".repeat(43)).unwrap();
        let client = ClientId::new("client-1").unwrap();
        let redirect = RedirectUri::new("https://client.example/callback").unwrap();

        assert_eq!(
            store
                .redeem(&code, 150, &client, &redirect, &wrong)
                .unwrap_err(),
            GateError::PkceVerificationFailed
        );
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn deterministic_test_insertion_detects_digest_collision() {
        let mut store = AuthorizationCodeStore::default();
        store.issue_with_bytes([7; 32], grant()).unwrap();
        assert_eq!(
            store.issue_with_bytes([7; 32], grant()).unwrap_err(),
            GateError::DuplicateAuthorizationCode
        );
    }
}
