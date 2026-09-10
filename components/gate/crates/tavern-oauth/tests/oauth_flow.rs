// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the OAuth OIDC provider (M9).
//!
//! Requires `DATABASE_URL`. Tests: discovery, authorize → token round-trip,
//! refresh-token rotation, userinfo, PKCE, and the token-exchange grant.
//! Uses RFC 6749-compliant form-encoded requests.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use tavern_oauth::{oauth_state, router};

const TEST_KEY_PEM: &str = include_str!("../../tavern-core/tests/data/test_key_pkcs8.pem");
const ISSUER: &str = "https://oauth.test";
const CLIENT_ID: &str = "057adb2af62a4d59904f74754838c4c8";
const REDIRECT_URI: &str = "https://account.battle.net/callback/oauth2/code/account-settings";

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

async fn setup() -> Option<Arc<tavern_oauth::OAuthState>> {
    let url = db_url()?;
    let pool = tavern_db::connect(&url).await.expect("connect");
    tavern_db::run_migrations(&pool).await.expect("migrate");
    Some(oauth_state(pool, ISSUER.to_string(), TEST_KEY_PEM).expect("oauth_state"))
}

async fn register_account(pool: &sqlx::PgPool, email: &str) -> i64 {
    let group = tavern_core::srp::Group::battle_net_v2();
    let _ = sqlx::query("DELETE FROM accounts WHERE email = $1")
        .bind(email)
        .execute(pool)
        .await;
    tavern_account::registration::register(pool, &group, email, "password123")
        .await
        .expect("register")
}

async fn mint_ticket(pool: &sqlx::PgPool, account_id: i64) -> String {
    tavern_account::ticket::mint_login_ticket(pool, account_id)
        .await
        .expect("mint ticket")
}

// --- Tests ---

#[tokio::test]
async fn discovery_lists_endpoints() {
    let Some(state) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let app = router(state);
    let resp = send(
        app,
        Method::GET,
        "/.well-known/openid-configuration",
        None,
        None,
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp_json(resp).await;
    assert_eq!(body["issuer"], ISSUER);
    assert!(
        body["grant_types_supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g == "client_credentials")
    );
    assert!(
        body["grant_types_supported"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g == "urn:ietf:params:oauth:grant-type:token-exchange")
    );
    assert_eq!(body["id_token_signing_alg_values_supported"][0], "RS256");
    assert_eq!(body["code_challenge_methods_supported"][0], "S256");
}

#[tokio::test]
async fn authorize_token_round_trip() {
    let Some(state) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("oauth");
    let account_id = register_account(&state.pool, &email).await;
    let st = mint_ticket(&state.pool, account_id).await;

    // 1. Authorize: consume the ticket, get a code via two-hop redirect.
    let code = authorize_two_hop(
        &state,
        &format!(
            "/authorize?client_id={CLIENT_ID}&redirect_uri={REDIRECT_URI}&response_type=code&scope=openid&ST={st}"
        ),
    )
    .await;

    // 2. Exchange the code for tokens (form-encoded per RFC 6749).
    let app = router(state.clone());
    let resp = send_form(
        app,
        Method::POST,
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("client_id", CLIENT_ID),
            ("scope", "openid"),
        ],
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let tokens: serde_json::Value = resp_json(resp).await;
    let access_token = tokens["access_token"].as_str().unwrap();
    assert!(!access_token.is_empty());
    assert!(!tokens["refresh_token"].as_str().unwrap().is_empty());
    assert!(!tokens["id_token"].as_str().unwrap().is_empty());
    assert_eq!(tokens["token_type"], "bearer");

    // 3. Call userinfo with the access token.
    let app = router(state.clone());
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {access_token}").parse().unwrap(),
    );
    let resp = send(app, Method::GET, "/userinfo", None, Some(headers)).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let user: serde_json::Value = resp_json(resp).await;
    assert_eq!(user["sub"], account_id.to_string());
}

#[tokio::test]
async fn refresh_token_rotates() {
    let Some(state) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("refresh");
    let account_id = register_account(&state.pool, &email).await;
    let st = mint_ticket(&state.pool, account_id).await;

    // Authorize + token to get a refresh token (two-hop redirect).
    let code = authorize_two_hop(
        &state,
        &format!(
            "/authorize?client_id={CLIENT_ID}&redirect_uri={REDIRECT_URI}&response_type=code&scope=openid&ST={st}"
        ),
    )
    .await;

    let app = router(state.clone());
    let resp = send_form(
        app,
        Method::POST,
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("client_id", CLIENT_ID),
        ],
    )
    .await;
    let tokens: serde_json::Value = resp_json(resp).await;
    let rt1 = tokens["refresh_token"].as_str().unwrap().to_string();

    // Use the refresh token.
    let app = router(state.clone());
    let resp = send_form(
        app,
        Method::POST,
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", CLIENT_ID),
        ],
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);
    let tokens2: serde_json::Value = resp_json(resp).await;
    let rt2 = tokens2["refresh_token"].as_str().unwrap().to_string();
    assert_ne!(rt1, rt2, "refresh token must rotate");

    // Old refresh token is now invalid.
    let app = router(state.clone());
    let resp = send_form(
        app,
        Method::POST,
        "/token",
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", &rt1),
            ("client_id", CLIENT_ID),
        ],
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "old refresh token must be revoked after rotation"
    );
}

