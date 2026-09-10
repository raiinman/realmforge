// SPDX-License-Identifier: AGPL-3.0-only

//! Integration tests for the account creation flow (M15).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Method, Request, StatusCode};
use http_body_util::BodyExt;
use realmforge_gate_account::AppState;

fn db_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

async fn setup() -> Option<(sqlx::PgPool, Arc<AppState>)> {
    let url = db_url()?;
    let pool = realmforge_gate_db::connect(&url).await.expect("connect");
    realmforge_gate_db::run_migrations(&pool)
        .await
        .expect("migrate");
    let state = realmforge_gate_account::app_state(pool.clone(), String::new());
    Some((pool, state))
}

async fn send_json(
    app: &axum::Router,
    method: Method,
    path: &str,
    body: serde_json::Value,
    headers: Option<HeaderMap>,
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(ref hdrs) = headers {
        for (name, val) in hdrs.iter() {
            builder = builder.header(name, val);
        }
    }
    let request = builder.body(Body::from(body.to_string())).unwrap();
    let resp = tower::ServiceExt::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, resp_headers, json)
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find(|c| c.starts_with(name))
        .map(|c| {
            c.split(';')
                .next()
                .unwrap_or("")
                .strip_prefix(&format!("{name}="))
                .unwrap_or("")
                .to_string()
        })
}

