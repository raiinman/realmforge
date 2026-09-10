use crate::{
    AccessToken, AccessTokenStore, AuthorizationCode, AuthorizationCodeStore, ClientId, GateError,
    IdentitySubject, OAuthClientRegistry, OidcAuthorizationContext, PkceCodeVerifier,
    PkceS256Challenge, RedirectUri,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationRequest {
    pub client_id: ClientId,
    pub redirect_uri: RedirectUri,
    pub pkce_challenge: PkceS256Challenge,
    pub state: Option<String>,
    pub oidc: Option<OidcAuthorizationContext>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationResult {
    pub code: AuthorizationCode,
    pub redirect_uri: RedirectUri,
    pub state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenExchangeRequest {
    pub code: AuthorizationCode,
    pub client_id: ClientId,
    pub redirect_uri: RedirectUri,
    pub verifier: PkceCodeVerifier,
}

#[derive(Clone, PartialEq, Eq)]
pub struct TokenResponse {
    pub access_token: AccessToken,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub subject: IdentitySubject,
    pub client_id: ClientId,
    pub oidc: Option<OidcAuthorizationContext>,
}

impl std::fmt::Debug for TokenResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenResponse")
            .field("access_token", &"[REDACTED]")
            .field("token_type", &self.token_type)
            .field("expires_in", &self.expires_in)
            .field("subject", &self.subject)
            .field("client_id", &self.client_id)
            .field("oidc", &self.oidc)
            .finish()
    }
}

/// Realmforge's standards-derived OAuth authorization-code core.
///
/// The caller must supply an identity that has already been authenticated by
/// Gate's login/session layer. This service deliberately does not decide how
/// browser or game-client credentials are verified.
#[derive(Debug)]
pub struct OAuthService {
    clients: OAuthClientRegistry,
    codes: AuthorizationCodeStore,
    tokens: AccessTokenStore,
    code_lifetime_seconds: u64,
    access_token_lifetime_seconds: u64,
}

impl OAuthService {
    pub fn new(
        clients: OAuthClientRegistry,
        code_lifetime_seconds: u64,
        access_token_lifetime_seconds: u64,
    ) -> Result<Self, GateError> {
        if code_lifetime_seconds == 0 {
            return Err(GateError::InvalidAuthorizationGrantLifetime);
        }
        if access_token_lifetime_seconds == 0 {
            return Err(GateError::InvalidAccessTokenLifetime);
        }
        Ok(Self {
            clients,
            codes: AuthorizationCodeStore::default(),
            tokens: AccessTokenStore::default(),
            code_lifetime_seconds,
            access_token_lifetime_seconds,
        })
    }

    pub fn authorize_authenticated(
        &mut self,
        subject: IdentitySubject,
        request: AuthorizationRequest,
        now_unix: u64,
    ) -> Result<AuthorizationResult, GateError> {
        self.clients
            .validate_redirect(&request.client_id, &request.redirect_uri)?;
        let expires_at_unix = now_unix
            .checked_add(self.code_lifetime_seconds)
            .ok_or(GateError::InvalidAuthorizationGrantLifetime)?;
        let mut grant = self.clients.create_authorization_grant(
            subject,
            request.client_id,
            request.redirect_uri.clone(),
            request.pkce_challenge,
            now_unix,
            expires_at_unix,
        )?;
        if let Some(oidc) = request.oidc {
            grant = grant.with_oidc_context(oidc);
        }
        let code = self.codes.issue(grant);

        Ok(AuthorizationResult {
            code,
            redirect_uri: request.redirect_uri,
            state: request.state,
        })
    }

    pub fn exchange_authorization_code(
        &mut self,
        request: TokenExchangeRequest,
        now_unix: u64,
    ) -> Result<TokenResponse, GateError> {
        let redeemed = self.codes.redeem_full(
            &request.code,
            now_unix,
            &request.client_id,
            &request.redirect_uri,
            &request.verifier,
        )?;
        let access_token = self.tokens.issue(
            redeemed.subject.clone(),
            now_unix,
            self.access_token_lifetime_seconds,
        )?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer",
            expires_in: self.access_token_lifetime_seconds,
            subject: redeemed.subject,
            client_id: redeemed.client_id,
            oidc: redeemed.oidc,
        })
    }

    pub fn resolve_access_token(
        &self,
        token: &AccessToken,
        now_unix: u64,
    ) -> Result<&IdentitySubject, GateError> {
        Ok(&self.tokens.resolve(token, now_unix)?.subject)
    }

    pub fn revoke_access_token(&mut self, token: &AccessToken) -> bool {
        self.tokens.revoke(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OAuthClient, PkceCodeVerifier};

    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    fn service() -> OAuthService {
        let client = OAuthClient::new(
            ClientId::new("client-1").unwrap(),
            [RedirectUri::new("https://client.example/callback").unwrap()],
        )
        .unwrap();
        OAuthService::new(OAuthClientRegistry::new([client]).unwrap(), 60, 3600).unwrap()
    }

    fn authorization_request(verifier: &PkceCodeVerifier) -> AuthorizationRequest {
        AuthorizationRequest {
            client_id: ClientId::new("client-1").unwrap(),
            redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
            pkce_challenge: PkceS256Challenge::from_verifier(verifier),
            state: Some("opaque-client-state".to_owned()),
            oidc: None,
        }
    }

    #[test]
    fn authorization_then_exchange_issues_resolvable_bearer_token() {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let mut service = service();
        let result = service
            .authorize_authenticated(
                IdentitySubject::new("subject-1").unwrap(),
                authorization_request(&verifier),
                100,
            )
            .unwrap();

        assert_eq!(result.state.as_deref(), Some("opaque-client-state"));
        let token = service
            .exchange_authorization_code(
                TokenExchangeRequest {
                    code: result.code,
                    client_id: ClientId::new("client-1").unwrap(),
                    redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
                    verifier,
                },
                110,
            )
            .unwrap();

        assert_eq!(token.token_type, "Bearer");
        assert_eq!(token.expires_in, 3600);
        assert_eq!(token.subject.as_str(), "subject-1");
        assert_eq!(token.client_id.as_str(), "client-1");
        assert!(token.oidc.is_none());
        assert_eq!(
            service
                .resolve_access_token(&token.access_token, 111)
                .unwrap()
                .as_str(),
            "subject-1"
        );
    }

    #[test]
    fn oidc_context_survives_authorization_and_exchange() {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let mut service = service();
        let mut request = authorization_request(&verifier);
        request.oidc = Some(OidcAuthorizationContext::new(Some("nonce-1".to_owned())));
        let result = service
            .authorize_authenticated(IdentitySubject::new("subject-1").unwrap(), request, 100)
            .unwrap();

        let token = service
            .exchange_authorization_code(
                TokenExchangeRequest {
                    code: result.code,
                    client_id: ClientId::new("client-1").unwrap(),
                    redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
                    verifier,
                },
                110,
            )
            .unwrap();
        assert_eq!(token.oidc.unwrap().nonce.as_deref(), Some("nonce-1"));
    }

    #[test]
    fn authorization_code_cannot_be_reused() {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let mut service = service();
        let result = service
            .authorize_authenticated(
                IdentitySubject::new("subject-1").unwrap(),
                authorization_request(&verifier),
                100,
            )
            .unwrap();
        let code_text = result.code.as_str().to_owned();

        service
            .exchange_authorization_code(
                TokenExchangeRequest {
                    code: result.code,
                    client_id: ClientId::new("client-1").unwrap(),
                    redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
                    verifier: verifier.clone(),
                },
                110,
            )
            .unwrap();

        assert_eq!(
            service
                .exchange_authorization_code(
                    TokenExchangeRequest {
                        code: AuthorizationCode::parse(code_text).unwrap(),
                        client_id: ClientId::new("client-1").unwrap(),
                        redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
                        verifier,
                    },
                    111,
                )
                .unwrap_err(),
            GateError::AuthorizationCodeNotFound
        );
    }

    #[test]
    fn wrong_verifier_does_not_destroy_valid_code() {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let mut service = service();
        let result = service
            .authorize_authenticated(
                IdentitySubject::new("subject-1").unwrap(),
                authorization_request(&verifier),
                100,
            )
            .unwrap();
        let code_text = result.code.as_str().to_owned();

        assert_eq!(
            service
                .exchange_authorization_code(
                    TokenExchangeRequest {
                        code: AuthorizationCode::parse(&code_text).unwrap(),
                        client_id: ClientId::new("client-1").unwrap(),
                        redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
                        verifier: PkceCodeVerifier::new("A".repeat(43)).unwrap(),
                    },
                    110,
                )
                .unwrap_err(),
            GateError::PkceVerificationFailed
        );

        service
            .exchange_authorization_code(
                TokenExchangeRequest {
                    code: AuthorizationCode::parse(code_text).unwrap(),
                    client_id: ClientId::new("client-1").unwrap(),
                    redirect_uri: RedirectUri::new("https://client.example/callback").unwrap(),
                    verifier,
                },
                111,
            )
            .unwrap();
    }

    #[test]
    fn authorization_rejects_unregistered_redirect_before_code_issue() {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let mut service = service();
        let request = AuthorizationRequest {
            client_id: ClientId::new("client-1").unwrap(),
            redirect_uri: RedirectUri::new("https://evil.example/callback").unwrap(),
            pkce_challenge: PkceS256Challenge::from_verifier(&verifier),
            state: None,
            oidc: None,
        };
        assert_eq!(
            service
                .authorize_authenticated(IdentitySubject::new("subject-1").unwrap(), request, 100)
                .unwrap_err(),
            GateError::RedirectUriNotRegistered
        );
    }
}
