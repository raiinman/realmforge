// SPDX-License-Identifier: AGPL-3.0-only

//! OAuth 2.0 / OpenID Connect provider.
//!
//! Implements the `oauth.battle.net` surface: discovery, authorize, token
//! (authorization_code, refresh_token, client_credentials, and the desktop-app
//! RFC 8693 token-exchange), userinfo, JWKS, introspection, and revocation.
//!
//! RFC compliance (see the Kanidm-supported RFC list): RFC 6749 (OAuth 2.0),
//! RFC 7009 (revocation), RFC 7662 (introspection), RFC 7636 (PKCE),
//! RFC 8414 (server metadata), RFC 8693 (token exchange), RFC 9068 (JWT
//! access tokens), OIDC Core 1.0, OIDC Discovery 1.0.

use std::sync::Arc;

use axum::extract::{Form, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use realmforge_gate_core::jwt::{self, Claims, JwtError, SigningKeys};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Shared application state for the OAuth provider.
pub struct OAuthState {
    /// The Postgres connection pool.
    pub pool: sqlx::PgPool,
    /// The OAuth issuer URL (e.g. `https://oauth.wowemu.dev`).
    pub issuer: String,
    /// The RSA signing keys (active + retired).
    pub signing_keys: SigningKeys,
    /// The account server base URL (e.g. `http://localhost:8081`).
    pub account_server_url: String,
    /// The 2-letter region code (e.g. `US`, `KR`). Used in authorization
    /// codes and service tickets.
    pub region: String,
    /// Precomputed JWKS JSON, served on /jwks/certs.
    pub jwks_json: serde_json::Value,
    /// Cached OAuth client lookups (5-min TTL).
    pub client_cache: moka::sync::Cache<String, realmforge_gate_core::OAuthClient>,
}

/// Build the OAuth provider router.
pub fn router(state: Arc<OAuthState>) -> Router {
    Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks/certs", get(jwks))
        .route("/authorize", get(authorize))
        .route("/token", post(token))
        .route("/sso", post(sso))
        .route("/userinfo", get(userinfo))
        .route("/revoke", post(revoke))
        .route("/v2/check_token", post(introspect))
        .route("/device/code", post(device_code))
        .route("/device/approve", post(device_approve))
        .route("/logout", get(logout))
        .with_state(state)
}

/// Construct OAuth state from a pool, issuer URL, and signing key.
pub fn oauth_state(
    pool: sqlx::PgPool,
    issuer: String,
    signing_key_pem: &str,
) -> Result<Arc<OAuthState>, JwtError> {
    oauth_state_with_region(pool, issuer, signing_key_pem, "US")
}

/// Construct OAuth state with an explicit region code.
pub fn oauth_state_with_region(
    pool: sqlx::PgPool,
    issuer: String,
    signing_key_pem: &str,
    region: &str,
) -> Result<Arc<OAuthState>, JwtError> {
    let keypair = jwt::Keypair::from_pkcs8_pem("realmforge-1", signing_key_pem)?;
    let signing_keys = SigningKeys::single(keypair);
    let jwks_json = serde_json::to_value(signing_keys.jwks()).unwrap();
    // The account web/login UI. For a local install the realmforge-gate-account-server runs
    // on 8080 while the realmforge-gate-oauth-server is on 8081; override with
    // ACCOUNT_SERVER_URL when they differ.
    let account_server_url =
        std::env::var("ACCOUNT_SERVER_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    Ok(Arc::new(OAuthState {
        pool,
        issuer,
        signing_keys,
        account_server_url,
        region: region.to_string(),
        jwks_json,
        client_cache: moka::sync::Cache::builder()
            .time_to_live(std::time::Duration::from_secs(300))
            .build(),
    }))
}

// --- RFC 6749 §5.2 error handling ---

/// An OAuth error with the RFC 6749 error code and HTTP status.
#[derive(Debug)]
pub struct OAuthError {
    /// RFC 6749 error code: `invalid_request`, `invalid_client`,
    /// `invalid_grant`, `unsupported_grant_type`, etc.
    code: &'static str,
    /// Human-readable description.
    description: String,
    /// HTTP status: 400 for most errors, 401 for invalid_client.
    status: StatusCode,
}

