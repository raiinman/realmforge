use std::fmt;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Serialize;

use crate::{ClientId, GateError, IdentitySubject, OidcSigningAuthority};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdTokenClaims {
    pub iss: String,
    pub sub: String,
    pub aud: String,
    pub exp: u64,
    pub iat: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

impl IdTokenClaims {
    pub fn new(
        issuer: impl Into<String>,
        subject: &IdentitySubject,
        audience: &ClientId,
        issued_at_unix: u64,
        lifetime_seconds: u64,
        nonce: Option<String>,
    ) -> Result<Self, GateError> {
        if lifetime_seconds == 0 {
            return Err(GateError::InvalidIdTokenLifetime);
        }
        let exp = issued_at_unix
            .checked_add(lifetime_seconds)
            .ok_or(GateError::InvalidIdTokenLifetime)?;
        Ok(Self {
            iss: issuer.into(),
            sub: subject.as_str().to_owned(),
            aud: audience.as_str().to_owned(),
            exp,
            iat: issued_at_unix,
            nonce,
        })
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SignedIdToken(String);

impl SignedIdToken {
    pub fn issue(
        authority: &OidcSigningAuthority,
        claims: &IdTokenClaims,
    ) -> Result<Self, GateError> {
        #[derive(Serialize)]
        struct Header<'a> {
            alg: &'static str,
            typ: &'static str,
            kid: &'a str,
        }

        let header = Header {
            alg: "RS256",
            typ: "JWT",
            kid: authority.key_id(),
        };
        let header_json =
            serde_json::to_vec(&header).map_err(|_| GateError::IdTokenSerializationFailed)?;
        let claims_json =
            serde_json::to_vec(claims).map_err(|_| GateError::IdTokenSerializationFailed)?;
        let encoded_header = URL_SAFE_NO_PAD.encode(header_json);
        let encoded_claims = URL_SAFE_NO_PAD.encode(claims_json);
        let signing_input = format!("{encoded_header}.{encoded_claims}");
        let signature = authority.sign_rs256(signing_input.as_bytes())?;
        let encoded_signature = URL_SAFE_NO_PAD.encode(signature);

        Ok(Self(format!(
            "{signing_input}.{encoded_signature}"
        )))
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SignedIdToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SignedIdToken([REDACTED])")
    }
}

#[cfg(test)]
mod tests {
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use rsa::{
        BigUint, RsaPublicKey,
        pkcs1v15::{Signature, VerifyingKey},
    };
    use sha2::Sha256;
    use signature::Verifier;

    use super::*;

    fn claims() -> IdTokenClaims {
        IdTokenClaims::new(
            "https://gate.realmforge.test",
            &IdentitySubject::new("subject-1").unwrap(),
            &ClientId::new("client-1").unwrap(),
            100,
            300,
            Some("client-nonce".to_owned()),
        )
        .unwrap()
    }

    #[test]
    fn signed_id_token_has_required_oidc_claims_and_verifiable_rs256_signature() {
        let authority = OidcSigningAuthority::generate_2048().unwrap();
        let token = SignedIdToken::issue(&authority, &claims()).unwrap();
        let parts: Vec<_> = token.expose().split('.').collect();
        assert_eq!(parts.len(), 3);

        let header: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        assert_eq!(header["alg"], "RS256");
        assert_eq!(header["typ"], "JWT");
        assert_eq!(header["kid"], authority.key_id());

        let claims: IdTokenClaims =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        assert_eq!(claims.iss, "https://gate.realmforge.test");
        assert_eq!(claims.sub, "subject-1");
        assert_eq!(claims.aud, "client-1");
        assert_eq!(claims.iat, 100);
        assert_eq!(claims.exp, 400);
        assert_eq!(claims.nonce.as_deref(), Some("client-nonce"));

        let jwk = &authority.jwks().keys[0];
        let public_key = RsaPublicKey::new(
            BigUint::from_bytes_be(&URL_SAFE_NO_PAD.decode(&jwk.n).unwrap()),
            BigUint::from_bytes_be(&URL_SAFE_NO_PAD.decode(&jwk.e).unwrap()),
        )
        .unwrap();
        let verifying_key = VerifyingKey::<Sha256>::new(public_key);
        let signature_bytes = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        let signature = Signature::try_from(signature_bytes.as_slice()).unwrap();
        verifying_key
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .unwrap();
    }

    #[test]
    fn id_token_debug_redacts_token_value() {
        let authority = OidcSigningAuthority::generate_2048().unwrap();
        let token = SignedIdToken::issue(&authority, &claims()).unwrap();
        assert_eq!(format!("{token:?}"), "SignedIdToken([REDACTED])");
    }

    #[test]
    fn invalid_id_token_lifetime_fails_closed() {
        assert_eq!(
            IdTokenClaims::new(
                "https://gate.realmforge.test",
                &IdentitySubject::new("subject-1").unwrap(),
                &ClientId::new("client-1").unwrap(),
                100,
                0,
                None,
            )
            .unwrap_err(),
            GateError::InvalidIdTokenLifetime
        );
        assert_eq!(
            IdTokenClaims::new(
                "https://gate.realmforge.test",
                &IdentitySubject::new("subject-1").unwrap(),
                &ClientId::new("client-1").unwrap(),
                u64::MAX,
                1,
                None,
            )
            .unwrap_err(),
            GateError::InvalidIdTokenLifetime
        );
    }
}
