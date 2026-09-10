// SPDX-License-Identifier: AGPL-3.0-only

//! JWT signing and JWKS publishing.
//!
//! A thin layer over the `jsonwebtoken` crate (no hand-rolled RSA). Tavern
//! parses a PKCS#8 RSA private key, signs RS256 access tokens with its claim
//! set, and publishes the public key as a JWK for the `/jwks/certs` endpoint.
//! [`SigningKeys`] models key rotation with an active key and retired keys.
//!
//! # References
//!
//! - RE spec `oauth-oidc-implementation.md`: RS256 only, JWKS at `/jwks/certs`,
//!   `battle_tag`/`country_code` custom claims, ~24h id-token lifetime.

use base64::Engine;
use jsonwebtoken::jwk::{
    AlgorithmParameters, CommonParameters, Jwk, JwkSet, KeyAlgorithm, PublicKeyUse,
    RSAKeyParameters, RSAKeyType,
};
use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, decode_header, encode,
};
use rsa::pkcs8::DecodePrivateKey;
use rsa::traits::PublicKeyParts;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Signing algorithm. Tavern uses RS256 only, matching the captured
/// `oauth.battle.net` discovery document.
const ALGORITHM: Algorithm = Algorithm::RS256;

/// Errors returned by JWT operations.
#[derive(Debug, Error)]
pub enum JwtError {
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("invalid signing key: {0}")]
    Key(#[from] rsa::errors::Error),

    #[error("invalid PKCS#8 key encoding: {0}")]
    Pkcs8(#[from] rsa::pkcs8::Error),

    #[error("invalid base64 in key material: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Tavern access-token / ID-token claims.
///
/// Carries the standard JWT registration claims plus the Battle.net-specific
/// custom fields observed in the captured ID/access tokens. All Battle.net-
/// specific fields are optional with `skip_serializing_if`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Claims {
    /// Subject — the numeric account id (string per JWT spec).
    pub sub: String,
    /// Issuer — `oauth.battle.net` equivalent, from config.
    pub iss: String,
    /// Issue time, Unix seconds.
    pub iat: i64,
    /// Expiry, Unix seconds.
    pub exp: i64,
    /// JWT id (unique per token).
    pub jti: String,
    /// Space-separated scope list.
    pub scope: String,
    /// Authorized client id.
    pub client_id: String,
    // Battle.net-specific claims (optional, only in user-authorized tokens).
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub battle_tag: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub country_code: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub account_identifier: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub birth_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mobile_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub country_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub verified_email_address_flag: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub employee_flag: Option<bool>,
    // Standard OIDC optional claims.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub azp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub at_hash: Option<String>,
    // Battle.net access-token claims (from client_credentials / introspection).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub client_roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub account_roles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub authorities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub programs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub account_authorities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub client_authorities: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub account_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub username: Option<String>,
}

/// A parsed RSA signing keypair: the private key for signing and the public
/// components (`n`, `e`) for JWKS publication.
#[derive(Clone)]
pub struct Keypair {
    /// `kid` used in the JWT header and the published JWK.
    pub kid: String,
    encoding: EncodingKey,
    n_base64url: String,
    e_base64url: String,
}

impl Keypair {
    /// Parse a PKCS#8 PEM private key and assign it a `kid`.
    pub fn from_pkcs8_pem(kid: impl Into<String>, pem: &str) -> Result<Self, JwtError> {
        let encoding = EncodingKey::from_rsa_pem(pem.as_bytes())?;
        let private_key = rsa::RsaPrivateKey::from_pkcs8_pem(pem)?;
        let public: rsa::RsaPublicKey = private_key.to_public_key();

        // RFC 7518: JWK RSA params are base64url-encoded big-endian integers
        // with no padding.
        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let n_base64url = engine.encode(public.n().to_bytes_be());
        let e_base64url = engine.encode(public.e().to_bytes_be());

        Ok(Self {
            kid: kid.into(),
            encoding,
            n_base64url,
            e_base64url,
        })
    }

    /// The JWK for `/jwks/certs`.
    pub fn jwk(&self) -> Jwk {
        Jwk {
            common: CommonParameters {
                public_key_use: Some(PublicKeyUse::Signature),
                key_id: Some(self.kid.clone()),
                key_algorithm: Some(KeyAlgorithm::RS256),
                ..Default::default()
            },
            algorithm: AlgorithmParameters::RSA(RSAKeyParameters {
                key_type: RSAKeyType::RSA,
                n: self.n_base64url.clone(),
                e: self.e_base64url.clone(),
            }),
        }
    }
}

/// A rotating set of signing keys: one active, zero or more retired.
///
/// The active key signs new tokens; retired keys remain valid for verification
/// until their tokens expire, then are removed. Mirrors the `signing_keys`
/// table (`retired_at` column) in the data model.
#[derive(Clone)]
pub struct SigningKeys {
    active: Keypair,
    retired: Vec<Keypair>,
}

impl SigningKeys {
    /// Create a set with a single active key and no retired keys.
    pub fn single(active: Keypair) -> Self {
        Self {
            active,
            retired: Vec::new(),
        }
    }

    /// Rotate: the previous active key becomes retired, `new_active` signs.
    pub fn rotate(&mut self, new_active: Keypair) {
        let old = std::mem::replace(&mut self.active, new_active);
        self.retired.push(old);
    }

    /// The currently active keypair.
    pub fn active(&self) -> &Keypair {
        &self.active
    }

    /// The JWKS document for `/jwks/certs`: the active key and all retired keys.
    pub fn jwks(&self) -> JwkSet {
        let keys = std::iter::once(&self.active)
            .chain(self.retired.iter())
            .map(Keypair::jwk)
            .collect();
        JwkSet { keys }
    }
}

/// Sign `claims` with the active key.
pub fn sign(claims: &Claims, keypair: &Keypair) -> Result<String, JwtError> {
    let mut header = Header::new(ALGORITHM);
    header.kid = Some(keypair.kid.clone());
    Ok(encode(&header, claims, &keypair.encoding)?)
}

/// Verify and decode a token issued by `issuer`, using the key behind the JWK
/// the token's `kid` points to in `jwks`. Rejects tokens whose `iss` does not
/// match.
pub fn verify(token: &str, issuer: &str, jwks: &JwkSet) -> Result<Claims, JwtError> {
    let header = decode_header(token)?;
    let kid = header.kid.ok_or_else(|| {
        JwtError::Jwt(jsonwebtoken::errors::Error::from(
            jsonwebtoken::errors::ErrorKind::InvalidToken,
        ))
    })?;
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| JwtError::Jwt(invalid_token()))?;
    let decoding = DecodingKey::from_jwk(jwk)?;

    let mut validation = Validation::new(ALGORITHM);
    validation.set_issuer(&[issuer]);
    validation.validate_exp = true;
    // Claims carry an optional `aud`; do not require it at this layer.
    validation.validate_aud = false;

    let data = decode::<Claims>(token, &decoding, &validation)?;
    Ok(data.claims)
}

