use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    AuthorizationCode, AuthorizationRequest, ClientId, GateError, IdTokenIssuer, Issuer,
    JsonWebKeySet, OAuthClientRegistry, OAuthService, OidcAuthorizationContext,
    OidcSigningAuthority, PkceCodeVerifier, PkceS256Challenge, ProviderMetadata, RedirectUri,
    SessionId, SessionRegistry, TokenExchangeRequest,
};

pub const GATE_SESSION_COOKIE: &str = "realmforge_session";
const ID_TOKEN_LIFETIME_SECONDS: u64 = 300;

#[derive(Debug)]
struct GateHttpRuntime {
    oauth: OAuthService,
    sessions: SessionRegistry,
}

#[derive(Debug, Clone)]
pub struct GateHttpState {
    metadata: ProviderMetadata,
    signing: OidcSigningAuthority,
    runtime: Arc<Mutex<GateHttpRuntime>>,
}

impl GateHttpState {
    pub fn new(
        metadata: ProviderMetadata,
        oauth: OAuthService,
        sessions: SessionRegistry,
        signing: OidcSigningAuthority,
    ) -> Self {
        Self {
            metadata,
            signing,
            runtime: Arc::new(Mutex::new(GateHttpRuntime { oauth, sessions })),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthorizeQuery {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    code_challenge: String,
    code_challenge_method: String,
    state: Option<String>,
    scope: Option<String>,
    nonce: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenForm {
    grant_type: String,
    code: String,
    client_id: String,
    redirect_uri: String,
    code_verifier: String,
}

#[derive(Debug, Serialize)]
struct OAuthErrorResponse {
    error: &'static str,
    error_description: &'static str,
}

pub fn gate_http_router(metadata: ProviderMetadata, signing: OidcSigningAuthority) -> Router {
    let oauth = OAuthService::new(OAuthClientRegistry::default(), 60, 3600)
        .expect("fixed OAuth lifetimes are non-zero");
    gate_http_router_with_state(GateHttpState::new(
        metadata,
        oauth,
        SessionRegistry::default(),
        signing,
    ))
}

pub fn gate_http_router_with_state(state: GateHttpState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks.json", get(jwks))
        .route("/authorize", get(authorize))
        .route("/token", post(token))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "realmforge-gate",
    })
}

async fn discovery(State(state): State<GateHttpState>) -> Json<ProviderMetadata> {
    Json(state.metadata)
}

async fn jwks(State(state): State<GateHttpState>) -> Json<JsonWebKeySet> {
    Json(state.signing.jwks())
}

async fn authorize(
    State(state): State<GateHttpState>,
    headers: HeaderMap,
    Query(query): Query<AuthorizeQuery>,
) -> Response {
    if query.response_type != "code" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_response_type",
            "only authorization code responses are supported",
        );
    }
    if query.code_challenge_method != "S256" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "code_challenge_method must be S256",
        );
    }

    let client_id = match ClientId::new(query.client_id) {
        Ok(value) => value,
        Err(_) => return invalid_request("client_id is invalid"),
    };
    let redirect_uri = match RedirectUri::new(query.redirect_uri) {
        Ok(value) => value,
        Err(_) => return invalid_request("redirect_uri is invalid"),
    };
    let pkce_challenge = match PkceS256Challenge::new(query.code_challenge) {
        Ok(value) => value,
        Err(_) => return invalid_request("code_challenge is invalid"),
    };
    let oidc = query
        .scope
        .as_deref()
        .filter(|scope| scope.split_ascii_whitespace().any(|value| value == "openid"))
        .map(|_| OidcAuthorizationContext::new(query.nonce));
    let session_id = match session_id_from_headers(&headers) {
        Some(value) => value,
        None => return login_required(),
    };
    let now_unix = match unix_now() {
        Some(value) => value,
        None => return server_error(),
    };

    let mut runtime = match state.runtime.lock() {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    let subject = match runtime.sessions.get(&session_id) {
        Ok(session) => match session.authenticated_subject() {
            Some(subject) => subject.clone(),
            None => return login_required(),
        },
        Err(_) => return login_required(),
    };
    let result = match runtime.oauth.authorize_authenticated(
        subject,
        AuthorizationRequest {
            client_id,
            redirect_uri,
            pkce_challenge,
            state: query.state,
            oidc,
        },
        now_unix,
    ) {
        Ok(value) => value,
        Err(GateError::OAuthClientNotFound | GateError::RedirectUriNotRegistered) => {
            return invalid_request("client or redirect URI is not registered");
        }
        Err(_) => return server_error(),
    };
    drop(runtime);

    let mut redirect = match Url::parse(result.redirect_uri.as_str()) {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    {
        let mut pairs = redirect.query_pairs_mut();
        pairs.append_pair("code", result.code.as_str());
        if let Some(state) = result.state.as_deref() {
            pairs.append_pair("state", state);
        }
    }

    (
        StatusCode::FOUND,
        [(header::LOCATION, redirect.to_string())],
    )
        .into_response()
}

async fn token(State(state): State<GateHttpState>, Form(form): Form<TokenForm>) -> Response {
    if form.grant_type != "authorization_code" {
        return oauth_error(
            StatusCode::BAD_REQUEST,
            "unsupported_grant_type",
            "only authorization_code is supported",
        );
    }

    let request = match token_exchange_request(form) {
        Ok(value) => value,
        Err(_) => return invalid_grant(),
    };
    let now_unix = match unix_now() {
        Some(value) => value,
        None => return server_error(),
    };
    let mut runtime = match state.runtime.lock() {
        Ok(value) => value,
        Err(_) => return server_error(),
    };
    let response = match runtime.oauth.exchange_authorization_code(request, now_unix) {
        Ok(value) => value,
        Err(
            GateError::AuthorizationCodeNotFound
            | GateError::AuthorizationGrantExpired
            | GateError::AuthorizationGrantConsumed
            | GateError::OAuthClientMismatch
            | GateError::OAuthRedirectMismatch
            | GateError::PkceVerificationFailed,
        ) => return invalid_grant(),
        Err(_) => return server_error(),
    };
    drop(runtime);

    let id_token = if let Some(oidc) = response.oidc.as_ref() {
        let issuer = match Issuer::new(state.metadata.issuer.clone()) {
            Ok(value) => value,
            Err(_) => return server_error(),
        };
        let issuer = match IdTokenIssuer::new(
            issuer,
            state.signing.clone(),
            ID_TOKEN_LIFETIME_SECONDS,
        ) {
            Ok(value) => value,
            Err(_) => return server_error(),
        };
        match issuer.issue(
            &response.subject,
            &response.client_id,
            now_unix,
            oidc.nonce.as_deref(),
        ) {
            Ok(value) => Some(value),
            Err(_) => return server_error(),
        }
    } else {
        None
    };

    Json(AccessTokenResponse {
        access_token: response.access_token.expose().to_owned(),
        token_type: response.token_type,
        expires_in: response.expires_in,
        id_token,
    })
    .into_response()
}

fn token_exchange_request(form: TokenForm) -> Result<TokenExchangeRequest, GateError> {
    Ok(TokenExchangeRequest {
        code: AuthorizationCode::parse(form.code)?,
        client_id: ClientId::new(form.client_id)?,
        redirect_uri: RedirectUri::new(form.redirect_uri)?,
        verifier: PkceCodeVerifier::new(form.code_verifier)?,
    })
}

fn session_id_from_headers(headers: &HeaderMap) -> Option<SessionId> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    for pair in cookie.split(';') {
        let (name, value) = pair.trim().split_once('=')?;
        if name == GATE_SESSION_COOKIE {
            return SessionId::new(value).ok();
        }
    }
    None
}