#[tokio::test]
async fn token_exchange_grant() {
    let Some(state) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("exchange");
    let account_id = register_account(&state.pool, &email).await;

    // Issue a JWT (simulating a BGS JWT from the auth session).
    let now = chrono::Utc::now().timestamp();
    let claims = tavern_core::jwt::Claims {
        sub: account_id.to_string(),
        iss: ISSUER.to_string(),
        iat: now,
        exp: now + 3600,
        jti: "test-bgs-jwt".to_string(),
        scope: "account.standard".to_string(),
        client_id: "a7f9b73e4e9c4e01a9c8056cddabff71".to_string(),
        battle_tag: "Test#1234".to_string(),
        country_code: "USA".to_string(),
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
    };
    let bgs_jwt = tavern_core::jwt::sign(&claims, state.signing_keys.active()).expect("sign jwt");

    // Exchange it for a DPLT (access token).
    let app = router(state.clone());
    // Phoenix client requires Basic auth with its known secret.
    let basic_auth =
        base64_encode("a7f9b73e4e9c4e01a9c8056cddabff71:2Lu0QbF5FLrQoLmIDjcPjojlEu4zirF2");
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Basic {basic_auth}").parse().unwrap(),
    );
    let resp = send_form_with_headers(
        app,
        Method::POST,
        "/token",
        &[
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("subject_token", &bgs_jwt),
            ("subject_token_type", "urn:ietf:params:oauth:token-type:jwt"),
            ("scope", "account.standard"),
        ],
        headers,
    )
    .await;
    if resp.status() != StatusCode::OK {
        let body: serde_json::Value = resp_json(resp).await;
        panic!("token exchange failed: {body}");
    }
    let tokens: serde_json::Value = resp_json(resp).await;
    let dplt = tokens["access_token"].as_str().unwrap();
    assert!(!dplt.is_empty(), "must get a DPLT access token");
    assert_eq!(tokens["token_type"], "bearer");
}

#[tokio::test]
async fn pkce_flow() {
    let Some(state) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = unique("pkce");
    let account_id = register_account(&state.pool, &email).await;
    let st = mint_ticket(&state.pool, account_id).await;

    // Generate PKCE verifier + challenge (S256).
    let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let challenge = {
        use base64::Engine;
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(verifier.as_bytes());
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
    };

    // Authorize with PKCE challenge (two-hop redirect).
    let code = authorize_two_hop(
        &state,
        &format!(
            "/authorize?client_id={CLIENT_ID}&redirect_uri={REDIRECT_URI}&response_type=code&scope=openid&ST={st}&code_challenge={challenge}&code_challenge_method=S256"
        ),
    )
    .await;

    // Token exchange with correct verifier.
    let app = router(state.clone());
    let resp = send_form(
        app,
        Method::POST,
        "/token",
        &[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("client_id", CLIENT_ID),
            ("code_verifier", verifier),
        ],
    )
    .await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "PKCE flow must succeed with correct verifier"
    );
    let tokens: serde_json::Value = resp_json(resp).await;
    assert!(!tokens["access_token"].as_str().unwrap().is_empty());
}