impl OAuthError {
    fn invalid_request(msg: impl Into<String>) -> Self {
        Self {
            code: "invalid_request",
            description: msg.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    fn invalid_client(msg: impl Into<String>) -> Self {
        Self {
            code: "invalid_client",
            description: msg.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    fn invalid_grant(msg: impl Into<String>) -> Self {
        Self {
            code: "invalid_grant",
            description: msg.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    fn unsupported_grant_type(msg: impl Into<String>) -> Self {
        Self {
            code: "unsupported_grant_type",
            description: msg.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    fn invalid_token(msg: impl Into<String>) -> Self {
        Self {
            code: "invalid_token",
            description: msg.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            code: "unauthorized",
            description: msg.into(),
            status: StatusCode::UNAUTHORIZED,
        }
    }

    fn authorization_pending() -> Self {
        Self {
            code: "authorization_pending",
            description: "The authorization request is still pending".into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    fn server(msg: impl Into<String>) -> Self {
        Self {
            code: "server_error",
            description: msg.into(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.description)
    }
}

impl From<sqlx::Error> for OAuthError {
    fn from(e: sqlx::Error) -> Self {
        Self::server(e.to_string())
    }
}

impl From<realmforge_gate_db::DbError> for OAuthError {
    fn from(e: realmforge_gate_db::DbError) -> Self {
        Self::server(e.to_string())
    }
}

impl From<JwtError> for OAuthError {
    fn from(e: JwtError) -> Self {
        Self::invalid_token(e.to_string())
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        // RFC 6749 §5.2: {"error":"code","error_description":"..."}
        (
            self.status,
            [("content-type", "application/json")],
            Json(serde_json::json!({
                "error": self.code,
                "error_description": self.description,
            })),
        )
            .into_response()
    }
}

// --- Discovery (RFC 8414 / OIDC Discovery 1.0) ---

#[derive(Serialize)]
struct Discovery {
    issuer: String,
    authorization_endpoint: String,
    end_session_endpoint: String,
    token_endpoint: String,
    userinfo_endpoint: String,
    jwks_uri: String,
    revocation_endpoint: String,
    introspection_endpoint: String,
    device_authorization_endpoint: String,
    response_types_supported: Vec<String>,
    subject_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    id_token_signing_alg_values_supported: Vec<String>,
    scopes_supported: Vec<String>,
    code_challenge_methods_supported: Vec<String>,
}

async fn discovery(State(state): State<Arc<OAuthState>>) -> impl IntoResponse {
    let base = &state.issuer;
    Json(Discovery {
        issuer: base.clone(),
        authorization_endpoint: format!("{base}/authorize"),
        end_session_endpoint: format!("{base}/logout"),
        token_endpoint: format!("{base}/token"),
        userinfo_endpoint: format!("{base}/userinfo"),
        jwks_uri: format!("{base}/jwks/certs"),
        revocation_endpoint: format!("{base}/revoke"),
        introspection_endpoint: format!("{base}/v2/check_token"),
        device_authorization_endpoint: format!("{base}/device/code"),
        response_types_supported: vec![
            "code".into(),
            "code id_token".into(),
            "id_token".into(),
            "token id_token".into(),
        ],
        subject_types_supported: vec!["public".into(), "pairwise".into()],
        grant_types_supported: vec![
            "authorization_code".into(),
            "client_credentials".into(),
            "implicit".into(),
            "token_extension".into(),
            "server_sso".into(),
            "client_sso".into(),
            "device_code".into(),
            "refresh_token".into(),
            "urn:ietf:params:oauth:grant-type:device_code".into(),
            "urn:ietf:params:oauth:grant-type:token-exchange".into(),
            "urn:blizzard:params:oauth:grant-type:implicit-exchange".into(),
            "urn:blizzard:params:oauth:grant-type:implicit-extension".into(),
        ],
        id_token_signing_alg_values_supported: vec!["RS256".into()],
        scopes_supported: vec![
            "openid".into(),
            "account.basic".into(),
            "account.full".into(),
        ],
        code_challenge_methods_supported: vec!["S256".into()],
    })
}

// --- JWKS ---

async fn jwks(State(state): State<Arc<OAuthState>>) -> impl IntoResponse {
    Json(state.jwks_json.clone())
}

// --- Authorize (RFC 6749 §4.1.1 + PKCE RFC 7636) ---

#[derive(Deserialize)]
struct AuthorizeParams {
    client_id: String,
    redirect_uri: String,
    #[serde(default)]
    #[allow(dead_code)]
    response_type: String,
    #[serde(default)]
    scope: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    #[serde(rename = "ST")]
    st: Option<String>,
    // PKCE (RFC 7636)
    #[serde(default)]
    code_challenge: Option<String>,
    #[serde(default)]
    code_challenge_method: Option<String>,
    // OIDC silent re-authentication (M19)
    #[serde(default)]
    prompt: Option<String>,
    // Battle.net cookie-based SSO (M19)
    #[serde(default)]
    #[allow(dead_code)]
    cookietoken: Option<String>,
}

async fn authorize(
    State(state): State<Arc<OAuthState>>,
    headers: HeaderMap,
    Query(params): Query<AuthorizeParams>,
) -> Result<axum::response::Response, OAuthError> {
    // Two-hop redirect matching the capture (A.5–A.6):
    // 1. First hop with ST: consume ticket → create oauth session →
    //    302 to same URL without ST. Sets SESSIONID cookie.
    // 2. Second hop without ST: resolve SESSIONID → mint code →
    //    303 to redirect_uri with code.
    if params.st.is_some() {
        return handle_authorize_first_hop(&state, &params).await;
    }
    handle_authorize_second_hop(&state, &params, &headers).await
}

/// First hop: consume the login ticket (ST), create an oauth authorize
/// session, set SESSIONID cookie, and 302 back to /authorize without the ST.
async fn handle_authorize_first_hop(
    state: &OAuthState,
    params: &AuthorizeParams,
) -> Result<Response, OAuthError> {
    let st = params.st.as_deref().unwrap();

    // Validate the client and redirect_uri BEFORE consuming the ticket.
    // The ticket is one-time-use; if validation fails after consuming it,
    // the ticket is lost and the user must re-login.
    let client = realmforge_gate_db::oauth_clients::find_by_id(&state.pool, &params.client_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_client("unknown client_id"))?;

    if !client.redirect_uris.contains(&params.redirect_uri) {
        return Err(OAuthError::invalid_request("invalid redirect_uri"));
    }

    let account_id = realmforge_gate_db::service_tickets::consume(&state.pool, st)
        .await?
        .map(|t| t.account_id)
        .ok_or_else(|| OAuthError::invalid_grant("login ticket expired or already used"))?;

    // Create a short-lived OAuth authorize session (2 min per capture).
    let session_id = Uuid::new_v4();
    let session = realmforge_gate_core::Session {
        session_id,
        account_id,
        expires_at: Utc::now() + Duration::minutes(2),
        created_at: Utc::now(),
        user_agent: None,
        ip: None,
    };
    realmforge_gate_db::sessions::insert(&state.pool, &session).await?;

    // Build the redirect URL: strip ST, preserve all other params.
    let redirect_url = build_authorize_url_without_st(params);

    let mut resp = Redirect::to(&redirect_url).into_response();
    // Override 303 status to 302 (Found) matching the capture.
    *resp.status_mut() = StatusCode::FOUND;
    let headers = resp.headers_mut();
    // Set the `opt` cookie with the session UUID (matching the capture's
    // `opt=1` cookie pattern). Hop 2 reads this cookie to resolve the session.
    let opt_cookie = format!(
        "opt={}; Max-Age=31556900; Path=/; Secure",
        base64_encode_uuid(session_id)
    );
    headers.append(header::SET_COOKIE, opt_cookie.parse().unwrap());

    Ok(resp)
}

/// Second hop: resolve the oauth authorize session from SESSIONID cookie,
/// mint an authorization code, and 303 to the client's redirect_uri.
/// When no valid SESSIONID exists, redirect to the account login page with
/// the original authorize URL as the `ref` parameter.
async fn handle_authorize_second_hop(
    state: &OAuthState,
    params: &AuthorizeParams,
    headers: &HeaderMap,
) -> Result<Response, OAuthError> {
    // Try to resolve an existing session. If none, redirect to login.
    let account_id = match resolve_authorize_session(state, headers).await {
        Ok(id) => id,
        Err(_) => {
            // No valid session — if prompt=none, return error per OIDC.
            if params.prompt.as_deref() == Some("none") {
                let redirect = format!(
                    "{}?error=interaction_required&state={}",
                    params.redirect_uri,
                    params.state.as_deref().unwrap_or("")
                );
                let mut resp = Redirect::to(&redirect).into_response();
                *resp.status_mut() = StatusCode::FOUND;
                return Ok(resp);
            }

            // No valid session — redirect to the account login page with the
            // original authorize URL as `ref`.
            let authorize_url = build_full_authorize_url(params);
            let login_url = format!(
                "{}/login/en/?ref={}",
                state.account_server_url,
                encode_uri_component(&authorize_url)
            );
            let mut resp = Redirect::to(&login_url).into_response();
            *resp.status_mut() = StatusCode::FOUND;
            return Ok(resp);
        }
    };

    // Validate the client.
    let client = realmforge_gate_db::oauth_clients::find_by_id(&state.pool, &params.client_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_client("unknown client_id"))?;

    if !client.redirect_uris.contains(&params.redirect_uri) {
        return Err(OAuthError::invalid_request("invalid redirect_uri"));
    }

    // Validate PKCE method if provided (RFC 7636).
    if params
        .code_challenge_method
        .as_deref()
        .is_some_and(|m| m != "S256")
    {
        return Err(OAuthError::invalid_request(
            "code_challenge_method must be S256",
        ));
    }

    let code = format_authorization_code(&state.region);
    let auth_code = realmforge_gate_core::AuthorizationCode {
        code: code.clone(),
        client_id: params.client_id.clone(),
        account_id,
        scope: params.scope.clone(),
        redirect_uri: params.redirect_uri.clone(),
        code_challenge: params.code_challenge.clone(),
        nonce: None,
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
    };
    realmforge_gate_db::authorization_codes::insert(&state.pool, &auth_code).await?;

    let mut url = format!("{}?code={}", params.redirect_uri, code);
    if let Some(s) = &params.state {
        url.push_str(&format!("&state={s}"));
    }

    // Set a SESSIONID cookie on the second hop (capture A.6: both hops set it).
    // This creates a persistent OAuth session so the user can re-authorize
    // without re-logging in.
    let oauth_session_id = Uuid::new_v4();
    let oauth_session = realmforge_gate_core::Session {
        session_id: oauth_session_id,
        account_id,
        expires_at: Utc::now() + Duration::minutes(30),
        created_at: Utc::now(),
        user_agent: None,
        ip: None,
    };
    realmforge_gate_db::sessions::insert(&state.pool, &oauth_session).await?;

    let mut resp = Redirect::to(&url).into_response();
    let headers = resp.headers_mut();
    let session_cookie = format!(
        "SESSIONID={}; Path=/; Secure; HttpOnly; SameSite=None",
        base64_encode_uuid(oauth_session_id)
    );
    headers.append(header::SET_COOKIE, session_cookie.parse().unwrap());

    Ok(resp)
}

/// Resolve the account_id from the SESSIONID cookie.
/// For the first OAuth authorize hop, the session was just created by
/// handle_authorize_first_hop. The browser sends the SESSIONID cookie on
/// the second request.
async fn resolve_authorize_session(
    state: &OAuthState,
    headers: &HeaderMap,
) -> Result<i64, OAuthError> {
    // Extract the SESSIONID cookie.
    let session_id = extract_oauth_session_id(headers)?;

    let session = realmforge_gate_db::sessions::find_by_id(&state.pool, session_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_grant("authorize session expired or not found"))?;

    if session.expires_at < Utc::now() {
        return Err(OAuthError::invalid_grant("authorize session expired"));
    }

    // Delete the session after consumption (one-time use for authorize flow).
    let _ = realmforge_gate_db::sessions::delete(&state.pool, session_id).await;

    Ok(session.account_id)
}

/// Extract a base64-encoded UUID session ID from the `opt` cookie set by
/// handle_authorize_first_hop. The capture sets `opt` on hop 1, not SESSIONID.
fn extract_oauth_session_id(headers: &HeaderMap) -> Result<Uuid, OAuthError> {
    let raw = extract_cookie(headers, "opt")
        .ok_or_else(|| OAuthError::invalid_grant("missing opt cookie"))?;
    decode_session_id(&raw).ok_or_else(|| OAuthError::invalid_grant("invalid opt cookie format"))
}

/// Build the full authorize URL for use in a login-page `ref` parameter.
fn build_full_authorize_url(params: &AuthorizeParams) -> String {
    let mut url = format!(
        "/authorize?client_id={}&redirect_uri={}&response_type={}",
        params.client_id, params.redirect_uri, params.response_type
    );
    if !params.scope.is_empty() {
        url.push_str(&format!("&scope={}", encode_uri_component(&params.scope)));
    }
    if let Some(ref s) = params.state {
        url.push_str(&format!("&state={}", encode_uri_component(s)));
    }
    if let Some(ref c) = params.code_challenge {
        url.push_str(&format!("&code_challenge={}", encode_uri_component(c)));
    }
    if let Some(ref m) = params.code_challenge_method {
        url.push_str(&format!(
            "&code_challenge_method={}",
            encode_uri_component(m)
        ));
    }
    url
}

/// Simple URL component encoding (only encodes characters problematic in URLs).
fn encode_uri_component(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
            '@' => "%40".to_string(),
            ' ' => "%20".to_string(),
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            other => format!("%{:02X}", other as u8),
        })
        .collect()
}

/// Build the authorize URL without the ST parameter, preserving all others.
fn build_authorize_url_without_st(params: &AuthorizeParams) -> String {
    let mut url = format!(
        "/authorize?client_id={}&redirect_uri={}",
        params.client_id, params.redirect_uri
    );
    if !params.response_type.is_empty() {
        url.push_str(&format!("&response_type={}", params.response_type));
    }
    if !params.scope.is_empty() {
        url.push_str(&format!("&scope={}", params.scope));
    }
    if let Some(ref s) = params.state {
        url.push_str(&format!("&state={s}"));
    }
    if let Some(ref c) = params.code_challenge {
        url.push_str(&format!("&code_challenge={c}"));
    }
    if let Some(ref m) = params.code_challenge_method {
        url.push_str(&format!("&code_challenge_method={m}"));
    }
    url
}

// --- Token (RFC 6749 §4.1.3 + §6 + §4.4 + RFC 8693) ---

/// Token request parsed from `application/x-www-form-urlencoded` per RFC 6749.
#[derive(Deserialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    redirect_uri: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    // token-exchange (RFC 8693)
    #[serde(default)]
    subject_token: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    subject_token_type: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    requested_token_type: Option<String>,
    // device_code (RFC 8628)
    #[serde(default)]
    device_code: Option<String>,
    // client identification
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    client_secret: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    // PKCE (RFC 7636)
    #[serde(default)]
    code_verifier: Option<String>,
}

#[derive(Serialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id_token: Option<String>,
    scope: String,
}

/// `client_sso` grant request (Phoenix desktop app).
///
/// The real Phoenix client sends these as query parameters on
/// `POST /sso?client_id=...&scope=...&token=...&grant_type=client_sso&token_type=jwt`.
#[derive(Deserialize)]
struct SsoRequest {
    client_id: String,
    #[serde(default)]
    scope: String,
    token: String,
    grant_type: String,
    /// Phoenix sends `token_type=jwt` but we always issue JWT access tokens.
    #[serde(default)]
    #[allow(dead_code)]
    token_type: String,
}

/// `POST /sso` — handle `client_sso` grant.
///
/// Exchanges a region-prefixed BNET auth token (e.g. `US-<hex>-<id>`,
/// `KR-<hex>-<id>`) from BGS AuthenticationService for an OAuth access
/// token without browser interaction. Phoenix sends parameters as query
/// string (confirmed via mitmproxy captures, Phoenix 2.52.3.17554).
async fn sso(
    State(state): State<Arc<OAuthState>>,
    Query(req): Query<SsoRequest>,
) -> Result<Json<TokenResponse>, OAuthError> {
    if req.grant_type != "client_sso" {
        return Err(OAuthError::unsupported_grant_type(
            "only client_sso is supported at /sso",
        ));
    }

    // Validate the client exists.
    let _client = realmforge_gate_db::oauth_clients::find_by_id(&state.pool, &req.client_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_client("unknown client_id"))?;

    if req.token.is_empty() {
        return Err(OAuthError::invalid_grant("missing BNET auth token"));
    }

    // BNET tokens are region-prefixed (e.g. US-, KR-, EU-).
    // Phoenix Gateway (0x0018af20) checks the token against "BNET" as a
    // type discriminator, but the actual token on the wire uses the region
    // as prefix. Accept both for compatibility.
    let region_prefixes = ["US-", "EU-", "KR-", "CN-", "BNET"];
    let is_bnet_token = region_prefixes.iter().any(|p| req.token.starts_with(p));
    if !is_bnet_token {
        return Err(OAuthError::invalid_grant(
            "token must be region-prefixed BNET token",
        ));
    }

    // Extract account_id from the token suffix (last segment after final dash).
    let account_id_str = req.token.rsplit('-').next().unwrap_or("0");
    let account_id: i64 = account_id_str.parse().unwrap_or(0);

    // Look up the account to populate claims.
    let account = realmforge_gate_db::accounts::find_by_id(&state.pool, account_id)
        .await
        .ok()
        .flatten();

    let now = chrono::Utc::now().timestamp();
    let claims = realmforge_gate_core::jwt::Claims {
        sub: account_id_str.to_string(),
        iss: state.issuer.clone(),
        iat: now,
        exp: now + 7776000, // ~90 days, matches real server
        jti: format!("KR{}", uuid::Uuid::new_v4().to_string().replace('-', "")),
        scope: req.scope.clone(),
        client_id: req.client_id.clone(),
        battle_tag: account
            .as_ref()
            .map(|a| a.battletag.clone())
            .unwrap_or_default(),
        country_code: account
            .as_ref()
            .map(|a| a.country_code.clone())
            .unwrap_or_default(),
        account_identifier: account
            .as_ref()
            .map(|a| a.email.clone())
            .unwrap_or_default(),
        first_name: account.as_ref().and_then(|a| a.first_name.clone()),
        last_name: account.as_ref().and_then(|a| a.last_name.clone()),
        birth_date: account.as_ref().and_then(|a| a.birth_date.clone()),
        mobile_number: None,
        country_id: account.as_ref().and_then(|a| a.country_id),
        verified_email_address_flag: None,
        employee_flag: None,
        aud: None,
        azp: None,
        nonce: None,
        at_hash: None,
        client_roles: Some(vec![
            "ROLE_FIRST_PARTY_CLIENT".into(),
            "ROLE_CLIENT".into(),
            "ROLE_SERVICE_SSO_CLIENT".into(),
            "ROLE_GAME_CLIENT".into(),
        ]),
        account_roles: Some(vec!["IS_AUTHENTICATED_FULLY".into(), "ROLE_USER".into()]),
        authorities: Some(vec![
            "ROLE_FIRST_PARTY_CLIENT".into(),
            "ROLE_CLIENT".into(),
            "ROLE_SERVICE_SSO_CLIENT".into(),
            "ROLE_GAME_CLIENT".into(),
        ]),
        programs: Some(vec!["ALL".into()]),
        env: Some("prod".into()),
        active: Some(true),
        account_authorities: Some(vec![]),
        client_authorities: Some(vec![format!(
            r#"{{"scope":"{}","authorities":["distributionchannel.*"]}}"#,
            req.scope
        )]),
        account_id: Some(account_id),
        username: Some(account_id_str.to_string()),
    };
    let access_token = jwt::sign(&claims, state.signing_keys.active())?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "bearer".to_string(),
        expires_in: 3600,
        refresh_token: None,
        id_token: None,
        scope: req.scope,
    }))
}

async fn token(
    State(state): State<Arc<OAuthState>>,
    headers: HeaderMap,
    Form(req): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, OAuthError> {
    // Resolve client authentication (Basic auth or form-encoded client_id).
    // For confidential clients (Phoenix), validate the secret. For public
    // clients (account-settings browser), allow auth_method=none.
    let authenticated_client = resolve_client_auth(&state, &headers, &req).await?;

    match req.grant_type.as_str() {
        "authorization_code" => handle_auth_code_grant(&state, req).await,
        "refresh_token" => handle_refresh_grant(&state, req).await,
        "client_credentials" => {
            // client_credentials requires a confidential client (has a secret).
            if authenticated_client.client_secret_hash.is_none() {
                return Err(OAuthError::invalid_client(
                    "client_credentials requires client authentication",
                ));
            }
            handle_client_credentials_grant(&state, &authenticated_client.client_id, req).await
        }
        "urn:ietf:params:oauth:grant-type:token-exchange" => {
            // token-exchange requires client authentication (Phoenix Basic auth).
            if authenticated_client.client_secret_hash.is_none() {
                return Err(OAuthError::invalid_client(
                    "token-exchange requires client authentication",
                ));
            }
            handle_token_exchange(&state, &authenticated_client.client_id, req).await
        }
        "device_code" | "urn:ietf:params:oauth:grant-type:device_code" => {
            handle_device_code_grant(&state, req).await
        }
        other => Err(OAuthError::unsupported_grant_type(format!(
            "grant type '{other}' is not supported"
        ))),
    }
}

async fn handle_auth_code_grant(
    state: &OAuthState,
    req: TokenRequest,
) -> Result<Json<TokenResponse>, OAuthError> {
    let code = req
        .code
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing 'code' parameter"))?;

    let auth_code = realmforge_gate_db::authorization_codes::consume(&state.pool, code)
        .await?
        .ok_or_else(|| OAuthError::invalid_grant("authorization code is invalid or expired"))?;

    let client_id = req
        .client_id
        .as_deref()
        .unwrap_or(auth_code.client_id.as_str());

    // PKCE verification (RFC 7636).
    if let Some(ref challenge) = auth_code.code_challenge {
        let verifier = req
            .code_verifier
            .as_deref()
            .ok_or_else(|| OAuthError::invalid_grant("missing 'code_verifier' for PKCE"))?;

        // S256: BASE64URL(SHA256(verifier)) == challenge
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let computed = base64_url_encode(&hasher.finalize());

        if computed != *challenge {
            return Err(OAuthError::invalid_grant("PKCE verification failed"));
        }
    }

    let scope = req.scope.as_deref().unwrap_or(auth_code.scope.as_str());
    let tokens = issue_tokens(state, auth_code.account_id, client_id, scope).await?;
    Ok(Json(tokens))
}

async fn handle_device_code_grant(
    state: &OAuthState,
    req: TokenRequest,
) -> Result<Json<TokenResponse>, OAuthError> {
    let device_code = req
        .device_code
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_grant("missing device_code"))?;

    let auth =
        realmforge_gate_db::device_authorizations::find_by_device_code(&state.pool, device_code)
            .await?
            .ok_or_else(|| OAuthError::invalid_grant("unknown or expired device_code"))?;

    if !auth.approved {
        return Err(OAuthError::authorization_pending());
    }

    let account_id = auth
        .account_id
        .ok_or_else(|| OAuthError::invalid_grant("not yet authorized"))?;

    let _account = realmforge_gate_db::accounts::find_by_id(&state.pool, account_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_grant("account not found"))?;

    let client_id = auth.client_id.clone();
    let scope = auth.scope.clone();

    // Remove the used authorization.
    realmforge_gate_db::device_authorizations::delete(&state.pool, device_code).await?;

    let tokens = issue_tokens(state, account_id, &client_id, &scope).await?;
    Ok(Json(tokens))
}

async fn handle_refresh_grant(
    state: &OAuthState,
    req: TokenRequest,
) -> Result<Json<TokenResponse>, OAuthError> {
    let rt = req
        .refresh_token
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing 'refresh_token' parameter"))?;

    let stored = realmforge_gate_db::refresh_tokens::find(&state.pool, rt)
        .await?
        .ok_or_else(|| OAuthError::invalid_grant("refresh token is invalid"))?;

    // Rotate: revoke the old token.
    realmforge_gate_db::refresh_tokens::revoke(&state.pool, rt).await?;

    let tokens = issue_tokens(state, stored.account_id, &stored.client_id, &stored.scope).await?;
    Ok(Json(tokens))
}

async fn handle_client_credentials_grant(
    state: &OAuthState,
    resolved_client_id: &str,
    req: TokenRequest,
) -> Result<Json<TokenResponse>, OAuthError> {
    let client_id = resolved_client_id;

    let client = realmforge_gate_db::oauth_clients::find_by_id(&state.pool, client_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_client("unknown client_id"))?;

    if !client
        .allowed_grants
        .contains(&"client_credentials".to_string())
    {
        return Err(OAuthError::unsupported_grant_type(
            "client is not authorized for the client_credentials grant",
        ));
    }

    let scope = req.scope.as_deref().unwrap_or("client");

    // Client credentials grant: no user, no refresh token. Issue a machine
    // access token with sub = client_id.
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: client_id.to_string(),
        iss: state.issuer.clone(),
        iat: now,
        exp: now + 86_400,
        jti: format!("tok-{}", uuid::Uuid::new_v4().simple()),
        scope: scope.to_string(),
        client_id: client_id.to_string(),
        battle_tag: String::new(),
        country_code: String::new(),
        account_identifier: String::new(),
        first_name: None,
        last_name: None,
        birth_date: None,
        mobile_number: None,
        country_id: None,
        verified_email_address_flag: None,
        employee_flag: None,
        aud: None,
        azp: None,
        nonce: None,
        at_hash: None,
        client_roles: Some(vec!["ROLE_CLIENT".to_string()]),
        account_roles: Some(vec![]),
        authorities: Some(vec!["ROLE_CLIENT".to_string()]),
        programs: Some(vec![]),
        env: Some("prod".to_string()),
        active: Some(true),
        account_authorities: Some(vec![]),
        client_authorities: Some(vec![]),
        account_id: None,
        username: None,
    };
    let access_token = jwt::sign(&claims, state.signing_keys.active())?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "bearer".to_string(),
        expires_in: 86_399,
        refresh_token: None,
        id_token: None,
        scope: scope.to_string(),
    }))
}

