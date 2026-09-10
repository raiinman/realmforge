use std::collections::{BTreeMap, BTreeSet};

use crate::{
    AuthorizationGrant, ClientId, GateError, IdentitySubject, PkceS256Challenge, RedirectUri,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OAuthClient {
    pub id: ClientId,
    redirect_uris: BTreeSet<RedirectUri>,
}

impl OAuthClient {
    pub fn new(
        id: ClientId,
        redirect_uris: impl IntoIterator<Item = RedirectUri>,
    ) -> Result<Self, GateError> {
        let redirect_uris: BTreeSet<_> = redirect_uris.into_iter().collect();
        if redirect_uris.is_empty() {
            return Err(GateError::OAuthClientHasNoRedirectUris);
        }
        Ok(Self { id, redirect_uris })
    }

    pub fn permits_redirect(&self, redirect_uri: &RedirectUri) -> bool {
        self.redirect_uris.contains(redirect_uri)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OAuthClientRegistry {
    clients: BTreeMap<ClientId, OAuthClient>,
}

impl OAuthClientRegistry {
    pub fn new(clients: impl IntoIterator<Item = OAuthClient>) -> Result<Self, GateError> {
        let mut registry = Self::default();
        for client in clients {
            registry.register(client)?;
        }
        Ok(registry)
    }

    pub fn register(&mut self, client: OAuthClient) -> Result<(), GateError> {
        if self.clients.contains_key(&client.id) {
            return Err(GateError::DuplicateOAuthClientId);
        }
        self.clients.insert(client.id.clone(), client);
        Ok(())
    }

    pub fn validate_redirect(
        &self,
        client_id: &ClientId,
        redirect_uri: &RedirectUri,
    ) -> Result<(), GateError> {
        let client = self
            .clients
            .get(client_id)
            .ok_or(GateError::OAuthClientNotFound)?;
        if !client.permits_redirect(redirect_uri) {
            return Err(GateError::RedirectUriNotRegistered);
        }
        Ok(())
    }

    pub fn create_authorization_grant(
        &self,
        subject: IdentitySubject,
        client_id: ClientId,
        redirect_uri: RedirectUri,
        pkce_challenge: PkceS256Challenge,
        issued_at_unix: u64,
        expires_at_unix: u64,
    ) -> Result<AuthorizationGrant, GateError> {
        self.validate_redirect(&client_id, &redirect_uri)?;
        AuthorizationGrant::new(
            subject,
            client_id,
            redirect_uri,
            pkce_challenge,
            issued_at_unix,
            expires_at_unix,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PkceCodeVerifier;

    fn registry() -> OAuthClientRegistry {
        OAuthClientRegistry::new([OAuthClient::new(
            ClientId::new("client-1").unwrap(),
            [
                RedirectUri::new("https://client.example/callback").unwrap(),
                RedirectUri::new("https://client.example/alternate").unwrap(),
            ],
        )
        .unwrap()])
        .unwrap()
    }

    #[test]
    fn redirects_require_exact_registration() {
        let registry = registry();
        let client = ClientId::new("client-1").unwrap();

        registry
            .validate_redirect(
                &client,
                &RedirectUri::new("https://client.example/callback").unwrap(),
            )
            .unwrap();

        assert_eq!(
            registry
                .validate_redirect(
                    &client,
                    &RedirectUri::new("https://client.example/callback?extra=1").unwrap(),
                )
                .unwrap_err(),
            GateError::RedirectUriNotRegistered
        );
    }

    #[test]
    fn unknown_clients_and_duplicates_fail_closed() {
        let registry = registry();
        assert_eq!(
            registry
                .validate_redirect(
                    &ClientId::new("missing").unwrap(),
                    &RedirectUri::new("https://client.example/callback").unwrap(),
                )
                .unwrap_err(),
            GateError::OAuthClientNotFound
        );

        let duplicate = OAuthClient::new(
            ClientId::new("client-1").unwrap(),
            [RedirectUri::new("https://other.example/callback").unwrap()],
        )
        .unwrap();
        let result = OAuthClientRegistry::new([
            OAuthClient::new(
                ClientId::new("client-1").unwrap(),
                [RedirectUri::new("https://client.example/callback").unwrap()],
            )
            .unwrap(),
            duplicate,
        ]);
        assert_eq!(result.unwrap_err(), GateError::DuplicateOAuthClientId);
    }

    #[test]
    fn grant_creation_is_blocked_for_unregistered_redirects() {
        let registry = registry();
        let verifier = PkceCodeVerifier::new("A".repeat(43)).unwrap();
        let result = registry.create_authorization_grant(
            IdentitySubject::new("subject-1").unwrap(),
            ClientId::new("client-1").unwrap(),
            RedirectUri::new("https://evil.example/callback").unwrap(),
            PkceS256Challenge::from_verifier(&verifier),
            100,
            200,
        );

        assert_eq!(result.unwrap_err(), GateError::RedirectUriNotRegistered);
    }
}
