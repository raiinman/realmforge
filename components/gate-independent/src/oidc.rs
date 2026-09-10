use serde::Serialize;

use crate::GateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issuer(String);

impl Issuer {
    pub fn new(value: impl Into<String>) -> Result<Self, GateError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(GateError::InvalidIssuer);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn endpoint(&self, path: &str) -> String {
        format!(
            "{}/{}",
            self.0.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

/// Standards-derived OpenID Provider metadata for Realmforge Gate's initial
/// authorization-code + PKCE surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
}

impl ProviderMetadata {
    pub fn authorization_code(issuer: &Issuer) -> Self {
        Self {
            issuer: issuer.as_str().to_owned(),
            authorization_endpoint: issuer.endpoint("authorize"),
            token_endpoint: issuer.endpoint("token"),
            jwks_uri: issuer.endpoint("jwks.json"),
            response_types_supported: vec!["code".to_owned()],
            grant_types_supported: vec!["authorization_code".to_owned()],
            subject_types_supported: vec!["public".to_owned()],
            id_token_signing_alg_values_supported: vec!["RS256".to_owned()],
            code_challenge_methods_supported: vec!["S256".to_owned()],
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_metadata_advertises_only_implemented_initial_flow() {
        let issuer = Issuer::new("https://gate.realmforge.test").unwrap();
        let metadata = ProviderMetadata::authorization_code(&issuer);

        assert_eq!(metadata.issuer, "https://gate.realmforge.test");
        assert_eq!(
            metadata.authorization_endpoint,
            "https://gate.realmforge.test/authorize"
        );
        assert_eq!(metadata.grant_types_supported, ["authorization_code"]);
        assert_eq!(metadata.code_challenge_methods_supported, ["S256"]);
        assert_eq!(metadata.response_types_supported, ["code"]);
    }

    #[test]
    fn metadata_serializes_as_discovery_json() {
        let issuer = Issuer::new("https://gate.realmforge.test/").unwrap();
        let metadata = ProviderMetadata::authorization_code(&issuer);
        let value: serde_json::Value = serde_json::from_str(&metadata.to_json().unwrap()).unwrap();

        assert_eq!(value["issuer"], "https://gate.realmforge.test/");
        assert_eq!(
            value["token_endpoint"],
            "https://gate.realmforge.test/token"
        );
        assert_eq!(value["code_challenge_methods_supported"][0], "S256");
    }
}