async fn handle_token_exchange(
    state: &OAuthState,
    resolved_client_id: &str,
    req: TokenRequest,
) -> Result<Json<TokenResponse>, OAuthError> {
    let subject_token = req
        .subject_token
        .as_deref()
        .ok_or_else(|| OAuthError::invalid_request("missing 'subject_token'"))?;

    // RFC 8693: exchange a BGS JWT (subject_token) for a DPLT (access token).
    // Validate requested_token_type per spec: only DPLT is accepted.
    if req
        .requested_token_type
        .as_deref()
        .is_some_and(|rtt| rtt != "urn:blizzard:params:oauth:token-type:dplt")
    {
        return Err(OAuthError::invalid_request(format!(
            "unsupported requested_token_type: {}",
            req.requested_token_type.as_deref().unwrap_or(""),
        )));
    }

    let jwks = state.signing_keys.jwks();
    let claims = jwt::verify(subject_token, &state.issuer, &jwks).map_err(|e| {
        OAuthError::invalid_grant(format!("subject_token verification failed: {e}"))
    })?;

    let client_id = resolved_client_id;
    let scope = req.scope.as_deref().unwrap_or("");

    // The subject_token's sub may be a numeric account id (user token) or a
    // client_id string (M2M token from client_credentials). Handle both:
    // if it parses as i64, issue user tokens; otherwise issue a client token.
    if let Ok(account_id) = claims.sub.parse::<i64>() {
        let tokens = issue_tokens(state, account_id, client_id, scope).await?;
        Ok(Json(tokens))
    } else {
        // M2M subject token (e.g., from client_credentials): issue a client
        // access token without refresh/id tokens.
        let now = chrono::Utc::now().timestamp();
        let new_claims = Claims {
            sub: claims.sub.clone(),
            iss: state.issuer.clone(),
            iat: now,
            exp: now + 86_400,
            jti: format!("tok-{}", uuid::Uuid::new_v4().simple()),
            scope: scope.to_string(),
            client_id: client_id.to_string(),
            battle_tag: String::new(),
            country_code: String::new(),
            account_identifier: claims.account_identifier.clone(),
            first_name: None,
            last_name: None,
            birth_date: None,
            mobile_number: None,
            country_id: None,
            verified_email_address_flag: None,
            employee_flag: None,
            aud: None,
            azp: None,
            nonce: None,
            at_hash: None,
            client_roles: Some(vec!["ROLE_CLIENT".to_string()]),
            account_roles: Some(vec![]),
            authorities: Some(vec!["ROLE_CLIENT".to_string()]),
            programs: Some(vec![]),
            env: Some("prod".to_string()),
            active: Some(true),
            account_authorities: Some(vec![]),
            client_authorities: Some(vec![]),
            account_id: None,
            username: None,
        };
        let access_token = jwt::sign(&new_claims, state.signing_keys.active())?;
        Ok(Json(TokenResponse {
            access_token,
            token_type: "bearer".to_string(),
            expires_in: 86_399,
            refresh_token: None,
            id_token: None,
            scope: scope.to_string(),
        }))
    }
}

