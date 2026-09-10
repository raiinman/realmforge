use std::fmt;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::rngs::OsRng;
use rsa::{
    RsaPrivateKey, RsaPublicKey,
    pkcs1v15::SigningKey,
    pkcs8::{DecodePrivateKey, EncodePrivateKey, LineEnding},
    traits::PublicKeyParts,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use signature::{SignatureEncoding, Signer};

use crate::GateError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JsonWebKey {
    pub kty: &'static str,
    #[serde(rename = "use")]
    pub key_use: &'static str,
    pub alg: &'static str,
    pub kid: String,
    pub n: String,
    pub e: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JsonWebKeySet {
    pub keys: Vec<JsonWebKey>,
}

#[derive(Clone)]
pub struct OidcSigningAuthority {
    private_key: RsaPrivateKey,
    public_jwk: JsonWebKey,
}

impl fmt::Debug for OidcSigningAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OidcSigningAuthority")
            .field("kid", &self.public_jwk.kid)
            .field("private_key", &"[REDACTED]")
            .finish()
    }
}

impl OidcSigningAuthority {
    pub fn from_pkcs8_pem(pem: &str) -> Result<Self, GateError> {
        let private_key =
            RsaPrivateKey::from_pkcs8_pem(pem).map_err(|_| GateError::InvalidSigningKey)?;
        Self::from_private_key(private_key)
    }

    pub fn generate_2048() -> Result<Self, GateError> {
        let private_key = RsaPrivateKey::new(&mut OsRng, 2048)
            .map_err(|_| GateError::SigningKeyGenerationFailed)?;
        Self::from_private_key(private_key)
    }

    fn from_private_key(private_key: RsaPrivateKey) -> Result<Self, GateError> {
        private_key
            .validate()
            .map_err(|_| GateError::InvalidSigningKey)?;
        if private_key.n().bits() < 2048 {
            return Err(GateError::InvalidSigningKey);
        }

        let public_key = RsaPublicKey::from(&private_key);
        let n = URL_SAFE_NO_PAD.encode(public_key.n().to_bytes_be());
        let e = URL_SAFE_NO_PAD.encode(public_key.e().to_bytes_be());
        let kid = rsa_jwk_thumbprint(&n, &e);
        let public_jwk = JsonWebKey {
            kty: "RSA",
            key_use: "sig",
            alg: "RS256",
            kid,
            n,
            e,
        };

        Ok(Self {
            private_key,
            public_jwk,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.public_jwk.kid
    }

    pub fn jwks(&self) -> JsonWebKeySet {
        JsonWebKeySet {
            keys: vec![self.public_jwk.clone()],
        }
    }

    pub fn to_pkcs8_pem(&self) -> Result<String, GateError> {
        self.private_key
            .to_pkcs8_pem(LineEnding::LF)
            .map(|pem| pem.to_string())
            .map_err(|_| GateError::SigningKeyEncodingFailed)
    }

    pub fn sign_rs256(&self, signing_input: &[u8]) -> Result<Vec<u8>, GateError> {
        let signing_key = SigningKey::<Sha256>::new(self.private_key.clone());
        Ok(signing_key.sign(signing_input).to_vec())
    }
}

fn rsa_jwk_thumbprint(n: &str, e: &str) -> String {
    let canonical = format!(r#"{{"e":"{e}","kty":"RSA","n":"{n}"}}"#);
    URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_key_round_trips_and_keeps_stable_kid() {
        let authority = OidcSigningAuthority::generate_2048().unwrap();
        let pem = authority.to_pkcs8_pem().unwrap();
        let restored = OidcSigningAuthority::from_pkcs8_pem(&pem).unwrap();

        assert_eq!(authority.key_id(), restored.key_id());
        assert_eq!(authority.jwks(), restored.jwks());
        assert_eq!(authority.jwks().keys[0].alg, "RS256");
        assert_eq!(authority.jwks().keys[0].key_use, "sig");
    }

    #[test]
    fn debug_output_never_contains_private_key_material() {
        let authority = OidcSigningAuthority::generate_2048().unwrap();
        let debug = format!("{authority:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("PRIVATE KEY"));
    }

    #[test]
    fn jwk_thumbprint_is_deterministic_for_same_public_key() {
        let authority = OidcSigningAuthority::generate_2048().unwrap();
        let jwk = &authority.jwks().keys[0];
        assert_eq!(authority.key_id(), rsa_jwk_thumbprint(&jwk.n, &jwk.e));
    }

    #[test]
    fn rsa_signature_is_non_empty_and_deterministic_for_pkcs1_v15() {
        let authority = OidcSigningAuthority::generate_2048().unwrap();
        let first = authority.sign_rs256(b"header.payload").unwrap();
        let second = authority.sign_rs256(b"header.payload").unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 256);
    }
}
