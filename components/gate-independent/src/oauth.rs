use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::{GateError, IdentitySubject};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientId(String);

impl ClientId {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidOAuthClientId);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedirectUri(String);

impl RedirectUri {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(GateError::InvalidRedirectUri);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// RFC 7636 code verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceCodeVerifier(String);

impl PkceCodeVerifier {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        let len = value.len();
        let valid_charset = value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'));

        if !(43..=128).contains(&len) || !valid_charset {
            return Err(GateError::InvalidPkceVerifier);
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// RFC 7636 S256 code challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkceS256Challenge(String);

impl PkceS256Challenge {
    pub fn from_verifier(verifier: &PkceCodeVerifier) -> Self {
        let digest = Sha256::digest(verifier.as_str().as_bytes());
        Self(URL_SAFE_NO_PAD.encode(digest))
    }

    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        if value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err(GateError::InvalidPkceChallenge);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn verifies(&self, verifier: &PkceCodeVerifier) -> bool {
        Self::from_verifier(verifier) == *self
    }
}

/// OIDC-only state that must survive authorization-code issuance and redemption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcAuthorizationContext {
    pub nonce: Option<String>,
}

impl OidcAuthorizationContext {
    pub fn new(nonce: Option<String>) -> Self {
        Self { nonce }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedeemedAuthorizationGrant {
    pub subject: IdentitySubject,
    pub client_id: ClientId,
    pub oidc: Option<OidcAuthorizationContext>,
}

/// Protocol-neutral authorization-code state derived from OAuth 2.0 + PKCE.
///
/// The opaque authorization-code string itself belongs to a storage/transport
/// adapter. This type owns the security properties Gate must enforce when a
/// code is redeemed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationGrant {
    pub subject: IdentitySubject,
    pub client_id: ClientId,
    pub redirect_uri: RedirectUri,
    pub pkce_challenge: PkceS256Challenge,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    oidc: Option<OidcAuthorizationContext>,
    consumed: bool,
}

impl AuthorizationGrant {
    pub fn new(
        subject: IdentitySubject,
        client_id: ClientId,
        redirect_uri: RedirectUri,
        pkce_challenge: PkceS256Challenge,
        issued_at_unix: u64,
        expires_at_unix: u64,
    ) -> Result<Self, GateError> {
        if expires_at_unix <= issued_at_unix {
            return Err(GateError::InvalidAuthorizationGrantLifetime);
        }

        Ok(Self {
            subject,
            client_id,
            redirect_uri,
            pkce_challenge,
            issued_at_unix,
            expires_at_unix,
            oidc: None,
            consumed: false,
        })
    }

    pub fn with_oidc_context(mut self, context: OidcAuthorizationContext) -> Self {
        self.oidc = Some(context);
        self
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed
    }

    pub fn redeem(
        &mut self,
        now_unix: u64,
        client_id: &ClientId,
        redirect_uri: &RedirectUri,
        verifier: &PkceCodeVerifier,
    ) -> Result<IdentitySubject, GateError> {
        Ok(self
            .redeem_full(now_unix, client_id, redirect_uri, verifier)?
            .subject)
    }