/// Issue access + refresh + ID tokens for an account.
async fn issue_tokens(
    state: &OAuthState,
    account_id: i64,
    client_id: &str,
    scope: &str,
) -> Result<TokenResponse, OAuthError> {
    let account = realmforge_gate_db::accounts::find_by_id(&state.pool, account_id)
        .await?
        .ok_or_else(|| OAuthError::server("account not found"))?;

    let now = chrono::Utc::now().timestamp();

    // Access token: no aud, no azp (per capture — these appear only in ID tokens).
    let access_claims = Claims {
        sub: account.id.to_string(),
        iss: state.issuer.clone(),
        iat: now,
        exp: now + 86_400,
        jti: format!("tok-{}", uuid::Uuid::new_v4().simple()),
        scope: scope.to_string(),
        client_id: client_id.to_string(),
        battle_tag: account.battletag.clone(),
        country_code: account.country_code.clone(),
        account_identifier: account.email.to_uppercase(),
        first_name: account.first_name.clone(),
        last_name: account.last_name.clone(),
        birth_date: account.birth_date.clone(),
        mobile_number: account.mobile_number.clone(),
        country_id: account.country_id,
        verified_email_address_flag: Some(account.email_verified),
        employee_flag: Some(false),
        aud: None,
        azp: None,
        nonce: None,
        at_hash: None,
        client_roles: Some(vec!["ROLE_USER".to_string()]),
        account_roles: Some(vec![
            "IS_AUTHENTICATED_FULLY".to_string(),
            "ROLE_USER".to_string(),
        ]),
        authorities: Some(vec![
            "IS_AUTHENTICATED_FULLY".to_string(),
            "ROLE_USER".to_string(),
        ]),
        programs: Some(vec!["ALL".to_string()]),
        env: Some("prod".to_string()),
        active: Some(true),
        account_authorities: Some(vec![]),
        client_authorities: Some(vec![]),
        account_id: None,
        username: None,
    };

    let access_token = jwt::sign(&access_claims, state.signing_keys.active())?;

    // Refresh token (not issued for client_credentials).
    let rt = format!("rt-{}", uuid::Uuid::new_v4().simple());
    let refresh = realmforge_gate_core::RefreshToken {
        token: rt.clone(),
        account_id,
        client_id: client_id.to_string(),
        scope: scope.to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::days(30),
        rotated_from: None,
    };
    realmforge_gate_db::refresh_tokens::insert(&state.pool, &refresh).await?;

    // ID token (OIDC Core 1.0): aud and azp are set here, not in the access token.
    let id_claims = Claims {
        aud: Some(client_id.to_string()),
        azp: Some(client_id.to_string()),
        ..access_claims.clone()
    };
    let id_token = jwt::sign(&id_claims, state.signing_keys.active())?;

    Ok(TokenResponse {
        access_token,
        token_type: "bearer".to_string(),
        expires_in: 86_399,
        refresh_token: Some(rt),
        id_token: Some(id_token),
        scope: scope.to_string(),
    })
}