fn invalid_token() -> jsonwebtoken::errors::Error {
    jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::InvalidToken)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fixed 2048-bit PKCS#8 test key generated for this milestone. Public
    /// only in the sense that it never protects real data; safe to commit.
    const TEST_KEY_PEM: &str = include_str!("../tests/data/test_key_pkcs8.pem");

    fn sample_claims(issuer: &str) -> Claims {
        // Far-future window so the token is never expired at test time.
        let iat = 2_000_000_000;
        let exp = iat + 86_399;
        Claims {
            sub: "42".to_string(),
            iss: issuer.to_string(),
            iat,
            exp,
            jti: "token-1".to_string(),
            scope: "openid account.basic".to_string(),
            client_id: "057adb2af62a4d59904f74754838c4c8".to_string(),
            battle_tag: "Tester#1234".to_string(),
            country_code: "USA".to_string(),
            account_identifier: "TEST@EXAMPLE.COM".to_string(),
            first_name: Some("Test".to_string()),
            last_name: Some("User".to_string()),
            birth_date: Some("2000-01-01".to_string()),
            mobile_number: None,
            country_id: Some(840),
            verified_email_address_flag: Some(true),
            employee_flag: Some(false),
            aud: Some("account-settings".to_string()),
            azp: Some("057adb2af62a4d59904f74754838c4c8".to_string()),
            nonce: None,
            at_hash: None,
            client_roles: None,
            account_roles: None,
            authorities: None,
            programs: None,
            env: None,
            active: None,
            account_authorities: None,
            client_authorities: None,
            account_id: None,
            username: None,
        }
    }

    #[test]
    fn sign_verify_round_trip() {
        let key = Keypair::from_pkcs8_pem("test-kid", TEST_KEY_PEM).unwrap();
        let claims = sample_claims("https://oauth.example");
        let token = sign(&claims, &key).unwrap();

        let keys = SigningKeys::single(key);
        let jwks = keys.jwks();

        let decoded = verify(&token, "https://oauth.example", &jwks).unwrap();
        assert_eq!(decoded, claims);
    }

    #[test]
    fn jwks_round_trip_verifies_signed_token() {
        // The published JWK is the only verification material: simulate a
        // verifier that knows nothing but the issuer and the JWKS document.
        let key = Keypair::from_pkcs8_pem("pub-kid", TEST_KEY_PEM).unwrap();
        let token = sign(&sample_claims("https://oauth.example"), &key).unwrap();
        let jwks = SigningKeys::single(key).jwks();

        let decoded = verify(&token, "https://oauth.example", &jwks).unwrap();
        assert_eq!(decoded.sub, "42");
        assert_eq!(decoded.battle_tag, "Tester#1234");
    }

    #[test]
    fn wrong_issuer_is_rejected() {
        let key = Keypair::from_pkcs8_pem("test-kid", TEST_KEY_PEM).unwrap();
        let token = sign(&sample_claims("https://oauth.example"), &key).unwrap();
        let jwks = SigningKeys::single(key).jwks();

        assert!(verify(&token, "https://attacker.example", &jwks).is_err());
    }

    #[test]
    fn unknown_kid_is_rejected() {
        let key = Keypair::from_pkcs8_pem("real-kid", TEST_KEY_PEM).unwrap();
        let token = sign(&sample_claims("https://oauth.example"), &key).unwrap();
        let jwks = SigningKeys::single(key).jwks();

        // Tamper the kid in the token header so it is absent from the JWKS.
        let parts: Vec<&str> = token.split('.').collect();
        let mut header_json = b64url_decode(parts[0]);
        header_json = header_json.replace("\"real-kid\"", "\"ghost-kid\"");
        let tampered = format!(
            "{}.{}.{}",
            b64url_encode_str(&header_json),
            parts[1],
            parts[2]
        );

        assert!(verify(&tampered, "https://oauth.example", &jwks).is_err());
    }

    #[test]
    fn rotation_publishes_active_and_retired() {
        let active = Keypair::from_pkcs8_pem("active", TEST_KEY_PEM).unwrap();
        let mut keys = SigningKeys::single(active);

        // A second distinct key for rotation. We reuse the test key with a
        // different kid; the Jwk set should still contain both entries.
        let rotated = Keypair::from_pkcs8_pem("retired", TEST_KEY_PEM).unwrap();
        let old_active_kid = keys.active().kid.clone();
        keys.rotate(rotated);

        let jwks = keys.jwks();
        assert_eq!(keys.active().kid, "retired");
        assert!(jwks.find("active").is_some(), "retired key stays published");
        assert!(jwks.find("retired").is_some());
        let _ = old_active_kid;
    }

    fn b64url_decode(s: &str) -> String {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(s)
            .unwrap();
        String::from_utf8(bytes).unwrap()
    }

    fn b64url_encode_str(s: &str) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes())
    }
}