    pub fn redeem_full(
        &mut self,
        now_unix: u64,
        client_id: &ClientId,
        redirect_uri: &RedirectUri,
        verifier: &PkceCodeVerifier,
    ) -> Result<RedeemedAuthorizationGrant, GateError> {
        if self.consumed {
            return Err(GateError::AuthorizationGrantConsumed);
        }
        if now_unix >= self.expires_at_unix {
            return Err(GateError::AuthorizationGrantExpired);
        }
        if client_id != &self.client_id {
            return Err(GateError::OAuthClientMismatch);
        }
        if redirect_uri != &self.redirect_uri {
            return Err(GateError::OAuthRedirectMismatch);
        }
        if !self.pkce_challenge.verifies(verifier) {
            return Err(GateError::PkceVerificationFailed);
        }

        self.consumed = true;
        Ok(RedeemedAuthorizationGrant {
            subject: self.subject.clone(),
            client_id: self.client_id.clone(),
            oidc: self.oidc.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 7636 Appendix B example vector.
    const RFC_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const RFC_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    fn grant() -> AuthorizationGrant {
        let verifier = PkceCodeVerifier::new(RFC_VERIFIER).unwrap();
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
    fn rfc_7636_s256_vector_matches() {
        let verifier = PkceCodeVerifier::new(RFC_VERIFIER).unwrap();
        let challenge = PkceS256Challenge::from_verifier(&verifier);
        assert_eq!(challenge.as_str(), RFC_CHALLENGE);
    }

    #[test]
    fn verifier_enforces_rfc_length_and_charset() {
        assert_eq!(
            PkceCodeVerifier::new("short").unwrap_err(),
            GateError::InvalidPkceVerifier
        );
        assert_eq!(
            PkceCodeVerifier::new("!".repeat(43)).unwrap_err(),
            GateError::InvalidPkceVerifier
        );
    }

    #[test]
    fn authorization_grant_is_single_use() {
        let verifier = PkceCodeVerifier::new(RFC_VERIFIER).unwrap();
        let client = ClientId::new("client-1").unwrap();
        let redirect = RedirectUri::new("https://client.example/callback").unwrap();
        let mut grant = grant();

        assert_eq!(
            grant
                .redeem(150, &client, &redirect, &verifier)
                .unwrap()
                .as_str(),
            "subject-1"
        );
        assert!(grant.is_consumed());
        assert_eq!(
            grant
                .redeem(151, &client, &redirect, &verifier)
                .unwrap_err(),
            GateError::AuthorizationGrantConsumed
        );
    }

    #[test]
    fn oidc_context_survives_successful_redemption() {
        let verifier = PkceCodeVerifier::new(RFC_VERIFIER).unwrap();
        let client = ClientId::new("client-1").unwrap();
        let redirect = RedirectUri::new("https://client.example/callback").unwrap();
        let mut grant =
            grant().with_oidc_context(OidcAuthorizationContext::new(Some("nonce-1".to_owned())));

        let redeemed = grant
            .redeem_full(150, &client, &redirect, &verifier)
            .unwrap();
        assert_eq!(redeemed.subject.as_str(), "subject-1");
        assert_eq!(redeemed.client_id.as_str(), "client-1");
        assert_eq!(redeemed.oidc.unwrap().nonce.as_deref(), Some("nonce-1"));
    }

    #[test]
    fn authorization_grant_rejects_expired_or_mismatched_redemptions() {
        let verifier = PkceCodeVerifier::new(RFC_VERIFIER).unwrap();
        let client = ClientId::new("client-1").unwrap();
        let redirect = RedirectUri::new("https://client.example/callback").unwrap();

        let mut expired = grant();
        assert_eq!(
            expired
                .redeem(200, &client, &redirect, &verifier)
                .unwrap_err(),
            GateError::AuthorizationGrantExpired
        );

        let mut wrong_client = grant();
        assert_eq!(
            wrong_client
                .redeem(
                    150,
                    &ClientId::new("other-client").unwrap(),
                    &redirect,
                    &verifier,
                )
                .unwrap_err(),
            GateError::OAuthClientMismatch
        );

        let mut wrong_redirect = grant();
        assert_eq!(
            wrong_redirect
                .redeem(
                    150,
                    &client,
                    &RedirectUri::new("https://evil.example/callback").unwrap(),
                    &verifier,
                )
                .unwrap_err(),
            GateError::OAuthRedirectMismatch
        );
    }

    #[test]
    fn authorization_grant_rejects_wrong_pkce_verifier() {
        let client = ClientId::new("client-1").unwrap();
        let redirect = RedirectUri::new("https://client.example/callback").unwrap();
        let wrong = PkceCodeVerifier::new("A".repeat(43)).unwrap();
        let mut grant = grant();

        assert_eq!(
            grant.redeem(150, &client, &redirect, &wrong).unwrap_err(),
            GateError::PkceVerificationFailed
        );
    }
}