// --- UserInfo (OIDC Core 1.0 §5.3) ---

async fn userinfo(
    State(state): State<Arc<OAuthState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, OAuthError> {
    let token = extract_bearer(&headers)?;
    let jwks = state.signing_keys.jwks();
    let claims = jwt::verify(&token, &state.issuer, &jwks)?;

    let account_id: i64 = claims.sub.parse().map_err(|_| {
        // Capture: userinfo returns 403 for M2M tokens (client_credentials).
        OAuthError {
            code: "insufficient_scope",
            description: "token does not have user identity".into(),
            status: StatusCode::FORBIDDEN,
        }
    })?;
    let account = realmforge_gate_db::accounts::find_by_id(&state.pool, account_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_token("account not found"))?;

    Ok(Json(serde_json::json!({
        "sub": account.id.to_string(),
        "id": account.id,
        "battletag": account.battletag,
        "first_name": account.first_name,
        "last_name": account.last_name,
        "email": account.email.to_uppercase(),
        "mobile_number": account.mobile_number,
        "account_identifier": account.email.to_uppercase(),
        "country_id": account.country_id,
        "birth_date": account.birth_date,
        "country_code": account.country_code,
        "employee_flag": false,
        "verified_email_address_flag": account.email_verified,
    })))
}

// --- Revocation (RFC 7009) ---

#[derive(Deserialize)]
struct RevokeRequest {
    token: String,
    #[serde(default)]
    #[allow(dead_code)]
    token_type_hint: Option<String>,
}

/// RFC 7009: the server always returns 200, even if the token was already
/// invalid, to prevent information leakage.
async fn revoke(
    State(state): State<Arc<OAuthState>>,
    Form(req): Form<RevokeRequest>,
) -> impl IntoResponse {
    let _ = realmforge_gate_db::refresh_tokens::revoke(&state.pool, &req.token).await;
    StatusCode::OK
}

// --- Logout (OIDC Session Management) ---

/// `GET /logout` — end-session endpoint. Clears the SESSIONID cookie and
/// returns 200. The discovery document advertises this at
/// `end_session_endpoint`.
async fn logout() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::SET_COOKIE,
            "SESSIONID=; Path=/; Max-Age=0; Secure; HttpOnly; SameSite=None",
        )],
        "Logged out",
    )
}