// --- helpers ---

fn unique(tag: &str) -> String {
    format!("m9-{tag}-{}@example.com", uuid::Uuid::new_v4().simple())
}

/// Execute the two-hop authorize redirect matching the capture (A.5–A.6):
/// 1. GET /authorize?...&ST=... → 302, sets opt cookie
/// 2. GET /authorize?... (no ST, with opt cookie) → 303 with code
async fn authorize_two_hop(state: &Arc<tavern_oauth::OAuthState>, url: &str) -> String {
    let app = router(state.clone());
    let resp = send(app, Method::GET, url, None, None).await;
    assert_eq!(
        resp.status(),
        StatusCode::FOUND,
        "authorize first hop must return 302"
    );
    let opt_cookie = extract_set_cookie(&resp, "opt");
    assert!(!opt_cookie.is_empty(), "first hop must set opt cookie");
    let redirect_url = resp
        .headers()
        .get("location")
        .expect("first hop must have Location")
        .to_str()
        .unwrap()
        .to_string();

    let app = router(state.clone());
    let mut hop2_headers = HeaderMap::new();
    hop2_headers.insert("cookie", opt_cookie.parse().unwrap());
    let resp = send(app, Method::GET, &redirect_url, None, Some(hop2_headers)).await;
    assert_eq!(
        resp.status(),
        StatusCode::SEE_OTHER,
        "authorize second hop must return 303"
    );
    let location = resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let code = extract_query_param(&location, "code");
    assert!(!code.is_empty(), "must get an authorization code");
    code
}

/// Extract a Set-Cookie value by cookie name from the response headers.
fn extract_set_cookie(resp: &axum::response::Response, name: &str) -> String {
    let cookies: Vec<&str> = resp
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .collect();
    for cookie in cookies {
        let value = cookie.split(';').next().unwrap_or("");
        if let Some((_, v)) = value.split_once('=').filter(|(k, _)| *k == name) {
            return format!("{name}={v}");
        }
    }
    String::new()
}

fn extract_query_param(url: &str, param: &str) -> String {
    let query = url.split('?').nth(1).unwrap_or("");
    for pair in query.split('&') {
        let mut kv = pair.splitn(2, '=');
        if kv.next() == Some(param) {
            return kv.next().unwrap_or("").to_string();
        }
    }
    String::new()
}

async fn send(
    app: axum::Router,
    method: Method,
    path: &str,
    body: Option<String>,
    headers: Option<HeaderMap>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(ref hdrs) = headers {
        for (name, val) in hdrs.iter() {
            builder = builder.header(name, val);
        }
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/x-www-form-urlencoded");
    }
    let request = builder.body(Body::from(body.unwrap_or_default())).unwrap();
    tower::ServiceExt::oneshot(app, request).await.unwrap()
}

async fn send_form(
    app: axum::Router,
    method: Method,
    path: &str,
    fields: &[(&str, &str)],
) -> axum::response::Response {
    let body = fields
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    send(app, method, path, Some(body), None).await
}

fn url_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '.' | '_' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

fn base64_encode(s: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(s)
}

async fn send_form_with_headers(
    app: axum::Router,
    method: Method,
    path: &str,
    fields: &[(&str, &str)],
    extra_headers: HeaderMap,
) -> axum::response::Response {
    let body = fields
        .iter()
        .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    send(app, method, path, Some(body), Some(extra_headers)).await
}

async fn resp_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
}