/// Post a creation step as `multipart/form-data` (the wire format the
/// handlers expect) and return the new CSRF token. The JSON value's
/// top-level fields become form fields; values are coerced to strings.
async fn post_step(
    app: &axum::Router,
    step: &str,
    csrf: &str,
    extra: serde_json::Value,
    headers: &HeaderMap,
) -> String {
    let mut body = extra;
    body["_csrf"] = serde_json::json!(csrf);
    let (status, _, resp) = send_multipart(
        app,
        Method::POST,
        &format!("/creation/flow/creation-full/step/{step}"),
        &body,
        Some(headers.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "step {step} failed: {resp}");
    resp["csrfToken"].as_str().unwrap().to_string()
}

/// Build a `multipart/form-data` body from a JSON object (string values) and
/// POST it. Returns status, headers, and the parsed JSON response body.
async fn send_multipart(
    app: &axum::Router,
    method: Method,
    path: &str,
    body: &serde_json::Value,
    headers: Option<HeaderMap>,
) -> (StatusCode, HeaderMap, serde_json::Value) {
    let boundary = "realmforge-test-boundary";
    let mut payload = String::new();
    if let serde_json::Value::Object(map) = body {
        for (name, value) in map {
            let text = match value {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            payload.push_str(&format!(
                "--{boundary}\r
Content-Disposition: form-data; name=\"{name}\"\r
\r
{text}\r
"
            ));
        }
    }
    payload.push_str(&format!(
        "--{boundary}--\r
"
    ));

    let mut builder = Request::builder().method(method).uri(path).header(
        "content-type",
        format!("multipart/form-data; boundary={boundary}"),
    );
    if let Some(ref hdrs) = headers {
        for (name, val) in hdrs.iter() {
            builder = builder.header(name, val);
        }
    }
    let request = builder.body(Body::from(payload)).unwrap();
    let resp = tower::ServiceExt::oneshot(app.clone(), request)
        .await
        .unwrap();
    let status = resp.status();
    let resp_headers = resp.headers().clone();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, resp_headers, json)
}

#[tokio::test]
async fn creation_flow_full_sequence() {
    let Some((pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let email = format!("m15-create-{}@example.com", uuid::Uuid::new_v4().simple());

    let _ = sqlx::query("DELETE FROM accounts WHERE email = $1")
        .bind(&email)
        .execute(&pool)
        .await;

    let app = realmforge_gate_account::router(state.clone());

    // Step 0: get creation page → session cookie + initial CSRF.
    let (status, resp_headers, resp) = send_json(
        &app,
        Method::GET,
        "/creation/api/init",
        serde_json::Value::Null,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let session_id = extract_cookie(&resp_headers, "creation-session").unwrap();
    let mut csrf = resp["csrfToken"].as_str().unwrap().to_string();

    let mut headers = HeaderMap::new();
    headers.insert(
        "cookie",
        format!("creation-session={session_id}").parse().unwrap(),
    );

    // Step 1: get-started
    csrf = post_step(
        &app,
        "get-started",
        &csrf,
        serde_json::json!({"country":"THA","dob-year":"1990","dob-month":"01","dob-day":"15"}),
        &headers,
    )
    .await;

    // Step 2: provide-name
    csrf = post_step(
        &app,
        "provide-name",
        &csrf,
        serde_json::json!({"firstName":"Test","lastName":"User"}),
        &headers,
    )
    .await;

    // Step 3: provide-credentials
    csrf = post_step(
        &app,
        "provide-credentials",
        &csrf,
        serde_json::json!({"email": email, "phone-number": "0812345678"}),
        &headers,
    )
    .await;

    // Step 4: sms-captcha-gate (stubbed)
    csrf = post_step(
        &app,
        "sms-captcha-gate",
        &csrf,
        serde_json::json!({}),
        &headers,
    )
    .await;

    // Step 5: phone-number-verification (stubbed)
    csrf = post_step(
        &app,
        "phone-number-verification",
        &csrf,
        serde_json::json!({"sms-verification-code": "123456"}),
        &headers,
    )
    .await;

    // Step 6: legal-and-opt-ins
    csrf = post_step(
        &app,
        "legal-and-opt-ins",
        &csrf,
        serde_json::json!({"tou-agreements-implicit":"uuid-123;1","opt-in-blizzard-news-special-offers":"true"}),
        &headers,
    )
    .await;

    // Step 7: set-password — account is created here.
    let (status, _, resp) = send_multipart(
        &app,
        Method::POST,
        "/creation/flow/creation-full/step/set-password",
        &serde_json::json!({"_csrf": csrf, "password": "test-password-456"}),
        Some(headers.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let account_id = resp["accountId"]
        .as_i64()
        .expect("accountId must be returned");
    assert!(account_id > 0);
    csrf = resp["csrfToken"].as_str().unwrap().to_string();

    // Verify both credential schemes stored.
    let cred = realmforge_gate_db::credentials::find_by_account_id(&pool, account_id)
        .await
        .expect("find credentials")
        .expect("credentials must exist");
    assert!(!cred.srp_verifier.is_empty());
    assert!(cred.sha_pass_hash.is_some());

    // Verify game account created. Uses runtime `sqlx::query` (not the
    // `query!` macro) so this verification query does not require an entry
    // in the `.sqlx/` offline cache to compile.
    let rows = sqlx::query("SELECT name FROM game_accounts WHERE account_id = $1")
        .bind(account_id)
        .fetch_all(&pool)
        .await
        .expect("query game accounts");
    assert!(!rows.is_empty());

    // Verify profile fields.
    let account = realmforge_gate_db::accounts::find_by_id(&pool, account_id)
        .await
        .expect("find account")
        .expect("account exists");
    assert_eq!(account.first_name.as_deref(), Some("Test"));
    assert_eq!(account.last_name.as_deref(), Some("User"));
    assert_eq!(account.country_code, "THA");

    // Step 8: set-battletag
    let (status, _, resp) = send_multipart(
        &app,
        Method::POST,
        "/creation/flow/creation-full/step/set-battletag",
        &serde_json::json!({"_csrf": csrf, "battletag": "TestUser#1234"}),
        Some(headers),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["success"], true);

    // Verify battletag updated.
    let account = realmforge_gate_db::accounts::find_by_id(&pool, account_id)
        .await
        .expect("find account")
        .expect("account exists");
    assert!(account.battletag.contains("TestUser"));

    // Clean up.
    let _ = sqlx::query("DELETE FROM credentials WHERE account_id = $1")
        .bind(account_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM game_accounts WHERE account_id = $1")
        .bind(account_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account_id)
        .execute(&pool)
        .await;
}

#[tokio::test]
async fn battletag_suggestion_returns_valid() {
    let Some((_pool, state)) = setup().await else {
        eprintln!("skipping: DATABASE_URL not set");
        return;
    };
    let app = realmforge_gate_account::router(state);
    let (status, _, resp) = send_json(
        &app,
        Method::GET,
        "/creation/api/battletag-suggestion",
        serde_json::Value::Null,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tag = resp["battletag"].as_str().unwrap();
    assert!(tag.starts_with("Player"));
    assert!(!resp["suggestions"].as_array().unwrap().is_empty());
}