// --- Introspection (RFC 7662) ---

#[derive(Deserialize)]
struct IntrospectRequest {
    token: String,
}

#[derive(Serialize)]
struct IntrospectionResponse {
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<i64>,
}

async fn introspect(
    State(state): State<Arc<OAuthState>>,
    Form(req): Form<IntrospectRequest>,
) -> Result<Json<IntrospectionResponse>, OAuthError> {
    let jwks = state.signing_keys.jwks();
    if let Ok(claims) = jwt::verify(&req.token, &state.issuer, &jwks) {
        return Ok(Json(IntrospectionResponse {
            active: true,
            sub: Some(claims.sub),
            scope: Some(claims.scope),
            client_id: Some(claims.client_id),
            exp: Some(claims.exp),
        }));
    }

    Ok(Json(IntrospectionResponse {
        active: false,
        sub: None,
        scope: None,
        client_id: None,
        exp: None,
    }))
}

// --- Helpers ---

fn extract_bearer(headers: &HeaderMap) -> Result<String, OAuthError> {
    let auth = headers
        .get("authorization")
        .ok_or_else(|| OAuthError::invalid_request("missing Authorization header"))?;
    let auth = auth
        .to_str()
        .map_err(|_| OAuthError::invalid_request("invalid Authorization header"))?;

    auth.strip_prefix("Bearer ")
        .map(|s| s.to_string())
        .ok_or_else(|| OAuthError::invalid_request("expected Bearer token"))
}

