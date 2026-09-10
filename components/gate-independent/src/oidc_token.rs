use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Serialize;

use crate::{ClientId, GateError, IdentitySubject, Issuer, OidcSigningAuthority};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IdTokenHeader<'a> {
    alg: &'static str,
    typ: &'static str,
    kid: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct IdTokenClaims<'a> {
    iss: &'a str,
    sub: &'a str,
    aud: &'a str,
    iat: u64,
    exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    nonce: Option<&'a str>,
}

/// Standards-derived OIDC ID-token issuer for Realmforge Gate.
///
/// This type owns only the compact-JWS token construction. Authentication,
/// authorization-code redemption, and client registration remain separate
/// responsibilities so protocol-specific login work cannot leak into OIDC.
#[derive(Debug, Clone)]
pub struct IdTokenIssuer {
    issuer: Issuer,
    signing: OidcSigningAuthority,
    lifetime_seconds: u64,
}

impl IdTokenIssuer {
    pub fn new(
        issuer: Issuer,
        signing: OidcSigningAuthority,
        lifetime_seconds: u64,
    ) -> Result<Self, GateError> {
        if lifetime_seconds == 0 {
            return Err(GateError::InvalidIdTokenLifetime);
        }
        Ok(Self {
            issuer,
            signing,
            lifetime_seconds,
        })
    }

    pub fn issue(
        &self,
        subject: &IdentitySubject,
        audience: &ClientId,
        now_unix: u64,
        nonce: Option<&str>,
    ) -> Result<String, GateError> {
        let expires_at_unix = now_unix
            .checked_add(self.lifetime_seconds)
            .ok_or(GateError::InvalidIdTokenLifetime)?;

        let header = IdTokenHeader {
            alg: "RS256",
            typ: "JWT",
            kid: self.signing.key_id(),
        };
        let claims = IdTokenClaims {
            iss: self.issuer.as_str(),
            sub: subject.as_str(),
            aud: audience.as_str(),
            iat: now_unix,
            exp: expires_at_unix,
            nonce,
        };

        let header_json = serde_json::to_vec(&header).map_err(|_| GateError::IdTokenEncodingFailed)?;
        let claims_json = serde_json::to_vec(&claims).map_err(|_| GateError::IdTokenEncodingFailed)?;
        let header_segment = URL_SAFE_NO_PAD.encode(header_json);
        let claims_segment = URL_SAFE_NO_PAD.encode(claims_json);
        let signing_input = format!("{header_segment}.{claims_segment}");
        let signature = self.signing.sign_rs256(signing_input.as_bytes())?;
        let signature_segment = URL_SAFE_NO_PAD.encode(signature);

        Ok(format!("{signing_input}.{signature_segment}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn decode_json_segment(segment: &str) -> Value {
        let bytes = URL_SAFE_NO_PAD.decode(segment).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn id_token_has_rs256_header_and_required_oidc_claims() {
        let signing = OidcSigningAuthority::generate_2048().unwrap();
        let expected_kid = signing.key_id().to_owned();
        let issuer = IdTokenIssuer::new(
            Issuer::new("https://gate.realmforge.test").unwrap(),
            signing,
            300,
        )
        .unwrap();
        let token = issuer
            .issue(
                &IdentitySubject::new("subject-1").unwrap(),
                &ClientId::new("client-1").unwrap(),
                100,
                Some("nonce-1"),
            )
            .unwrap();
        let parts: Vec<_> = token.split('.').collect();
        assert_eq!(parts.len(), 3);

        let header = decode_json_segment(parts[0]);
        assert_eq!(header["alg"], "RS256");
        assert_eq!(header["typ"], "JWT");
        assert_eq!(header["kid"], expected_kid);

        let claims = decode_json_segment(parts[1]);
        assert_eq!(claims["iss"], "https://gate.realmforge.test");
        assert_eq!(claims["sub"], "subject-1");
        assert_eq!(claims["aud"], "client-1");
        assert_eq!(claims["iat"], 100);
        assert_eq!(claims["exp"], 400);
        assert_eq!(claims["nonce"], "nonce-1");
        assert!(!parts[2].is_empty());
    }

    #[test]
    fn nonce_is_omitted_when_not_requested() {
        let issuer = IdTokenIssuer::new(
            Issuer::new("https://gate.realmforge.test").unwrap(),
            OidcSigningAuthority::generate_2048().unwrap(),
            300,
        )
        .unwrap();
        let token = issuer
            .issue(
                &IdentitySubject::new("subject-1").unwrap(),
                &ClientId::new("client-1").unwrap(),
                100,
                None,
            )
            .unwrap();
        let parts: Vec<_> = token.split('.').collect();
        let claims = decode_json_segment(parts[1]);
        assert!(claims.get("nonce").is_none());
    }

    #[test]
    fn zero_or_overflowing_lifetime_fails_closed() {
        assert_eq!(
            IdTokenIssuer::new(
                Issuer::new("https://gate.realmforge.test").unwrap(),
                OidcSigningAuthority::generate_2048().unwrap(),
                0,
            )
            .unwrap_err(),
            GateError::InvalidIdTokenLifetime
        );

        let issuer = IdTokenIssuer::new(
            Issuer::new("https://gate.realmforge.test").unwrap(),
            OidcSigningAuthority::generate_2048().unwrap(),
            1,
        )
        .unwrap();
        assert_eq!(
            issuer
                .issue(
                    &IdentitySubject::new("subject-1").unwrap(),
                    &ClientId::new("client-1").unwrap(),
                    u64::MAX,
                    None,
                )
                .unwrap_err(),
            GateError::InvalidIdTokenLifetime
        );
    }
}