fn unix_now() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn invalid_request(description: &'static str) -> Response {
    oauth_error(StatusCode::BAD_REQUEST, "invalid_request", description)
}

fn invalid_grant() -> Response {
    oauth_error(
        StatusCode::BAD_REQUEST,
        "invalid_grant",
        "authorization code exchange failed",
    )
}

fn login_required() -> Response {
    oauth_error(
        StatusCode::UNAUTHORIZED,
        "login_required",
        "an authenticated Realmforge session is required",
    )
}

fn server_error() -> Response {
    oauth_error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "server_error",
        "Realmforge Gate could not complete the request",
    )
}

fn oauth_error(
    status: StatusCode,
    error: &'static str,
    error_description: &'static str,
) -> Response {
    (
        status,
        Json(OAuthErrorResponse {
            error,
            error_description,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    use tower::ServiceExt;

    use super::*;
    use crate::{GateSession, IdentitySubject, OAuthClient, PkceCodeVerifier, SessionId};

    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    fn metadata() -> ProviderMetadata {
        ProviderMetadata::authorization_code(&Issuer::new("https://gate.realmforge.test").unwrap())
    }

    fn signing() -> OidcSigningAuthority {
        OidcSigningAuthority::generate_2048().unwrap()
    }

    fn seeded_router() -> Router {
        let client = OAuthClient::new(
            ClientId::new("client-1").unwrap(),
            [RedirectUri::new("https://client.example/callback").unwrap()],
        )
        .unwrap();
        let oauth =
            OAuthService::new(OAuthClientRegistry::new([client]).unwrap(), 60, 3600).unwrap();
        let mut sessions = SessionRegistry::default();
        let mut session = GateSession::new(SessionId::new("session-1").unwrap());
        session
            .authenticate(IdentitySubject::new("subject-1").unwrap())
            .unwrap();
        sessions.insert(session).unwrap();
        gate_http_router_with_state(GateHttpState::new(metadata(), oauth, sessions, signing()))
    }

    fn authorize_uri(redirect_uri: &str) -> String {
        authorize_uri_with_scope(redirect_uri, Some("openid"), Some("nonce-1"))
    }

    fn authorize_uri_with_scope(
        redirect_uri: &str,
        scope: Option<&str>,
        nonce: Option<&str>,
    ) -> String {
        let verifier = PkceCodeVerifier::new(VERIFIER).unwrap();
        let challenge = PkceS256Challenge::from_verifier(&verifier);
        let mut url = Url::parse("http://localhost/authorize").unwrap();
        let mut pairs = url.query_pairs_mut();
        pairs
            .append_pair("response_type", "code")
            .append_pair("client_id", "client-1")
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("code_challenge", challenge.as_str())
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", "opaque-state");
        if let Some(scope) = scope {
            pairs.append_pair("scope", scope);
        }
        if let Some(nonce) = nonce {
            pairs.append_pair("nonce", nonce);
        }
        drop(pairs);
        format!("{}?{}", url.path(), url.query().unwrap())
    }

    fn token_body(code: &str) -> String {
        let mut form = url::form_urlencoded::Serializer::new(String::new());
        form.append_pair("grant_type", "authorization_code")
            .append_pair("code", code)
            .append_pair("client_id", "client-1")
            .append_pair("redirect_uri", "https://client.example/callback")
            .append_pair("code_verifier", VERIFIER);
        form.finish()
    }

    fn authorization_code_from_response(response: &Response) -> String {
        let location = response
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();
        let redirect = Url::parse(location).unwrap();
        let params: std::collections::BTreeMap<_, _> =
            redirect.query_pairs().into_owned().collect();
        params.get("code").unwrap().clone()
    }

    #[tokio::test]
    async fn health_is_realmforge_native() {
        let Json(body) = health().await;
        assert_eq!(body.status, "ok");
        assert_eq!(body.service, "realmforge-gate");
    }

    #[tokio::test]
    async fn discovery_returns_configured_provider_metadata() {
        let expected = metadata();
        let state = GateHttpState::new(
            expected.clone(),
            OAuthService::new(OAuthClientRegistry::default(), 60, 3600).unwrap(),
            SessionRegistry::default(),
            signing(),
        );

        let Json(actual) = discovery(State(state)).await;
        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn jwks_returns_active_rs256_public_key() {
        let authority = signing();
        let expected_kid = authority.key_id().to_owned();
        let state = GateHttpState::new(
            metadata(),
            OAuthService::new(OAuthClientRegistry::default(), 60, 3600).unwrap(),
            SessionRegistry::default(),
            authority,
        );

        let Json(actual) = jwks(State(state)).await;
        assert_eq!(actual.keys.len(), 1);
        assert_eq!(actual.keys[0].kid, expected_kid);
        assert_eq!(actual.keys[0].kty, "RSA");
        assert_eq!(actual.keys[0].alg, "RS256");
        assert_eq!(actual.keys[0].key_use, "sig");
    }

    #[tokio::test]
    async fn authorize_fails_closed_without_authenticated_session() {
        let response = gate_http_router(metadata(), signing())
            .oneshot(
                Request::builder()
                    .uri(authorize_uri("https://client.example/callback"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(response.headers().get(header::LOCATION).is_none());
    }

    #[tokio::test]
    async fn oidc_authorize_and_token_exchange_returns_signed_id_token() {
        let app = seeded_router();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(authorize_uri("https://client.example/callback"))
                    .header(header::COOKIE, format!("{GATE_SESSION_COOKIE}=session-1"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FOUND);
        let location = response
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();
        let redirect = Url::parse(location).unwrap();
        let params: std::collections::BTreeMap<_, _> =
            redirect.query_pairs().into_owned().collect();
        let code = params.get("code").unwrap().clone();
        assert_eq!(
            params.get("state").map(String::as_str),
            Some("opaque-state")
        );

        let body = token_body(&code);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/token")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let token_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(token_json["token_type"], "Bearer");
        assert_eq!(token_json["expires_in"], 3600);
        assert_eq!(token_json["access_token"].as_str().unwrap().len(), 43);

        let id_token = token_json["id_token"].as_str().unwrap();
        let parts: Vec<_> = id_token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let claims: serde_json::Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD.decode(parts[1]).unwrap(),
        )
        .unwrap();
        assert_eq!(claims["iss"], "https://gate.realmforge.test");
        assert_eq!(claims["sub"], "subject-1");
        assert_eq!(claims["aud"], "client-1");
        assert_eq!(claims["nonce"], "nonce-1");

        let replay = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/token")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
        let bytes = to_bytes(replay.into_body(), usize::MAX).await.unwrap();
        let error_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(error_json["error"], "invalid_grant");
    }

    #[tokio::test]
    async fn oauth_request_without_openid_scope_does_not_return_id_token() {
        let app = seeded_router();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(authorize_uri_with_scope(
                        "https://client.example/callback",
                        None,
                        None,
                    ))
                    .header(header::COOKIE, format!("{GATE_SESSION_COOKIE}=session-1"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        let code = authorization_code_from_response(&response);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/token")
                    .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(token_body(&code)))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let token_json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert!(token_json.get("id_token").is_none());
    }

    #[tokio::test]
    async fn authorize_never_redirects_to_unregistered_uri() {
        let response = seeded_router()
            .oneshot(
                Request::builder()
                    .uri(authorize_uri("https://evil.example/callback"))
                    .header(header::COOKIE, format!("{GATE_SESSION_COOKIE}=session-1"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(response.headers().get(header::LOCATION).is_none());
    }
}