/// Resolve client authentication from either Basic auth header or form fields.
/// Returns the client record. For confidential clients, validates the secret.
async fn resolve_client_auth(
    state: &OAuthState,
    headers: &HeaderMap,
    req: &TokenRequest,
) -> Result<realmforge_gate_core::OAuthClient, OAuthError> {
    use base64::Engine;

    // Try Basic auth first (Authorization: Basic <base64(id:secret)>).
    let auth_val = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let (client_id, client_secret) = if let Some(av) = auth_val {
        if av.len() > 6 && av[..6].eq_ignore_ascii_case("Basic ") {
            let decoded = base64::engine::general_purpose::STANDARD
                .decode(&av[6..])
                .map_err(|_| OAuthError::invalid_client("invalid Basic auth encoding"))?;
            let creds = std::str::from_utf8(&decoded)
                .map_err(|_| OAuthError::invalid_client("invalid Basic auth"))?;
            let (id, secret) = creds
                .split_once(':')
                .ok_or_else(|| OAuthError::invalid_client("invalid Basic auth format"))?;
            (id.to_string(), Some(secret.to_string()))
        } else {
            let id = req
                .client_id
                .clone()
                .ok_or_else(|| OAuthError::invalid_client("missing client_id"))?;
            (id, req.client_secret.clone())
        }
    } else {
        let id = req
            .client_id
            .clone()
            .ok_or_else(|| OAuthError::invalid_client("missing client_id"))?;
        (id, req.client_secret.clone())
    };

    let client = if let Some(cached) = state.client_cache.get(&client_id) {
        cached
    } else {
        let db_client = realmforge_gate_db::oauth_clients::find_by_id(&state.pool, &client_id)
            .await?
            .ok_or_else(|| OAuthError::invalid_client("unknown client_id"))?;
        state.client_cache.insert(client_id, db_client.clone());
        db_client
    };

    // If the client has a stored secret hash, validate the provided secret.
    if let Some(ref stored_hash) = client.client_secret_hash {
        let provided = client_secret.unwrap_or_default();
        let computed = hex::encode(sha2::Sha256::digest(provided.as_bytes()));
        if computed != *stored_hash {
            return Err(OAuthError::invalid_client("invalid client secret"));
        }
    }

    Ok(client)
}

/// RFC 7636 §4.2: BASE64URL-ENCODE(SHA256(ASCII(code_verifier))).
fn base64_url_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Extract a cookie value by name from the Cookie header.
fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    for pair in cookie_header.split(';') {
        let pair = pair.trim();
        if let Some((_, v)) = pair.split_once('=').filter(|(k, _)| *k == name) {
            return Some(v.to_string());
        }
    }
    None
}

/// Base64-encode a UUID (standard base64, matching the capture's SESSIONID format).
fn base64_encode_uuid(uuid: Uuid) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(uuid.to_string())
}

/// Decode a SESSIONID value: base64-encoded UUID or plain UUID.
fn decode_session_id(raw: &str) -> Option<Uuid> {
    use base64::Engine;
    if let Ok(uuid) = Uuid::parse_str(raw) {
        return Some(uuid);
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw).ok()?;
    let s = std::str::from_utf8(&bytes).ok()?;
    Uuid::parse_str(s).ok()
}

/// Generate an authorization code in the captured format:
/// `<REGION>STP<34-uppercase-alphanumeric>`. The region is a 2-letter code;
/// the random part produces a 34-character total (matching the capture's
/// `KRSTPFJSTPJKQ6VFFEGAONXJGN94KKM8FA`).
fn format_authorization_code(region: &str) -> String {
    use rand::RngCore;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let random: String = (0..29)
        .map(|_| {
            let mut byte = [0u8; 1];
            rng.fill_bytes(&mut byte);
            CHARSET[(byte[0] as usize) % CHARSET.len()] as char
        })
        .collect();
    format!("{region}STP{random}")
}

// ---------------------------------------------------------------------------
// Device authorization (RFC 8628)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DeviceCodeRequest {
    client_id: String,
    #[serde(default)]
    scope: String,
}

#[derive(Serialize)]
struct DeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: String,
    expires_in: i64,
    interval: i64,
}

/// `POST /device/code` — RFC 8628 device authorization endpoint.
///
/// Creates a pending device authorization. The user visits the
/// verification_uri on a browser, enters the user_code, logs in,
/// and approves. The device polls `/token` with
/// `grant_type=device_code` until authorization completes.
async fn device_code(
    State(state): State<Arc<OAuthState>>,
    Form(req): Form<DeviceCodeRequest>,
) -> Result<Json<DeviceCodeResponse>, OAuthError> {
    // Validate the client exists.
    let _client = realmforge_gate_db::oauth_clients::find_by_id(&state.pool, &req.client_id)
        .await?
        .ok_or_else(|| OAuthError::invalid_client(String::from("Invalid client ID.")))?;

    let device_code = random_code(40);
    let user_code = random_user_code();
    let verification_uri = format!("{}/device", state.account_server_url);

    let auth = realmforge_gate_db::device_authorizations::DeviceAuthorization {
        device_code: device_code.clone(),
        user_code: user_code.clone(),
        client_id: req.client_id,
        account_id: None,
        scope: req.scope,
        expires_at: Utc::now() + Duration::minutes(10),
        approved: false,
    };

    realmforge_gate_db::device_authorizations::insert(&state.pool, &auth).await?;

    let user_code_clone = user_code.clone();
    Ok(Json(DeviceCodeResponse {
        device_code,
        user_code,
        verification_uri: verification_uri.clone(),
        verification_uri_complete: format!("{verification_uri}?user_code={user_code_clone}"),
        expires_in: 600,
        interval: 5,
    }))
}

/// Generate a random alphanumeric device code.
fn random_code(len: usize) -> String {
    use rand::RngCore;
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..len)
        .map(|_| {
            let mut b = [0u8; 1];
            rng.fill_bytes(&mut b);
            CHARS[(b[0] as usize) % CHARS.len()] as char
        })
        .collect()
}

/// Generate a user-friendly 8-character code (e.g. `ABCD-EFGH`).
fn random_user_code() -> String {
    let code = random_code(8).to_uppercase();
    format!("{}-{}", &code[..4], &code[4..])
}

/// `POST /device/approve` — approve a pending device authorization.
///
/// Called by the verification page after the user authenticates.
/// Requires a valid session (SESSIONID cookie).
pub async fn device_approve(
    State(state): State<Arc<OAuthState>>,
    headers: HeaderMap,
    Form(req): Form<DeviceApproveRequest>,
) -> Result<Json<serde_json::Value>, OAuthError> {
    // Resolve the account from the session cookie.
    let account_id = resolve_session_for_device(&state, &headers).await?;

    let approved = realmforge_gate_db::device_authorizations::approve_by_user_code(
        &state.pool,
        &req.user_code,
        account_id,
    )
    .await?;

    if approved {
        Ok(Json(serde_json::json!({"status": "approved"})))
    } else {
        Err(OAuthError::invalid_grant("invalid or expired user code"))
    }
}

#[derive(Deserialize)]
pub struct DeviceApproveRequest {
    user_code: String,
}

/// Resolve an account from the SESSIONID cookie for device approval.
async fn resolve_session_for_device(
    state: &OAuthState,
    headers: &HeaderMap,
) -> Result<i64, OAuthError> {
    let raw = headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| {
            c.split(';')
                .find(|p| p.trim().starts_with("SESSIONID="))
                .map(|p| p.trim().strip_prefix("SESSIONID=").unwrap_or(""))
        })
        .unwrap_or("");

    if raw.is_empty() {
        return Err(OAuthError::unauthorized("no session"));
    }

    let session_id =
        decode_session_id(raw).ok_or_else(|| OAuthError::unauthorized("invalid session ID"))?;

    let session = realmforge_gate_db::sessions::find_by_id(&state.pool, session_id)
        .await?
        .ok_or_else(|| OAuthError::unauthorized("session not found"))?;

    Ok(session.account_id)
}
